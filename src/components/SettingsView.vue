<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, reactive, ref, watch } from 'vue'
import { Download, Image, RotateCcw, Upload } from '@lucide/vue'
import { translate } from '../i18n'
import type { MessageKey } from '../i18n'
import type { AccentColor, AppSettings, Language, PopupBackgroundFit, SettingsTab, Theme } from '../types'
import PopupBackground from './PopupBackground.vue'

const props = defineProps<{
  language: Language
  settings: AppSettings
  accentColors: AccentColor[]
  autostartError: string
  autostartNotice?: string
  notificationError: string
  actionMessage: string
  canReset: boolean
  backgroundPreview: string
  initialTab?: SettingsTab
}>()

const emit = defineEmits<{
  'update:initialTab': [tab: SettingsTab]
  'update:setting': [key: keyof AppSettings, value: AppSettings[keyof AppSettings], event?: MouseEvent]
  importData: []
  exportData: []
  resetSettings: []
  pickPopupImage: []
  clearPopupImage: []
}>()

const fitOptions = computed<{ value: PopupBackgroundFit; label: string }[]>(() => [
  { value: 'stretch', label: t('settings.fitStretch') },
  { value: 'contain', label: t('settings.fitContain') },
])

const previewMode = ref<'fullscreen' | 'windowed'>('fullscreen')
const inputDraft = reactive<Partial<AppSettings>>({})
const previewSettings = computed(() => ({ ...props.settings, ...inputDraft }))
const previewMessage = ref<HTMLElement | null>(null)
let previewObserver: ResizeObserver | undefined

// 预览尺寸小于真实弹窗，按文字可用空间收缩字号，避免长文案被裁切。
async function fitPreviewText() {
  await nextTick()
  const message = previewMessage.value
  if (!message || !message.clientHeight) return
  let size = props.settings.popupTitleSize
  message.style.fontSize = `${size}px`
  while (size > 8 && (message.scrollHeight > message.clientHeight || message.scrollWidth > message.clientWidth)) {
    size -= 1
    message.style.fontSize = `${size}px`
  }
}

watch(() => [props.settings.popupTitleSize, props.settings.restMessage, previewMode.value], fitPreviewText)
onMounted(() => {
  previewObserver = new ResizeObserver(() => void fitPreviewText())
  if (previewMessage.value) previewObserver.observe(previewMessage.value)
  void fitPreviewText()
})
onUnmounted(() => previewObserver?.disconnect())
const activeTab = ref(props.initialTab || 'general')
watch(() => props.initialTab, (tab) => { activeTab.value = tab || 'general' })
watch(activeTab, (tab) => emit('update:initialTab', tab))
const tabs = [
  { id: 'popup', label: 'settings.groupPopup' },
  { id: 'general', label: 'settings.groupGeneral' },
  { id: 'appearance', label: 'settings.groupAppearance' },
  { id: 'notification', label: 'settings.groupNotification' },
  { id: 'data', label: 'settings.groupData' },
] as const

function navigateTabs(event: KeyboardEvent) {
  const index = tabs.findIndex(tab => tab.id === activeTab.value)
  let next = index
  if (event.key === 'ArrowRight') next = (index + 1) % tabs.length
  else if (event.key === 'ArrowLeft') next = (index + tabs.length - 1) % tabs.length
  else if (event.key === 'Home') next = 0
  else if (event.key === 'End') next = tabs.length - 1
  else return
  event.preventDefault()
  activeTab.value = tabs[next]!.id
  const buttons = (event.currentTarget as HTMLElement).querySelectorAll<HTMLButtonElement>('[role="tab"]')
  buttons[next]?.focus()
}
const imageControls = [
  { key: 'popupBackgroundScale', label: 'settings.imageScale', min: 1, max: 400 },
  { key: 'popupBackgroundOffsetX', label: 'settings.imageOffsetX', min: -50, max: 50 },
  { key: 'popupBackgroundOffsetY', label: 'settings.imageOffsetY', min: -50, max: 50 },
] as const

