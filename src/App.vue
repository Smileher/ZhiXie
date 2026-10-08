<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, reactive, ref, watch } from 'vue'
import { getVersion } from '@tauri-apps/api/app'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { ask, confirm, open } from '@tauri-apps/plugin-dialog'
import { isPermissionGranted, requestPermission } from '@tauri-apps/plugin-notification'
import ReminderPopup from './components/ReminderPopup.vue'
import AppTopBar from './components/AppTopBar.vue'
import EventsView from './components/EventsView.vue'
import RestView from './components/RestView.vue'
import SettingsView from './components/SettingsView.vue'
import AboutView from './components/AboutView.vue'
import AgentView from './components/AgentView.vue'
import { translate } from './i18n'
import type { MessageKey } from './i18n'
import { logError } from './error'
import type { AccentColor, AgentDraft, AppData, AutostartStatus, Language, NativeErrors, PowerAction, Reminder, ReminderForm, ReminderTriggeredEvent, ReminderType, RestTimerStatus, SettingsTab, TestReminderKind } from './types'
import { defaultData, defaultReminders } from './types'

type View = 'events' | 'rest' | 'settings' | 'about' | 'agent'
type EditableReminderType = Exclude<ReminderType, 'interval'>
type AutomaticPowerAction = Extract<PowerAction, 'shutdown' | 'lock' | 'restart'>

const isPopup = window.location.hash === '#/reminder'
const systemTheme = window.matchMedia('(prefers-color-scheme: dark)')
const systemDark = ref(systemTheme.matches)
const data = ref<AppData>(defaultData())
const displayedTheme = computed(() => data.value.settings.theme === 'system' ? (systemDark.value ? 'dark' : 'light') : data.value.settings.theme)
const currentView = ref<View>('rest')
const settingsTab = ref<SettingsTab>('rhythm')
const showForm = ref(false)
const editingId = ref<string | null>(null)
const actionMessage = ref('')
const now = ref(Date.now())
const nextRestTrigger = ref<string | null>(null)
const restIsActive = ref(false)
const restMessageDraft = ref(defaultData().settings.restMessage)
const notificationError = ref('')
const persistenceError = ref('')
const schedulerError = ref('')
const autostartError = ref('')
const autostartNotice = ref('')
const appVersion = ref('1.0.0')
const popupBackgroundPreview = ref('')
const popupImageIsDefault = ref(false)
let unlisten: (() => void) | undefined
let unlistenNavigation: (() => void) | undefined
let unlistenRestTimer: (() => void) | undefined
let unlistenNotificationFailure: (() => void) | undefined
let unlistenPersistenceFailure: (() => void) | undefined
let unlistenSchedulerFailure: (() => void) | undefined
let unlistenSettingsSync: (() => void) | undefined
let unlistenWindowFocus: (() => void) | undefined
let clockTimer: number | undefined
let timerRefreshToken = 0
let nativeErrorRevision = 0
let actionMessageTimer: number | undefined

// 状态消息只短暂停留，避免一直占位或把旁边的按钮挤走。
watch(actionMessage, (value) => {
  if (actionMessageTimer) window.clearTimeout(actionMessageTimer)
  actionMessageTimer = undefined
  if (value) {
    actionMessageTimer = window.setTimeout(() => {
      actionMessage.value = ''
    }, 4000)
  }
})

function t(key: MessageKey, params: Record<string, string | number> = {}) {
  return translate(data.value.settings.language, key, params)
}

function formatError(error: unknown) {
  if (error instanceof Error) return error.message
  if (typeof error === 'string') return error
  try {
    const serialized = JSON.stringify(error)
    return serialized && serialized !== '{}' ? serialized : String(error)
  } catch {
    return String(error)
  }
}

const frequencyOptions = computed<Array<{ value: EditableReminderType; label: string }>>(() => [
  { value: 'once', label: t('frequency.once') },
  { value: 'daily', label: t('frequency.daily') },
  { value: 'weekly', label: t('frequency.weekly') },
  { value: 'monthly', label: t('frequency.monthly') },
])

const powerActionOptions = computed<Array<{ value: AutomaticPowerAction; label: string }>>(() => [
  { value: 'shutdown', label: t('power.shutdown') },
  { value: 'lock', label: t('power.lock') },
  { value: 'restart', label: t('power.restart') },
])
const accentColors: AccentColor[] = ['blue', 'mint', 'violet', 'amber', 'cyan', 'rose', 'coral', 'graphite']

const weekdayOptions = computed(() => Array.from({ length: 7 }, (_, index) => ({
  value: index + 1,
  label: t(`weekday.${index + 1}` as MessageKey),
})))

