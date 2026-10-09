import { translate } from './i18n.ts'

export type ReminderType = 'once' | 'daily' | 'weekly' | 'monthly' | 'interval'
export type Theme = 'dark' | 'light' | 'system'
export type Language = 'zh-CN' | 'en'
export type AccentColor = 'mint' | 'blue' | 'violet' | 'amber' | 'cyan' | 'rose' | 'coral' | 'graphite'
export type PowerAction = 'shutdown' | 'lock' | 'restart'
export type TestReminderKind = 'event' | 'rest'
export type PopupBackgroundFit = 'stretch' | 'contain'
export type SettingsTab = 'rhythm' | 'stage' | 'system' | 'data'

export interface ReminderForm {
  title: string
  type: Exclude<ReminderType, 'interval'>
  triggerAt: string
  time: string
  weekdays: number[]
  monthDays: number[]
  powerAction: PowerAction | null
}

export interface Reminder {
  id: string
  title: string
  type: ReminderType
  triggerAt?: string | null
  time?: string | null
  weekdays?: number[]
  monthDays?: number[]
  enabled: boolean
  powerAction?: PowerAction | null
  nextTriggerAt?: string | null
}

export interface AppSettings {
  language: Language
  autostart: boolean
  minimizeToTray: boolean
  popupAlwaysOnTop: boolean
  popupFullscreen: boolean
  restEnabled: boolean
  restIntervalMinutes: number
  restMessage: string
  systemNotificationEnabled: boolean
  theme: Theme
  accentColor: AccentColor
  popupBackgroundFit: PopupBackgroundFit
  popupBackgroundScale: number
  popupBackgroundOffsetX: number
  popupBackgroundOffsetY: number
  popupFadeEnabled: boolean
  popupTextColor: string
  popupTitleSize: number
  popupOverlayOpacity: number
}

export interface AppData {
  version: number
  autostartOwner?: { platform: string; channel: string; target: string } | null
  settings: AppSettings
  reminders: Reminder[]
}

export interface AutostartStatus {
  enabled: boolean
  conflict: boolean
  reason: string | null
}

export interface ReminderTriggeredEvent {
  sessionId: number
  id: string
  title: string
  type: ReminderType
  isRest: boolean
  powerAction?: PowerAction | null
  isTest: boolean
  restStartedAtMs?: number | null
  powerDeadlineMs?: number | null
}

export interface RestTimerStatus {
  nextTriggerAt: string | null
  isResting: boolean
}

export interface NativeErrors {
  autostartError: string | null
  notificationError: string | null
  persistenceError: string | null
  schedulerError: string | null
}

// —— 智能体模块 ——

export interface AgentConfig {
  enabled: boolean
  provider: string
  endpoint: string
  model: string
  apiKey: string
}

export interface AgentDraft {
  title: string
  reminderType: string
  triggerAt: string | null
  time: string | null
  weekdays: number[]
  monthDays: number[]
  powerAction: PowerAction | null
  popupMessage: string
}

export interface AgentResult {
  draft: AgentDraft
  source: 'cloud' | 'local'
  note: string
}

export interface AgentAdvice {
  advice: string
  source: 'cloud' | 'local'
}

export const defaultAgentConfig = (): AgentConfig => ({
  enabled: false,
  provider: 'deepseek',
  endpoint: 'https://api.deepseek.com/chat/completions',
  model: 'deepseek-chat',
  apiKey: '',
})

export const defaultReminders = (language: Language): Reminder[] => [
  { id: 'preset-lunch', title: translate(language, 'preset.lunch'), type: 'daily', time: '11:50', powerAction: 'lock', enabled: false },
  { id: 'preset-workday', title: translate(language, 'preset.workday'), type: 'weekly', time: '17:30', weekdays: [1, 2, 3, 4], powerAction: 'lock', enabled: false },
  { id: 'preset-weekend', title: translate(language, 'preset.weekend'), type: 'weekly', time: '17:30', weekdays: [5], powerAction: 'shutdown', enabled: false },
]

export const defaultData = (language: Language = 'zh-CN'): AppData => ({
  version: 6,
  settings: {
    language,
    autostart: false,
    minimizeToTray: true,
    popupAlwaysOnTop: true,
    popupFullscreen: true,
    restEnabled: true,
    restIntervalMinutes: 40,
    restMessage: translate(language, 'rest.defaultMessage'),
    systemNotificationEnabled: true,
    theme: 'system',
    accentColor: 'blue',
    popupBackgroundFit: 'stretch',
    popupBackgroundScale: 100,
    popupBackgroundOffsetX: 0,
    popupBackgroundOffsetY: 0,
    popupFadeEnabled: true,
    popupTextColor: '',
    popupTitleSize: 32,
    popupOverlayOpacity: 30,
  },
  reminders: defaultReminders(language),
})
