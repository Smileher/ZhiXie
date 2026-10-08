use chrono::{Duration, Local, TimeZone, Timelike};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::Duration as StdDuration;
use tauri::{AppHandle, Manager};

const AGENT_CONFIG_FILE: &str = "agent-config.json";

// 智能体配置单独存放，避免改动主数据结构与数据版本号。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentConfig {
    pub enabled: bool,
    pub provider: String,
    pub endpoint: String,
    pub model: String,
    pub api_key: String,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: "deepseek".into(),
            endpoint: "https://api.deepseek.com/chat/completions".into(),
            model: "deepseek-chat".into(),
            api_key: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AgentDraft {
    pub title: String,
    pub reminder_type: String,
    pub trigger_at: Option<String>,
    pub time: Option<String>,
    pub weekdays: Vec<u8>,
    pub month_days: Vec<u8>,
    pub power_action: Option<String>,
    pub popup_message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentResult {
    pub draft: AgentDraft,
    pub source: String,
    pub note: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentAdvice {
    pub advice: String,
    pub source: String,
}

fn agent_config_path(app: &AppHandle) -> Result<PathBuf, String> {
    let directory = app.path().app_config_dir().map_err(|error| error.to_string())?;
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    Ok(directory.join(AGENT_CONFIG_FILE))
}

#[tauri::command]
pub fn load_agent_config(app: AppHandle) -> Result<AgentConfig, String> {
    let path = agent_config_path(&app)?;
    if !path.exists() {
        return Ok(AgentConfig::default());
    }
    let raw = fs::read_to_string(&path).map_err(|error| error.to_string())?;
    let config: AgentConfig = serde_json::from_str(&raw).unwrap_or_default();
    Ok(normalize_config(config))
}

#[tauri::command]
pub fn save_agent_config(app: AppHandle, config: AgentConfig) -> Result<AgentConfig, String> {
    let normalized = normalize_config(config);
    let path = agent_config_path(&app)?;
    let raw = serde_json::to_string_pretty(&normalized).map_err(|error| error.to_string())?;
    fs::write(&path, raw).map_err(|error| error.to_string())?;
    Ok(normalized)
}

fn normalize_config(config: AgentConfig) -> AgentConfig {
    AgentConfig {
        enabled: config.enabled,
        provider: if config.provider.trim().is_empty() {
            "custom".into()
        } else {
            config.provider.trim().into()
        },
        endpoint: config.endpoint.trim().into(),
        model: config.model.trim().into(),
        api_key: config.api_key.trim().into(),
    }
}

/// 把自然语言解析为提醒草稿。优先调用已配置的大模型接口，失败或无配置时回落到本地规则引擎。
#[tauri::command]
pub async fn agent_parse_reminder(app: AppHandle, text: String) -> Result<AgentResult, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("empty request".into());
    }
    let config = load_agent_config(app.clone())?;
    if config.enabled && !config.api_key.is_empty() && !config.endpoint.is_empty() {
        match parse_with_cloud(&config, trimmed).await {
            Ok(draft) => {
                return Ok(AgentResult {
                    draft,
                    source: "cloud".into(),
                    note: String::new(),
                })
            }
            Err(error) => {
                return Ok(AgentResult {
                    draft: parse_locally(trimmed),
                    source: "local".into(),
                    note: error,
                })
            }
        }
    }
    Ok(AgentResult {
        draft: parse_locally(trimmed),
        source: "local".into(),
        note: String::new(),
    })
}

/// 休息建议。本地规则始终可用，开启云端后改由大模型生成，失败自动回落。
#[tauri::command]
pub async fn agent_break_advice(
    app: AppHandle,
    rest_interval_minutes: u32,
    reminder_count: usize,
    elapsed_minutes: u32,
) -> Result<AgentAdvice, String> {
    let config = load_agent_config(app)?;
    if config.enabled && !config.api_key.is_empty() && !config.endpoint.is_empty() {
        let prompt = format!(
            "用户每 {} 分钟休息一次，今日已触发 {} 次提醒，本次已连续工作 {} 分钟。请给出一句简短、具体、温和的中文休息建议，不超过 30 字，不要客套话。",
            rest_interval_minutes, reminder_count, elapsed_minutes
        );
        if let Ok(advice) = request_text(&config, &prompt).await {
            let advice = advice.trim().to_string();
            if !advice.is_empty() {
                return Ok(AgentAdvice {
                    advice,
                    source: "cloud".into(),
                });
            }
        }
    }
    Ok(AgentAdvice {
        advice: local_advice(rest_interval_minutes, reminder_count, elapsed_minutes),
        source: "local".into(),
    })
}

