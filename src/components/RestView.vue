<script setup lang="ts">
import { ref, watch } from 'vue'
import { ArrowUpRight, Coffee, Play, Settings2 } from '@lucide/vue'
import { translate } from '../i18n'
import type { MessageKey } from '../i18n'
import type { Language } from '../types'

const props = defineProps<{
  language: Language
  subtitle: string
  enabled: boolean
  intervalMinutes: number
  message: string
  progress: number
  status: string
  actionMessage: string
}>()

const emit = defineEmits<{
  testNotification: []
  previewSettings: []
  'update:enabled': [value: boolean]
  'update:interval': [value: string]
  'update:message': [value: string]
  messageCommitted: [value: string]
}>()

const intervalDraft = ref(String(props.intervalMinutes))
const messageDraft = ref(props.message)
const editingInterval = ref(false)
const editingMessage = ref(false)

// 倒计时和后端同步会刷新父组件，输入草稿只在未编辑时跟随已保存参数。
watch(() => props.intervalMinutes, value => {
  if (!editingInterval.value) intervalDraft.value = String(value)
})
watch(() => props.message, value => {
  if (!editingMessage.value) messageDraft.value = value
})

function commitInterval() {
  const value = Number(intervalDraft.value)
  if (Number.isInteger(value) && value >= 1 && value <= 1440) {
    emit('update:interval', intervalDraft.value)
  } else {
    intervalDraft.value = String(props.intervalMinutes)
  }
}

function commitMessage() {
  emit('update:message', messageDraft.value)
  emit('messageCommitted', messageDraft.value)
}

function finishInput(event: KeyboardEvent) {
  if (!event.isComposing) (event.target as HTMLInputElement).blur()
}

function t(key: MessageKey, params: Record<string, string | number> = {}) {
  return translate(props.language, key, params)
}
</script>

<template>
  <section class="page-section rest-page">
    <header class="page-header reminder-header">
      <div>
        <h1>{{ t('rest.title') }}</h1>
        <p class="page-subtitle">{{ subtitle }}</p>
      </div>
      <button class="button test-notification" type="button" @click="emit('testNotification')">
        <Play :size="14" />{{ t('settings.testNotification') }}
      </button>
    </header>

    <div class="status-panel">
      <div class="status-panel-top">
        <span class="status-icon"><Coffee :size="18" /></span>
        <div>
          <span class="card-label">{{ t('rest.next') }}</span>
          <strong>{{ status }}</strong>
        </div>
        <label class="setting-toggle compact-toggle">
          <input :checked="enabled" :aria-label="t('rest.title')" type="checkbox" @change="emit('update:enabled', ($event.target as HTMLInputElement).checked)" />
        </label>
      </div>
      <div class="progress-track"><span :style="{ width: `${progress}%` }"></span></div>
      <p>{{ t('rest.scheduleHint') }}</p>
    </div>

    <div class="settings-group">
      <div class="setting-card">
        <div><strong>{{ t('rest.interval') }}</strong><span>{{ t('rest.intervalHint') }}</span></div>
        <label class="number-field">
          <input :value="intervalDraft" :aria-label="t('rest.interval')" type="number" min="1" max="1440" @input="intervalDraft = ($event.target as HTMLInputElement).value" @focus="editingInterval = true" @blur="editingInterval = false" @change="commitInterval" @keydown.enter="finishInput" />
          <span>{{ t('common.minutes') }}</span>
        </label>
      </div>
      <label class="setting-card stacked-setting">
        <div><strong>{{ t('rest.message') }}</strong><span>{{ t('rest.messageHint') }}</span></div>
        <input v-model="messageDraft" type="text" maxlength="120" @focus="editingMessage = true" @blur="editingMessage = false" @change="commitMessage" @keydown.enter="finishInput" />
      </label>
      <div class="setting-card">
        <div><strong>{{ t('settings.groupPopup') }}</strong></div>
        <button class="button popup-settings-link" type="button" @click="emit('previewSettings')"><Settings2 :size="14" />{{ t('rest.previewSettings') }}<ArrowUpRight :size="14" /></button>
      </div>
    </div>
    <small v-if="actionMessage" class="status-message page-message">{{ actionMessage }}</small>
  </section>
</template>
