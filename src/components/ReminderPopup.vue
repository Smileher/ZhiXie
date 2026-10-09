<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { invoke } from '@tauri-apps/api/core'
import { confirm } from '@tauri-apps/plugin-dialog'
import { Check, ChevronDown, Clock3, Maximize2, Minimize2, Settings2 } from '@lucide/vue'
import brandIcon from '../assets/logo.svg'
import { translate } from '../i18n'
import type { MessageKey } from '../i18n'
import { logError } from '../error'
import type { AppData, AppSettings, ReminderTriggeredEvent } from '../types'
import { defaultData } from '../types'
import PopupBackground from './PopupBackground.vue'

const current = ref<ReminderTriggeredEvent | null>(null)
const settings = ref<AppData['settings']>(defaultData().settings)
const triggeredAt = ref<Date | null>(null)
const currentWindow = getCurrentWindow()
const isController = currentWindow.label === 'reminder' || currentWindow.label === 'reminder-windowed'
const snoozeStorageKey = 'zhixie.popup.snoozeSeconds'
const snoozeValues = [30, 60, 300, 600, 1800, 3600, 7200, 10800, 14400]

function readStoredSnoozeSeconds() {
  try {
    const value = Number(localStorage.getItem(snoozeStorageKey))
    return snoozeValues.includes(value) ? value : 300
  } catch {
    return 300
  }
}

const snoozeSeconds = ref(readStoredSnoozeSeconds())
const snoozeMenu = ref<HTMLDetailsElement | null>(null)
const restElapsedSeconds = ref(0)
const powerCountdown = ref(60)
const powerError = ref('')
const popupAnimating = ref(false)
const popupAnimationKey = ref(0)
// 窗口透明时独立背景层渐入，文字先浮现；不透明窗口维持纯色底。
const seeThrough = ref(false)
let unlisten: (() => void) | undefined
let unlistenSettings: (() => void) | undefined
let unlistenImage: (() => void) | undefined
let unlistenRestCancelled: (() => void) | undefined
let unlistenReset: (() => void) | undefined
let unlistenClosed: (() => void) | undefined
let restTimer: number | undefined
let powerTimer: number | undefined
let restStartedAt = 0
let powerDeadline = 0
let notificationSequence = 0
let lastSessionId = 0

const popupClass = computed(() => [
  `theme-${settings.value.theme}`,
  `accent-${settings.value.accentColor}`,
  {
    'popup-fullscreen': settings.value.popupFullscreen,
    'popup-enter': popupAnimating.value,
    'popup-see-through': seeThrough.value,
    'popup-no-fade': !settings.value.popupFadeEnabled,
    'popup-has-image': Boolean(popupBackgroundUrl.value),
  },
])

// 自定义外观通过内联 CSS 变量下发，样式表只负责消费这些变量。
// 背景图没有配置项：文件存在就显示，删掉文件就没有背景。
const popupBackgroundUrl = ref('')
let imageReadToken = 0

// 弹窗窗口没有 app-shell 包裹，主题变量挂到 html 根上，
// body 底色与原生控件的 color-scheme 才能跟随深浅色。
watch(() => settings.value.theme, (theme) => {
  const root = document.documentElement
  root.classList.remove('theme-light', 'theme-dark', 'theme-system')
  root.classList.add(`theme-${theme}`)
}, { immediate: true })

async function refreshPopupBackground() {
  const token = ++imageReadToken
  try {
    const dataUrl = await invoke<string | null>('read_popup_image')
    if (token === imageReadToken) popupBackgroundUrl.value = dataUrl ?? ''
  } catch (error) {
    logError('read popup image', error)
    if (token === imageReadToken) popupBackgroundUrl.value = ''
  }
}

const popupStyleVars = computed(() => {
  const current = settings.value
  const style: Record<string, string> = {}
  if (current.popupTextColor) style['--popup-text'] = current.popupTextColor
  if (current.popupTitleSize) style['--popup-title-size'] = `${current.popupTitleSize}px`
  return style
})

function t(key: MessageKey, params: Record<string, string | number> = {}) {
  return translate(settings.value.language, key, params)
}

const triggeredAtLabel = computed(() => triggeredAt.value
  ? new Intl.DateTimeFormat(settings.value.language, { hour: '2-digit', minute: '2-digit' }).format(triggeredAt.value)
  : '')

const category = computed(() => {
  if (current.value?.isRest) return t('popup.rest')
  return t('popup.event')
})

const isAutomaticPower = computed(() =>
  current.value?.powerAction === 'shutdown'
    || current.value?.powerAction === 'lock'
    || current.value?.powerAction === 'restart',
)

const powerVerb = computed(() => {
  if (current.value?.powerAction === 'restart') return t('popup.restart')
  if (current.value?.powerAction === 'lock') return t('popup.lock')
  return t('popup.shutdown')
})