const typeLabels = computed<Record<ReminderType, string>>(() => ({
  once: t('frequency.once'), daily: t('frequency.daily'), weekly: t('frequency.weekly'), monthly: t('frequency.monthly'),
  interval: t('frequency.interval'),
}))

const form = reactive<ReminderForm>({
  title: '',
  type: 'once' as EditableReminderType,
  triggerAt: '',
  time: '09:00',
  weekdays: [1] as number[],
  monthDays: [1] as number[],
  powerAction: null as PowerAction | null,
})

const sortedReminders = computed(() =>
  [...data.value.reminders].sort((a, b) => {
    const left = a.nextTriggerAt || a.triggerAt || a.time || ''
    const right = b.nextTriggerAt || b.triggerAt || b.time || ''
    return left.localeCompare(right)
  }),
)

const restStatusText = computed(() =>
  restIsActive.value
    ? t('rest.resting')
    : data.value.settings.restEnabled
      ? (nextRestTrigger.value ? formatCountdown(nextRestTrigger.value) : t('common.calculating'))
      : t('common.paused'),
)

// 重置只恢复参数默认值，提醒列表必须保留。
// 默认文案随语言变化，所以比较时要按当前语言重新生成一份默认值，否则切到英文后永远判定为「已修改」。
const canResetSettings = computed(() => {
  const defaults = defaultData().settings
  const current = data.value.settings
  const language = current.language
  const localized = {
    ...defaults,
    language,
    autostart: current.autostart,
    restMessage: translate(language, 'rest.defaultMessage'),
  }
  return !popupImageIsDefault.value || (Object.keys(localized) as Array<keyof typeof localized>).some(
    (key) => JSON.stringify(current[key]) !== JSON.stringify(localized[key]),
  )
})

const restProgress = computed(() => {
  if (restIsActive.value) return 0
  if (!data.value.settings.restEnabled || !nextRestTrigger.value) return 0
  const remaining = new Date(nextRestTrigger.value).getTime() - now.value
  const total = data.value.settings.restIntervalMinutes * 60 * 1000
  if (!Number.isFinite(remaining) || total <= 0) return 0
  return Math.max(0, Math.min(100, (remaining / total) * 100))
})

function patchForm(patch: Partial<ReminderForm>) {
  Object.assign(form, patch)
}

function resetForm() {
  form.title = ''
  form.powerAction = null
  form.type = 'once'
  form.triggerAt = toDateTimeLocal(new Date(Date.now() + 10 * 60 * 1000))
  form.time = '09:00'
  form.weekdays = [new Date().getDay() || 7]
  form.monthDays = [new Date().getDate()]
  editingId.value = null
  actionMessage.value = ''
}

function toDateTimeLocal(date: Date) {
  const offset = date.getTimezoneOffset()
  return new Date(date.getTime() - offset * 60 * 1000).toISOString().slice(0, 16)
}

function openAddForm() {
  resetForm()
  showForm.value = true
}

function editReminder(reminder: Reminder) {
  editingId.value = reminder.id
  form.title = reminder.title
  form.powerAction = reminder.powerAction ?? null
  form.type = reminder.type === 'interval' ? 'once' : reminder.type
  form.triggerAt = reminder.triggerAt ? toDateTimeLocal(new Date(reminder.triggerAt)) : ''
  form.time = reminder.time || '09:00'
  form.weekdays = [...(reminder.weekdays || [])]
  form.monthDays = [...(reminder.monthDays || [])]
  showForm.value = true
  actionMessage.value = ''
}

function toggleNumber(values: number[], value: number) {
  const index = values.indexOf(value)
  if (index >= 0) values.splice(index, 1)
  else values.push(value)
  values.sort((a, b) => a - b)
}

function formatCountdown(value?: string | null) {
  if (!value) return ''
  const diff = new Date(value).getTime() - now.value
  if (Number.isNaN(diff)) return ''
  if (diff <= 0) return t('countdown.due')
  const totalSeconds = Math.ceil(diff / 1000)
  const days = Math.floor(totalSeconds / 86400)
  const hours = Math.floor((totalSeconds % 86400) / 3600)
  const minutes = Math.floor((totalSeconds % 3600) / 60)
  const seconds = totalSeconds % 60
  if (days > 0) return t('countdown.days', { days, hours, minutes, seconds })
  if (hours > 0) return t('countdown.hours', { hours, minutes, seconds })
  if (minutes > 0) return t('countdown.minutes', { minutes, seconds })
  return t('countdown.seconds', { seconds })
}

