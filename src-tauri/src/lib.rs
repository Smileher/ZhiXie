use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, NaiveTime, TimeZone};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration as StdDuration, Instant, SystemTime, UNIX_EPOCH};
use tauri::menu::{Menu, MenuBuilder, MenuItemBuilder};
use tauri::tray::{TrayIconBuilder, TrayIconEvent};
use tauri::{
    AppHandle, Emitter, LogicalSize, Manager, Monitor, PhysicalPosition, PhysicalSize, RunEvent,
    State, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent,
};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_dialog::DialogExt;

mod agent;
mod browser;
mod i18n;
#[cfg(target_os = "windows")]
mod notification;
mod startup;

const DATA_VERSION: u32 = 6;
const REST_ID: &str = "__rest__";
const TEST_REST_ID: &str = "__test_rest__";
const TRAY_ID: &str = "main-tray";
const REMINDER_LABEL: &str = "reminder";
const WINDOWED_REMINDER_LABEL: &str = "reminder-windowed";
const REMINDER_MONITOR_PREFIX: &str = "reminder-monitor-";
const REMINDER_WINDOW_WIDTH: f64 = 520.0;
const REMINDER_WINDOW_HEIGHT: f64 = 320.0;
const MAIN_WINDOW_WIDTH: f64 = 780.0;
const MAIN_WINDOW_HEIGHT: f64 = 540.0;
const WINDOW_DESTROY_DELAY: StdDuration = StdDuration::from_secs(30);
const POWER_COMMAND_TIMEOUT: StdDuration = StdDuration::from_secs(10);
const MAX_POPUP_IMAGE_BYTES: u64 = 10 * 1024 * 1024;
const DEFAULT_POPUP_IMAGE: &[u8] = include_bytes!("../assets/default-popup.png");
const POPUP_IMAGE_INITIALIZED: &str = "popup-image-initialized";
const POWER_GRACE_SECONDS: i64 = 60;
const WINDOW_DESTROY_TIMEOUT: StdDuration = StdDuration::from_secs(2);
const WINDOW_DESTROY_POLL: StdDuration = StdDuration::from_millis(10);
const SCHEDULER_INTERVAL: StdDuration = StdDuration::from_secs(1);
#[cfg(target_os = "macos")]
const THEME_POLL_INTERVAL: StdDuration = StdDuration::from_secs(2);

/// 窗口背景色需与前端 `--page-bg` 保持一致。
///
/// macOS 标题栏设为透明后，标题栏区域透出的就是窗口背景色，
/// 不再依赖系统外观（NSApp.appearance）的实时过渡，因此切换深浅色必然同步。
#[cfg(target_os = "macos")]
const WINDOW_BG_DARK: tauri::window::Color = tauri::window::Color(32, 34, 37, 255);
#[cfg(target_os = "macos")]
const WINDOW_BG_LIGHT: tauri::window::Color = tauri::window::Color(238, 241, 245, 255);

fn default_rest_message() -> String {
    i18n::default_rest_message(Language::ZhCn).to_string()
}

fn default_system_notification_enabled() -> bool {
    true
}