const SYSTEM_PROMPT: &str = "你是一个提醒解析助手。把用户的自然语言请求解析成 JSON，字段：title(提醒标题，简短), reminderType(once/daily/weekly/monthly), triggerAt(仅 once，RFC3339 本地时间，带时区偏移), time(重复提醒，HH:MM), weekdays(每周，0=周日..6=周六，数字数组), monthDays(每月几号，1-31，数字数组), powerAction(null/lock/shutdown/restart), popupMessage(弹窗正文，简短)。只输出 JSON，不要解释。";

async fn parse_with_cloud(config: &AgentConfig, text: &str) -> Result<AgentDraft, String> {
    let raw = request_text(config, &format!("请解析这条提醒需求：{}", text)).await?;
    let json = extract_json_object(&raw).ok_or_else(|| "no json in response".to_string())?;
    let draft: AgentDraft = serde_json::from_str(json).map_err(|error| error.to_string())?;
    Ok(validate_draft(draft, text))
}

async fn request_text(config: &AgentConfig, prompt: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(StdDuration::from_secs(40))
        .build()
        .map_err(|error| error.to_string())?;
    let payload = serde_json::json!({
        "model": config.model,
        "temperature": 0.2,
        "messages": [
            { "role": "system", "content": SYSTEM_PROMPT },
            { "role": "user", "content": prompt }
        ]
    });
    let response = client
        .post(&config.endpoint)
        .bearer_auth(&config.api_key)
        .json(&payload)
        .send()
        .await
        .map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        return Err(format!("http {}", response.status().as_u16()));
    }
    let body: serde_json::Value = response.json().await.map_err(|error| error.to_string())?;
    body["choices"][0]["message"]["content"]
        .as_str()
        .map(|value| value.to_string())
        .ok_or_else(|| "malformed response".to_string())
}

fn extract_json_object(raw: &str) -> Option<&str> {
    let trimmed = raw.trim();
    let without_fence = trimmed
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    let start = without_fence.find('{')?;
    let end = without_fence.rfind('}')?;
    if end < start {
        return None;
    }
    Some(&without_fence[start..=end])
}

/// 云端结果必须过一遍校验，避免模型返回非法字段写入主数据。
fn validate_draft(draft: AgentDraft, fallback_text: &str) -> AgentDraft {
    let reminder_type = match draft.reminder_type.as_str() {
        "once" | "daily" | "weekly" | "monthly" => draft.reminder_type,
        _ => "once".into(),
    };
    let power_action = match draft.power_action.as_deref() {
        Some("lock") | Some("shutdown") | Some("restart") => draft.power_action,
        _ => None,
    };
    let weekdays = draft
        .weekdays
        .into_iter()
        .filter(|day| *day <= 6)
        .collect::<Vec<u8>>();
    let month_days = draft
        .month_days
        .into_iter()
        .filter(|day| (1..=31).contains(day))
        .collect::<Vec<u8>>();
    let title = if draft.title.trim().is_empty() {
        fallback_text.chars().take(20).collect()
    } else {
        draft.title.trim().into()
    };
    AgentDraft {
        title,
        reminder_type,
        trigger_at: draft.trigger_at.filter(|value| !value.trim().is_empty()),
        time: draft
            .time
            .filter(|value| parse_hhmm(value).is_some())
            .or_else(|| Some("09:00".into())),
        weekdays,
        month_days,
        power_action,
        popup_message: draft.popup_message.trim().into(),
    }
}

// ——— 本地规则引擎 ———

fn parse_locally(text: &str) -> AgentDraft {
    let weekdays = detect_weekdays(text);
    let month_days = detect_month_days(text);
    let power_action = detect_power_action(text);
    let (hour, minute) = time_of_day(text).unwrap_or((9, 0));
    let time = format!("{:02}:{:02}", hour, minute);
    let title = clean_title(text);

    if let Some(days) = weekdays {
        return AgentDraft {
            title,
            reminder_type: "weekly".into(),
            trigger_at: None,
            time: Some(time),
            weekdays: days,
            month_days: vec![],
            power_action,
            popup_message: String::new(),
        };
    }
    if !month_days.is_empty() {
        return AgentDraft {
            title,
            reminder_type: "monthly".into(),
            trigger_at: None,
            time: Some(time),
            weekdays: vec![],
            month_days,
            power_action,
            popup_message: String::new(),
        };
    }
    if has(text, &["每天", "每日", "天天", "every day"]) {
        return AgentDraft {
            title,
            reminder_type: "daily".into(),
            trigger_at: None,
            time: Some(time),
            weekdays: vec![],
            month_days: vec![],
            power_action,
            popup_message: String::new(),
        };
    }

    let trigger_at = relative_trigger(text, hour, minute);
    AgentDraft {
        title,
        reminder_type: "once".into(),
        trigger_at: Some(trigger_at),
        time: None,
        weekdays: vec![],
        month_days: vec![],
        power_action,
        popup_message: String::new(),
    }
}