function formatRule(reminder: Reminder) {
  if (reminder.type === 'once') {
    return reminder.triggerAt
      ? new Intl.DateTimeFormat(data.value.settings.language, { dateStyle: 'medium', timeStyle: 'short' }).format(new Date(reminder.triggerAt))
      : t('rule.unset')
  }
  if (reminder.type === 'weekly') {
    const prefix = t('rule.weekPrefix')
    const separator = t('common.listSeparator')
    const days = (reminder.weekdays || []).map((day) => `${prefix}${weekdayOptions.value[day - 1]?.label || day}`).join(separator)
    return `${days} ${reminder.time}`
  }
  if (reminder.type === 'monthly') {
    return t('rule.monthly', { days: (reminder.monthDays || []).join(t('common.listSeparator')), time: reminder.time || '' })
  }
  return `${typeLabels.value[reminder.type]} ${reminder.time}`
}

function formatNext(reminder: Reminder) {
  const value = reminder.nextTriggerAt || reminder.triggerAt
  const countdown = formatCountdown(value)
  return countdown ? `${formatRule(reminder)} · ${countdown}` : formatRule(reminder)
}

// 串行写入并在实际发送时读取最新数据，防止旧快照最后落盘。
let persistToken = 0
let externalModeRevision = 0
let persistQueue: Promise<unknown> = Promise.resolve()
let settingsQueue: Promise<unknown> = Promise.resolve()

// Queue the mutation and rollback together, not just the IPC write.
function queueSettings<T>(operation: () => Promise<T>): Promise<T> {
  const queued = settingsQueue.then(operation)
  settingsQueue = queued.catch(() => {})
  return queued
}
async function persist(popupFullscreen?: boolean) {
  const token = ++persistToken
  const operation = persistQueue.then(() => invoke<AppData>('save_data', {
    data: data.value,
    popupFullscreen: popupFullscreen ?? null,
  }))
  persistQueue = operation.catch(() => {})
  const saved = await operation
  if (token === persistToken) {
    data.value = saved
    restMessageDraft.value = saved.settings.restMessage
  }
}

async function saveReminder() {
  const title = form.title.trim()
  if (!title || (form.type === 'once' && !form.triggerAt) || (form.type !== 'once' && !form.time)) {
    actionMessage.value = t('validation.complete')
    return
  }
  if (form.type === 'weekly' && form.weekdays.length === 0) {
    actionMessage.value = t('validation.weekday')
    return
  }
  if (form.type === 'monthly' && form.monthDays.length === 0) {
    actionMessage.value = t('validation.monthDay')
    return
  }

  const existing = editingId.value
    ? data.value.reminders.find((item) => item.id === editingId.value)
    : undefined
  const reminder: Reminder = {
    id: editingId.value || crypto.randomUUID(),
    title,
    type: form.type,
    triggerAt: form.type === 'once' ? new Date(form.triggerAt).toISOString() : null,
    time: form.type === 'once' ? null : form.time,
    weekdays: form.type === 'weekly' ? [...form.weekdays] : [],
    monthDays: form.type === 'monthly' ? [...form.monthDays] : [],
    enabled: existing?.enabled ?? true,
    powerAction: form.powerAction,
    nextTriggerAt: null,
  }

  return queueSettings(async () => {
    const previous = [...data.value.reminders]
    const index = data.value.reminders.findIndex((item) => item.id === reminder.id)
    if (index >= 0) data.value.reminders.splice(index, 1, reminder)
    else data.value.reminders.push(reminder)
    try {
      await persist()
      showForm.value = false
      actionMessage.value = t('status.reminderSaved')
    } catch (error) {
      data.value.reminders = previous
      logError('save reminder', error)
      actionMessage.value = t('status.saveFailed')
    }
  })
}

async function addAgentReminder(draft: AgentDraft) {
  const type = (['once', 'daily', 'weekly', 'monthly'].includes(draft.reminderType)
    ? draft.reminderType
    : 'once') as ReminderType
  const reminder: Reminder = {
    id: crypto.randomUUID(),
    title: draft.title.trim() || t('agent.heading'),
    type,
    triggerAt: type === 'once' ? draft.triggerAt : null,
    time: type === 'once' ? null : draft.time,
    weekdays: type === 'weekly' ? [...draft.weekdays] : [],
    monthDays: type === 'monthly' ? [...draft.monthDays] : [],
    enabled: true,
    powerAction: draft.powerAction,
    nextTriggerAt: null,
  }
  await queueSettings(async () => {
    const previous = [...data.value.reminders]
    data.value.reminders.push(reminder)
    try {
      await persist()
      currentView.value = 'events'
      actionMessage.value = t('agent.added')
    } catch (error) {
      data.value.reminders = previous
      logError('add agent reminder', error)
      actionMessage.value = t('status.saveFailed')
    }
  })
}