fn default_popup_fullscreen() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    #[serde(default)]
    pub language: Language,
    pub autostart: bool,
    pub minimize_to_tray: bool,
    pub popup_always_on_top: bool,
    #[serde(default = "default_popup_fullscreen")]
    pub popup_fullscreen: bool,
    pub rest_enabled: bool,
    pub rest_interval_minutes: u32,
    #[serde(default = "default_rest_message")]
    pub rest_message: String,
    #[serde(default = "default_system_notification_enabled")]
    pub system_notification_enabled: bool,
    #[serde(default)]
    pub theme: Theme,
    #[serde(default)]
    pub accent_color: AccentColor,
    #[serde(default)]
    pub popup_background_fit: PopupBackgroundFit,
    #[serde(default = "default_popup_background_scale")]
    pub popup_background_scale: u32,
    #[serde(default)]
    pub popup_background_offset_x: i32,
    #[serde(default)]
    pub popup_background_offset_y: i32,
    #[serde(default = "default_popup_fade_enabled")]
    pub popup_fade_enabled: bool,
    #[serde(default)]
    pub popup_text_color: String,
    #[serde(default = "default_popup_title_size")]
    pub popup_title_size: u32,
    #[serde(default = "default_popup_overlay_opacity")]
    pub popup_overlay_opacity: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum PowerAction {
    Shutdown,
    Lock,
    Restart,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum PopupBackgroundFit {
    #[serde(alias = "original")]
    Contain,
    #[serde(alias = "cover", alias = "repeat")]
    Stretch,
}

impl Default for PopupBackgroundFit {
    fn default() -> Self {
        Self::Stretch
    }
}

fn default_popup_background_scale() -> u32 {
    100
}

fn default_popup_title_size() -> u32 {
    32
}

fn default_popup_overlay_opacity() -> u32 {
    30
}

fn default_popup_fade_enabled() -> bool {
    true
}

impl Default for PowerAction {
    fn default() -> Self {
        Self::Shutdown
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Language {
    #[serde(rename = "zh-CN")]
    ZhCn,
    #[serde(rename = "en")]
    En,
}

impl Default for Language {
    fn default() -> Self {
        Self::ZhCn
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Dark,
    Light,
    System,
}

impl Default for Theme {
    fn default() -> Self {
        Self::System
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AccentColor {
    Mint,
    Blue,
    Violet,
    Amber,
    Cyan,
    Rose,
    Coral,
    Graphite,
}

impl Default for AccentColor {
    fn default() -> Self {
        Self::Blue
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            language: Language::ZhCn,
            autostart: false,
            minimize_to_tray: true,
            popup_always_on_top: true,
            popup_fullscreen: true,
            rest_enabled: true,
            rest_interval_minutes: 40,
            rest_message: default_rest_message(),
            system_notification_enabled: true,
            theme: Theme::System,
            accent_color: AccentColor::Blue,
            popup_background_fit: PopupBackgroundFit::Stretch,
            popup_background_scale: 100,
            popup_background_offset_x: 0,
            popup_background_offset_y: 0,
            popup_fade_enabled: default_popup_fade_enabled(),
            popup_text_color: String::new(),
            popup_title_size: default_popup_title_size(),
            popup_overlay_opacity: default_popup_overlay_opacity(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ReminderType {
    Once,
    Daily,
    Weekly,
    Monthly,
    Interval,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
enum TestReminderKind {
    Event,
    Rest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reminder {
    pub id: String,
    pub title: String,
    #[serde(rename = "type")]
    pub reminder_type: ReminderType,
    pub trigger_at: Option<String>,
    pub time: Option<String>,
    #[serde(default)]
    pub weekdays: Vec<u32>,
    #[serde(default)]
    pub month_days: Vec<u32>,
    pub enabled: bool,
    pub power_action: Option<PowerAction>,
    #[serde(default)]
    pub next_trigger_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppData {
    pub version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub autostart_owner: Option<startup::AutostartOwner>,
    pub settings: AppSettings,
    pub reminders: Vec<Reminder>,
}

impl Default for AppData {
    fn default() -> Self {
        Self {
            version: DATA_VERSION,
            autostart_owner: None,
            settings: AppSettings::default(),
            reminders: default_reminders(Language::ZhCn),
        }
    }
}

fn default_reminders(language: Language) -> Vec<Reminder> {
    let messages = i18n::preset_messages(language);
    [
        ("preset-lunch", messages[0], ReminderType::Daily, "11:50", vec![], PowerAction::Lock),
        ("preset-workday", messages[1], ReminderType::Weekly, "17:30", vec![1, 2, 3, 4], PowerAction::Lock),
        ("preset-weekend", messages[2], ReminderType::Weekly, "17:30", vec![5], PowerAction::Shutdown),
    ].into_iter().map(|(id, title, reminder_type, time, weekdays, action)| Reminder {
        id: id.to_string(),
        title: title.to_string(),
        reminder_type,
        trigger_at: None,
        time: Some(time.to_string()),
        weekdays,
        month_days: Vec::new(),
        enabled: false,
        power_action: Some(action),
        next_trigger_at: None,
    }).collect()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReminderTriggeredEvent {
    session_id: u64,
    id: String,
    title: String,
    #[serde(rename = "type")]
    reminder_type: ReminderType,
    is_rest: bool,
    power_action: Option<PowerAction>,
    is_test: bool,
    rest_started_at_ms: Option<i64>,
    power_deadline_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RestTimerStatus {
    next_trigger_at: Option<String>,
    is_resting: bool,
}

#[derive(Debug, Clone)]
struct QueuedReminder {
    due_at: DateTime<Local>,
    event: ReminderTriggeredEvent,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct NativeErrors {
    autostart_error: Option<String>,
    notification_error: Option<String>,
    persistence_error: Option<String>,
    scheduler_error: Option<String>,
}

struct InnerState {
    data: Mutex<AppData>,
    data_path: PathBuf,
    paused: AtomicBool,
    scheduler_started: AtomicBool,
    scheduler_stop: AtomicBool,
    // 故障原因独立保存，业务状态锁中毒也不能丢失停止原因。
    scheduler_error: Mutex<Option<String>>,
    next_reminder_session: AtomicU64,
    power_action_session: AtomicU64,
    active_reminder: Mutex<Option<ReminderTriggeredEvent>>,
    // 休息状态的读取和切换统一持有 data 锁，避免设置保存与弹窗状态交错。
    rest_active: AtomicBool,
    rest_round_pending: AtomicBool,
    rest_next: Mutex<Option<DateTime<Local>>>,
    reminder_queue: Mutex<VecDeque<QueuedReminder>>,
    pending_navigation: Mutex<Option<String>>,
    // All native window work is serialized off the UI thread, including destruction.
    window_operations: Mutex<()>,
    popup_image_operations: Mutex<()>,
    window_cache: Mutex<WindowCache>,
    reminder_targets: Mutex<HashSet<String>>,
    main_ready: AtomicBool,
    main_visible: AtomicBool,
    native_errors: Mutex<NativeErrors>,
}

#[derive(Clone)]
struct AppState(Arc<InnerState>);

fn data_path(app: &AppHandle) -> Result<PathBuf, String> {
    let directory = config_directory(app)?;
    fs::create_dir_all(&directory)
        .map_err(|error| format!("Failed to create the app configuration directory: {error}"))?;
    Ok(directory.join("zhixie.json"))
}

fn config_directory(app: &AppHandle) -> Result<PathBuf, String> {
    #[cfg(target_os = "windows")]
    return app.path().home_dir().map(|path| path.join(".zhixie"))
        .map_err(|error| format!("Failed to resolve the user directory: {error}"));
    #[cfg(not(target_os = "windows"))]
    app.path().app_config_dir()
        .map_err(|error| format!("Failed to resolve the app configuration directory: {error}"))
}

fn write_json(path: &Path, data: &AppData) -> Result<(), String> {
    let content = serde_json::to_string_pretty(data)
        .map_err(|error| format!("Failed to serialize settings: {error}"))?;
    atomic_write(path, content.as_bytes()).map_err(|error| format!("Failed to save settings: {error}"))
}

fn commit_app_data(path: &Path, current: &mut AppData, candidate: AppData) -> Result<(), String> {
    write_json(path, &candidate)?;
    *current = candidate;
    Ok(())
}

fn atomic_write(path: &Path, content: &[u8]) -> std::io::Result<()> {
    static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);
    let name = path.file_name().ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "Missing file name"))?;
    let mut temp_name = name.to_os_string();
    temp_name.push(format!(".{}.{}.tmp", std::process::id(), NEXT_TEMP.fetch_add(1, Ordering::Relaxed)));
    let temp = path.with_file_name(temp_name);
    let mut file = fs::OpenOptions::new().write(true).create_new(true).open(&temp)?;
    let result = (|| {
        file.write_all(content)?;
        file.sync_all()?;
        drop(file);
        replace_file(&temp, path)
    })();
    if result.is_err() {
        if let Err(error) = fs::remove_file(&temp) {
            eprintln!("Failed to remove temporary file {}: {error}", temp.display());
        }
    }
    result
}

#[cfg(target_os = "windows")]
fn replace_file(source: &Path, target: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH};
    let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let target: Vec<u16> = target.as_os_str().encode_wide().chain(Some(0)).collect();
    // Same-directory replacement never deletes the live configuration first.
    if unsafe { MoveFileExW(source.as_ptr(), target.as_ptr(), MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH) } == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn replace_file(source: &Path, target: &Path) -> std::io::Result<()> {
    fs::rename(source, target)
}

fn corrupt_backup_path(path: &Path) -> PathBuf {
    backup_path(path, "corrupt")
}

// 旧版本配置是可用的用户数据，单独标记以区别于解析失败产生的损坏文件。
fn unsupported_backup_path(path: &Path) -> PathBuf {
    backup_path(path, "unsupported")
}

fn backup_path(path: &Path, kind: &str) -> PathBuf {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs())
        .unwrap_or_default();
    PathBuf::from(format!("{}.{kind}.{seconds}", path.display()))
}

fn load_json(path: &Path) -> Result<AppData, String> {
    if !path.exists() {
        return Ok(AppData::default());
    }
    let content =
        fs::read_to_string(path).map_err(|error| format!("Failed to read settings: {error}"))?;
    match serde_json::from_str::<AppData>(&content) {
        Ok(data) if data.version == DATA_VERSION => Ok(data),
        Ok(_) => {
            // 旧版本配置本身是有效数据，先备份再重写，避免降级时提醒不可恢复地丢失。
            let backup = unsupported_backup_path(path);
            fs::rename(path, &backup).map_err(|rename_error| {
                format!("Failed to preserve unsupported settings: {rename_error}")
            })?;
            let defaults = AppData::default();
            write_json(path, &defaults)?;
            Ok(defaults)
        }
        Err(error) => {
            let backup = corrupt_backup_path(path);
            fs::rename(path, &backup).map_err(|rename_error| {
                format!(
                    "Settings are invalid ({error}) and the corrupted file could not be preserved: {rename_error}"
                )
            })?;
            Ok(AppData::default())
        }
    }
}

fn parse_datetime(value: &str) -> Result<DateTime<Local>, String> {
    DateTime::parse_from_rfc3339(value)
        .map(|datetime| datetime.with_timezone(&Local))
        .map_err(|error| format!("Invalid date-time format: {error}"))
}

fn parse_time(value: &str) -> Result<NaiveTime, String> {
    NaiveTime::parse_from_str(value, "%H:%M")
        .map_err(|_| "Reminder time must use HH:MM format".to_string())
}

fn local_datetime(date: NaiveDate, time: NaiveTime) -> Result<DateTime<Local>, String> {
    Local
        .from_local_datetime(&date.and_time(time))
        .earliest()
        .or_else(|| Local.from_local_datetime(&date.and_time(time)).latest())
        .ok_or_else(|| "Failed to resolve the local reminder time".to_string())
}

#[cfg(test)]
fn next_daily(time: &str, after: DateTime<Local>) -> Result<String, String> {
    let parsed_time = parse_time(time)?;
    let today = local_datetime(after.date_naive(), parsed_time)?;
    let candidate = if today > after {
        today
    } else {
        local_datetime(after.date_naive() + Duration::days(1), parsed_time)?
    };
    Ok(candidate.to_rfc3339())
}

fn next_recurring(reminder: &Reminder, after: DateTime<Local>) -> Result<String, String> {
    let time = reminder
        .time
        .as_deref()
        .ok_or_else(|| "Repeating reminder is missing time".to_string())?;
    let parsed_time = parse_time(time)?;

    for offset in 0..=370 {
        let date = after.date_naive() + Duration::days(offset);
        let weekday = date.weekday().number_from_monday();
        let matches = match reminder.reminder_type {
            ReminderType::Daily => true,
            ReminderType::Weekly => reminder.weekdays.contains(&weekday),
            ReminderType::Monthly => reminder.month_days.contains(&date.day()),
            ReminderType::Once | ReminderType::Interval => false,
        };
        if !matches {
            continue;
        }
        if let Ok(candidate) = local_datetime(date, parsed_time) {
            if candidate > after {
                return Ok(candidate.to_rfc3339());
            }
        }
    }
    Err("Failed to calculate the next reminder time".to_string())
}

fn should_trigger_power_action(due: DateTime<Local>, now: DateTime<Local>) -> bool {
    due <= now && now.signed_duration_since(due) <= Duration::seconds(POWER_GRACE_SECONDS)
}

fn complete_rest_round(state: &AppState) -> bool {
    let data = state.0.data.lock().expect("settings lock poisoned");
    // 稍后提醒已关闭本次弹窗；重复关闭或关闭其他通知不能覆盖延后的时间。
    if !state.0.rest_active.swap(false, Ordering::SeqCst) {
        return false;
    }
    state.0.rest_round_pending.store(false, Ordering::SeqCst);
    if data.settings.rest_enabled {
        *state
            .0
            .rest_next
            .lock()
            .expect("break reminder lock poisoned") =
            Some(Local::now() + Duration::minutes(data.settings.rest_interval_minutes as i64));
    } else {
        *state
            .0
            .rest_next
            .lock()
            .expect("break reminder lock poisoned") = None;
    }
    true
}

fn snooze_rest_round(state: &AppState, seconds: u32) -> bool {
    let data = state.0.data.lock().expect("settings lock poisoned");
    if !state.0.rest_active.swap(false, Ordering::SeqCst) {
        return false;
    }
    state
        .0
        .rest_round_pending
        .store(data.settings.rest_enabled, Ordering::SeqCst);
    *state
        .0
        .rest_next
        .lock()
        .expect("break reminder lock poisoned") = data
        .settings
        .rest_enabled
        .then(|| Local::now() + Duration::seconds(seconds.max(1) as i64));
    true
}

fn validate_and_normalize(data: &mut AppData) -> Result<(), String> {
    if data.version != DATA_VERSION {
        return Err(format!("Unsupported data version: {} (expected {DATA_VERSION})", data.version));
    }
    data.settings.popup_title_size = data.settings.popup_title_size.clamp(20, 72);
    data.settings.popup_overlay_opacity = data.settings.popup_overlay_opacity.min(100);
    let color = data.settings.popup_text_color.trim();
    if color.is_empty()
        || (color.len() == 7
            && color.starts_with('#')
            && color.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit))
    {
        data.settings.popup_text_color = color.to_string();
    } else {
        data.settings.popup_text_color.clear();
    }
    data.settings.popup_background_scale = data.settings.popup_background_scale.clamp(1, 400);
    data.settings.popup_background_offset_x = data.settings.popup_background_offset_x.clamp(-50, 50);
    data.settings.popup_background_offset_y = data.settings.popup_background_offset_y.clamp(-50, 50);
    data.settings.rest_interval_minutes = data.settings.rest_interval_minutes.clamp(1, 1440);
    if data.settings.rest_message.trim().is_empty() {
        data.settings.rest_message = i18n::default_rest_message(data.settings.language).to_string();
    }
    let now = Local::now();
    let mut ids = HashSet::new();
    for reminder in &mut data.reminders {
        if reminder.id.trim().is_empty() || reminder.id.starts_with("__") || !ids.insert(reminder.id.clone()) {
            return Err("Reminder id is missing, reserved or duplicated".to_string());
        }
        if reminder.title.trim().is_empty() {
            return Err("Reminder text cannot be empty".to_string());
        }
        match reminder.reminder_type {
            ReminderType::Once => {
                let trigger = reminder
                    .trigger_at
                    .as_deref()
                    .ok_or_else(|| "One-time reminder is missing triggerAt".to_string())?;
                parse_datetime(trigger)?;
                if let Some(next) = reminder.next_trigger_at.as_deref() {
                    parse_datetime(next)?;
                } else {
                    reminder.next_trigger_at = Some(trigger.to_string());
                }
            }
            ReminderType::Daily | ReminderType::Weekly | ReminderType::Monthly => {
                let time = reminder
                    .time
                    .as_deref()
                    .ok_or_else(|| "Repeating reminder is missing time".to_string())?;
                parse_time(time)?;
                reminder.weekdays.sort_unstable();
                reminder.weekdays.dedup();
                reminder.month_days.sort_unstable();
                reminder.month_days.dedup();
                if reminder.reminder_type == ReminderType::Weekly
                    && (reminder.weekdays.is_empty()
                        || reminder.weekdays.iter().any(|day| !(1..=7).contains(day)))
                {
                    return Err("Weekly reminder must include at least one weekday".to_string());
                }
                if reminder.reminder_type == ReminderType::Monthly
                    && (reminder.month_days.is_empty()
                        || reminder
                            .month_days
                            .iter()
                            .any(|day| !(1..=31).contains(day)))
                {
                    return Err("Monthly reminder must include at least one date".to_string());
                }
                if reminder.next_trigger_at.is_none() {
                    reminder.next_trigger_at = Some(next_recurring(reminder, now)?);
                } else if let Some(next) = reminder.next_trigger_at.as_deref() {
                    parse_datetime(next)?;
                }
            }
            ReminderType::Interval => {
                return Err("Interval reminders belong to break settings".to_string());
            }
        }
    }
    Ok(())
}

fn app_data(state: &AppState) -> AppData {
    state.0.data.lock().expect("settings lock poisoned").clone()
}

fn rest_timer_status(state: &AppState) -> RestTimerStatus {
    let data = state.0.data.lock().expect("settings lock poisoned");
    if state.0.rest_active.load(Ordering::SeqCst) {
        return RestTimerStatus {
            next_trigger_at: None,
            is_resting: true,
        };
    }
    if !data.settings.rest_enabled {
        return RestTimerStatus {
            next_trigger_at: None,
            is_resting: false,
        };
    }
    let mut next = state
        .0
        .rest_next
        .lock()
        .expect("break reminder lock poisoned");
    if next.is_none() && !state.0.reminder_queue.lock().expect("reminder queue lock poisoned")
        .iter().any(|item| item.event.is_rest) {
        *next = Some(Local::now() + Duration::minutes(data.settings.rest_interval_minutes as i64));
    }
    RestTimerStatus {
        next_trigger_at: next.as_ref().map(DateTime::to_rfc3339),
        is_resting: false,
    }
}

fn emit_rest_timer_updated(app: &AppHandle, state: &AppState) {
    let _ = report_native("Emit native event", app.emit_to("main", "rest-timer-updated", rest_timer_status(state)));
}

fn is_reminder_window_label(label: &str) -> bool {
    matches!(window_kind(label), WindowKind::FullscreenReminder | WindowKind::WindowedReminder)
}

#[derive(Debug, PartialEq)]
enum WindowKind { Main, FullscreenReminder, WindowedReminder, Other }

fn window_kind(label: &str) -> WindowKind {
    match label {
        "main" => WindowKind::Main,
        WINDOWED_REMINDER_LABEL => WindowKind::WindowedReminder,
        REMINDER_LABEL => WindowKind::FullscreenReminder,
        _ if label.starts_with(REMINDER_MONITOR_PREFIX) => WindowKind::FullscreenReminder,
        _ => WindowKind::Other,
    }
}

fn reminder_windows(app: &AppHandle) -> Vec<WebviewWindow> {
    app.webview_windows()
        .into_values()
        .filter(|window| is_reminder_window_label(window.label()))
        .collect()
}

fn emit_to_reminder_windows<S: Clone + Serialize>(app: &AppHandle, event: &str, payload: S) {
    for window in reminder_windows(app) {
        let _ = report_native("Emit native event", app.emit_to(window.label(), event, payload.clone()));
    }
}

fn report_native<T, E: std::fmt::Display>(context: &str, result: Result<T, E>) -> Result<T, E> {
    if let Err(error) = &result {
        eprintln!("{context}: {error}");
    }
    result
}

fn report_window<T>(window: &WebviewWindow, property: &str, result: tauri::Result<T>) {
    let _ = report_native(&format!("Window {} {property}", window.label()), result);
}

fn lock_error<T>(error: std::sync::PoisonError<T>) -> String {
    format!("State lock poisoned: {error}")
}

// An internal panic may poison state; return the error without recovering that state.
fn guarded<T>(work: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(work)).unwrap_or_else(|payload| {
        let detail = payload.downcast_ref::<String>().map(String::as_str)
            .or_else(|| payload.downcast_ref::<&str>().copied()).unwrap_or("unknown panic");
        Err(format!("Background operation stopped: {detail}"))
    })
}

async fn blocking_command<T: Send + 'static>(work: impl FnOnce() -> Result<T, String> + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(move || guarded(work)).await
        .map_err(|error| format!("Background worker failed: {error}"))?
}

#[derive(Default)]
struct WindowCache {
    idle: HashMap<String, Instant>,
    destroying: HashSet<String>,
}

impl WindowCache {
    fn destroyed(&mut self, label: &str) {
        self.idle.remove(label);
        self.destroying.remove(label);
    }

    fn needs_destroy_wait(&mut self, label: &str, exists: bool) -> bool {
        if !exists {
            self.destroyed(label);
        }
        self.destroying.contains(label)
    }

    fn hide(&mut self, label: &str, now: Instant) {
        // Repeated close events must not extend the same idle period.
        self.idle
            .entry(label.to_string())
            .or_insert(now + WINDOW_DESTROY_DELAY);
    }

    fn reuse(&mut self, label: &str) {
        self.idle.remove(label);
    }

    fn expired(&self, now: Instant) -> Vec<String> {
        self.idle
            .iter()
            .filter(|(_, deadline)| **deadline <= now)
            .map(|(label, _)| label.clone())
            .collect()
    }
}

// Internal helpers require window_operations; native UI callbacks must enqueue work.
fn hide_cached_window(window: &WebviewWindow, state: &AppState) -> Result<(), String> {
    window.hide().map_err(|error| error.to_string())?;
    #[cfg(target_os = "macos")]
    if is_reminder_window_label(window.label()) {
        report_window(window, "set_simple_fullscreen", window.set_simple_fullscreen(false));
        window.hide().map_err(|error| error.to_string())?;
    }
    state
        .0
        .window_cache
        .lock().map_err(lock_error)?
        .hide(window.label(), Instant::now());
    if window.label() == "main" && state.0.main_visible.swap(false, Ordering::SeqCst) {
        notify_tray_background(window.app_handle(), state);
    }
    Ok(())
}

// 托盘提示独立于提醒通知开关；失败保留到下次打开主窗口时展示。
fn notify_tray_background(app: &AppHandle, state: &AppState) {
    let language = app_data(state).settings.language;
    let body = i18n::tray_background_message(language);
    #[cfg(target_os = "windows")]
    let result = if option_env!("ZHIXIE_STORE_BUILD").is_some() {
        notification::show_packaged("知歇", body)
    } else {
        app.notification().builder().title("知歇").body(body).show().map_err(|error| error.to_string())
    };
    #[cfg(not(target_os = "windows"))]
    let result = app.notification().builder().title("知歇").body(body).show().map_err(|error| error.to_string());
    let error = result.err();
    state.0.native_errors.lock().expect("native errors lock poisoned").notification_error = error.clone();
    if let Some(error) = error {
        let _ = report_native("Emit native event", app.emit_to("main", "notification-failed", error));
    }
}

fn hide_reminder_windows(app: &AppHandle, state: &AppState) {
    state
        .0
        .reminder_targets
        .lock()
        .expect("reminder targets lock poisoned")
        .clear();
    for window in reminder_windows(app) {
        if let Err(error) = hide_cached_window(&window, state) {
            eprintln!("Failed to hide {}: {error}", window.label());
        }
    }
}

fn wait_for_window_destroyed(app: &AppHandle, state: &AppState, label: &str) -> Result<(), String> {
    if !state
        .0
        .window_cache
        .lock().map_err(lock_error)?
        .needs_destroy_wait(label, app.get_webview_window(label).is_some())
    {
        return Ok(());
    }
    // destroy() only posts a message. Reuse the label after the manager handles Destroyed.
    // Waiting is safe only on a background thread while the native event loop keeps running.
    let deadline = Instant::now() + WINDOW_DESTROY_TIMEOUT;
    while app.get_webview_window(label).is_some() {
        if Instant::now() >= deadline {
            return Err(format!("Window {label} is still being destroyed"));
        }
        thread::sleep(WINDOW_DESTROY_POLL);
    }
    state
        .0
        .window_cache
        .lock().map_err(lock_error)?
        .destroyed(label);
    Ok(())
}

fn reclaim_idle_windows(app: &AppHandle, state: &AppState) -> Result<(), String> {
    let labels = state
        .0
        .window_cache
        .lock()
        .map_err(lock_error)?
        .expired(Instant::now());
    for label in labels {
        let Some(window) = app.get_webview_window(&label) else {
            let mut cache = state
                .0
                .window_cache
                .lock()
                .map_err(lock_error)?;
            cache.destroyed(&label);
            continue;
        };
        if !matches!(window.is_visible(), Ok(false)) {
            continue;
        }
        if state
            .0
            .reminder_targets
            .lock()
            .map_err(lock_error)?
            .contains(&label)
        {
            continue;
        }
        state
            .0
            .window_cache
            .lock()
            .map_err(lock_error)?
            .destroying
            .insert(label.clone());
        match window.destroy() {
            Ok(()) => {
                state
                    .0
                    .window_cache
                    .lock()
                    .map_err(lock_error)?
                    .idle
                    .remove(&label);
                if label == "main" {
                    state.0.main_ready.store(false, Ordering::SeqCst);
                }
                // Destroyed performs cleanup; reclamation must not wait under the window lock.
            }
            Err(error) => {
                state
                    .0
                    .window_cache
                    .lock()
                    .map_err(lock_error)?
                    .destroying
                    .remove(&label);
                eprintln!("Failed to destroy {label}: {error}");
            }
        }
    }
    Ok(())
}

fn queue_window_action<F>(app: &AppHandle, action: F)
where
    F: FnOnce(&AppHandle, &AppState) -> Result<(), String> + Send + 'static,
{
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let result = guarded(|| {
        let state = app.state::<AppState>();
        let _operation = state
            .0
            .window_operations
            .lock().map_err(lock_error)?;
        action(&app, state.inner())
        });
        let _ = report_native("Window operation failed", result);
    });
}

fn close_reminder_session(app: &AppHandle, state: &AppState, session_id: u64) -> bool {
    let mut active = state
        .0
        .active_reminder
        .lock()
        .expect("reminder session lock poisoned");
    if active.as_ref().map(|event| event.session_id) != Some(session_id) {
        return false;
    }
    *active = None;
    drop(active);
    let _ = state.0.power_action_session.compare_exchange(
        session_id,
        0,
        Ordering::SeqCst,
        Ordering::SeqCst,
    );
    emit_to_reminder_windows(app, "reminder-closed", session_id);
    hide_reminder_windows(app, state);
    true
}

fn reset_reminder_session(app: &AppHandle, state: &AppState, event: &str) {
    state.0.reminder_queue.lock().expect("reminder queue lock poisoned").clear();
    *state
        .0
        .active_reminder
        .lock()
        .expect("reminder session lock poisoned") = None;
    state.0.power_action_session.store(0, Ordering::SeqCst);
    emit_to_reminder_windows(app, event, ());
    hide_reminder_windows(app, state);
}

fn cancel_active_rest_reminder(app: &AppHandle, state: &AppState) {
    let mut active = state
        .0
        .active_reminder
        .lock()
        .expect("reminder session lock poisoned");
    if !active.as_ref().is_some_and(|event| event.is_rest) {
        return;
    }
    *active = None;
    drop(active);
    emit_to_reminder_windows(app, "rest-cancelled", ());
    hide_reminder_windows(app, state);
}

fn active_reminder_matches(state: &AppState, id: &str, session_id: u64) -> bool {
    state
        .0
        .active_reminder
        .lock()
        .expect("reminder session lock poisoned")
        .as_ref()
        .is_some_and(|event| event.session_id == session_id && event.id == id)
}

fn activate_reminder_session(app: &AppHandle, state: &AppState, event: ReminderTriggeredEvent) {
    state.0.power_action_session.store(0, Ordering::SeqCst);
    let previous = state
        .0
        .active_reminder
        .lock()
        .expect("reminder session lock poisoned")
        .replace(event);
    if let Some(previous) = previous {
        emit_to_reminder_windows(app, "reminder-closed", previous.session_id);
        hide_reminder_windows(app, state);
    }
}

fn same_monitor(left: &Monitor, right: &Monitor) -> bool {
    left.position() == right.position() && left.size() == right.size()
}

fn ordered_monitors(app: &AppHandle) -> Vec<Monitor> {
    let Ok(mut monitors) = app.available_monitors() else {
        return Vec::new();
    };
    if let Ok(Some(primary)) = app.primary_monitor() {
        if let Some(index) = monitors
            .iter()
            .position(|monitor| same_monitor(monitor, &primary))
        {
            monitors.swap(0, index);
        }
    }
    monitors
}

fn configure_windowed_reminder(window: &WebviewWindow, settings: &AppSettings) {
    report_window(window, "hide", window.hide());
    report_window(window, "set_title", window.set_title(i18n::notification_window_title(settings.language)));
    report_window(window, "set_always_on_top", window.set_always_on_top(settings.popup_always_on_top));
    report_window(window, "set_decorations", window.set_decorations(true));
    // 窗口模式弹窗有原生标题栏，同样用透明标题栏 + 显式背景色跟随深浅色。
    // 全屏弹窗是 transparent 的，不能上色，所以只在窗口模式生效。
    #[cfg(target_os = "macos")]
    if !is_transparent_popup(window.label()) {
        report_window(window, "set_title_bar_style", window.set_title_bar_style(tauri::TitleBarStyle::Transparent));
        report_window(window, "set_background_color", window.set_background_color(Some(background_color_for(&settings.theme))));
    }
    report_window(window, "set_resizable", window.set_resizable(false));
    report_window(window, "set_maximizable", window.set_maximizable(false));
    report_window(window, "set_skip_taskbar", window.set_skip_taskbar(false));
    report_window(window, "set_size", window.set_size(LogicalSize::new(
        REMINDER_WINDOW_WIDTH,
        REMINDER_WINDOW_HEIGHT,
    )));
    report_window(window, "center", window.center());
    report_window(window, "hide", window.hide());
}

fn configure_fullscreen_reminder(
    window: &WebviewWindow,
    monitor: &Monitor,
    settings: &AppSettings,
    is_controller: bool,
) {
    #[cfg(not(target_os = "macos"))]
    let reuse_native_fullscreen = window.is_fullscreen().unwrap_or(false)
        && window
            .current_monitor()
            .ok()
            .flatten()
            .as_ref()
            .is_some_and(|current| same_monitor(current, monitor));
    #[cfg(not(target_os = "macos"))]
    if !reuse_native_fullscreen {
        report_window(window, "hide", window.hide());
        report_window(window, "set_fullscreen", window.set_fullscreen(false));
        report_window(window, "hide", window.hide());
    }
    report_window(window, "set_title", window.set_title(i18n::notification_window_title(settings.language)));
    report_window(window, "set_always_on_top", window.set_always_on_top(settings.popup_always_on_top));
    report_window(window, "set_decorations", window.set_decorations(false));
    report_window(window, "set_resizable", window.set_resizable(false));
    report_window(window, "set_maximizable", window.set_maximizable(false));
    report_window(window, "set_skip_taskbar", window.set_skip_taskbar(true));

    #[cfg(target_os = "macos")]
    {
        report_window(window, "set_position", window.set_position(PhysicalPosition::new(
            monitor.position().x,
            monitor.position().y,
        )));
        if is_controller {
            report_window(window, "set_simple_fullscreen", window.set_simple_fullscreen(true));
            report_window(window, "hide", window.hide());
            return;
        }
        report_window(window, "set_size", window.set_size(PhysicalSize::new(
            monitor.size().width,
            monitor.size().height,
        )));
        report_window(window, "hide", window.hide());
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = is_controller;
        if !reuse_native_fullscreen {
            report_window(window, "set_position", window.set_position(PhysicalPosition::new(
                monitor.position().x,
                monitor.position().y,
            )));
            report_window(window, "set_size", window.set_size(PhysicalSize::new(
                monitor.size().width,
                monitor.size().height,
            )));
            report_window(window, "set_fullscreen", window.set_fullscreen(true));
        }
        // 原生全屏切换可能自行显示窗口，内容准备完成前必须保持隐藏。
        report_window(window, "hide", window.hide());
    }
}

/// 弹窗窗口是否启用透明。透明让弹窗先只显示文字、背景层再渐入。
/// 仅无装饰的弹窗（全屏模式）启用：macOS 上透明会连原生标题栏一起变透明，
/// 窗口模式弹窗（有标题栏）在所有平台都保持不透明。
fn is_transparent_popup(label: &str) -> bool {
    window_kind(label) == WindowKind::FullscreenReminder
}

#[tauri::command]
fn popup_window_is_transparent(label: String) -> bool {
    is_transparent_popup(&label)
}

/// 弹窗上的「全屏/窗口」切换按钮：翻转设置并保存。
/// 有正在显示的提醒时立即换窗——用新 session 重新分发，旧窗口由
/// activate_reminder_session 统一关闭隐藏，避免两个模式的窗口同时挂着。
#[tauri::command]
async fn toggle_popup_fullscreen(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let state = state.inner().clone();
    blocking_command(move || {
        let _operation = state
            .0
            .window_operations
            .lock().map_err(lock_error)?;
        let mut data = state.0.data.lock().map_err(lock_error)?;
        let mut candidate = data.clone();
        candidate.settings.popup_fullscreen = !candidate.settings.popup_fullscreen;
        commit_app_data(&state.0.data_path, &mut data, candidate)?;
        let settings = data.settings.clone();
        drop(data);
        sync_reminder_settings(&app, &settings);
        let _ = report_native("Emit native event", app.emit_to("main", "popup-fullscreen-updated", settings.popup_fullscreen));
        let active = state
            .0
            .active_reminder
            .lock().map_err(lock_error)?
            .clone();
        if let Some(mut event) = active {
            event.session_id = state.0.next_reminder_session.fetch_add(1, Ordering::SeqCst) + 1;
            activate_reminder_session(&app, &state, event.clone());
            let labels = match prepare_reminder_windows(&app, &state, &settings) {
                Ok(labels) => labels,
                Err(error) => {
                    if event.is_rest && complete_rest_round(&state) {
                        emit_rest_timer_updated(&app, &state);
                    }
                    close_reminder_session(&app, &state, event.session_id);
                    let _ = report_native("Report popup mode failure", app.emit_to("main", "notification-failed", &error));
                    return Err(error);
                }
            };
            for label in labels {
                if app.get_webview_window(&label).is_some() {
                    let _ = report_native("Emit native event", app.emit_to(&label, "reminder-triggered", event.clone()));
                }
            }
        }
        Ok(settings.popup_fullscreen)
    })
    .await
}

fn create_reminder_window(
    app: &AppHandle,
    label: String,
    settings: AppSettings,
) -> Result<WebviewWindow, String> {
    let transparent = is_transparent_popup(&label);
    WebviewWindowBuilder::new(app, label, WebviewUrl::App("index.html#/reminder".into()))
        .devtools(false)
        .zoom_hotkeys_enabled(false)
        .on_new_window(|_, _| tauri::webview::NewWindowResponse::Deny)
        .title(i18n::notification_window_title(settings.language))
        .visible(false)
        .decorations(false)
        .transparent(transparent)
        .resizable(false)
        .maximizable(false)
        .skip_taskbar(true)
        .always_on_top(settings.popup_always_on_top)
        .theme(tauri_theme(&settings.theme))
        .build()
        .map_err(|error| format!("Failed to create reminder window: {error}"))
}

fn prepare_reminder_windows(
    app: &AppHandle,
    state: &AppState,
    settings: &AppSettings,
) -> Result<Vec<String>, String> {
    let monitors = if settings.popup_fullscreen {
        ordered_monitors(app)
    } else {
        Vec::new()
    };
    // Keep native fullscreen and windowed instances separate: exiting fullscreen
    // restores saved window placement and can briefly show the old fullscreen surface.
    let labels: Vec<String> = (0..monitors.len().max(1))
        .map(|index| {
            if monitors.is_empty() {
                WINDOWED_REMINDER_LABEL.to_string()
            } else if index == 0 {
                REMINDER_LABEL.to_string()
            } else {
                format!("{REMINDER_MONITOR_PREFIX}{index}")
            }
        })
        .collect();
    for window in reminder_windows(app) {
        if !labels.iter().any(|label| label == window.label()) {
            hide_cached_window(&window, state)?;
        }
    }
    for (index, label) in labels.iter().enumerate() {
        wait_for_window_destroyed(app, state, label)?;
        let window = match app.get_webview_window(label) {
            Some(window) => window,
            None => create_reminder_window(app, label.clone(), settings.clone())?,
        };
        state
            .0
            .window_cache
            .lock().map_err(lock_error)?
            .reuse(label);
        if let Some(monitor) = monitors.get(index) {
            configure_fullscreen_reminder(&window, monitor, settings, index == 0);
        } else {
            configure_windowed_reminder(&window, settings);
        }
        // 新建与缓存复用都由后端同步，弹窗前端不再改写整个应用的主题。
        apply_window_appearance(&window, &settings.theme);
    }
    *state
        .0
        .reminder_targets
        .lock().map_err(lock_error)? = labels.iter().cloned().collect();
    Ok(labels)
}

fn sync_reminder_settings(app: &AppHandle, settings: &AppSettings) {
    for window in reminder_windows(app) {
        report_window(&window, "set_title", window.set_title(i18n::notification_window_title(settings.language)));
        report_window(&window, "set_always_on_top", window.set_always_on_top(settings.popup_always_on_top));
        // 原生主题统一由后端同步；macOS 同时更新透明标题栏的背景色。
        apply_window_appearance(&window, &settings.theme);
        let _ = report_native("Emit native event", app.emit_to(window.label(), "settings-updated", settings));
    }
    if let Some(window) = app.get_webview_window("main") {
        apply_window_appearance(&window, &settings.theme);
    }
}

fn prepare_notification(state: &AppState, event: &ReminderTriggeredEvent) -> Option<AppSettings> {
    let data = state.0.data.lock().expect("settings lock poisoned");
    let settings = &data.settings;
    // 到期事件生成后若用户关闭了休息提醒，不再重新激活已取消的弹窗。
    if event.is_rest && !event.is_test && !settings.rest_enabled {
        return None;
    }

    // 测试和定时休息弹窗使用相同的暂停及完成逻辑。
    if event.is_rest {
        state.0.rest_active.store(true, Ordering::SeqCst);
        *state
            .0
            .rest_next
            .lock()
            .expect("break reminder lock poisoned") = None;
        state.0.rest_round_pending.store(true, Ordering::SeqCst);
    }
    Some(settings.clone())
}

fn dispatch_trigger(
    app: &AppHandle,
    state: &AppState,
    mut event: ReminderTriggeredEvent,
) -> Result<(), String> {
    let Some(settings) = prepare_notification(state, &event) else {
        return Ok(());
    };
    emit_rest_timer_updated(app, state);
    event.session_id = state.0.next_reminder_session.fetch_add(1, Ordering::SeqCst) + 1;
    let now_ms = Local::now().timestamp_millis();
    if event.is_rest && event.rest_started_at_ms.is_none() {
        event.rest_started_at_ms = Some(now_ms);
    }
    if event.power_action.is_some() && event.power_deadline_ms.is_none() {
        event.power_deadline_ms = Some(now_ms + POWER_GRACE_SECONDS * 1000);
    }
    activate_reminder_session(app, state, event.clone());
    let labels = match prepare_reminder_windows(app, state, &settings) {
        Ok(labels) => labels,
        Err(error) => {
            if event.is_rest && complete_rest_round(state) {
                emit_rest_timer_updated(app, state);
            }
            close_reminder_session(app, state, event.session_id);
            return Err(error);
        }
    };
    for label in labels {
        if app.get_webview_window(&label).is_some() {
            let _ = report_native("Emit native event", app.emit_to(&label, "reminder-triggered", event.clone()));
        }
    }

    // 主窗口必须先收到事件，即使可选的系统通知发送失败，也不能留下过期倒计时。
    let _ = report_native("Emit native event", app.emit_to("main", "reminder-triggered", event.clone()));

    if settings.system_notification_enabled {
        let title = i18n::notification_title(settings.language, &event);
        #[cfg(target_os = "windows")]
        let result = if option_env!("ZHIXIE_STORE_BUILD").is_some() {
            notification::show_packaged(title, &event.title)
        } else {
            app.notification()
                .builder()
                .title(title)
                .body(&event.title)
                .show()
                .map_err(|error| error.to_string())
        };
        #[cfg(not(target_os = "windows"))]
        let result = app
            .notification()
            .builder()
            .title(title)
            .body(&event.title)
            .show()
            .map_err(|error| error.to_string());
        result.map_err(|error| i18n::notification_send_failed(settings.language, &error))?;
    }

    Ok(())
}

#[derive(Debug)]
enum SchedulerError {
    Persistence(String),
    State(String),
}

fn process_due(app: &AppHandle, state: &AppState) -> Result<(), String> {
    if state.0.native_errors.lock().map_err(lock_error)?.persistence_error.is_some() {
        return Ok(());
    }
    match collect_due_reminders(state, Local::now()) {
        Ok(events) => enqueue_reminders(state, events)?,
        Err(SchedulerError::State(error)) => return Err(error),
        Err(SchedulerError::Persistence(error)) => {
            eprintln!("Scheduler persistence failed: {error}");
            state.0.native_errors.lock().map_err(lock_error)?.persistence_error = Some(error.clone());
            let _ = report_native("Emit native event", app.emit_to("main", "persistence-failed", Some(error)));
            return Ok(());
        }
    }
    drain_reminder_queue(app, state)?;
    Ok(())
}

fn collect_due_reminders(state: &AppState, now: DateTime<Local>) -> Result<Vec<QueuedReminder>, SchedulerError> {
    let mut triggered = Vec::new();
    let mut changed = false;
    {
        let mut current = state.0.data.lock().map_err(|error| SchedulerError::State(lock_error(error)))?;
        let mut data = current.clone();
        for reminder in &mut data.reminders {
            if !reminder.enabled {
                continue;
            }
            let Some(next_value) = reminder.next_trigger_at.as_deref() else {
                continue;
            };
            let Ok(next) = parse_datetime(next_value) else {
                continue;
            };
            if next > now {
                continue;
            }
            if reminder.power_action.is_none() || should_trigger_power_action(next, now) {
                triggered.push(QueuedReminder {
                    due_at: next,
                    event: ReminderTriggeredEvent {
                        session_id: 0,
                        id: reminder.id.clone(),
                        title: reminder.title.clone(),
                        reminder_type: reminder.reminder_type.clone(),
                        is_rest: false,
                        power_action: reminder.power_action.clone(),
                        is_test: false,
                        rest_started_at_ms: None,
                        power_deadline_ms: None,
                    },
                });
            }
            changed = true;
            match reminder.reminder_type {
                ReminderType::Once => {
                    reminder.enabled = false;
                    reminder.next_trigger_at = None;
                }
                ReminderType::Daily | ReminderType::Weekly | ReminderType::Monthly => {
                    reminder.next_trigger_at = next_recurring(reminder, now).ok();
                }
                ReminderType::Interval => {}
            }
        }
        // Commit scheduled deadlines before changing rest state or emitting any reminders.
        if changed {
            commit_app_data(&state.0.data_path, &mut current, data.clone()).map_err(SchedulerError::Persistence)?;
        }
        if data.settings.rest_enabled {
            if !state.0.rest_active.load(Ordering::SeqCst)
                && !state.0.reminder_queue.lock().map_err(|error| SchedulerError::State(lock_error(error)))?.iter().any(|item| item.event.is_rest) {
                let due = {
                    let mut next_rest = state
                        .0
                        .rest_next
                        .lock().map_err(|error| SchedulerError::State(lock_error(error)))?;
                    if next_rest.is_none() {
                        *next_rest = Some(
                            now + Duration::minutes(data.settings.rest_interval_minutes as i64),
                        );
                        None
                    } else if next_rest.is_some_and(|value| value <= now) {
                        next_rest.take()
                    } else {
                        None
                    }
                };
                if let Some(due_at) = due {
                    triggered.push(QueuedReminder {
                        due_at,
                        event: ReminderTriggeredEvent {
                            session_id: 0,
                            id: REST_ID.to_string(),
                            title: data.settings.rest_message.clone(),
                            reminder_type: ReminderType::Interval,
                            is_rest: true,
                            power_action: None,
                            is_test: false,
                            rest_started_at_ms: None,
                            power_deadline_ms: None,
                        },
                    });
                    *state
                        .0
                        .rest_next
                        .lock().map_err(|error| SchedulerError::State(lock_error(error)))? = None;
                    state.0.rest_round_pending.store(true, Ordering::SeqCst);
                }
            }
        }
    }
    Ok(triggered)
}

fn clear_persistence_error(app: &AppHandle, state: &AppState) {
    if state.0.native_errors.lock().expect("native errors lock poisoned").persistence_error.take().is_some() {
        let _ = report_native("Emit native event", app.emit_to("main", "persistence-failed", Option::<String>::None));
    }
}

fn enqueue_reminders(state: &AppState, events: Vec<QueuedReminder>) -> Result<(), String> {
    let mut queue = state.0.reminder_queue.lock().map_err(lock_error)?;
    queue.extend(events);
    queue.make_contiguous().sort_by(|a, b| a.due_at.cmp(&b.due_at)
        .then_with(|| a.event.is_rest.cmp(&b.event.is_rest))
        .then_with(|| a.event.id.cmp(&b.event.id)));
    Ok(())
}

fn next_queued_reminder(state: &AppState, now: DateTime<Local>) -> Result<Option<ReminderTriggeredEvent>, String> {
    if state.0.active_reminder.lock().map_err(lock_error)?.is_some() {
        return Ok(None);
    }
    let mut queue = state.0.reminder_queue.lock().map_err(lock_error)?;
    while let Some(queued) = queue.pop_front() {
        if queued.event.is_test || queued.event.power_action.is_none()
            || should_trigger_power_action(queued.due_at, now) {
            return Ok(Some(queued.event));
        }
    }
    Ok(None)
}

// 调用方持有窗口操作锁；活动会话结束后调度器在下一轮继续队列。
fn drain_reminder_queue(app: &AppHandle, state: &AppState) -> Result<(), String> {
    while let Some(event) = next_queued_reminder(state, Local::now())? {
        if let Err(error) = dispatch_trigger(app, state, event) {
            let _ = report_native("Emit native event", app.emit_to("main", "notification-failed", error));
        }
    }
    Ok(())
}

fn spawn_scheduler(app: AppHandle, state: AppState) {
    if state.0.scheduler_started.swap(true, Ordering::SeqCst) {
        return;
    }
    state.0.scheduler_stop.store(false, Ordering::SeqCst);
    thread::spawn(move || {
        while !state.0.scheduler_stop.load(Ordering::SeqCst) {
            let result = guarded(|| {
                let _operation = state
                    .0
                    .window_operations
                    .lock().map_err(lock_error)?;
                if !state.0.paused.load(Ordering::SeqCst) {
                    process_due(&app, &state)?;
                }
                reclaim_idle_windows(&app, &state)?;
                Ok(())
            });
            if let Err(error) = result {
                eprintln!("Scheduler stopped: {error}");
                if let Err(record_error) = record_scheduler_failure(&state, &error) {
                    eprintln!("Failed to record scheduler failure: {record_error}");
                }
                let _ = report_native("Report scheduler failure", app.emit_to("main", "scheduler-failed", &error));
                break;
            }
            thread::sleep(SCHEDULER_INTERVAL);
        }
        state.0.scheduler_started.store(false, Ordering::SeqCst);
    });
}

fn record_scheduler_failure(state: &AppState, error: &str) -> Result<(), String> {
    state.0.scheduler_stop.store(true, Ordering::SeqCst);
    *state.0.scheduler_error.lock().map_err(lock_error)? = Some(error.to_string());
    Ok(())
}

fn ensure_scheduler_healthy(state: &AppState) -> Result<(), String> {
    match state.0.scheduler_error.lock().map_err(lock_error)?.as_ref() {
        Some(error) => Err(format!("Scheduler stopped; restart the application: {error}")),
        None => Ok(()),
    }
}

#[tauri::command]
async fn load_data(app: AppHandle, state: State<'_, AppState>) -> Result<AppData, String> {
    let state = state.inner().clone();
    blocking_command(move || {
        let data = state.0.data.lock().map_err(lock_error)?;
        if state.0.native_errors.lock().map_err(lock_error)?.persistence_error.is_some() {
            ensure_scheduler_healthy(&state)?;
            write_json(&state.0.data_path, &data)?;
            clear_persistence_error(&app, &state);
        }
        Ok(data.clone())
    }).await
}

#[tauri::command]
async fn save_data(
    mut data: AppData,
    popup_fullscreen: Option<bool>,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<AppData, String> {
    let state = state.inner().clone();
    blocking_command(move || {
        let _operation = state
            .0
            .window_operations
            .lock().map_err(lock_error)?;
        let mut current = state.0.data.lock().map_err(lock_error)?;
        // 普通保存保留后端当前模式，只有主窗口明确切换时才覆盖它。
        data.settings.popup_fullscreen = popup_fullscreen.unwrap_or(current.settings.popup_fullscreen);
        startup::preserve_local_state(&mut data, &current);
        let current_by_id: HashMap<_, _> = current.reminders.iter().map(|item| (&item.id, item)).collect();
        for reminder in &mut data.reminders {
            if current_by_id.get(&reminder.id).is_none_or(|old| !same_reminder_plan(old, reminder)) {
                reminder.next_trigger_at = None;
            }
        }
        validate_and_normalize(&mut data)?;
        let rest_enabled_changed = current.settings.rest_enabled != data.settings.rest_enabled;
        let rest_interval_changed =
            current.settings.rest_interval_minutes != data.settings.rest_interval_minutes;
        let changed_ids = changed_reminder_ids(&current.reminders, &data.reminders);
        commit_app_data(&state.0.data_path, &mut current, data.clone())?;
        clear_persistence_error(&app, &state);
        if rest_enabled_changed && !data.settings.rest_enabled {
            state.0.rest_active.store(false, Ordering::SeqCst);
            state.0.rest_round_pending.store(false, Ordering::SeqCst);
            *state
                .0
                .rest_next
                .lock().map_err(lock_error)? = None;
        } else if (rest_enabled_changed || rest_interval_changed)
            && !state.0.rest_round_pending.load(Ordering::SeqCst)
        {
            *state
                .0
                .rest_next
                .lock().map_err(lock_error)? = None;
        }
        drop(current);
        state.0.reminder_queue.lock().map_err(lock_error)?
            .retain(|item| !changed_ids.contains(&item.event.id)
                && (!item.event.is_rest || data.settings.rest_enabled));
        let active = state.0.active_reminder.lock().map_err(lock_error)?.clone();
        if let Some(active) = active.filter(|event| changed_ids.contains(&event.id)) {
            close_reminder_session(&app, &state, active.session_id);
        }
        let _ = report_native("Update tray menu", update_tray_menu(
            &app,
            data.settings.language,
        ));
        #[cfg(target_os = "macos")]
        apply_application_menu(&app, data.settings.language);
        sync_reminder_settings(&app, &data.settings);
        if rest_enabled_changed && !data.settings.rest_enabled {
            cancel_active_rest_reminder(&app, &state);
        }
        if rest_enabled_changed || rest_interval_changed {
            emit_rest_timer_updated(&app, &state);
        }
        Ok(data)
    }).await
}

fn same_reminder_plan(a: &Reminder, b: &Reminder) -> bool {
    a.title == b.title && a.reminder_type == b.reminder_type && a.trigger_at == b.trigger_at
        && a.time == b.time && a.weekdays == b.weekdays && a.month_days == b.month_days
        && a.enabled == b.enabled && a.power_action == b.power_action
}

fn changed_reminder_ids(previous: &[Reminder], next: &[Reminder]) -> HashSet<String> {
    let next_by_id: HashMap<_, _> = next.iter().map(|item| (&item.id, item)).collect();
    previous.iter().filter(|old| next_by_id.get(&old.id)
        .is_none_or(|new| !same_reminder_plan(old, new))).map(|item| item.id.clone()).collect()
}

#[tauri::command(rename = "start_scheduler")]
fn start_scheduler_command(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    ensure_scheduler_healthy(&state)?;
    spawn_scheduler(app, state.inner().clone());
    Ok(())
}

#[tauri::command(rename = "stop_scheduler")]
fn stop_scheduler_command(state: State<'_, AppState>) -> Result<(), String> {
    state.0.scheduler_stop.store(true, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
fn set_scheduler_paused(paused: bool, state: State<'_, AppState>) -> Result<(), String> {
    state.0.paused.store(paused, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
async fn snooze_reminder(
    id: String,
    seconds: u32,
    session_id: u64,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let state = state.inner().clone();
    blocking_command(move || {
        let _operation = state
            .0
            .window_operations
            .lock().map_err(lock_error)?;
        if !active_reminder_matches(&state, &id, session_id) {
            return Ok(());
        }
        if state.0.active_reminder.lock().map_err(lock_error)?
            .as_ref().is_some_and(|event| event.power_action.is_some()) {
            return Err("Scheduled actions cannot be snoozed".to_string());
        }
        if id == REST_ID || id == TEST_REST_ID {
            if snooze_rest_round(&state, seconds) {
                emit_rest_timer_updated(&app, &state);
            }
            close_reminder_session(&app, &state, session_id);
            return Ok(());
        }
        let mut data = state.0.data.lock().map_err(lock_error)?;
        let delay = Duration::seconds(seconds.max(1) as i64);
        let mut candidate = data.clone();
        if let Some(reminder) = candidate.reminders.iter_mut().find(|item| item.id == id) {
            reminder.enabled = true;
            reminder.next_trigger_at = Some((Local::now() + delay).to_rfc3339());
            commit_app_data(&state.0.data_path, &mut data, candidate)?;
        }
        drop(data);
        close_reminder_session(&app, &state, session_id);
        Ok(())
    }).await
}

#[tauri::command]
async fn dismiss_reminder(
    id: String,
    session_id: u64,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let state = state.inner().clone();
    blocking_command(move || {
        let _operation = state
            .0
            .window_operations
            .lock().map_err(lock_error)?;
        if !active_reminder_matches(&state, &id, session_id) {
            return Ok(());
        }
        if id == REST_ID || id == TEST_REST_ID {
            if complete_rest_round(&state) {
                emit_rest_timer_updated(&app, &state);
            }
        }
        close_reminder_session(&app, &state, session_id);
        Ok(())
    }).await
}

#[tauri::command]
async fn get_active_reminder(
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<Option<ReminderTriggeredEvent>, String> {
    let state = state.inner().clone();
    blocking_command(move || {
        let _operation = state
            .0
            .window_operations
            .lock().map_err(lock_error)?;
        if !state
            .0
            .reminder_targets
            .lock().map_err(lock_error)?
            .contains(window.label())
        {
            return Ok(None);
        }
        Ok(state
            .0
            .active_reminder
            .lock().map_err(lock_error)?
            .clone())
    }).await
}

#[tauri::command]
async fn show_reminder(
    window: WebviewWindow,
    session_id: u64,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let state = state.inner().clone();
    blocking_command(move || {
        let _operation = state
            .0
            .window_operations
            .lock().map_err(lock_error)?;
        let active = state
            .0
            .active_reminder
            .lock().map_err(lock_error)?
            .clone();
        let current = active
            .as_ref()
            .is_some_and(|event| event.session_id == session_id);
        if !current
            || !state
                .0
                .reminder_targets
                .lock().map_err(lock_error)?
                .contains(window.label())
        {
            return Ok(false);
        }
        // Restore only after content and geometry are ready; unminimize itself may show a window.
        if let Err(error) = window.unminimize().and_then(|_| window.show()) {
            if active.as_ref().is_some_and(|event| event.is_rest) && complete_rest_round(&state)
            {
                emit_rest_timer_updated(&app, &state);
            }
            close_reminder_session(&app, &state, session_id);
            let message = error.to_string();
            let _ = report_native("Emit native event", app.emit_to("main", "notification-failed", &message));
            return Err(message);
        }
        if window.label() == REMINDER_LABEL || window.label() == WINDOWED_REMINDER_LABEL {
            window.set_focus().map_err(|error| error.to_string())?;
        }
        Ok(true)
    }).await
}

#[tauri::command]
async fn hide_idle_window(window: WebviewWindow, state: State<'_, AppState>) -> Result<(), String> {
    let state = state.inner().clone();
    blocking_command(move || {
        let _operation = state
            .0
            .window_operations
            .lock().map_err(lock_error)?;
        if window.label() == "main"
            || (is_reminder_window_label(window.label())
                && !state
                    .0
                    .reminder_targets
                    .lock().map_err(lock_error)?
                    .contains(window.label()))
        {
            hide_cached_window(&window, &state)?;
        }
        Ok(())
    }).await
}

#[tauri::command]
async fn get_rest_timer_status(state: State<'_, AppState>) -> Result<RestTimerStatus, String> {
    let state = state.inner().clone();
    blocking_command(move || {
        Ok(rest_timer_status(&state))
    }).await
}

#[tauri::command]
fn get_native_errors(state: State<'_, AppState>) -> Result<NativeErrors, String> {
    let mut errors = state.0.native_errors.lock().map_err(lock_error)?.clone();
    errors.scheduler_error = state.0.scheduler_error.lock().map_err(lock_error)?.clone();
    Ok(errors)
}

#[cfg(target_os = "windows")]
fn windows_power_command(action: &PowerAction, system_root: &Path) -> (PathBuf, Vec<&'static str>) {
    let system32 = system_root.join("System32");
    match action {
        PowerAction::Lock => (
            system32.join("rundll32.exe"),
            vec!["user32.dll,LockWorkStation"],
        ),
        PowerAction::Shutdown => (system32.join("shutdown.exe"), vec!["/s", "/t", "0"]),
        PowerAction::Restart => (system32.join("shutdown.exe"), vec!["/r", "/t", "0"]),
    }
}

fn execute_power_action_impl(action: &PowerAction) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let system_root = std::env::var_os("SystemRoot")
            .ok_or_else(|| "Failed to resolve the Windows system directory".to_string())?;
        let (program, args) = windows_power_command(action, &PathBuf::from(system_root));
        return run_system_command(Command::new(program).args(args));
    }

    #[cfg(target_os = "macos")]
    {
        if *action == PowerAction::Lock {
            return run_system_command(Command::new(
                "/System/Library/CoreServices/Menu Extras/User.menu/Contents/Resources/CGSession",
            )
            .arg("-suspend"));
        }
        let script = if *action == PowerAction::Shutdown {
            "tell application \"System Events\" to shut down"
        } else {
            "tell application \"System Events\" to restart"
        };
        return run_system_command(Command::new("/usr/bin/osascript").args(["-e", script]));
    }

    #[allow(unreachable_code)]
    Err("The current platform does not support this scheduled action".to_string())
}

fn run_system_command(command: &mut Command) -> Result<(), String> {
    let mut child = command.spawn().map_err(|error| format!("Failed to start system action: {error}"))?;
    wait_for_system_command(&mut child, POWER_COMMAND_TIMEOUT)
}

fn wait_for_system_command(child: &mut Child, timeout: StdDuration) -> Result<(), String> {
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return Ok(()),
            Ok(Some(status)) => return Err(format!("System action exited with {status}")),
            Ok(None) if Instant::now() < deadline => thread::sleep(StdDuration::from_millis(20)),
            result => {
                let error = match result {
                    Err(error) => format!("Failed to wait for system action: {error}"),
                    _ => "System action timed out".to_string(),
                };
                // Reap the child on errors too; dropping Child does not terminate it.
                if let Err(error) = child.kill() { eprintln!("Failed to terminate system command: {error}"); }
                if let Err(error) = child.wait() { eprintln!("Failed to reap system command: {error}"); }
                return Err(error);
            }
        }
    }
}

#[tauri::command]
async fn execute_power_action(
    action: PowerAction,
    session_id: u64,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let state = state.inner().clone();
    blocking_command(move || {
        let _operation = state
            .0
            .window_operations
            .lock().map_err(lock_error)?;
        let matches = state
            .0
            .active_reminder
            .lock().map_err(lock_error)?
            .as_ref()
            .is_some_and(|event| {
                event.session_id == session_id && event.power_action.as_ref() == Some(&action)
            });
        if !matches {
            return Ok(false);
        }
        if state
            .0
            .power_action_session
            .compare_exchange(0, session_id, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Ok(false);
        }
        drop(_operation);
        if let Err(error) = execute_power_action_impl(&action) {
            let _ = state.0.power_action_session.compare_exchange(
                session_id,
                0,
                Ordering::SeqCst,
                Ordering::SeqCst,
            );
            return Err(error);
        }
        let _operation = state.0.window_operations.lock().map_err(lock_error)?;
        close_reminder_session(&app, &state, session_id);
        Ok(true)
    }).await
}

fn test_reminder_event(settings: &AppSettings, kind: TestReminderKind) -> ReminderTriggeredEvent {
    match kind {
        TestReminderKind::Event => ReminderTriggeredEvent {
            session_id: 0,
            id: "__test_event__".to_string(),
            title: i18n::test_notification(settings.language).to_string(),
            reminder_type: ReminderType::Once,
            is_rest: false,
            power_action: None,
            is_test: true,
            rest_started_at_ms: None,
            power_deadline_ms: None,
        },
        TestReminderKind::Rest => ReminderTriggeredEvent {
            session_id: 0,
            id: TEST_REST_ID.to_string(),
            title: settings.rest_message.clone(),
            reminder_type: ReminderType::Interval,
            is_rest: true,
            power_action: None,
            is_test: true,
            rest_started_at_ms: None,
            power_deadline_ms: None,
        },
    }
}

#[tauri::command]
async fn test_reminder(
    kind: TestReminderKind,
    reminder: Option<Reminder>,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let state = state.inner().clone();
    blocking_command(move || {
        let _operation = state
            .0
            .window_operations
            .lock().map_err(lock_error)?;
        let settings = app_data(&state).settings;
        let mut event = test_reminder_event(&settings, kind);
        if let Some(reminder) = reminder {
            if event.is_rest {
                return Err("Break preview cannot include a scheduled reminder".to_string());
            }
            let mut preview = AppData { version: DATA_VERSION, autostart_owner: None, settings: settings.clone(), reminders: vec![reminder] };
            validate_and_normalize(&mut preview)?;
            event.title = preview.reminders[0].title.clone();
            event.power_action = preview.reminders[0].power_action.clone();
            event.reminder_type = preview.reminders[0].reminder_type.clone();
        }
        // 弹窗窗口会复用，触发前必须把最新设置同步过去，否则外观类设置不会生效。
        sync_reminder_settings(&app, &settings);
        enqueue_reminders(&state, vec![QueuedReminder { due_at: Local::now(), event }])?;
        drain_reminder_queue(&app, &state)?;
        Ok(())
    }).await
}

#[tauri::command]
async fn import_data(
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<AppData>, String> {
    if window.label() != "main" { return Err("Import is only available in the main window".into()); }
    let state = state.inner().clone();
    blocking_command(move || {
        let Some(path) = app.dialog().file().add_filter("JSON", &["json"]).blocking_pick_file() else {
            return Ok(None);
        };
        let path = path.into_path().map_err(|error| error.to_string())?;
        let _operation = state
            .0
            .window_operations
            .lock().map_err(lock_error)?;
        let mut data = read_import_file(&path)?;
        let mut current = state.0.data.lock().map_err(lock_error)?;
        startup::preserve_local_state(&mut data, &current);
        commit_app_data(&state.0.data_path, &mut current, data.clone())?;
        state.0.rest_active.store(false, Ordering::SeqCst);
        state.0.rest_round_pending.store(false, Ordering::SeqCst);
        *state
            .0
            .rest_next
            .lock().map_err(lock_error)? = None;
        state.0.paused.store(false, Ordering::SeqCst);
        drop(current);
        reset_reminder_session(&app, &state, "reminders-reset");
        clear_persistence_error(&app, &state);
        let _ = report_native("Update tray menu", update_tray_menu(&app, data.settings.language));
        #[cfg(target_os = "macos")]
        apply_application_menu(&app, data.settings.language);
        sync_reminder_settings(&app, &data.settings);
        emit_rest_timer_updated(&app, &state);
        Ok(Some(data))
    }).await
}

#[tauri::command]
async fn export_data(window: WebviewWindow, app: AppHandle, state: State<'_, AppState>) -> Result<bool, String> {
    if window.label() != "main" { return Err("Export is only available in the main window".into()); }
    let state = state.inner().clone();
    blocking_command(move || {
        let Some(path) = app.dialog().file().add_filter("JSON", &["json"]).set_file_name("知歇-backup.json").blocking_save_file() else {
            return Ok(false);
        };
        let path = path.into_path().map_err(|error| error.to_string())?;
        validate_json_path(&path)?;
        let mut data = app_data(&state);
        data.autostart_owner = None;
        write_json(&path, &data)?;
        Ok(true)
    }).await
}

fn validate_json_path(path: &Path) -> Result<(), String> {
    if path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("json")) {
        Ok(())
    } else {
        Err("Please choose a JSON file".into())
    }
}

fn read_import_file(path: &Path) -> Result<AppData, String> {
    validate_json_path(path)?;
    if !path.is_file() { return Err("Import file does not exist".into()); }
    let content = fs::read_to_string(path).map_err(|error| format!("Failed to read import file: {error}"))?;
    let mut data: AppData = serde_json::from_str(&content).map_err(|error| format!("Invalid import file: {error}"))?;
    validate_and_normalize(&mut data)?;
    Ok(data)
}

/// Copy a user-picked image into the configuration folder and return it as a data URL.
/// The file lives next to `zhixie.json`, so development and installed builds share one copy.
#[tauri::command]
async fn import_popup_image(
    source: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let state = state.inner().clone();
    blocking_command(move || {
        let _image = state.0.popup_image_operations.lock().map_err(lock_error)?;
        let origin = PathBuf::from(&source);
        if !origin.is_file() {
            return Err("The selected image does not exist".to_string());
        }
        image_extension(&origin)?;
        let directory = config_directory(&app)?;
        fs::create_dir_all(&directory)
            .map_err(|error| format!("Failed to create the app configuration directory: {error}"))?;
        // 固定文件名，用户选什么图都存成同一个名字，配置里不需要记录来源。
        let target = directory.join("popup-background.img");
        let data_url = save_popup_image(&origin, &target)?;
        emit_to_reminder_windows(&app, "popup-image-updated", ());
        Ok(data_url)
    }).await
}

fn read_bounded_image(path: &Path) -> Result<Vec<u8>, String> {
    let file = fs::File::open(path).map_err(|error| format!("Failed to open image: {error}"))?;
    if file.metadata().map_err(|error| error.to_string())?.len() > MAX_POPUP_IMAGE_BYTES {
        return Err("Image must not exceed 10 MiB".to_string());
    }
    let mut bytes = Vec::new();
    // Also bound the actual read in case the file grows after the metadata check.
    file.take(MAX_POPUP_IMAGE_BYTES + 1).read_to_end(&mut bytes).map_err(|error| error.to_string())?;
    if bytes.len() as u64 > MAX_POPUP_IMAGE_BYTES {
        return Err("Image must not exceed 10 MiB".to_string());
    }
    Ok(bytes)
}

// 标记与图片分开保存，用户移除图片后不会在下次启动时重新补入。
fn initialize_popup_image(directory: &Path, settings: &mut AppSettings) -> Result<(), String> {
    if directory.join(POPUP_IMAGE_INITIALIZED).exists() {
        return Ok(());
    }
    if !directory.join("popup-background.img").is_file() {
        restore_default_popup_image(directory)?;
        let defaults = AppSettings::default();
        settings.popup_background_fit = defaults.popup_background_fit;
        settings.popup_background_scale = defaults.popup_background_scale;
        settings.popup_background_offset_x = defaults.popup_background_offset_x;
        settings.popup_background_offset_y = defaults.popup_background_offset_y;
    } else {
        atomic_write(&directory.join(POPUP_IMAGE_INITIALIZED), b"1").map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn restore_default_popup_image(directory: &Path) -> Result<(), String> {
    fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    atomic_write(&directory.join("popup-background.img"), DEFAULT_POPUP_IMAGE).map_err(|error| error.to_string())?;
    atomic_write(&directory.join(POPUP_IMAGE_INITIALIZED), b"1").map_err(|error| error.to_string())
}

#[tauri::command]
async fn reset_popup_image(app: AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    let state = state.inner().clone();
    blocking_command(move || {
        let _image = state.0.popup_image_operations.lock().map_err(lock_error)?;
        restore_default_popup_image(&config_directory(&app)?)?;
        emit_to_reminder_windows(&app, "popup-image-updated", ());
        Ok(format!("data:image/png;base64,{}", base64_encode(DEFAULT_POPUP_IMAGE)))
    }).await
}

#[tauri::command]
async fn is_popup_image_default(app: AppHandle, state: State<'_, AppState>) -> Result<bool, String> {
    let state = state.inner().clone();
    blocking_command(move || {
        let _image = state.0.popup_image_operations.lock().map_err(lock_error)?;
        let target = config_directory(&app)?.join("popup-background.img");
        if !target.is_file() { return Ok(false); }
        Ok(read_bounded_image(&target)? == DEFAULT_POPUP_IMAGE)
    }).await
}

fn save_popup_image(origin: &Path, target: &Path) -> Result<String, String> {
    image_extension(origin)?;
    let bytes = read_bounded_image(origin)?;
    let mime = image_mime(&bytes)?;
    atomic_write(target, &bytes).map_err(|error| format!("Failed to save image: {error}"))?;
    Ok(format!("data:{mime};base64,{}", base64_encode(&bytes)))
}

fn image_extension(path: &Path) -> Result<String, String> {
    path.extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .filter(|value| {
            matches!(
                value.as_str(),
                "png" | "jpg" | "jpeg" | "webp" | "gif" | "bmp" | "avif"
            )
        })
        .ok_or_else(|| "Unsupported image format".to_string())
}

// 固定 .img 文件没有格式信息，按文件签名识别也兼容已经保存的图片。
fn image_mime(bytes: &[u8]) -> Result<&'static str, String> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Ok("image/png");
    }
    if bytes.starts_with(b"\xff\xd8\xff") {
        return Ok("image/jpeg");
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return Ok("image/gif");
    }
    if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        return Ok("image/webp");
    }
    if bytes.starts_with(b"BM") {
        return Ok("image/bmp");
    }
    if bytes.len() >= 16 && &bytes[4..8] == b"ftyp" {
        let size = u32::from_be_bytes(bytes[..4].try_into().unwrap()) as usize;
        if size >= 16 && size <= bytes.len()
            && std::iter::once(&bytes[8..12]).chain(bytes[16..size].chunks_exact(4))
                .any(|brand| brand == b"avif" || brand == b"avis")
        {
            return Ok("image/avif");
        }
    }
    Err("Unsupported or invalid image format".to_string())
}

/// Read the stored background image back as a data URL. An absent file simply means no background.
#[tauri::command]
async fn read_popup_image(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let state = state.inner().clone();
    blocking_command(move || {
        let _image = state.0.popup_image_operations.lock().map_err(lock_error)?;
        let directory = match config_directory(&app) {
            Ok(directory) => directory,
            Err(_) => return Ok(None),
        };
        // 固定文件名，找不到就代表用户还没设置背景。
        let target = directory.join("popup-background.img");
        if !target.is_file() {
            return Ok(None);
        }
        let bytes = read_bounded_image(&target)?;
        let mime = image_mime(&bytes)?;
        Ok(Some(format!(
            "data:{};base64,{}",
            mime,
            base64_encode(&bytes)
        )))
    }).await
}

fn base64_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let triple = (b0 << 16) | (b1 << 8) | b2;
        output.push(ALPHABET[(triple >> 18) as usize & 0x3f] as char);
        output.push(ALPHABET[(triple >> 12) as usize & 0x3f] as char);
        output.push(if chunk.len() > 1 {
            ALPHABET[(triple >> 6) as usize & 0x3f] as char
        } else {
            '='
        });
        output.push(if chunk.len() > 2 {
            ALPHABET[triple as usize & 0x3f] as char
        } else {
            '='
        });
    }
    output
}

#[tauri::command]
async fn clear_popup_image(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let state = state.inner().clone();
    blocking_command(move || {
        let _image = state.0.popup_image_operations.lock().map_err(lock_error)?;
        let directory = match config_directory(&app) {
            Ok(directory) => directory,
            Err(_) => return Ok(()),
        };
        let target = directory.join("popup-background.img");
        atomic_write(&directory.join(POPUP_IMAGE_INITIALIZED), b"1").map_err(|error| error.to_string())?;
        if target.is_file() {
            fs::remove_file(&target).map_err(|error| format!("Failed to remove the image: {error}"))?;
        }
        // 清理旧版本按扩展名保存的残留文件。
        if let Ok(entries) = fs::read_dir(&directory) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with("popup-background.") && name != "popup-background.img" {
                    let _ = fs::remove_file(entry.path());
                }
            }
        }
        emit_to_reminder_windows(&app, "popup-image-updated", ());
        Ok(())
    }).await
}

fn tray_menu(
    app: &AppHandle,
    language: Language,
) -> tauri::Result<Menu<tauri::Wry>> {
    let (show_text, quit_text, about_text) = i18n::tray_labels(language);
    let show = MenuItemBuilder::with_id("show", show_text).build(app)?;
    let about = MenuItemBuilder::with_id("about", about_text).build(app)?;
    let quit = MenuItemBuilder::with_id("quit", quit_text).build(app)?;
    MenuBuilder::new(app)
        .item(&show)
        .item(&quit)
        .separator()
        .item(&about)
        .build()
}

fn update_tray_menu(app: &AppHandle, language: Language) -> tauri::Result<()> {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        tray.set_menu(Some(tray_menu(app, language)?))?;
    }
    Ok(())
}

// 项目自定义主题含 System，映射为跟随系统的 None；原生标题栏只认 Dark/Light。
fn tauri_theme(theme: &Theme) -> Option<tauri::Theme> {
    match theme {
        Theme::Dark => Some(tauri::Theme::Dark),
        Theme::Light => Some(tauri::Theme::Light),
        Theme::System => None,
    }
}

/// 读系统当前是否深色（`AppleInterfaceStyle` = Dark）。
#[cfg(target_os = "macos")]
fn system_prefers_dark() -> Result<bool, String> {
    use std::ffi::c_void;

    extern "C" {
        fn CFPreferencesCopyAppValue(key: *const c_void, app_id: *const c_void) -> *mut c_void;
        fn CFPreferencesAppSynchronize(app_id: *const c_void) -> u8;
        fn CFGetTypeID(value: *const c_void) -> usize;
        fn CFStringGetTypeID() -> usize;
        fn CFRelease(cf: *const c_void);
        fn CFStringCreateWithCString(
            alloc: *const c_void,
            c_str: *const std::ffi::c_char,
            encoding: u32,
        ) -> *mut c_void;
        fn CFStringCompare(a: *const c_void, b: *const c_void, options: u64) -> i32;
    }
    const UTF8: u32 = 0x0800_0100;
    unsafe {
        let key = CFStringCreateWithCString(
            std::ptr::null(),
            c"AppleInterfaceStyle".as_ptr(),
            UTF8,
        );
        let domain = CFStringCreateWithCString(
            std::ptr::null(),
            c"Apple Global Domain".as_ptr(),
            UTF8,
        );
        if key.is_null() || domain.is_null() {
            if !key.is_null() { CFRelease(key); }
            if !domain.is_null() { CFRelease(domain); }
            return Err("Failed to allocate system appearance preference strings".into());
        }
        if CFPreferencesAppSynchronize(domain) == 0 {
            CFRelease(key);
            CFRelease(domain);
            return Err("Failed to synchronize system appearance preferences".into());
        }
        let value = CFPreferencesCopyAppValue(key, domain);
        CFRelease(key);
        CFRelease(domain);
        // 缺省即浅色。
        if value.is_null() {
            return Ok(false);
        }
        if CFGetTypeID(value) != CFStringGetTypeID() {
            CFRelease(value);
            return Err("System appearance preference is not a string".into());
        }
        let dark = CFStringCreateWithCString(std::ptr::null(), c"Dark".as_ptr(), UTF8);
        if dark.is_null() {
            CFRelease(value);
            return Err("Failed to allocate dark appearance string".into());
        }
        let equal = CFStringCompare(value, dark, 0) == 0;
        CFRelease(dark);
        CFRelease(value);
        Ok(equal)
    }
}

/// 把主题里的 System 解析成实际生效的深浅色。
///
/// 窗口背景色必须是具体颜色，写不了"跟随系统"，所以 System 在这里落地成
/// 当前系统的实际配色；系统外观变化时由 `start_system_theme_watcher` 重新应用。
#[cfg(target_os = "macos")]
fn resolve_theme(theme: &Theme) -> Theme {
    match theme {
        Theme::System => {
            if report_native("Read system appearance", system_prefers_dark()).unwrap_or(false) {
                Theme::Dark
            } else {
                Theme::Light
            }
        }
        other => other.clone(),
    }
}

/// 把某一主题对应的原生外观（appearance + 窗口背景色）同步到单个窗口。
///
/// macOS 关键点：标题栏是 vibrancy 材质，其颜色由 **窗口背景色** 决定，
/// 而非 NSApp.appearance。appearance 的实时过渡会被 setLevel、窗口复用等动作打断，
/// 导致标题栏卡在旧配色。所以这里同时设置背景色，让标题栏颜色变成显式赋值的结果。
#[cfg(target_os = "macos")]
fn apply_window_appearance(window: &WebviewWindow, theme: &Theme) {
    let resolved = resolve_theme(theme);
    // System 保持 None 让窗口继续跟随系统外观，只有显式 Dark/Light 才钉死。
    report_window(window, "set_theme", window.set_theme(match theme {
        Theme::System => None,
        _ => tauri_theme(&resolved),
    }));
    // 透明弹窗自身是透明的，不能给窗口上色，否则会盖掉背景图。
    if !is_transparent_popup(window.label()) {
        let background = match resolved {
            Theme::Light => WINDOW_BG_LIGHT,
            _ => WINDOW_BG_DARK,
        };
        report_window(window, "set_background_color", window.set_background_color(Some(background)));
    }
}

#[cfg(not(target_os = "macos"))]
fn apply_window_appearance(window: &WebviewWindow, theme: &Theme) {
    report_window(window, "set_theme", window.set_theme(tauri_theme(theme)));
}

/// 把主题同步到主窗口与所有提醒弹窗，用于设置变更与启动时统一对齐。
fn apply_theme_to_windows(app: &AppHandle, theme: &Theme) {
    if let Some(window) = app.get_webview_window("main") {
        apply_window_appearance(&window, theme);
    }
    for window in reminder_windows(app) {
        apply_window_appearance(&window, theme);
    }
}

/// 轮询系统外观，跟随系统模式下系统深浅色切换时同步窗口背景色。
///
/// 只在偏好为 System 时才真正生效；显式 Dark/Light 不受影响。
#[cfg(target_os = "macos")]
fn start_system_theme_watcher(app: AppHandle, state: AppState) {
    let result = thread::Builder::new()
        .name("system-theme-watcher".into())
        .spawn(move || {
            let mut last = match report_native("Read initial system appearance", system_prefers_dark()) {
                Ok(value) => value,
                Err(_) => return,
            };
            while !state.0.scheduler_stop.load(Ordering::SeqCst) {
                thread::sleep(THEME_POLL_INTERVAL);
                let now = match report_native("Read system appearance", system_prefers_dark()) {
                    Ok(value) => value,
                    Err(_) => continue,
                };
                if now == last {
                    continue;
                }
                last = now;
                // 设置可能被并发改写，取锁后再判断当前偏好。
                let theme = match state.0.data.lock() {
                    Ok(data) => data.settings.theme.clone(),
                    Err(error) => { eprintln!("System theme watcher stopped: {error}"); return; }
                };
                if !matches!(theme, Theme::System) {
                    continue;
                }
                apply_theme_to_windows(&app, &Theme::System);
            }
        });
    let _ = report_native("Start system theme watcher", result);
}

/// macOS 上把窗口背景色钉到当前主题，创建时用，避免透明标题栏闪出默认灰。
#[cfg(target_os = "macos")]
fn background_color_for(theme: &Theme) -> tauri::window::Color {
    match resolve_theme(theme) {
        Theme::Light => WINDOW_BG_LIGHT,
        _ => WINDOW_BG_DARK,
    }
}

fn create_main_window(
    app: &AppHandle,
    theme: Option<tauri::Theme>,
    _app_theme: &Theme,
) -> Result<WebviewWindow, String> {
    let mut builder = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
        .devtools(false)
        .zoom_hotkeys_enabled(false)
        .on_new_window(|_, _| tauri::webview::NewWindowResponse::Deny)
        .title("知歇")
        .inner_size(MAIN_WINDOW_WIDTH, MAIN_WINDOW_HEIGHT)
        .min_inner_size(MAIN_WINDOW_WIDTH, MAIN_WINDOW_HEIGHT)
        .resizable(true)
        .center()
        .visible(false);
    // 主窗口创建时即按用户设置钉好主题，避免重建后回落到硬编码的深色。
    if let Some(theme) = theme {
        builder = builder.theme(Some(theme));
    }
    // 透明标题栏：只让材质透出窗口背景色，不加 FullSizeContentView，
    // 因此拖动 / 双击最大化还原 / 最小化 / 关闭 / 阴影圆角全部保持原生行为。
    #[cfg(target_os = "macos")]
    let builder = builder
        .title_bar_style(tauri::TitleBarStyle::Transparent)
        .background_color(background_color_for(_app_theme));
    builder
        .build()
        .map_err(|error| format!("Failed to create main window: {error}"))
}

fn ensure_main_window(
    app: &AppHandle,
    state: &AppState,
    navigation: Option<&str>,
) -> Result<WebviewWindow, String> {
    wait_for_window_destroyed(app, state, "main")?;
    state
        .0
        .window_cache
        .lock().map_err(lock_error)?
        .reuse("main");
    let app_theme = app_data(state).settings.theme;
    let window = match app.get_webview_window("main") {
        Some(window) => {
            // 缓存的窗口可能停留在上一次主题，显示前按当前设置同步原生标题栏。
            apply_window_appearance(&window, &app_theme);
            window
        }
        None => {
            state.0.main_ready.store(false, Ordering::SeqCst);
            let theme = tauri_theme(&app_theme);
            create_main_window(app, theme, &app_theme)?
        }
    };
    if let Some(navigation) = navigation {
        if state.0.main_ready.load(Ordering::SeqCst) {
            app.emit_to("main", "navigate-to", navigation)
                .map_err(|error| error.to_string())?;
        } else {
            *state
                .0
                .pending_navigation
                .lock().map_err(lock_error)? = Some(navigation.to_string());
        }
    }
    window.unminimize().map_err(|error| error.to_string())?;
    window.show().map_err(|error| error.to_string())?;
    state.0.main_visible.store(true, Ordering::SeqCst);
    window.set_focus().map_err(|error| error.to_string())?;
    Ok(window)
}

fn show_main_window(app: &AppHandle) {
    queue_window_action(app, |app, state| {
        ensure_main_window(app, state, None).map(|_| ())
    });
}

fn toggle_main_window(app: &AppHandle) {
    queue_window_action(app, |app, state| {
        if let Some(window) = app.get_webview_window("main") {
            if window.is_visible().unwrap_or(false) && !window.is_minimized().unwrap_or(false) {
                return hide_cached_window(&window, state);
            }
        }
        ensure_main_window(app, state, None).map(|_| ())
    });
}

fn show_about(app: &AppHandle) {
    queue_window_action(app, |app, state| {
        ensure_main_window(app, state, Some("about")).map(|_| ())
    });
}

#[tauri::command]
async fn open_reminder_settings(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let state = state.inner().clone();
    blocking_command(move || {
        let _operation = state
            .0
            .window_operations
            .lock().map_err(lock_error)?;
        ensure_main_window(&app, &state, Some("events")).map(|_| ())
    }).await
}

#[tauri::command]
async fn take_pending_navigation(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let state = state.inner().clone();
    blocking_command(move || {
        let _operation = state
            .0
            .window_operations
            .lock().map_err(lock_error)?;
        state.0.main_ready.store(true, Ordering::SeqCst);
        Ok(state
            .0
            .pending_navigation
            .lock().map_err(lock_error)?
            .take())
    }).await
}

fn setup_tray(app: &tauri::App, language: Language) -> tauri::Result<()> {
    let menu = tray_menu(app.handle(), language)?;
    TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .show_menu_on_left_click(false)
        // 托盘图标用带 12/128 留白的专用版本：菜单栏可用高度只有 18pt，
        // 全出血版本在这个尺寸下没有呼吸感，比系统图标更抢眼。
        .icon(tauri::include_image!("icons/tray.png"))
        .tooltip("知歇")
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::DoubleClick {
                    button: tauri::tray::MouseButton::Left,
                    ..
                }
            ) {
                toggle_main_window(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

/// macOS 的应用菜单默认是英文，这里按当前语言替换成中文。
/// 菜单栏只接受子菜单，平铺的菜单项在 macOS 上渲染行为未定义（时有时无），
/// 所以这里用标准的「应用子菜单 + 窗口子菜单」结构。
/// Windows 不受影响，Linux 保持发行版默认行为。
#[cfg(target_os = "macos")]
fn apply_application_menu(app: &tauri::AppHandle, language: Language) {
    use tauri::menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem, SubmenuBuilder};
    let chinese = language == Language::ZhCn;
    let label = |zh: &str, en: &str| if chinese { zh.to_string() } else { en.to_string() };
    let result = (|| -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
        let about = PredefinedMenuItem::about(app, Some("知歇"), None)?;
        let services = PredefinedMenuItem::services(app, Some(&label("服务", "Services")))?;
        let hide = PredefinedMenuItem::hide(app, Some(&label("隐藏 知歇", "Hide 知歇")))?;
        let hide_others = PredefinedMenuItem::hide_others(app, Some(&label("隐藏其他", "Hide Others")))?;
        let show_all = PredefinedMenuItem::show_all(app, Some(&label("全部显示", "Show All")))?;
        let quit = MenuItemBuilder::with_id("quit", label("退出 知歇", "Quit 知歇"))
            .accelerator("Cmd+Q")
            .build(app)?;
        let app_submenu = SubmenuBuilder::new(app, "知歇")
            .item(&about)
            .separator()
            .item(&services)
            .separator()
            .item(&hide)
            .item(&hide_others)
            .item(&show_all)
            .separator()
            .item(&quit)
            .build()?;
        let minimize = PredefinedMenuItem::minimize(app, Some(&label("最小化", "Minimize")))?;
        let close = PredefinedMenuItem::close_window(app, Some(&label("关闭窗口", "Close Window")))?;
        let window_submenu = SubmenuBuilder::new(app, label("窗口", "Window"))
            .item(&minimize)
            .item(&close)
            .build()?;
        MenuBuilder::new(app).items(&[&app_submenu, &window_submenu]).build()
    })();
    if let Ok(menu) = result {
        let _ = report_native("Set application menu", app.set_menu(menu));
    }
}

pub fn run() {
    tauri::Builder::default()
        .plugin(browser::init())
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            show_main_window(app);
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            let path = data_path(app.handle()).map_err(std::io::Error::other)?;
            let mut data = load_json(&path).map_err(std::io::Error::other)?;
            let first_start = !path.exists();
            validate_and_normalize(&mut data).map_err(std::io::Error::other)?;
            initialize_popup_image(path.parent().ok_or_else(|| std::io::Error::other("Missing configuration directory"))?, &mut data.settings)
                .map_err(std::io::Error::other)?;
            let mut native_errors = NativeErrors::default();
            if let Err(error) = startup::observe(app.handle(), &mut data) {
                native_errors.autostart_error = Some(error);
            }
            write_json(&path, &data).map_err(std::io::Error::other)?;
            let hide_on_start = data.settings.minimize_to_tray && !first_start;
            let language = data.settings.language;
            let state = AppState(Arc::new(InnerState {
                data: Mutex::new(data),
                data_path: path,
                paused: AtomicBool::new(false),
                scheduler_started: AtomicBool::new(false),
                scheduler_stop: AtomicBool::new(false),
                scheduler_error: Mutex::new(None),
                next_reminder_session: AtomicU64::new(0),
                power_action_session: AtomicU64::new(0),
                active_reminder: Mutex::new(None),
                rest_active: AtomicBool::new(false),
                rest_round_pending: AtomicBool::new(false),
                rest_next: Mutex::new(None),
                reminder_queue: Mutex::new(VecDeque::new()),
                pending_navigation: Mutex::new(None),
                window_operations: Mutex::new(()),
                popup_image_operations: Mutex::new(()),
                window_cache: Mutex::new(WindowCache::default()),
                reminder_targets: Mutex::new(HashSet::new()),
                main_ready: AtomicBool::new(false),
                main_visible: AtomicBool::new(false),
                native_errors: Mutex::new(native_errors),
            }));
            app.manage(state.clone());
            setup_tray(app, language)?;
            // 应用菜单要在启动时就替换好，否则要等第一次保存设置后才生效。
            #[cfg(target_os = "macos")]
            apply_application_menu(app.handle(), language);
            if !hide_on_start {
                show_main_window(app.handle());
            } else {
                notify_tray_background(app.handle(), &state);
            }
            spawn_scheduler(app.handle().clone(), state.clone());
            // 启动即对齐一次原生外观（含窗口背景色），避免标题栏先显示默认灰再跳变。
            let startup_theme = app_data(&state).settings.theme;
            apply_theme_to_windows(app.handle(), &startup_theme);
            #[cfg(target_os = "macos")]
            start_system_theme_watcher(app.handle().clone(), state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            load_data,
            save_data,
            startup::get_autostart_status,
            startup::set_autostart,
            start_scheduler_command,
            stop_scheduler_command,
            set_scheduler_paused,
            snooze_reminder,
            dismiss_reminder,
            get_active_reminder,
            show_reminder,
            hide_idle_window,
            get_rest_timer_status,
            get_native_errors,
            execute_power_action,
            test_reminder,
            import_data,
            export_data,
            import_popup_image,
            read_popup_image,
            clear_popup_image,
            reset_popup_image,
            is_popup_image_default,
            take_pending_navigation,
            popup_window_is_transparent,
            toggle_popup_fullscreen,
            open_reminder_settings,
            agent::load_agent_config,
            agent::save_agent_config,
            agent::agent_parse_reminder,
            agent::agent_break_advice
        ])
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => {
                show_main_window(app);
            }
            "about" => show_about(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .build(tauri::generate_context!())
        .expect("知歇 initialization failed")
        .run(|app, event| {
            if let RunEvent::WindowEvent { label, event: WindowEvent::Destroyed, .. } = &event {
                let state = app.state::<AppState>();
                match state.0.window_cache.lock() {
                    Ok(mut cache) => cache.destroyed(label),
                    Err(error) => eprintln!("Window cleanup stopped: {error}"),
                };
            }
            if let RunEvent::ExitRequested { code, ref api, .. } = event {
                if code.is_none() {
                    // Keep the tray-only process alive after the last WebView is destroyed.
                    api.prevent_exit();
                } else {
                    app.state::<AppState>()
                        .0
                        .scheduler_stop
                        .store(true, Ordering::SeqCst);
                }
            }
            if let RunEvent::WindowEvent {
                label,
                event: WindowEvent::CloseRequested { api, .. },
                ..
            } = event
            {
                if label == "main" || is_reminder_window_label(&label) {
                    api.prevent_close();
                    // Capture the closing session before queuing: a newer reminder may arrive
                    // while an earlier native window operation is still completing.
                    let closing_reminder = if is_reminder_window_label(&label) {
                        match app.state::<AppState>()
                            .0
                            .active_reminder
                            .lock()
                        {
                            Ok(active) => active.clone(),
                            Err(error) => { eprintln!("Close callback stopped: {error}"); return; }
                        }
                    } else {
                        None
                    };
                    queue_window_action(app, move |app, state| {
                        if label == "main" {
                            if let Some(window) = app.get_webview_window(&label) {
                                hide_cached_window(&window, state)?;
                            }
                        } else if state
                            .0
                            .reminder_targets
                            .lock()
                            .expect("reminder targets lock poisoned")
                            .contains(&label)
                        {
                            if let Some(active) = closing_reminder {
                                if !active_reminder_matches(state, &active.id, active.session_id) {
                                    return Ok(());
                                }
                                if active.is_rest && complete_rest_round(state) {
                                    emit_rest_timer_updated(app, state);
                                }
                                close_reminder_session(app, state, active.session_id);
                            }
                        } else if let Some(window) = app.get_webview_window(&label) {
                            hide_cached_window(&window, state)?;
                        }
                        Ok(())
                    });
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_reporting_preserves_success_and_failure() {
        assert_eq!(report_native("window main size", Ok::<_, String>(42)), Ok(42));
        assert_eq!(report_native::<(), _>("window main size", Err("native failure")), Err("native failure"));
    }

    #[test]
    fn poisoned_locks_return_errors_without_recovering_state() {
        let lock = Mutex::new(0);
        let _: Result<(), _> = std::panic::catch_unwind(|| {
            let mut value = lock.lock().unwrap();
            *value = 1;
            panic!("simulated mutation failure");
        });
        let error = guarded(|| { drop(lock.lock().map_err(lock_error)?); Ok(()) }).unwrap_err();
        assert!(error.contains("lock poisoned"));
        assert!(lock.is_poisoned());
        assert!(guarded::<()>(|| panic!("internal failure")).unwrap_err().contains("internal failure"));
    }

    #[test]
    fn scheduler_fault_is_sticky_and_separate_from_persistence() {
        let state = rest_state(true);
        let _: Result<(), _> = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _data = state.0.data.lock().unwrap();
            panic!("simulated scheduler fault");
        }));
        let error = collect_due_reminders(&state, Local::now()).unwrap_err();
        let SchedulerError::State(error) = error else { panic!("poisoned data must be a state error"); };
        record_scheduler_failure(&state, &error).unwrap();
        assert!(state.0.scheduler_stop.load(Ordering::SeqCst));
        assert!(ensure_scheduler_healthy(&state).unwrap_err().contains("restart"));
        assert!(state.0.native_errors.lock().unwrap().persistence_error.is_none());
        assert_eq!(state.0.scheduler_error.lock().unwrap().as_deref(), Some(error.as_str()));
        assert!(state.0.data.is_poisoned());
    }

    #[test]
    fn failed_mode_or_snooze_write_does_not_commit_the_candidate() {
        let mut current = AppData::default();
        let mut candidate = current.clone();
        candidate.settings.popup_fullscreen = !current.settings.popup_fullscreen;
        candidate.reminders[0].enabled = true;
        candidate.reminders[0].next_trigger_at = Some(Local::now().to_rfc3339());
        assert!(commit_app_data(Path::new(""), &mut current, candidate).is_err());
        assert_eq!(current.settings.popup_fullscreen, AppData::default().settings.popup_fullscreen);
        assert!(!current.reminders[0].enabled);
        assert!(current.reminders[0].next_trigger_at.is_none());
    }

    #[test]
    fn imports_require_json_files_and_valid_current_data() {
        assert!(validate_json_path(Path::new("backup.JSON")).is_ok());
        assert!(validate_json_path(Path::new("backup.txt")).is_err());
        let path = std::env::temp_dir().join(format!("zhixie-import-{}-{}.json", std::process::id(), Local::now().timestamp_nanos_opt().unwrap()));
        assert!(read_import_file(&path).is_err());
        fs::write(&path, b"invalid").unwrap();
        assert!(read_import_file(&path).is_err());
        let data = AppData::default();
        write_json(&path, &data).unwrap();
        assert_eq!(read_import_file(&path).unwrap().version, DATA_VERSION);
        let mut invalid = data;
        invalid.reminders[0].title.clear();
        write_json(&path, &invalid).unwrap();
        assert!(read_import_file(&path).is_err());
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn image_limit_accepts_boundary_and_preserves_existing_image_on_failure() {
        let directory = std::env::temp_dir().join(format!("zhixie-image-{}-{}", std::process::id(), Local::now().timestamp_nanos_opt().unwrap()));
        fs::create_dir_all(&directory).unwrap();
        let source = directory.join("source.png");
        let target = directory.join("popup-background.img");
        fs::write(&target, b"original").unwrap();
        let mut file = fs::File::create(&source).unwrap();
        file.write_all(b"\x89PNG\r\n\x1a\n").unwrap();
        file.set_len(MAX_POPUP_IMAGE_BYTES).unwrap();
        assert_eq!(read_bounded_image(&source).unwrap().len() as u64, MAX_POPUP_IMAGE_BYTES);
        file.set_len(MAX_POPUP_IMAGE_BYTES + 1).unwrap();
        drop(file);
        assert!(save_popup_image(&source, &target).unwrap_err().contains("10 MiB"));
        assert_eq!(fs::read(&target).unwrap(), b"original");
        fs::write(&source, b"invalid").unwrap();
        assert!(save_popup_image(&source, &target).is_err());
        assert_eq!(fs::read(&target).unwrap(), b"original");
        fs::remove_file(source).unwrap();
        fs::remove_file(target).unwrap();
        fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn system_command_reports_nonzero_exit_and_timeout() {
        #[cfg(target_os = "windows")]
        let mut failed = Command::new("cmd.exe").args(["/C", "exit 7"]).spawn().unwrap();
        #[cfg(not(target_os = "windows"))]
        let mut failed = Command::new("/bin/sh").args(["-c", "exit 7"]).spawn().unwrap();
        assert!(wait_for_system_command(&mut failed, StdDuration::from_secs(2)).unwrap_err().contains("exited"));
        #[cfg(target_os = "windows")]
        let mut delayed = Command::new("ping.exe").args(["-n", "5", "127.0.0.1"]).stdout(std::process::Stdio::null()).spawn().unwrap();
        #[cfg(not(target_os = "windows"))]
        let mut delayed = Command::new("/bin/sleep").arg("5").spawn().unwrap();
        assert!(wait_for_system_command(&mut delayed, StdDuration::ZERO).unwrap_err().contains("timed out"));
        assert!(delayed.try_wait().unwrap().is_some());
    }

    #[test]
    fn destroying_windows_remain_isolated_until_destroyed_or_absent() {
        let mut cache = WindowCache::default();
        cache.hide("main", Instant::now());
        cache.destroying.insert("main".into());
        assert!(cache.needs_destroy_wait("main", true));
        // A timeout must preserve the marker while the old native window still exists.
        assert!(cache.needs_destroy_wait("main", true));
        cache.destroyed("main");
        assert!(!cache.needs_destroy_wait("main", true));
        assert!(!cache.idle.contains_key("main"));
        cache.destroying.insert("main".into());
        assert!(!cache.needs_destroy_wait("main", false));
        cache.hide("main", Instant::now());
        assert!(!cache.needs_destroy_wait("main", true));
    }

    #[test]
    fn current_image_settings_preserve_reminders() {
        for (legacy, expected) in [
            ("original", PopupBackgroundFit::Contain),
            ("contain", PopupBackgroundFit::Contain),
            ("cover", PopupBackgroundFit::Stretch),
            ("repeat", PopupBackgroundFit::Stretch),
            ("stretch", PopupBackgroundFit::Stretch),
        ] {
            let mut value = serde_json::to_value(AppData {
                reminders: vec![sample_once(Local::now().to_rfc3339())],
                ..AppData::default()
            }).unwrap();
            let settings = value["settings"].as_object_mut().unwrap();
            settings.insert("popupBackgroundFit".into(), serde_json::json!(legacy));
            settings.insert("popupBackgroundPosition".into(), serde_json::json!("bottomRight"));
            for key in ["popupBackgroundScale", "popupBackgroundOffsetX", "popupBackgroundOffsetY"] {
                settings.remove(key);
            }
            let mut data: AppData = serde_json::from_value(value).unwrap();
            let reminder_id = data.reminders[0].id.clone();
            validate_and_normalize(&mut data).unwrap();
            assert_eq!(data.version, DATA_VERSION);
            assert_eq!(data.settings.popup_background_fit, expected);
            assert_eq!(data.settings.popup_background_scale, 100);
            assert_eq!(data.settings.popup_background_offset_x, 0);
            assert_eq!(data.settings.popup_background_offset_y, 0);
            assert_eq!(data.reminders[0].id, reminder_id);
            assert!(!serde_json::to_value(data.settings).unwrap().as_object().unwrap().contains_key("popupBackgroundPosition"));
        }
    }

    #[test]
    fn default_image_initialization_preserves_custom_and_removed_images() {
        let directory = std::env::temp_dir().join(format!("zhixie-default-image-{}-{}", std::process::id(), Local::now().timestamp_nanos_opt().unwrap()));
        fs::create_dir_all(&directory).unwrap();
        let target = directory.join("popup-background.img");
        let marker = directory.join(POPUP_IMAGE_INITIALIZED);
        let mut settings = AppSettings::default();
        settings.popup_background_scale = 99;
        initialize_popup_image(&directory, &mut settings).unwrap();
        assert_eq!(fs::read(&target).unwrap(), DEFAULT_POPUP_IMAGE);
        assert_eq!(settings.popup_background_scale, 100);
        assert_eq!(settings.popup_background_offset_x, 0);
        assert_eq!(settings.popup_background_offset_y, 0);
        assert_eq!(settings.popup_background_fit, PopupBackgroundFit::Stretch);
        fs::remove_file(&target).unwrap();
        initialize_popup_image(&directory, &mut settings).unwrap();
        assert!(!target.exists());
        restore_default_popup_image(&directory).unwrap();
        assert_eq!(fs::read(&target).unwrap(), DEFAULT_POPUP_IMAGE);
        fs::remove_file(&marker).unwrap();
        fs::write(&target, b"custom image").unwrap();
        settings.popup_background_scale = 77;
        initialize_popup_image(&directory, &mut settings).unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"custom image");
        assert_eq!(settings.popup_background_scale, 77);
        assert!(marker.exists());
        fs::remove_dir_all(&directory).unwrap();
    }

    #[test]
    fn image_scale_and_offsets_are_bounded() {
        let mut data = AppData::default();
        data.settings.popup_background_scale = 0;
        data.settings.popup_background_offset_x = -90;
        data.settings.popup_background_offset_y = 90;
        validate_and_normalize(&mut data).unwrap();
        assert_eq!(data.settings.popup_background_scale, 1);
        assert_eq!(data.settings.popup_background_offset_x, -50);
        assert_eq!(data.settings.popup_background_offset_y, 50);
        data.settings.popup_background_scale = 999;
        validate_and_normalize(&mut data).unwrap();
        assert_eq!(data.settings.popup_background_scale, 400);
    }

    #[test]
    fn expanded_accents_round_trip_without_changing_configuration_version() {
        for name in ["mint", "blue", "violet", "amber", "cyan", "rose", "coral", "graphite"] {
            let mut value = serde_json::to_value(AppData::default()).unwrap();
            value["settings"]["accentColor"] = serde_json::json!(name);
            let mut data: AppData = serde_json::from_value(value).unwrap();
            validate_and_normalize(&mut data).unwrap();
            assert_eq!(serde_json::to_value(&data).unwrap()["settings"]["accentColor"], name);
            assert_eq!(data.version, DATA_VERSION);
        }
    }

    #[test]
    fn popup_appearance_values_are_normalized_before_saving_or_importing() {
        let mut data = AppData::default();
        data.settings.popup_title_size = 999;
        data.settings.popup_overlay_opacity = 200;
        data.settings.popup_text_color = "not-a-color".into();
        validate_and_normalize(&mut data).unwrap();
        assert_eq!(data.settings.popup_title_size, 72);
        assert_eq!(data.settings.popup_overlay_opacity, 100);
        assert!(data.settings.popup_text_color.is_empty());

        data.settings.popup_title_size = 0;
        data.settings.popup_overlay_opacity = 0;
        data.settings.popup_text_color = " #Ab01Ff ".into();
        validate_and_normalize(&mut data).unwrap();
        assert_eq!(data.settings.popup_title_size, 20);
        assert_eq!(data.settings.popup_overlay_opacity, 0);
        assert_eq!(data.settings.popup_text_color, "#Ab01Ff");
    }

    #[test]
    fn fixed_filename_images_keep_their_real_format() {
        assert_eq!(image_mime(b"\x89PNG\r\n\x1a\n").unwrap(), "image/png");
        assert_eq!(image_mime(b"\xff\xd8\xff").unwrap(), "image/jpeg");
        assert_eq!(image_mime(b"GIF89a").unwrap(), "image/gif");
        assert_eq!(image_mime(b"RIFF0000WEBP").unwrap(), "image/webp");
        assert_eq!(image_mime(b"BM").unwrap(), "image/bmp");
        assert_eq!(image_mime(b"\0\0\0\x18ftypmif1\0\0\0\0avifmif1").unwrap(), "image/avif");
        assert!(image_mime(b"not an image").is_err());
        assert!(image_mime(b"\0\0\0\xffftypavif\0\0\0\0").is_err());
    }

    #[test]
    fn image_extensions_and_base64_encoding_are_stable() {
        assert_eq!(image_extension(Path::new("photo.PNG")).unwrap(), "png");
        assert_eq!(image_extension(Path::new("photo.jpeg")).unwrap(), "jpeg");
        assert!(image_extension(Path::new("photo.svg")).is_err());
        assert_eq!(base64_encode(b"Man"), "TWFu");
        assert_eq!(base64_encode(b"Ma"), "TWE=");
        assert_eq!(base64_encode(b"M"), "TQ==");
    }

    #[test]
    fn reopening_cancels_reclamation_and_reclosing_starts_a_new_delay() {
        let mut cache = WindowCache::default();
        let now = Instant::now();
        cache.hide("main", now);
        cache.reuse("main");
        assert!(cache.expired(now + WINDOW_DESTROY_DELAY).is_empty());
        cache.hide("main", now + StdDuration::from_secs(20));
        assert!(cache.expired(now + StdDuration::from_secs(49)).is_empty());
        assert_eq!(
            cache.expired(now + StdDuration::from_secs(50)),
            vec!["main"]
        );
    }

    #[test]
    fn duplicate_close_does_not_extend_idle_time_or_affect_other_windows() {
        let mut cache = WindowCache::default();
        let now = Instant::now();
        cache.hide("main", now);
        cache.hide("reminder", now + StdDuration::from_secs(10));
        cache.hide("main", now + StdDuration::from_secs(20));
        assert!(cache.expired(now + StdDuration::from_secs(29)).is_empty());
        assert_eq!(cache.expired(now + WINDOW_DESTROY_DELAY), vec!["main"]);
        cache.reuse("main");
        assert_eq!(
            cache.expired(now + StdDuration::from_secs(40)),
            vec!["reminder"]
        );
    }

    fn rest_state(enabled: bool) -> AppState {
        let mut data = AppData::default();
        data.settings.rest_enabled = enabled;
        data.settings.rest_interval_minutes = 1;
        AppState(Arc::new(InnerState {
            data: Mutex::new(data),
            data_path: PathBuf::new(),
            paused: AtomicBool::new(false),
            scheduler_started: AtomicBool::new(false),
            scheduler_stop: AtomicBool::new(false),
            scheduler_error: Mutex::new(None),
            next_reminder_session: AtomicU64::new(0),
            power_action_session: AtomicU64::new(0),
            active_reminder: Mutex::new(None),
            rest_active: AtomicBool::new(false),
            rest_round_pending: AtomicBool::new(false),
            rest_next: Mutex::new(None),
            reminder_queue: Mutex::new(VecDeque::new()),
            pending_navigation: Mutex::new(None),
            window_operations: Mutex::new(()),
            popup_image_operations: Mutex::new(()),
            window_cache: Mutex::new(WindowCache::default()),
            reminder_targets: Mutex::new(HashSet::new()),
            main_ready: AtomicBool::new(false),
            main_visible: AtomicBool::new(false),
            native_errors: Mutex::new(NativeErrors::default()),
        }))
    }

    fn rest_event(state: &AppState, is_test: bool) -> ReminderTriggeredEvent {
        let mut event = test_reminder_event(&app_data(state).settings, TestReminderKind::Rest);
        if !is_test {
            event.id = REST_ID.to_string();
            event.is_test = false;
        }
        event
    }

    fn sample_once(trigger_at: String) -> Reminder {
        Reminder {
            id: "sample".to_string(),
            title: "测试提醒".to_string(),
            reminder_type: ReminderType::Once,
            trigger_at: Some(trigger_at),
            time: None,
            weekdays: Vec::new(),
            month_days: Vec::new(),
            enabled: true,
            power_action: None,
            next_trigger_at: None,
        }
    }

    fn sample_recurring(reminder_type: ReminderType, time: &str) -> Reminder {
        Reminder {
            id: "recurring".to_string(),
            title: "重复提醒".to_string(),
            reminder_type,
            trigger_at: None,
            time: Some(time.to_string()),
            weekdays: Vec::new(),
            month_days: Vec::new(),
            enabled: true,
            power_action: None,
            next_trigger_at: None,
        }
    }

    #[test]
    fn daily_time_after_current_time_moves_to_next_day() {
        let after = Local.with_ymd_and_hms(2026, 9, 3, 10, 0, 0).unwrap();
        let next = next_daily("09:00", after).unwrap();
        let parsed = parse_datetime(&next).unwrap();
        assert_eq!(parsed.date_naive(), after.date_naive() + Duration::days(1));
        assert_eq!(parsed.time(), NaiveTime::from_hms_opt(9, 0, 0).unwrap());
    }

    #[test]
    fn validation_rejects_empty_title() {
        let mut data = AppData {
            reminders: vec![sample_once(Local::now().to_rfc3339())],
            ..AppData::default()
        };
        data.reminders[0].title.clear();
        assert!(validate_and_normalize(&mut data).is_err());
    }

    #[test]
    fn once_reminder_gets_next_trigger_at() {
        let trigger = Local::now().to_rfc3339();
        let mut data = AppData {
            reminders: vec![sample_once(trigger.clone())],
            ..AppData::default()
        };
        validate_and_normalize(&mut data).unwrap();
        assert_eq!(
            data.reminders[0].next_trigger_at.as_deref(),
            Some(trigger.as_str())
        );
    }

    #[test]
    fn weekly_reminder_uses_selected_weekdays() {
        let after = Local.with_ymd_and_hms(2026, 9, 4, 10, 0, 0).unwrap();
        let mut reminder = sample_recurring(ReminderType::Weekly, "09:00");
        reminder.weekdays = vec![1, 3];
        let next = parse_datetime(&next_recurring(&reminder, after).unwrap()).unwrap();
        assert_eq!(
            next.date_naive(),
            NaiveDate::from_ymd_opt(2026, 9, 7).unwrap()
        );
    }

    #[test]
    fn monthly_reminder_skips_unselected_dates() {
        let after = Local.with_ymd_and_hms(2026, 9, 4, 10, 0, 0).unwrap();
        let mut reminder = sample_recurring(ReminderType::Monthly, "09:00");
        reminder.month_days = vec![4, 15];
        let next = parse_datetime(&next_recurring(&reminder, after).unwrap()).unwrap();
        assert_eq!(
            next.date_naive(),
            NaiveDate::from_ymd_opt(2026, 9, 15).unwrap()
        );
    }

    #[test]
    fn current_settings_are_normalized() {
        let mut data = AppData::default();
        data.settings.rest_interval_minutes = 0;
        validate_and_normalize(&mut data).unwrap();
        assert_eq!(data.version, DATA_VERSION);
        assert_eq!(data.settings.rest_interval_minutes, 1);
        assert_eq!(data.settings.language, Language::ZhCn);
        assert!(data.settings.minimize_to_tray);
        assert!(data.settings.popup_fullscreen);
    }

    #[test]
    fn defaults_include_disabled_non_overlapping_presets_and_never_restore_deleted_entries() {
        let mut data = AppData::default();
        assert!(data.settings.rest_enabled);
        assert_eq!(data.settings.rest_interval_minutes, 40);
        assert_eq!(data.settings.theme, Theme::System);
        assert_eq!(data.settings.accent_color, AccentColor::Blue);
        assert_eq!(data.reminders.len(), 3);
        assert!(data.reminders.iter().all(|item| !item.enabled));
        assert_eq!(data.reminders[0].time.as_deref(), Some("11:50"));
        assert_eq!(data.reminders[1].weekdays, vec![1, 2, 3, 4]);
        assert_eq!(data.reminders[2].weekdays, vec![5]);
        assert_eq!(data.reminders[2].power_action, Some(PowerAction::Shutdown));
        assert_eq!(default_reminders(Language::En)[0].title, i18n::preset_messages(Language::En)[0]);
        data.reminders.clear();
        validate_and_normalize(&mut data).unwrap();
        assert!(data.reminders.is_empty());
    }

    #[test]
    fn unsupported_settings_version_is_rejected() {
        let mut value = serde_json::to_value(AppData::default()).unwrap();
        value["version"] = serde_json::json!(3);
        value["settings"]
            .as_object_mut()
            .unwrap()
            .remove("popupFullscreen");
        let mut data: AppData = serde_json::from_value(value).unwrap();

        assert!(data.settings.popup_fullscreen);
        assert!(validate_and_normalize(&mut data).is_err());
    }

    #[test]
    fn reminder_sessions_reject_stale_ids() {
        let state = rest_state(true);
        let mut event = rest_event(&state, false);
        event.session_id = 7;
        *state.0.active_reminder.lock().unwrap() = Some(event);

        assert!(active_reminder_matches(&state, REST_ID, 7));
        assert!(!active_reminder_matches(&state, REST_ID, 6));
        assert!(!active_reminder_matches(&state, "other", 7));
    }

    #[test]
    fn reminder_window_labels_do_not_match_unrelated_windows() {
        assert!(is_reminder_window_label(REMINDER_LABEL));
        assert!(is_reminder_window_label(WINDOWED_REMINDER_LABEL));
        assert!(is_reminder_window_label("reminder-monitor-1"));
        assert!(!is_reminder_window_label("main"));
        assert!(!is_reminder_window_label("reminder-preview"));
    }

    #[test]
    fn scheduled_and_test_rest_popups_stop_the_existing_countdown() {
        for is_test in [false, true] {
            let state = rest_state(true);
            assert!(rest_timer_status(&state).next_trigger_at.is_some());
            assert!(prepare_notification(&state, &rest_event(&state, is_test)).is_some());

            // 焦点切换和最小化会再次读取状态，不能因此启动下一轮。
            for _ in 0..3 {
                let status = rest_timer_status(&state);
                assert!(status.is_resting);
                assert!(status.next_trigger_at.is_none());
                assert!(state.0.rest_next.lock().unwrap().is_none());
            }
        }
    }

    #[test]
    fn rest_completion_starts_a_full_interval_from_completion() {
        let state = rest_state(true);
        prepare_notification(&state, &rest_event(&state, false)).unwrap();
        let before = Local::now();
        assert!(complete_rest_round(&state));
        let after = Local::now();
        let status = rest_timer_status(&state);
        let next = parse_datetime(status.next_trigger_at.as_deref().unwrap()).unwrap();

        assert!(!status.is_resting);
        assert!(next >= before + Duration::minutes(1));
        assert!(next <= after + Duration::minutes(1));
        assert!(!complete_rest_round(&state));
        assert_eq!(
            rest_timer_status(&state).next_trigger_at,
            status.next_trigger_at
        );
    }

    #[test]
    fn four_hour_snooze_only_delays_the_current_round() {
        for is_test in [false, true] {
            let state = rest_state(true);
            prepare_notification(&state, &rest_event(&state, is_test)).unwrap();
            let before_snooze = Local::now();
            assert!(snooze_rest_round(&state, 4 * 60 * 60));
            let after_snooze = Local::now();
            let status = rest_timer_status(&state);
            let next = parse_datetime(status.next_trigger_at.as_deref().unwrap()).unwrap();

            assert!(!status.is_resting);
            assert!(next >= before_snooze + Duration::hours(4));
            assert!(next <= after_snooze + Duration::hours(4));
            assert_eq!(app_data(&state).settings.rest_interval_minutes, 1);

            // 延后到期再次弹出时进入休息；完成后恢复原来的 1 分钟间隔。
            prepare_notification(&state, &rest_event(&state, false)).unwrap();
            assert!(rest_timer_status(&state).is_resting);
            let before_completion = Local::now();
            assert!(complete_rest_round(&state));
            let after_completion = Local::now();
            let next = parse_datetime(
                rest_timer_status(&state)
                    .next_trigger_at
                    .as_deref()
                    .unwrap(),
            )
            .unwrap();
            assert!(next >= before_completion + Duration::minutes(1));
            assert!(next <= after_completion + Duration::minutes(1));
        }
    }

    #[test]
    fn repeated_close_and_other_notifications_preserve_snooze() {
        let state = rest_state(true);
        prepare_notification(&state, &rest_event(&state, false)).unwrap();
        assert!(snooze_rest_round(&state, 4 * 60 * 60));
        let delayed = rest_timer_status(&state).next_trigger_at;
        assert!(!complete_rest_round(&state));
        assert!(!snooze_rest_round(&state, 30));
        let event = test_reminder_event(&app_data(&state).settings, TestReminderKind::Event);
        prepare_notification(&state, &event).unwrap();
        assert!(!complete_rest_round(&state));
        assert_eq!(rest_timer_status(&state).next_trigger_at, delayed);
        assert!(state.0.rest_round_pending.load(Ordering::SeqCst));
    }

    #[test]
    fn waiting_notifications_do_not_finish_an_active_rest() {
        let state = rest_state(true);
        prepare_notification(&state, &rest_event(&state, false)).unwrap();
        let event = test_reminder_event(&app_data(&state).settings, TestReminderKind::Event);
        prepare_notification(&state, &event).unwrap();
        let status = rest_timer_status(&state);
        assert!(status.is_resting);
        assert!(status.next_trigger_at.is_none());
        assert!(complete_rest_round(&state));
    }

    #[test]
    fn disabled_rest_test_pauses_without_enabling_scheduled_reminders() {
        for snooze in [false, true] {
            let state = rest_state(false);
            prepare_notification(&state, &rest_event(&state, true)).unwrap();
            let status = rest_timer_status(&state);
            assert!(status.is_resting);
            assert!(status.next_trigger_at.is_none());
            if snooze {
                assert!(snooze_rest_round(&state, 4 * 60 * 60));
            } else {
                assert!(complete_rest_round(&state));
            }
            let status = rest_timer_status(&state);
            assert!(!status.is_resting);
            assert!(status.next_trigger_at.is_none());
            assert!(!app_data(&state).settings.rest_enabled);
        }
    }

    #[test]
    fn cancelled_scheduled_rest_cannot_reactivate_a_popup() {
        let state = rest_state(false);
        assert!(prepare_notification(&state, &rest_event(&state, false)).is_none());
        assert!(!rest_timer_status(&state).is_resting);
    }

    #[test]
    fn appended_system_rest_notifications_keep_the_interval_running() {
        let state = rest_state(true);
        state
            .0
            .data
            .lock()
            .unwrap()
            .settings
            .system_notification_enabled = true;
        let before = rest_timer_status(&state).next_trigger_at;
        prepare_notification(&state, &rest_event(&state, true)).unwrap();

        assert!(rest_timer_status(&state).is_resting);
        assert!(rest_timer_status(&state).next_trigger_at.is_none());
        assert!(before.is_some());
    }

    #[test]
    fn overdue_automatic_power_action_is_skipped() {
        let now = Local.with_ymd_and_hms(2026, 9, 4, 23, 32, 0).unwrap();
        let due = Local.with_ymd_and_hms(2026, 9, 4, 23, 30, 0).unwrap();
        assert!(!should_trigger_power_action(due, now));
        let recent_due = Local.with_ymd_and_hms(2026, 9, 4, 23, 31, 30).unwrap();
        assert!(should_trigger_power_action(recent_due, now));
    }

    #[test]
    fn lock_action_uses_stable_json_value() {
        assert_eq!(
            serde_json::to_string(&PowerAction::Lock).unwrap(),
            "\"lock\""
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_power_actions_use_expected_commands() {
        let system_root = Path::new(r"C:\Windows");
        let (lock_program, lock_args) = windows_power_command(&PowerAction::Lock, system_root);
        let (shutdown_program, shutdown_args) =
            windows_power_command(&PowerAction::Shutdown, system_root);
        let (restart_program, restart_args) =
            windows_power_command(&PowerAction::Restart, system_root);

        assert_eq!(
            lock_program,
            system_root.join("System32").join("rundll32.exe")
        );
        assert_eq!(lock_args, vec!["user32.dll,LockWorkStation"]);
        assert_eq!(
            shutdown_program,
            system_root.join("System32").join("shutdown.exe")
        );
        assert_eq!(shutdown_args, vec!["/s", "/t", "0"]);
        assert_eq!(
            restart_program,
            system_root.join("System32").join("shutdown.exe")
        );
        assert_eq!(restart_args, vec!["/r", "/t", "0"]);
    }

    #[test]
    fn event_test_reminder_is_generic_and_non_power() {
        let event = test_reminder_event(&AppSettings::default(), TestReminderKind::Event);

        assert!(event.is_test);
        assert!(!event.is_rest);
        assert_eq!(event.reminder_type, ReminderType::Once);
        assert_eq!(event.title, "这是一条测试通知");
        assert!(event.power_action.is_none());
    }

    #[test]
    fn rest_test_reminder_uses_configured_content_without_real_rest_id() {
        let mut settings = AppSettings::default();
        settings.rest_message = "起来活动一下".to_string();
        let event = test_reminder_event(&settings, TestReminderKind::Rest);

        assert!(event.is_test);
        assert!(event.is_rest);
        assert_eq!(event.title, settings.rest_message);
        assert_ne!(event.id, REST_ID);
        assert!(event.power_action.is_none());
    }

    #[test]
    fn queued_reminders_order_and_skip_expired_actions() {
        let state = rest_state(true);
        let now = Local::now();
        let mut event = test_reminder_event(&AppSettings::default(), TestReminderKind::Event);
        event.is_test = false;
        event.id = "b".into();
        let mut first = event.clone();
        first.id = "a".into();
        let mut expired = event.clone();
        expired.power_action = Some(PowerAction::Lock);
        let mut rest = rest_event(&state, false);
        rest.id = "__rest__".into();
        enqueue_reminders(&state, vec![
            QueuedReminder { due_at: now, event },
            QueuedReminder { due_at: now, event: rest },
            QueuedReminder { due_at: now - Duration::minutes(2), event: expired },
            QueuedReminder { due_at: now, event: first },
        ]).unwrap();
        assert_eq!(next_queued_reminder(&state, now).unwrap().unwrap().id, "a");
        let active = next_queued_reminder(&state, now).unwrap().unwrap();
        assert_eq!(active.id, "b");
        *state.0.active_reminder.lock().unwrap() = Some(active);
        assert!(next_queued_reminder(&state, now).unwrap().is_none());
        *state.0.active_reminder.lock().unwrap() = None;
        assert!(next_queued_reminder(&state, now).unwrap().unwrap().is_rest);
        assert!(next_queued_reminder(&state, now).unwrap().is_none());
    }

    #[test]
    fn queue_lock_failure_does_not_poison_the_window_operation_lock() {
        let state = rest_state(false);
        let _: Result<(), _> = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _queue = state.0.reminder_queue.lock().unwrap();
            panic!("simulated queue failure");
        }));
        let result = guarded(|| {
            let _operation = state.0.window_operations.lock().map_err(lock_error)?;
            next_queued_reminder(&state, Local::now())
        });
        assert!(result.unwrap_err().contains("lock poisoned"));
        assert!(enqueue_reminders(&state, Vec::new()).is_err());
        assert!(state.0.reminder_queue.is_poisoned());
        assert!(!state.0.window_operations.is_poisoned());
    }

    #[test]
    fn due_once_reminders_complete_and_recurring_actions_advance() {
        let mut state = rest_state(false);
        let path = std::env::temp_dir().join(format!("zhixie-due-{}-{}.json", std::process::id(), Local::now().timestamp_nanos_opt().unwrap()));
        Arc::get_mut(&mut state.0).unwrap().data_path = path.clone();
        let now = Local::now();
        let mut once = sample_once((now - Duration::seconds(10)).to_rfc3339());
        once.next_trigger_at = once.trigger_at.clone();
        let mut daily = sample_recurring(ReminderType::Daily, "09:00");
        daily.next_trigger_at = Some((now - Duration::minutes(2)).to_rfc3339());
        daily.power_action = Some(PowerAction::Lock);
        state.0.data.lock().unwrap().reminders = vec![once, daily];
        let due = collect_due_reminders(&state, now).unwrap();
        assert_eq!(due.len(), 1);
        let data = app_data(&state);
        assert!(!data.reminders[0].enabled);
        assert!(data.reminders[0].next_trigger_at.is_none());
        assert!(parse_datetime(data.reminders[1].next_trigger_at.as_deref().unwrap()).unwrap() > now);
        assert!(collect_due_reminders(&state, now).unwrap().is_empty());
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn failed_scheduler_write_preserves_reminder_and_rest_state() {
        let state = rest_state(true);
        let now = Local::now();
        let mut once = sample_once((now - Duration::seconds(10)).to_rfc3339());
        once.next_trigger_at = once.trigger_at.clone();
        state.0.data.lock().unwrap().reminders = vec![once];
        *state.0.rest_next.lock().unwrap() = Some(now - Duration::seconds(5));
        assert!(matches!(collect_due_reminders(&state, now), Err(SchedulerError::Persistence(_))));
        let data = app_data(&state);
        assert!(data.reminders[0].enabled);
        assert!(data.reminders[0].next_trigger_at.is_some());
        assert_eq!(*state.0.rest_next.lock().unwrap(), Some(now - Duration::seconds(5)));
        assert!(!state.0.rest_round_pending.load(Ordering::SeqCst));
        assert!(state.0.reminder_queue.lock().unwrap().is_empty());
    }

    #[test]
    fn monthly_31st_skips_february() {
        let after = Local.with_ymd_and_hms(2026, 2, 1, 8, 0, 0).unwrap();
        let mut reminder = sample_recurring(ReminderType::Monthly, "09:00");
        reminder.month_days = vec![31];
        let next = parse_datetime(&next_recurring(&reminder, after).unwrap()).unwrap();
        assert_eq!(next.date_naive(), NaiveDate::from_ymd_opt(2026, 3, 31).unwrap());
    }

    #[test]
    fn plan_changes_include_action_disable_delete_but_not_runtime_deadline() {
        let original = sample_once(Local::now().to_rfc3339());
        let mut next = original.clone();
        next.next_trigger_at = Some((Local::now() + Duration::minutes(5)).to_rfc3339());
        assert!(changed_reminder_ids(&[original.clone()], &[next.clone()]).is_empty());
        for updated in [Some(PowerAction::Lock), Some(PowerAction::Restart)] {
            next.power_action = updated;
            assert!(changed_reminder_ids(&[original.clone()], &[next.clone()]).contains("sample"));
        }
        next = original.clone();
        next.enabled = false;
        assert!(changed_reminder_ids(&[original.clone()], &[next]).contains("sample"));
        assert!(changed_reminder_ids(&[original], &[]).contains("sample"));
    }

    #[test]
    fn one_time_snooze_deadline_survives_settings_validation() {
        let mut reminder = sample_once(Local::now().to_rfc3339());
        let delayed = (Local::now() + Duration::minutes(5)).to_rfc3339();
        reminder.next_trigger_at = Some(delayed.clone());
        let mut data = AppData { reminders: vec![reminder], ..AppData::default() };
        validate_and_normalize(&mut data).unwrap();
        assert_eq!(data.reminders[0].next_trigger_at.as_deref(), Some(delayed.as_str()));
    }

    #[test]
    fn old_configuration_is_rewritten_with_current_defaults() {
        let directory = std::env::temp_dir().join(format!("zhixie-test-{}-{}", std::process::id(), Local::now().timestamp_nanos_opt().unwrap()));
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("zhixie.json");
        let mut old = AppData::default();
        old.version = DATA_VERSION - 1;
        let preset_count = old.reminders.len();
        old.reminders.push(Reminder {
            id: "keep-me".to_string(),
            title: "旧版本提醒".to_string(),
            reminder_type: ReminderType::Once,
            trigger_at: Some("2026-10-06T08:00:00+08:00".to_string()),
            time: None,
            weekdays: Vec::new(),
            month_days: Vec::new(),
            enabled: true,
            power_action: None,
            next_trigger_at: None,
        });
        write_json(&path, &old).unwrap();
        assert_eq!(load_json(&path).unwrap().version, DATA_VERSION);
        let saved: AppData = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(saved.version, DATA_VERSION);
        // 旧配置是有效用户数据，必须留下备份，否则降级会不可恢复地丢失提醒。
        let backups: Vec<PathBuf> = fs::read_dir(&directory)
            .unwrap()
            .filter_map(|entry| entry.ok().map(|item| item.path()))
            .filter(|item| item.to_string_lossy().contains(".unsupported."))
            .collect();
        assert_eq!(backups.len(), 1, "the unsupported configuration must be preserved exactly once");
        let preserved: AppData =
            serde_json::from_str(&fs::read_to_string(&backups[0]).unwrap()).unwrap();
        assert_eq!(preserved.version, DATA_VERSION - 1);
        // 预设提醒之外，用户的自定义提醒也必须完整留在备份里。
        assert_eq!(preserved.reminders.len(), preset_count + 1);
        assert!(preserved.reminders.iter().any(|item| item.id == "keep-me"));
        fs::remove_file(path).unwrap();
        for backup in backups {
            fs::remove_file(backup).unwrap();
        }
        fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn atomic_write_replaces_content_and_cleans_failed_replacements() {
        let directory = std::env::temp_dir().join(format!("zhixie-atomic-{}-{}", std::process::id(), Local::now().timestamp_nanos_opt().unwrap()));
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("data.json");
        atomic_write(&path, b"original").unwrap();
        atomic_write(&path, "中文".as_bytes()).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "中文");
        let occupied = directory.join("occupied.json");
        fs::create_dir(&occupied).unwrap();
        assert!(atomic_write(&occupied, b"replacement").is_err());
        assert!(occupied.is_dir());
        assert_eq!(fs::read_dir(&directory).unwrap().count(), 2);
        fs::remove_file(path).unwrap();
        fs::remove_dir(occupied).unwrap();
        fs::remove_dir(directory).unwrap();
    }
}