const snoozeOptions = computed(() => [
  { seconds: 30, label: t('popup.seconds', { seconds: 30 }) },
  { seconds: 60, label: t('popup.minutes', { minutes: 1 }) },
  { seconds: 300, label: t('popup.minutes', { minutes: 5 }) },
  { seconds: 600, label: t('popup.minutes', { minutes: 10 }) },
  { seconds: 1800, label: t('popup.minutes', { minutes: 30 }) },
  { seconds: 3600, label: t('popup.hours', { hours: 1 }) },
  { seconds: 7200, label: t('popup.hours', { hours: 2 }) },
  { seconds: 10800, label: t('popup.hours', { hours: 3 }) },
  { seconds: 14400, label: t('popup.hours', { hours: 4 }) },
])

const selectedSnoozeLabel = computed(() =>
  snoozeOptions.value.find((option) => option.seconds === snoozeSeconds.value)?.label
    || t('popup.minutes', { minutes: 5 }),
)

const restElapsed = computed(() => {
  const hours = Math.floor(restElapsedSeconds.value / 3600)
  const minutes = Math.floor(restElapsedSeconds.value / 60) % 60
  const seconds = restElapsedSeconds.value % 60
  return hours > 0
    ? t('popup.restedHours', { hours, minutes, seconds })
    : t('popup.rested', { minutes, seconds })
})

async function toggleFullscreenMode() {
  try {
    // 后端会保存设置并按新模式换窗，本窗口随后收到 reminder-closed 自动清理。
    await invoke('toggle_popup_fullscreen')
  } catch (error) {
    logError('toggle popup fullscreen', error)
  }
}

async function closePopup() {
  snoozeMenu.value?.removeAttribute('open')
  popupAnimating.value = false
  await invoke('hide_idle_window')
}

function clearPowerTimer() {
  if (powerTimer) window.clearInterval(powerTimer)
  powerTimer = undefined
}

function clearRestTimer() {
  if (restTimer) window.clearInterval(restTimer)
  restTimer = undefined
}

function startRestTimer(startedAtMs?: number | null) {
  clearRestTimer()
  restStartedAt = startedAtMs ?? Date.now()
  restElapsedSeconds.value = Math.max(0, Math.floor((Date.now() - restStartedAt) / 1000))
  restTimer = window.setInterval(() => {
    restElapsedSeconds.value = Math.max(0, Math.floor((Date.now() - restStartedAt) / 1000))
  }, 1000)
}

function selectSnooze(seconds: number) {
  snoozeSeconds.value = seconds
  try {
    localStorage.setItem(snoozeStorageKey, String(seconds))
  } catch {
    // Remembering the selection is best effort when local storage is unavailable.
  }
  snoozeMenu.value?.removeAttribute('open')
}

function closeSnoozeMenuOnOutsideClick(event: MouseEvent) {
  const menu = snoozeMenu.value
  if (menu?.open && event.target instanceof Node && !menu.contains(event.target)) {
    menu.removeAttribute('open')
  }
}

async function dismiss() {
  notificationSequence += 1
  const notification = current.value
  clearPowerTimer()
  clearRestTimer()
  if (notification) {
    await invoke('dismiss_reminder', { id: notification.id, sessionId: notification.sessionId })
  } else {
    await closePopup()
  }
}

async function snooze(seconds: number) {
  notificationSequence += 1
  const notification = current.value
  clearPowerTimer()
  clearRestTimer()
  if (notification) {
    await invoke('snooze_reminder', { id: notification.id, seconds, sessionId: notification.sessionId })
  } else {
    await closePopup()
  }
}

function cancelRest() {
  notificationSequence += 1
  if (!current.value?.isRest) return
  clearRestTimer()
  current.value = null
}

function resetReminders() {
  // 导入会取消当前提醒，也要阻止正在等待原生调用的旧弹窗重新显示。
  notificationSequence += 1
  clearPowerTimer()
  clearRestTimer()
  current.value = null
  triggeredAt.value = null
  restElapsedSeconds.value = 0
  powerCountdown.value = 60
  powerError.value = ''
  restStartedAt = 0
  powerDeadline = 0
}

async function executePowerAction() {
  clearPowerTimer()
  const sequence = notificationSequence
  const action = current.value?.powerAction
  if (action !== 'shutdown' && action !== 'lock' && action !== 'restart') return
  try {
    if (current.value?.isTest) {
      const confirmed = await confirm(t('popup.testPowerConfirm', { action: powerVerb.value }), {
        title: t('popup.testMode'),
        kind: 'warning',
        okLabel: t('popup.confirmExecute'),
        cancelLabel: t('common.cancel'),
      })
      if (sequence !== notificationSequence) return
      if (!confirmed) {
        await dismiss()
        return
      }
    }
    const executed = await invoke<boolean>('execute_power_action', { action, sessionId: current.value?.sessionId })
    if (!executed) return
    if (sequence !== notificationSequence) return
  } catch (error) {
    logError('execute power action', error)
    if (sequence !== notificationSequence) return
    powerError.value = t('popup.actionFailed', { action: powerVerb.value, error: String(error) })
  }
}