async function toggleReminder(reminder: Reminder) {
  return queueSettings(async () => {
    const current = data.value.reminders.find((item) => item.id === reminder.id)
    if (!current) return
    const previous = current.enabled
    current.enabled = !current.enabled
    try {
      await persist()
    } catch (error) {
      current.enabled = previous
      logError('toggle reminder', error)
      actionMessage.value = t('status.saveFailed')
    }
  })
}

async function removeReminder(id: string) {
  const reminder = data.value.reminders.find((item) => item.id === id)
  if (!reminder) return
  const confirmed = await ask(t('events.deleteConfirm', { title: reminder.title }), {
    title: t('events.deleteConfirmTitle'),
    kind: 'warning',
    okLabel: t('events.delete'),
    cancelLabel: t('common.cancel'),
  })
  if (!confirmed) return

  return queueSettings(async () => {
    const previous = data.value.reminders
    data.value.reminders = data.value.reminders.filter((item) => item.id !== id)
    try {
      await persist()
      actionMessage.value = t('status.reminderDeleted')
    } catch (error) {
      data.value.reminders = previous
      logError('delete reminder', error)
      actionMessage.value = t('status.saveFailed')
    }
  })
}

async function updateSetting<K extends keyof AppData['settings']>(key: K, value: AppData['settings'][K], event?: MouseEvent) {
  const rect = (event?.currentTarget as HTMLElement | null)?.getBoundingClientRect()
  const origin = event && rect ? { x: event.detail ? event.clientX : rect.left + rect.width / 2, y: event.detail ? event.clientY : rect.top + rect.height / 2 } : null
  return queueSettings(async () => {
    if (!origin || (key !== 'theme' && key !== 'accentColor') || data.value.settings[key] === value) {
      return saveSetting(key, value)
    }
    return animateAppearance(key, origin, () => saveSetting(key, value))
  })
}

// 手动主题、系统主题和配色共用快照动画；动画不可用时照常执行原操作。
async function animateAppearance(kind: 'theme' | 'accentColor', origin: { x: number; y: number }, operation: () => Promise<boolean>) {
  if (!document.startViewTransition || window.matchMedia('(prefers-reduced-motion: reduce)').matches) return operation()
  let result: Promise<boolean> | undefined
  document.documentElement.classList.add('appearance-changing')
  try {
    const transition = document.startViewTransition(async () => {
      result = operation()
      await nextTick()
    })
    try {
      await transition.ready
      const radius = Math.hypot(Math.max(origin.x, window.innerWidth - origin.x), Math.max(origin.y, window.innerHeight - origin.y))
      const frames = kind === 'theme'
        ? { clipPath: [`circle(0px at ${origin.x}px ${origin.y}px)`, `circle(${radius}px at ${origin.x}px ${origin.y}px)`] }
        : { opacity: [0, 1] }
      document.documentElement.animate(frames, { duration: kind === 'theme' ? 450 : 480, easing: 'ease-out', pseudoElement: '::view-transition-new(root)' })
    } catch {
      // 隐藏窗口或不支持快照动画时，设置保存仍照常完成。
    }
    await transition.finished.catch(() => {})
    return await (result ?? operation())
  } catch {
    return await (result ?? operation())
  } finally {
    document.documentElement.classList.remove('appearance-changing')
  }
}

function handleSystemThemeChange(event: MediaQueryListEvent) {
  void queueSettings(async () => {
    if (event.matches === systemDark.value) return
    const apply = async () => { systemDark.value = event.matches; return true }
    if (data.value.settings.theme !== 'system') { await apply(); return }
    const rect = document.querySelector<HTMLElement>('[data-system-theme]')?.getBoundingClientRect()
    const origin = rect && rect.width && rect.top >= 0 && rect.bottom <= window.innerHeight
      ? { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 }
      : { x: window.innerWidth / 2, y: window.innerHeight / 2 }
    await animateAppearance('theme', origin, apply)
  })
}