fn has(text: &str, keys: &[&str]) -> bool {
    keys.iter().any(|key| text.contains(key))
}

fn detect_power_action(text: &str) -> Option<String> {
    if has(text, &["锁屏", "锁定屏幕", "锁上"]) {
        Some("lock".into())
    } else if has(text, &["关机", "关电脑", "关闭电脑"]) {
        Some("shutdown".into())
    } else if has(text, &["重启", "重新启动"]) {
        Some("restart".into())
    } else {
        None
    }
}

fn weekday_value(ch: char) -> Option<u8> {
    match ch {
        '一' => Some(1),
        '二' => Some(2),
        '三' => Some(3),
        '四' => Some(4),
        '五' => Some(5),
        '六' => Some(6),
        '日' | '天' => Some(0),
        _ => None,
    }
}

fn chinese_digit(ch: char) -> Option<u32> {
    match ch {
        '零' | '〇' => Some(0),
        '一' => Some(1),
        '二' | '两' => Some(2),
        '三' => Some(3),
        '四' => Some(4),
        '五' => Some(5),
        '六' => Some(6),
        '七' => Some(7),
        '八' => Some(8),
        '九' => Some(9),
        _ => None,
    }
}

fn is_chinese_numeral(ch: char) -> bool {
    chinese_digit(ch).is_some() || ch == '十'
}

/// 支持“三”“十”“十五”“二十三”这类口语数字。
fn chinese_number(text: &str) -> Option<u32> {
    if text.is_empty() {
        return None;
    }
    if let Some(position) = text.find('十') {
        let high = if position == 0 {
            1
        } else {
            chinese_digit(text.chars().next()?)?
        };
        let low = text[position + '十'.len_utf8()..]
            .chars()
            .next()
            .and_then(chinese_digit)
            .unwrap_or(0);
        return Some(high * 10 + low);
    }
    let mut total = 0u32;
    for ch in text.chars() {
        let digit = chinese_digit(ch)?;
        total = total * 10 + digit;
    }
    Some(total)
}

/// 取字符串末尾的数字，阿拉伯数字与中文数字均可。
fn trailing_number(text: &str) -> Option<u32> {
    if let Some(value) = text
        .chars()
        .rev()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<String>()
        .parse::<u32>()
        .ok()
    {
        return Some(value);
    }
    let chinese: String = text
        .chars()
        .rev()
        .take_while(|ch| is_chinese_numeral(*ch))
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    chinese_number(&chinese)
}

/// 取字符串开头的数字，用于“点半”“一刻”之后的分钟部分。
fn leading_number(text: &str) -> Option<u32> {
    if let Some(value) = text
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>()
        .parse::<u32>()
        .ok()
    {
        return Some(value);
    }
    let chinese: String = text
        .chars()
        .take_while(|ch| is_chinese_numeral(*ch))
        .collect();
    chinese_number(&chinese)
}

fn detect_weekdays(text: &str) -> Option<Vec<u8>> {
    if has(text, &["工作日", "每周一到周五", "周一到周五", "周一至周五"]) {
        return Some(vec![1, 2, 3, 4, 5]);
    }
    if has(text, &["周末", "双休"]) {
        return Some(vec![6, 0]);
    }
    // 扫描“周/期/拜”之后连写的星期数字，可覆盖“每周一三五”“星期天”。
    let chars: Vec<char> = text.chars().collect();
    let mut days = Vec::new();
    for index in 0..chars.len() {
        if !matches!(chars[index], '周' | '期' | '拜') {
            continue;
        }
        let mut position = index + 1;
        while position < chars.len() {
            match weekday_value(chars[position]) {
                Some(day) => {
                    days.push(day);
                    position += 1;
                }
                None => break,
            }
        }
    }
    if days.is_empty() {
        None
    } else {
        days.sort_unstable();
        days.dedup();
        Some(days)
    }
}