async function openPowerSettings() {
  try {
    await invoke('open_reminder_settings')
    await dismiss()
  } catch (error) {
    logError('open power settings', error)
    powerError.value = t('popup.settingsOpenFailed')
  }
}

function startPowerCountdown(deadlineMs?: number | null) {
  clearPowerTimer()
  powerDeadline = deadlineMs ?? Date.now() + 60_000
  powerCountdown.value = Math.max(0, Math.ceil((powerDeadline - Date.now()) / 1000))
  powerTimer = window.setInterval(() => {
    const remaining = Math.ceil((powerDeadline - Date.now()) / 1000)
    powerCountdown.value = Math.max(0, remaining)
    if (remaining <= 0) {
      clearPowerTimer()
      if (!isController) return
      if (Date.now() - powerDeadline <= 5_000) void executePowerAction()
      else void dismiss()
    }
  }, 1000)
}

async function handleTrigger(event: ReminderTriggeredEvent) {
  if (event.sessionId <= lastSessionId) return
  lastSessionId = event.sessionId
  const sequence = ++notificationSequence
  clearPowerTimer()
  clearRestTimer()
  current.value = event
  snoozeSeconds.value = readStoredSnoozeSeconds()
  restElapsedSeconds.value = 0
  powerError.value = ''
  try {
    const data = await invoke<AppData>('load_data')
    if (sequence !== notificationSequence) return
    settings.value = data.settings
  } catch {
    // Keep the last known settings if the backend is unavailable for a moment.
  }
  if (sequence !== notificationSequence) return
  triggeredAt.value = new Date()
  if (isAutomaticPower.value) startPowerCountdown(event.powerDeadlineMs)
  else if (event.isRest) startRestTimer(event.restStartedAtMs)
  popupAnimationKey.value += 1
  popupAnimating.value = true
  await nextTick()
  if (sequence !== notificationSequence) return
  // The backend checks the session again so a delayed response cannot reopen a closed popup.
  try {
    const shown = await invoke<boolean>('show_reminder', { sessionId: event.sessionId })
    if (!shown && sequence === notificationSequence) handleClosed(event.sessionId)
  } catch (error) {
    logError('show reminder', error)
  }
}

function handleEscape(event: KeyboardEvent) {
  if (event.key !== 'Escape' || event.repeat) return
  event.preventDefault()
  void dismiss()
}

function handleClosed(sessionId: number) {
  if (sessionId < lastSessionId) return
  lastSessionId = sessionId
  notificationSequence += 1
  clearPowerTimer()
  clearRestTimer()
  current.value = null
}

onMounted(async () => {
  document.addEventListener('click', closeSnoozeMenuOnOutsideClick)
  window.addEventListener('keydown', handleEscape)
  try {
    settings.value = (await invoke<AppData>('load_data')).settings
  } catch {
    // The standalone Vite preview has no Tauri command bridge.
  }
  await refreshPopupBackground()
  try {
    seeThrough.value = await invoke<boolean>('popup_window_is_transparent', { label: currentWindow.label })
    // body 默认刷了一层不透明底色，窗口透明时必须去掉，否则透明被挡死。
    if (seeThrough.value) document.body.classList.add('popup-transparent-body')
  } catch {
    seeThrough.value = false
  }
  try {
    unlistenReset = await currentWindow.listen('reminders-reset', () => void resetReminders())
  } catch {
    // The standalone Vite preview has no Tauri event bridge.
  }
  try {
    unlisten = await currentWindow.listen<ReminderTriggeredEvent>('reminder-triggered', (event) => void handleTrigger(event.payload))
  } catch {
    // The standalone Vite preview has no Tauri event bridge.
  }
  try {
    unlistenClosed = await currentWindow.listen<number>('reminder-closed', (event) => void handleClosed(event.payload))
  } catch {
    // The standalone Vite preview has no Tauri event bridge.
  }
  try {
    unlistenSettings = await currentWindow.listen<AppSettings>('settings-updated', async (event) => {
      const previous = settings.value
      settings.value = event.payload
      // macOS 上 setLevel 会打断进行中的外观过渡，冻结原生标题栏材质；
      // 置顶状态没变时必须跳过，否则切换深浅色后弹窗标题栏不再跟随。
      if (previous.popupAlwaysOnTop !== settings.value.popupAlwaysOnTop) {
        try {
          await currentWindow.setAlwaysOnTop(settings.value.popupAlwaysOnTop)
        } catch {
          // Window synchronization is best effort while the popup is closing.
        }
      }
    })
  } catch {
    // The standalone Vite preview has no Tauri event bridge.
  }
  try {
    unlistenImage = await currentWindow.listen('popup-image-updated', () => void refreshPopupBackground())
    // 监听建立前可能恰好发生选图，再读一次保证初始化没有漏掉变化。
    await refreshPopupBackground()
  } catch {
    // The standalone Vite preview has no Tauri event bridge.
  }
  try {
    unlistenRestCancelled = await currentWindow.listen('rest-cancelled', () => void cancelRest())
  } catch {
    // The standalone Vite preview has no Tauri event bridge.
  }
  try {
    const sequence = notificationSequence
    const active = await invoke<ReminderTriggeredEvent | null>('get_active_reminder')
    if (sequence === notificationSequence && active) await handleTrigger(active)
  } catch {
    // The standalone Vite preview has no Tauri command bridge.
  }
})