async function saveSetting<K extends keyof AppData['settings']>(key: K, value: AppData['settings'][K]) {
  const previous = data.value.settings[key]
  const modeRevision = externalModeRevision
  data.value.settings = { ...data.value.settings, [key]: value }
  try {
    await persist(key === 'popupFullscreen' ? value as boolean : undefined)
    await refreshTimers()
    if (key === 'systemNotificationEnabled') notificationError.value = ''
    return true
  } catch (error) {
    if (key !== 'popupFullscreen' || modeRevision === externalModeRevision) {
      data.value.settings = { ...data.value.settings, [key]: previous }
    }
    if (key === 'restMessage') restMessageDraft.value = data.value.settings.restMessage
    logError(`save setting: ${String(key)}`, error)
    actionMessage.value = t('status.saveFailed')
    return false
  }
}

async function applySetting(key: keyof AppData['settings'], value: AppData['settings'][keyof AppData['settings']], event?: MouseEvent) {
  if (key === 'autostart') {
    await updateAutostart(Boolean(value))
    return
  }
  if (key === 'systemNotificationEnabled' && value === true) {
    // macOS 上被用户拒绝过的通知会静默失败，开启前先确认权限。
    if (!await ensureNotificationPermission()) return
  }
  if (key === 'language') {
    await updateLanguage(value as Language)
    return
  }
  await updateSetting(key, value as never, event)
}

async function ensureNotificationPermission() {
  try {
    if (await isPermissionGranted()) return true
    if (await requestPermission() === 'granted') return true
  } catch {
    // The standalone Vite preview has no permission bridge; do not block the toggle there.
    return true
  }
  notificationError.value = t('status.notificationDenied')
  return false
}

async function updateAutostart(value: boolean) {
  return queueSettings(async () => {
    autostartError.value = ''
    try {
      applyAutostartStatus(await invoke<AutostartStatus>('set_autostart', { enabled: value }))
    } catch (error) {
      logError('update autostart', error)
      await refreshAutostart()
      autostartError.value = startupMessage(formatError(error))
    }
  })
}

function startupMessage(reason: string) {
  const keys: Record<string, MessageKey> = {
    conflict: 'settings.autostartConflict',
    disabledByUser: 'settings.autostartDisabledByUser',
    disabledByPolicy: 'settings.autostartDisabledByPolicy',
    enabledByPolicy: 'settings.autostartDisabledByPolicy',
    temporaryPath: 'settings.autostartTemporaryPath',
  }
  return keys[reason] ? t(keys[reason]) : t('status.autostartFailed', { error: reason })
}

function applyAutostartStatus(status: AutostartStatus) {
  data.value.settings.autostart = status.enabled
  autostartNotice.value = status.conflict ? t('settings.autostartConflict') : status.reason ? startupMessage(status.reason) : ''
  autostartError.value = ''
}

async function refreshAutostart() {
  try {
    applyAutostartStatus(await invoke<AutostartStatus>('get_autostart_status'))
  } catch (error) {
    logError('read autostart status', error)
    autostartError.value = startupMessage(formatError(error))
  }
}

function localizedDefaultMessages(language: Language) {
  return {
    rest: translate(language, 'rest.defaultMessage'),
  }
}

async function updateLanguage(language: Language) {
  return queueSettings(async () => {
    const previous = data.value.settings
    const previousReminders = data.value.reminders
    const currentPresets = defaultReminders(previous.language)
    const nextPresets = defaultReminders(language)
    data.value.reminders = previousReminders.map((reminder) => {
      const index = currentPresets.findIndex((preset) => preset.id === reminder.id && preset.title === reminder.title)
      return index >= 0 ? { ...reminder, title: nextPresets[index].title } : reminder
    })
    const currentDefaults = localizedDefaultMessages(previous.language)
    const nextDefaults = localizedDefaultMessages(language)
    data.value.settings = {
      ...previous,
      language,
      restMessage: previous.restMessage === currentDefaults.rest ? nextDefaults.rest : previous.restMessage,
    }
    restMessageDraft.value = data.value.settings.restMessage
    try {
      await persist()
      await refreshTimers()
    } catch (error) {
      data.value.settings = { ...data.value.settings, language: previous.language, restMessage: previous.restMessage }
      data.value.reminders = previousReminders
      restMessageDraft.value = previous.restMessage
      logError('save language', error)
      actionMessage.value = t('status.saveFailed')
    }
  })
}

async function updateRestInterval(raw: string) {
  const value = Number(raw)
  if (Number.isInteger(value) && value >= 1 && value <= 1440) {
    await updateSetting('restIntervalMinutes', value)
  }
}

async function saveRestMessage(value: string) {
  return updateSetting('restMessage', value)
}