function updateImageValue(key: typeof imageControls[number]['key'] | 'popupTitleSize' | 'popupOverlayOpacity', event: Event, min: number, max: number, previewOnly = false) {
  const value = Number((event.target as HTMLInputElement).value)
  if (!Number.isFinite(value)) return
  const normalized = Math.round(Math.min(max, Math.max(min, value)))
  if (previewOnly) inputDraft[key] = normalized
  else {
    delete inputDraft[key]
    emit('update:setting', key, normalized)
  }
}

function updateTextColor(event: Event, previewOnly = false) {
  const value = (event.target as HTMLInputElement).value
  if (previewOnly) inputDraft.popupTextColor = value
  else {
    delete inputDraft.popupTextColor
    emit('update:setting', 'popupTextColor', value)
  }
}

function t(key: MessageKey, params: Record<string, string | number> = {}) {
  return translate(props.language, key, params)
}
</script>

<template>
  <section class="page-section preferences-page">
    <header class="page-header compact-header">
      <div>
        <h1>{{ t('settings.title') }}</h1>
        <p class="page-subtitle">{{ t('settings.subtitle') }}</p>
      </div>
    </header>

    <div class="settings-tabs" role="tablist" :aria-label="t('settings.title')" @keydown="navigateTabs">
      <button v-for="tab in tabs" :id="`settings-tab-${tab.id}`" :key="tab.id" type="button" role="tab" :aria-selected="activeTab === tab.id" :aria-controls="`settings-panel-${tab.id}`" :tabindex="activeTab === tab.id ? 0 : -1" @click="activeTab = tab.id">{{ t(tab.label) }}</button>
    </div>

    <div v-show="activeTab === 'general'" id="settings-panel-general" class="settings-group" role="tabpanel" aria-labelledby="settings-tab-general">
      <div class="setting-card setting-choice">
        <div><strong>{{ t('settings.language') }}</strong><span>{{ t('settings.languageHint') }}</span></div>
        <div class="segmented">
          <button :class="{ selected: settings.language === 'zh-CN' }" type="button" @click="emit('update:setting', 'language', 'zh-CN')">{{ t('settings.zh') }}</button>
          <button :class="{ selected: settings.language === 'en' }" type="button" @click="emit('update:setting', 'language', 'en')">{{ t('settings.en') }}</button>
        </div>
      </div>
      <label class="setting-card setting-toggle">
        <div><strong>{{ t('settings.autostart') }}</strong><span>{{ t('settings.autostartHint') }}</span><span v-if="autostartNotice && autostartNotice !== autostartError">{{ autostartNotice }}</span><small v-if="autostartError" class="setting-error">{{ autostartError }}</small></div>
        <input :checked="settings.autostart" type="checkbox" @change="emit('update:setting', 'autostart', ($event.target as HTMLInputElement).checked)" />
      </label>
      <label class="setting-card setting-toggle">
        <div><strong>{{ t('settings.startHidden') }}</strong><span>{{ t('settings.startHiddenHint') }}</span></div>
        <input :checked="settings.minimizeToTray" type="checkbox" @change="emit('update:setting', 'minimizeToTray', ($event.target as HTMLInputElement).checked)" />
      </label>
    </div>

    <div v-show="activeTab === 'appearance'" id="settings-panel-appearance" class="settings-group" role="tabpanel" aria-labelledby="settings-tab-appearance">
      <div class="setting-card setting-choice">
        <div><strong>{{ t('settings.appearance') }}</strong><span>{{ t('settings.appearanceHint') }}</span></div>
        <div class="segmented">
          <button data-system-theme :class="{ selected: settings.theme === 'system' }" type="button" @click="emit('update:setting', 'theme', 'system' as Theme, $event)">{{ t('settings.system') }}</button>
          <button :class="{ selected: settings.theme === 'dark' }" type="button" @click="emit('update:setting', 'theme', 'dark' as Theme, $event)">{{ t('settings.dark') }}</button>
          <button :class="{ selected: settings.theme === 'light' }" type="button" @click="emit('update:setting', 'theme', 'light' as Theme, $event)">{{ t('settings.light') }}</button>
        </div>
      </div>
      <div class="setting-card color-setting">
        <div><strong>{{ t('settings.accent') }}</strong><span>{{ t('settings.accentHint') }}</span></div>
        <div class="color-options">
          <button
            v-for="color in accentColors"
            :key="color"
            :class="['color-swatch', `swatch-${color}`, { selected: settings.accentColor === color }]"
            type="button"
            :aria-label="t(`settings.color${color}`)"
            :title="t(`settings.color${color}`)"
            @click="emit('update:setting', 'accentColor', color, $event)"
          ></button>
        </div>
      </div>
    </div>

    <div v-show="activeTab === 'popup'" id="settings-panel-popup" class="settings-group" role="tabpanel" aria-labelledby="settings-tab-popup">
      <label class="setting-card setting-toggle">
        <div><strong>{{ t('settings.fullscreenPopup') }}</strong><span>{{ t('settings.fullscreenPopupHint') }}</span></div>
        <input :checked="settings.popupFullscreen" :aria-label="t('settings.fullscreenPopup')" type="checkbox" @change="emit('update:setting', 'popupFullscreen', ($event.target as HTMLInputElement).checked)" />
      </label>
      <label class="setting-card setting-toggle">
        <div><strong>{{ t('settings.alwaysOnTop') }}</strong><span>{{ t('settings.alwaysOnTopHint') }}</span></div>
        <input :checked="settings.popupAlwaysOnTop" :aria-label="t('settings.alwaysOnTop')" type="checkbox" @change="emit('update:setting', 'popupAlwaysOnTop', ($event.target as HTMLInputElement).checked)" />
      </label>
      <div class="setting-card stacked-setting">
        <div><strong>{{ t('settings.popupBackground') }}</strong><span>{{ t('settings.popupBackgroundHint') }}</span></div>
        <div class="background-picker">
          <img v-if="backgroundPreview" class="background-thumb" :src="backgroundPreview" alt="" />
          <span v-else class="background-thumb placeholder" aria-hidden="true"></span>
          <div class="action-row">
            <button class="button" type="button" @click="emit('pickPopupImage')"><Image :size="14" />{{ t('settings.popupChooseImage') }}</button>
            <button v-if="backgroundPreview" class="button" type="button" @click="emit('clearPopupImage')"><RotateCcw :size="14" />{{ t('settings.popupClearImage') }}</button>
          </div>
        </div>
      </div>
      <div v-if="backgroundPreview" class="setting-card stacked-setting">
        <div><strong>{{ t('settings.popupBackgroundFit') }}</strong><span>{{ t('settings.popupBackgroundFitHint') }}</span></div>
        <div class="segmented fit-segments">
          <button v-for="option in fitOptions" :key="option.value" :class="{ selected: settings.popupBackgroundFit === option.value }" type="button" @click="emit('update:setting', 'popupBackgroundFit', option.value)">{{ option.label }}</button>
        </div>
        <template v-if="settings.popupBackgroundFit === 'contain'">
          <label v-for="control in imageControls" :key="control.key" class="slider-row image-slider">
            <span>{{ t(control.label) }}</span>
            <input :aria-label="t(control.label)" :value="previewSettings[control.key]" :style="{ '--fill': `${(previewSettings[control.key] - control.min) / (control.max - control.min) * 100}%` }" type="range" :min="control.min" :max="control.max" step="1" @input="updateImageValue(control.key, $event, control.min, control.max, true)" @change="updateImageValue(control.key, $event, control.min, control.max)" />
            <span class="number-field"><input :aria-label="t(control.label)" :value="previewSettings[control.key]" type="number" :min="control.min" :max="control.max" step="1" @change="updateImageValue(control.key, $event, control.min, control.max)" /><span>%</span></span>
          </label>
        </template>
        <label class="slider-row">
          <span>{{ t('settings.popupOverlay') }}</span>
          <input :value="previewSettings.popupOverlayOpacity" :style="{ '--fill': `${previewSettings.popupOverlayOpacity}%` }" type="range" min="0" max="100" step="5" @input="updateImageValue('popupOverlayOpacity', $event, 0, 100, true)" @change="updateImageValue('popupOverlayOpacity', $event, 0, 100)" />
          <em>{{ previewSettings.popupOverlayOpacity }}%</em>
        </label>
      </div>
      <label class="setting-card setting-toggle">
        <div><strong>{{ t('settings.popupFade') }}</strong><span>{{ t('settings.popupFadeHint') }}</span></div>
        <input :checked="settings.popupFadeEnabled" type="checkbox" @change="emit('update:setting', 'popupFadeEnabled', ($event.target as HTMLInputElement).checked)" />
      </label>
      <div class="setting-card">
        <div><strong>{{ t('settings.popupText') }}</strong><span>{{ t('settings.popupTextHint') }}</span></div>
        <div class="text-controls">
          <span class="color-control">
            <input type="color" :aria-label="t('settings.popupText')" :value="previewSettings.popupTextColor || '#f3f4f6'" @input="updateTextColor($event, true)" @change="updateTextColor($event)" />
            <button v-if="settings.popupTextColor" class="button" type="button" @click="emit('update:setting', 'popupTextColor', '')">{{ t('settings.popupResetColor') }}</button>
          </span>
          <label class="number-field"><input :aria-label="t('settings.popupText')" :value="settings.popupTitleSize" type="number" min="20" max="72" step="1" @change="updateImageValue('popupTitleSize', $event, 20, 72)" /><span>px</span></label>
        </div>
      </div>
      <div class="popup-preview-section">
        <div class="preview-toolbar">
          <strong>{{ t('settings.imagePreview') }}</strong>
          <div class="segmented">
            <button type="button" :class="{ selected: previewMode === 'fullscreen' }" @click="previewMode = 'fullscreen'">{{ t('settings.previewFullscreen') }}</button>
            <button type="button" :class="{ selected: previewMode === 'windowed' }" @click="previewMode = 'windowed'">{{ t('settings.previewWindowed') }}</button>
          </div>
        </div>
        <div :class="['popup-preview', previewMode, { 'popup-has-image': Boolean(backgroundPreview) }]" :style="{ '--popup-text': previewSettings.popupTextColor || undefined }">
          <PopupBackground :settings="previewSettings" :url="backgroundPreview" />
          <span class="preview-title">知歇</span>
          <strong ref="previewMessage" class="preview-message">{{ settings.restMessage }}</strong>
        </div>
      </div>
    </div>

    <div v-show="activeTab === 'notification'" id="settings-panel-notification" class="settings-group" role="tabpanel" aria-labelledby="settings-tab-notification">
      <label class="setting-card setting-toggle">
        <div><strong>{{ t('settings.systemNotification') }}</strong><span>{{ t('settings.systemNotificationHint') }}</span><small v-if="notificationError" class="setting-error">{{ notificationError }}</small></div>
        <input :checked="settings.systemNotificationEnabled" type="checkbox" @change="emit('update:setting', 'systemNotificationEnabled', ($event.target as HTMLInputElement).checked)" />
      </label>
    </div>

    <div v-show="activeTab === 'data'" id="settings-panel-data" class="settings-group" role="tabpanel" aria-labelledby="settings-tab-data">
      <div class="data-actions">
        <div class="data-actions-row">
          <div><strong>{{ t('settings.data') }}</strong><span>{{ t('settings.dataHint') }}</span></div>
          <div class="action-row">
            <button class="button" type="button" @click="emit('importData')"><Upload :size="14" />{{ t('settings.import') }}</button>
            <button class="button" type="button" @click="emit('exportData')"><Download :size="14" />{{ t('settings.export') }}</button>
          </div>
        </div>
      </div>
      <div class="setting-card">
        <div><strong>{{ t('settings.reset') }}</strong><span>{{ t('settings.resetHint') }}</span></div>
        <button class="button danger" type="button" :disabled="!canReset" @click="emit('resetSettings')"><RotateCcw :size="14" />{{ t('settings.reset') }}</button>
      </div>
    </div>
    <small v-if="actionMessage" class="status-message page-message" role="status">{{ actionMessage }}</small>
  </section>
</template>