fn detect_month_days(text: &str) -> Vec<u8> {
    if !has(text, &["每月", "每个月"]) {
        return Vec::new();
    }
    let mut days = Vec::new();
    let mut cursor = 0;
    while let Some(offset) = text[cursor..].find(|ch: char| ch == '号' || ch == '日') {
        let position = cursor + offset;
        let before = &text[..position];
        let digits: String = before
            .chars()
            .rev()
            .take_while(|ch| ch.is_ascii_digit())
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        if let Ok(day) = digits.parse::<u8>() {
            if (1..=31).contains(&day) {
                days.push(day);
            }
        }
        cursor = position + 1;
    }
    days.sort_unstable();
    days.dedup();
    days
}

fn digits_before(text: &str, unit: &str) -> Option<u32> {
    let position = text.find(unit)?;
    let before = &text[..position];
    let digits: String = before
        .chars()
        .rev()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    digits.parse::<u32>().ok()
}

fn time_of_day(text: &str) -> Option<(u32, u32)> {
    if let Some(position) = text.find(':').or_else(|| text.find('：')) {
        let before = &text[..position];
        let after = &text[position + 1..];
        let hour: String = before
            .chars()
            .rev()
            .take_while(|ch| ch.is_ascii_digit())
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        let minute: String = after
            .chars()
            .take_while(|ch| ch.is_ascii_digit())
            .collect();
        if let (Ok(hour), Ok(minute)) = (hour.parse::<u32>(), minute.parse::<u32>()) {
            if hour <= 23 && minute <= 59 {
                return Some((hour, minute));
            }
        }
    }
    if let Some(position) = text.find('点') {
        let before = &text[..position];
        let after = &text[position + '点'.len_utf8()..];
        if let Some(mut hour) = trailing_number(before) {
            // “下午三点”“晚上十点”需要换算成 24 小时制。
            if hour < 12 && has(text, &["下午", "晚上", "傍晚"]) {
                hour += 12;
            }
            let minute = if after.starts_with('半') {
                30
            } else if after.starts_with("一刻") {
                15
            } else {
                leading_number(after).unwrap_or(0)
            };
            if hour <= 23 && minute <= 59 {
                return Some((hour, minute));
            }
        }
    }
    None
}

fn relative_trigger(text: &str, fallback_hour: u32, fallback_minute: u32) -> String {
    let now = Local::now();
    let zeroed = |value: chrono::DateTime<chrono::Local>| {
        value
            .with_second(0)
            .unwrap()
            .with_nanosecond(0)
            .unwrap()
            .to_rfc3339()
    };

    // “30分钟后”“2小时后”这类相对表达优先于具体时间点。
    if has(text, &["分钟后", "分钟之后"]) {
        if let Some(minutes) = digits_before(text, "分钟") {
            return zeroed(now + Duration::minutes(minutes as i64));
        }
    }
    if has(text, &["小时后", "小时之后"]) {
        if let Some(hours) = digits_before(text, "小时") {
            return zeroed(now + Duration::hours(hours as i64));
        }
    }

    let day_offset = if has(text, &["后天"]) {
        2
    } else if has(text, &["明天", "明日"]) {
        1
    } else {
        0
    };
    let mut target = Local::now().date_naive().and_hms_opt(fallback_hour, fallback_minute, 0).unwrap()
        + Duration::days(day_offset);
    // 该时间点已经过去则顺延一天，避免用户拿到一个立刻过期的提醒。
    if target <= now.naive_local() {
        target += Duration::days(1);
    }
    zeroed(chrono::Local.from_local_datetime(&target).unwrap())
}

fn clean_title(text: &str) -> String {
    let noise = [
        "每天", "每日", "天天", "每周", "每星期", "工作日", "周末", "每月", "今天", "明天", "后天",
        "分钟后", "分钟之后", "小时后", "小时之后", "点", "分", "：", ":", "提醒我", "提醒", "帮我",
        "设置一个", "创建", "添加", "记一下", "上午", "下午", "中午", "晚上", "早上", "凌晨",
    ];
    let mut title = text.to_string();
    for item in noise {
        title = title.replace(item, " ");
    }
    for unit in ["锁屏", "锁定屏幕", "关机", "关电脑", "关闭电脑", "重启", "重新启动"] {
        title = title.replace(unit, " ");
    }
    let collapsed = title.split_whitespace().collect::<Vec<_>>().join(" ");
    let trimmed = collapsed.trim().trim_matches(|ch: char| ch == '，' || ch == ',' || ch == '。').trim();
    if trimmed.is_empty() {
        "提醒".into()
    } else {
        trimmed.chars().take(30).collect()
    }
}