// 选图后由 Rust 复制成配置目录里的固定文件名，清空则是删掉该文件。
// 配置里不记录图片信息，弹窗按文件是否存在决定要不要显示背景。
async function pickPopupImage() {
  actionMessage.value = ''
  try {
    const path = await open({
      multiple: false,
      directory: false,
      filters: [{ name: t('settings.imageFilter'), extensions: ['png', 'jpg', 'jpeg', 'webp', 'gif', 'bmp', 'avif'] }],
    })
    if (typeof path !== 'string') return
    const dataUrl = await invoke<string>('import_popup_image', { source: path })
    popupBackgroundPreview.value = dataUrl
    popupImageIsDefault.value = await invoke<boolean>('is_popup_image_default').catch(() => false)
    actionMessage.value = t('status.imageSaved')
  } catch (error) {
    logError('pick popup image', error)
    actionMessage.value = t('status.imageImportFailed', { error: formatError(error) })
  }
}

async function clearPopupImage() {
  actionMessage.value = ''
  try {
    await invoke('clear_popup_image')
    popupBackgroundPreview.value = ''
    popupImageIsDefault.value = false
    actionMessage.value = t('status.imageCleared')
  } catch (error) {
    logError('clear popup image', error)
    actionMessage.value = t('status.imageFailed')
  }
}

async function loadPopupImagePreview() {
  try {
    popupBackgroundPreview.value = (await invoke<string | null>('read_popup_image')) ?? ''
    popupImageIsDefault.value = await invoke<boolean>('is_popup_image_default').catch(() => false)
  } catch {
    popupBackgroundPreview.value = ''
    popupImageIsDefault.value = false
  }
}

async function importData() {
  return queueSettings(async () => {
    actionMessage.value = ''
    try {
      await persistQueue
      const imported = await invoke<AppData | null>('import_data')
      if (imported) {
        data.value = imported
        restMessageDraft.value = data.value.settings.restMessage
        actionMessage.value = t('status.imported')
        await refreshTimers()
      }
    } catch (error) {
      logError('import data', error)
      actionMessage.value = t('status.importFailed')
    }
  })
}

async function exportData() {
  await settingsQueue
  await persistQueue
  actionMessage.value = ''
  try {
    if (await invoke<boolean>('export_data')) {
      actionMessage.value = t('status.exported')
    }
  } catch (error) {
    logError('export data', error)
    actionMessage.value = t('status.exportFailed')
  }
}

// 只恢复参数默认值，提醒列表原样保留。自启单独同步，原生主题由后端保存时统一应用。
async function resetSettings() {
  actionMessage.value = ''
  const confirmed = await confirm(t('settings.resetConfirmBody'), {
    title: t('settings.resetConfirmTitle'),
    kind: 'warning',
  })
  if (!confirmed) return

  return queueSettings(async () => {
    await persistQueue
    const previous = data.value.settings
    const modeRevision = externalModeRevision
    const defaults = defaultData().settings
    data.value.settings = {
      ...defaults,
      autostart: previous.autostart,
      language: previous.language,
      restMessage: translate(previous.language, 'rest.defaultMessage'),
    }
    restMessageDraft.value = data.value.settings.restMessage
    try {
      await persist(defaults.popupFullscreen)
      await refreshTimers()
    } catch (error) {
      data.value.settings = {
        ...previous,
        popupFullscreen: modeRevision === externalModeRevision ? previous.popupFullscreen : data.value.settings.popupFullscreen,
      }
      restMessageDraft.value = previous.restMessage
      logError('reset settings', error)
      actionMessage.value = t('status.saveFailed')
      return
    }
    autostartError.value = ''
    notificationError.value = ''
    // 参数已落盘，恢复资源失败时保留真实图片状态，不回滚成旧参数。
    try {
      popupBackgroundPreview.value = await invoke<string>('reset_popup_image')
      popupImageIsDefault.value = true
      actionMessage.value = t('status.resetDone')
    } catch (error) {
      logError('restore popup image on reset', error)
      await loadPopupImagePreview()
      actionMessage.value = t('status.imageFailed')
    }
  })
}

async function testNotification(kind: TestReminderKind, reminder?: Reminder) {
  await settingsQueue
  actionMessage.value = ''
  notificationError.value = ''
  try {
    await persist()
    await invoke('test_reminder', { kind, reminder: reminder ?? null })
  } catch (error) {
    logError('test notification', error)
    const message = t('status.notificationFailed', { error: formatError(error) })
    notificationError.value = message
    actionMessage.value = message
  }
}