onUnmounted(() => {
  notificationSequence += 1
  imageReadToken += 1
  document.body.classList.remove('popup-transparent-body')
  document.removeEventListener('click', closeSnoozeMenuOnOutsideClick)
  window.removeEventListener('keydown', handleEscape)
  clearPowerTimer()
  clearRestTimer()
  unlisten?.()
  unlistenSettings?.()
  unlistenImage?.()
  unlistenRestCancelled?.()
  unlistenReset?.()
  unlistenClosed?.()
})
</script>

<template>
  <main :key="popupAnimationKey" :class="['popup-shell', ...popupClass]" :style="popupStyleVars">
    <PopupBackground :settings="settings" :url="popupBackgroundUrl" />
    <header class="popup-header">
      <div class="popup-identity"><img :src="brandIcon" alt="" /><div><strong>知歇</strong><span>{{ t('popup.time', { category, time: triggeredAtLabel || t('common.now') }) }}</span></div></div>
      <div class="popup-header-actions">
        <button
          class="popup-mode-button"
          type="button"
          :title="settings.popupFullscreen ? t('popup.toWindowed') : t('popup.toFullscreen')"
          :aria-label="settings.popupFullscreen ? t('popup.toWindowed') : t('popup.toFullscreen')"
          @click="toggleFullscreenMode"
        ><Minimize2 v-if="settings.popupFullscreen" :size="13" /><Maximize2 v-else :size="13" /></button>
        <button class="popup-escape-hint" type="button" :title="t('common.close')" :aria-label="t('common.close')" @click="dismiss">ESC</button>
      </div>
    </header>
    <section class="popup-content">
      <p v-if="current?.isRest" class="rest-elapsed">{{ restElapsed }}</p>
      <h1>{{ current?.title || t('popup.defaultTitle') }}</h1>
      <div v-if="isAutomaticPower" class="power-countdown"><strong>{{ powerCountdown }}</strong><span>{{ t('popup.secondsUntil', { action: powerVerb }) }}</span></div>
      <p v-if="powerError" class="popup-error">{{ powerError }}</p>
      <button v-if="powerError" class="button popup-settings-button" type="button" @click="openPowerSettings"><Settings2 :size="14" />{{ t('popup.openPowerSettings') }}</button>
    </section>
    <footer :class="['popup-actions', { 'split-actions': !isAutomaticPower }]">
      <template v-if="isAutomaticPower"><button class="button" type="button" @click="dismiss">{{ t('popup.cancelAction', { action: powerVerb }) }}</button></template>
      <template v-else>
        <div class="popup-action-group">
          <details ref="snoozeMenu" class="snooze-picker">
            <summary><Clock3 :size="15" /><span>{{ selectedSnoozeLabel }}</span><ChevronDown :size="15" class="select-chevron" /></summary>
            <div class="snooze-options">
              <button v-for="option in snoozeOptions" :key="option.seconds" :class="{ selected: snoozeSeconds === option.seconds }" type="button" @click="selectSnooze(option.seconds)"><span>{{ option.label }}</span><Check v-if="snoozeSeconds === option.seconds" :size="14" /></button>
            </div>
          </details>
          <button class="button" type="button" @click="snooze(snoozeSeconds)">{{ t('popup.snooze') }}</button>
        </div>
        <button v-if="current?.isRest" class="button button-primary" type="button" @click="dismiss"><Check :size="15" />{{ t('popup.finishRest') }}</button>
        <button v-else class="button button-primary" type="button" @click="dismiss"><Check :size="15" />{{ t('popup.done') }}</button>
      </template>
    </footer>
  </main>
</template>