fn parse_hhmm(value: &str) -> Option<(u32, u32)> {
    let parts: Vec<&str> = value.split(':').collect();
    if parts.len() != 2 {
        return None;
    }
    let hour = parts[0].parse::<u32>().ok()?;
    let minute = parts[1].parse::<u32>().ok()?;
    if hour <= 23 && minute <= 59 {
        Some((hour, minute))
    } else {
        None
    }
}

fn local_advice(rest_interval_minutes: u32, reminder_count: usize, elapsed_minutes: u32) -> String {
    if elapsed_minutes >= rest_interval_minutes * 2 {
        return format!(
            "你已经连续工作 {} 分钟，先站起来走几步，喝口水再回来。",
            elapsed_minutes
        );
    }
    if reminder_count >= 6 {
        return "今天提醒已经触发很多次了，考虑收个尾，把剩下的事留到明天。".into();
    }
    if elapsed_minutes >= rest_interval_minutes {
        return "到点了，抬头看看远处，活动一下肩膀和手腕。".into();
    }
    match reminder_count {
        0 => "刚开始工作，先把手头这件事做完再说。".into(),
        1..=2 => "状态还行，记得喝口水。".into(),
        _ => "节奏挺稳的，保持这个间隔就好。".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_parser_understands_daily_time() {
        let draft = parse_locally("每天下午三点提醒我喝水");
        assert_eq!(draft.reminder_type, "daily");
        assert_eq!(draft.time.as_deref(), Some("15:00"));
        assert!(draft.title.contains("喝水"));
    }

    #[test]
    fn local_parser_understands_weekdays() {
        let draft = parse_locally("每周一三五 17:30 提醒我锁屏下班");
        assert_eq!(draft.reminder_type, "weekly");
        assert_eq!(draft.weekdays, vec![1, 3, 5]);
        assert_eq!(draft.power_action.as_deref(), Some("lock"));
    }

    #[test]
    fn local_parser_understands_relative_minutes() {
        let draft = parse_locally("30分钟后提醒我休息一下");
        assert_eq!(draft.reminder_type, "once");
        assert!(draft.trigger_at.is_some());
        assert!(draft.time.is_none());
    }

    #[test]
    fn local_parser_understands_power_action() {
        assert_eq!(
            parse_locally("晚上11点关机").power_action.as_deref(),
            Some("shutdown")
        );
        assert_eq!(
            parse_locally("明早重启电脑").power_action.as_deref(),
            Some("restart")
        );
    }

    #[test]
    fn local_parser_falls_back_to_once_without_keywords() {
        let draft = parse_locally("记得寄快递");
        assert_eq!(draft.reminder_type, "once");
        assert_eq!(draft.title, "记得寄快递");
        assert!(draft.trigger_at.is_some());
    }

    #[test]
    fn cloud_drafts_are_validated() {
        let raw = AgentDraft {
            title: String::new(),
            reminder_type: "weekly?".into(),
            trigger_at: Some(String::new()),
            time: Some("99:99".into()),
            weekdays: vec![9, 10, 2],
            month_days: vec![40, 15],
            power_action: Some("format".into()),
            popup_message: "  hi  ".into(),
        };
        let draft = validate_draft(raw, "提醒我开会");
        assert_eq!(draft.reminder_type, "once");
        assert_eq!(draft.power_action, None);
        assert_eq!(draft.time.as_deref(), Some("09:00"));
        assert_eq!(draft.weekdays, vec![2]);
        assert_eq!(draft.month_days, vec![15]);
        assert_eq!(draft.popup_message, "hi");
        assert_eq!(draft.title, "提醒我开会");
    }

    #[test]
    fn json_extraction_tolerates_code_fences() {
        let content = "```json\n{\"title\": \"x\"}\n```";
        assert_eq!(extract_json_object(content), Some("{\"title\": \"x\"}"));
        assert_eq!(extract_json_object("noise {\"a\":1} tail"), Some("{\"a\":1}"));
    }

    #[test]
    fn local_advice_escalates_with_work_time() {
        assert!(local_advice(40, 0, 5).contains("刚开始"));
        assert!(local_advice(40, 1, 45).contains("抬头看看远处"));
        assert!(local_advice(40, 3, 90).contains("连续工作"));
        assert!(local_advice(40, 8, 10).contains("收个尾"));
    }
}