async function refreshTimers() {
  if (isPopup) return
  const refreshToken = ++timerRefreshToken
  const [rest] = await Promise.allSettled([
    invoke<RestTimerStatus>('get_rest_timer_status'),
  ])
  if (refreshToken !== timerRefreshToken) return
  if (rest.status === 'fulfilled') {
    nextRestTrigger.value = rest.value.nextTriggerAt
    restIsActive.value = rest.value.isResting
  }
  now.value = Date.now()
}

async function retryPersistence() {
  return queueSettings(async () => {
    await persistQueue
    await refreshNativeErrors()
    if (schedulerError.value) return
    try {
      data.value = await invoke<AppData>('load_data')
      restMessageDraft.value = data.value.settings.restMessage
      persistenceError.value = ''
      await refreshTimers()
    } catch (error) {
      logError('retry persistence', error)
      persistenceError.value = formatError(error)
    }
  })
}

async function refreshNativeErrors() {
  const revision = nativeErrorRevision
  try {
    const errors = await invoke<NativeErrors>('get_native_errors')
    if (errors.autostartError) autostartError.value = t('status.autostartFailed', { error: errors.autostartError })
    if (errors.notificationError) notificationError.value = t('status.notificationFailed', { error: errors.notificationError })
    // 致命故障只在进程重启后恢复，不能被较早的状态响应或保存成功覆盖。
    schedulerError.value ||= errors.schedulerError || ''
    if (revision === nativeErrorRevision) persistenceError.value = errors.persistenceError || ''
  } catch (error) {
    if ('__TAURI_INTERNALS__' in window) {
      schedulerError.value ||= formatError(error)
      logError('read native errors', error)
    }
  }
}

function applyRestTimerStatus(status: RestTimerStatus) {
  timerRefreshToken += 1
  restIsActive.value = status.isResting
  nextRestTrigger.value = status.nextTriggerAt
  now.value = Date.now()
}

function handleMainWindowEscape(event: KeyboardEvent) {
  if (event.key !== 'Escape' || event.repeat) return
  void invoke('hide_idle_window').catch((error) => logError('hide main window', error))
}

onMounted(async () => {
  if (isPopup) return
  systemTheme.addEventListener?.('change', handleSystemThemeChange)
  window.addEventListener('keydown', handleMainWindowEscape)
  void loadPopupImagePreview()
  try {
    unlistenSchedulerFailure = await getCurrentWindow().listen<string>('scheduler-failed', (event) => {
      nativeErrorRevision += 1
      schedulerError.value = event.payload
    })
    unlistenPersistenceFailure = await getCurrentWindow().listen<string | null>('persistence-failed', (event) => {
      nativeErrorRevision += 1
      persistenceError.value = event.payload || ''
    })
    unlistenNavigation = await getCurrentWindow().listen<View>('navigate-to', (event) => {
      currentView.value = event.payload
    })
    data.value = await invoke<AppData>('load_data')
    restMessageDraft.value = data.value.settings.restMessage
    try {
      const pendingNavigation = await invoke<View | null>('take_pending_navigation')
      if (pendingNavigation) currentView.value = pendingNavigation
    } catch {
      // The standalone preview and older bridge mocks do not expose navigation state.
    }
    try {
      appVersion.value = (await getVersion()).replace(/\.0$/, '')
    } catch {
      // Keep the package-version fallback in standalone preview mode.
    }
    await refreshNativeErrors()
    await refreshAutostart()
    unlistenWindowFocus = await getCurrentWindow().onFocusChanged(() => {
      void refreshTimers()
      void refreshNativeErrors()
      void queueSettings(refreshAutostart)
    })
    unlisten = await getCurrentWindow().listen<ReminderTriggeredEvent>('reminder-triggered', async () => {
      await queueSettings(async () => {
        try {
          data.value = await invoke<AppData>('load_data')
          restMessageDraft.value = data.value.settings.restMessage
        } catch {
          // Keep the current data while the backend is temporarily unavailable.
        }
        await refreshTimers()
      })
    })
    unlistenRestTimer = await getCurrentWindow().listen<RestTimerStatus>('rest-timer-updated', (event) => {
      applyRestTimerStatus(event.payload)
    })
    unlistenNotificationFailure = await getCurrentWindow().listen<string>('notification-failed', (event) => {
      const message = t('status.notificationFailed', { error: event.payload })
      notificationError.value = message
      actionMessage.value = message
    })
    // 外部切换只更新相关字段，不能覆盖正在编辑的其他参数。
    unlistenSettingsSync = await getCurrentWindow().listen<boolean>('popup-fullscreen-updated', (event) => {
      externalModeRevision += 1
      persistToken += 1
      data.value.settings.popupFullscreen = event.payload
    })
    await refreshTimers()
    clockTimer = window.setInterval(() => {
      now.value = Date.now()
    }, 1000)
  } catch (error) {
    await refreshNativeErrors()
    logError('load application data', error)
    actionMessage.value = t('status.loadFailed')
  }
})

