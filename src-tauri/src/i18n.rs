use crate::{Language, ReminderTriggeredEvent};

pub fn default_rest_message(language: Language) -> &'static str {
    match language {
        Language::ZhCn => "起身走走，喝口水，作业先放一放。",
        Language::En => "Stand up, drink some water, and let the homework wait.",
    }
}

pub fn preset_messages(language: Language) -> [&'static str; 3] {
    match language {
        Language::ZhCn => ["下课啦，先去食堂吃口热的。", "今天的课结束了，回宿舍好好歇歇。", "一周的课上完了，好好过周末。"],
        Language::En => ["Class is over. Grab a hot meal at the canteen.", "Classes are done for today. Head back and get some rest.", "Classes are over for the week. Enjoy your weekend."],
    }
}

pub fn notification_title(language: Language, event: &ReminderTriggeredEvent) -> &'static str {
    match (language, event.is_rest) {
        (Language::ZhCn, true) => "知歇 · 休息提醒",
        (Language::ZhCn, false) => "知歇 · 定时提醒",
        (Language::En, true) => "知歇 · Break reminder",
        (Language::En, false) => "知歇 · Scheduled reminder",
    }
}

pub fn notification_window_title(language: Language) -> &'static str {
    match language {
        Language::ZhCn => "知歇 通知",
        Language::En => "知歇 Notification",
    }
}

pub fn test_notification(language: Language) -> &'static str {
    match language {
        Language::ZhCn => "这是一条测试通知",
        Language::En => "This is a test notification",
    }
}

pub fn tray_labels(language: Language) -> (&'static str, &'static str, &'static str) {
    match language {
        Language::ZhCn => ("打开", "关闭", "关于"),
        Language::En => ("Open", "Close", "About"),
    }
}

pub fn tray_background_message(language: Language) -> &'static str {
    match language {
        Language::ZhCn => "知歇 已在后台运行，提醒仍会继续。",
        Language::En => "知歇 is running in the background. Reminders will continue.",
    }
}

pub fn notification_send_failed(language: Language, error: &str) -> String {
    match language {
        Language::ZhCn => format!("系统通知发送失败：{error}"),
        Language::En => format!("Failed to send system notification: {error}"),
    }
}