onUnmounted(() => {
  systemTheme.removeEventListener?.('change', handleSystemThemeChange)
  window.removeEventListener('keydown', handleMainWindowEscape)
  unlisten?.()
  unlistenNavigation?.()
  unlistenRestTimer?.()
  unlistenNotificationFailure?.()
  unlistenPersistenceFailure?.()
  unlistenSchedulerFailure?.()
  unlistenSettingsSync?.()
  unlistenWindowFocus?.()
  if (clockTimer) window.clearInterval(clockTimer)
  if (actionMessageTimer) window.clearTimeout(actionMessageTimer)
})
</script>

<template>
  <ReminderPopup v-if="isPopup" />
  <div v-else :class="['app-shell', `theme-${displayedTheme}`, `accent-${data.settings.accentColor}`]">
    <div class="ambient" aria-hidden="true"><i class="orb orb-a"></i><i class="orb orb-b"></i><i class="orb orb-c"></i></div>

    <AppTopBar
      :current-view="currentView"
      :language="data.settings.language"
      :app-version="appVersion"
      @navigate="currentView = $event; settingsTab = 'rhythm'"
    />

    <main :class="['content', { 'content-about': currentView === 'about' }]">
      <div v-if="schedulerError" class="page-banner" role="alert">
        <span>{{ t('status.schedulerFailed', { error: schedulerError }) }}</span>
      </div>
      <div v-if="persistenceError" class="page-banner" role="alert">
        <span>{{ t('status.persistenceFailed', { error: persistenceError }) }}</span>
        <button v-if="!schedulerError" class="button" type="button" @click="retryPersistence">{{ t('common.retry') }}</button>
      </div>

      <Transition name="view" mode="out-in">
      <EventsView
        v-if="currentView === 'events'"
        :language="data.settings.language"
        :subtitle="t('events.subtitle')"
        :show-form="showForm"
        :editing-id="editingId"
        :form="form"
        :power-action-options="powerActionOptions"
        :frequency-options="frequencyOptions"
        :weekday-options="weekdayOptions"
        :type-labels="typeLabels"
        :reminders="sortedReminders"
        :action-message="actionMessage"
        :format-next="formatNext"
        @test-notification="testNotification('event')"
        @test-reminder="testNotification('event', $event)"
        @add="openAddForm"
        @close-form="showForm = false"
        @save="saveReminder"
        @update:form="patchForm"
        @toggle-weekday="toggleNumber(form.weekdays, $event)"
        @toggle-month-day="toggleNumber(form.monthDays, $event)"
        @toggle-reminder="toggleReminder"
        @edit-reminder="editReminder"
        @remove-reminder="removeReminder"
      />

      <RestView
        v-else-if="currentView === 'rest'"
        :language="data.settings.language"
        :subtitle="t('app.tagline')"
        :enabled="data.settings.restEnabled"
        :interval-minutes="data.settings.restIntervalMinutes"
        :message="restMessageDraft"
        :progress="restProgress"
        :status="restStatusText"
        :action-message="actionMessage"
        @test-notification="testNotification('rest')"
        @preview-settings="settingsTab = 'stage'; currentView = 'settings'"
        @update:enabled="updateSetting('restEnabled', $event)"
        @update:interval="updateRestInterval"
        @update:message="restMessageDraft = $event"
        @message-committed="saveRestMessage($event)"
      />

      <SettingsView
        v-else-if="currentView === 'settings'"
        :language="data.settings.language"
        :settings="data.settings"
        v-model:initial-tab="settingsTab"
        :accent-colors="accentColors"
        :autostart-error="autostartError"
        :autostart-notice="autostartNotice"
        :notification-error="notificationError"
        :action-message="actionMessage"
        :can-reset="canResetSettings"
        :background-preview="popupBackgroundPreview"
        @update:setting="applySetting"
        @import-data="importData"
        @export-data="exportData"
        @reset-settings="resetSettings"
        @pick-popup-image="pickPopupImage"
        @clear-popup-image="clearPopupImage"
      />

      <AgentView
        v-else-if="currentView === 'agent'"
        :language="data.settings.language"
        @add-reminder="addAgentReminder"
        @notify="actionMessage = $event"
      />

      <AboutView
        v-else
        :language="data.settings.language"
        :app-version="appVersion"
      />
      </Transition>
    </main>
  </div>
</template>
