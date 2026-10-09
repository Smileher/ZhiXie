<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { Sparkles, Plus, Save, Plug, Zap, Wand } from '@lucide/vue'
import { invoke } from '@tauri-apps/api/core'
import { translate } from '../i18n'
import type { MessageKey } from '../i18n'
import type { AgentAdvice, AgentConfig, AgentDraft, AgentResult, Language } from '../types'
import { defaultAgentConfig } from '../types'
import { logError } from '../error'

const props = defineProps<{ language: Language }>()
const emit = defineEmits<{
  addReminder: [draft: AgentDraft]
  notify: [message: string]
}>()

const request = ref('')
const result = ref<AgentResult | null>(null)
const parsing = ref(false)
const parseError = ref('')
const config = ref<AgentConfig>(defaultAgentConfig())
const configMessage = ref('')
const advice = ref<AgentAdvice | null>(null)
const adviceLoading = ref(false)
const panel = ref<'usage' | 'config'>('usage')

const examples = ['agent.example1', 'agent.example2', 'agent.example3'] as const

function t(key: MessageKey, params: Record<string, string | number> = {}) {
  return translate(props.language, key, params)
}

onMounted(loadConfig)

async function loadConfig() {
  try {
    config.value = await invoke<AgentConfig>('load_agent_config')
  } catch (error) {
    logError('load agent config', error)
    emit('notify', t('agent.loadFailed'))
  }
}

async function saveConfig() {
  configMessage.value = ''
  try {
    config.value = await invoke<AgentConfig>('save_agent_config', { config: config.value })
    configMessage.value = t('agent.saved')
  } catch (error) {
    logError('save agent config', error)
    configMessage.value = t('agent.saveFailed')
  }
}

async function parseRequest() {
  const text = request.value.trim()
  if (!text) {
    parseError.value = t('agent.empty')
    return
  }
  parseError.value = ''
  parsing.value = true
  result.value = null
  try {
    result.value = await invoke<AgentResult>('agent_parse_reminder', { text })
  } catch (error) {
    logError('parse agent request', error)
    parseError.value = t('agent.parseFailed')
  } finally {
    parsing.value = false
  }
}

async function refreshAdvice() {
  adviceLoading.value = true
  try {
    advice.value = await invoke<AgentAdvice>('agent_break_advice', {
      restIntervalMinutes: 40,
      reminderCount: 0,
      elapsedMinutes: 0,
    })
  } catch (error) {
    logError('load break advice', error)
  } finally {
    adviceLoading.value = false
  }
}

function addReminder() {
  if (!result.value) return
  emit('addReminder', result.value.draft)
  request.value = ''
  result.value = null
}

const repeatLabel = computed(() => {
  const draft = result.value?.draft
  if (!draft) return ''
  if (draft.reminderType === 'weekly' && draft.weekdays.length > 0) {
    return draft.weekdays.map((day) => t(`agent.weekday.${day}` as MessageKey)).join('、')
  }
  if (draft.reminderType === 'monthly' && draft.monthDays.length > 0) {
    return t('agent.monthDay', { days: draft.monthDays.map(String).join('、') })
  }
  if (draft.reminderType === 'daily') return t('agent.type.daily')
  return '-'
})

const timeLabel = computed(() => {
  const draft = result.value?.draft
  if (!draft) return ''
  if (draft.reminderType === 'once' && draft.triggerAt) {
    const date = new Date(draft.triggerAt)
    return date.toLocaleString(props.language, {
      month: 'numeric',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    })
  }
  return draft.time || '-'
})

const powerLabel = computed(() => {
  const action = result.value?.draft.powerAction
  if (!action) return t('agent.power.none')
  return t(`agent.power.${action}` as MessageKey)
})

const typeLabel = computed(() => {
  const type = result.value?.draft.reminderType ?? 'once'
  return t(`agent.type.${type}` as MessageKey)
})
</script>

<template>
  <section class="page-section agent-page">
    <header class="agent-hero">
      <span class="agent-hero-badge">{{ t('agent.badge') }}<i></i></span>
      <div class="agent-hero-copy">
        <h1>{{ t('agent.heading') }}</h1>
        <p>{{ t('agent.heroSub') }}</p>
      </div>
      <span :class="['agent-mode', config.enabled ? 'is-cloud' : 'is-local']">
        <component :is="config.enabled ? Zap : Plug" :size="13" />
        {{ config.enabled ? t('agent.statusCloud') : t('agent.statusLocal') }}
      </span>
    </header>

    <div class="agent-tabs" role="tablist" :aria-label="t('agent.badge')">
      <button type="button" role="tab" :aria-selected="panel === 'usage'" @click="panel = 'usage'">{{ t('agent.tabUsage') }}</button>
      <button type="button" role="tab" :aria-selected="panel === 'config'" @click="panel = 'config'">{{ t('agent.tabConfig') }}</button>
    </div>

    <div v-show="panel === 'usage'" class="agent-usage">
      <div class="agent-composer">
        <textarea
          v-model="request"
          class="agent-input"
          rows="3"
          :placeholder="t('agent.placeholder')"
          @keydown.enter.meta="parseRequest"
        ></textarea>
        <div class="agent-composer-actions">
          <button class="agent-run" type="button" :disabled="parsing" @click="parseRequest">
            <Sparkles :size="16" />{{ parsing ? t('agent.parsing') : t('agent.parse') }}
          </button>
          <span v-if="result" class="agent-source">{{ result.source === 'cloud' ? t('agent.sourceCloud') : t('agent.sourceLocal') }}</span>
        </div>
        <div class="agent-examples">
          <span class="agent-examples-label">{{ t('agent.examplesLabel') }}</span>
          <button v-for="example in examples" :key="example" class="agent-chip" type="button" @click="request = t(example)">
            <Wand :size="12" />{{ t(example) }}
          </button>
        </div>
      </div>

      <p v-if="parseError" class="page-message" role="alert">{{ parseError }}</p>
      <p v-if="result?.note" class="agent-note" role="status">{{ t('agent.fallbackNote') }}</p>

      <div v-if="result" class="agent-result">
        <h3 class="card-label">{{ t('agent.resultTitle') }}</h3>
        <dl class="agent-fields">
          <div><dt>{{ t('agent.field.type') }}</dt><dd>{{ typeLabel }}</dd></div>
          <div><dt>{{ t('agent.field.time') }}</dt><dd>{{ timeLabel }}</dd></div>
          <div><dt>{{ t('agent.field.repeat') }}</dt><dd>{{ repeatLabel }}</dd></div>
          <div><dt>{{ t('agent.field.power') }}</dt><dd>{{ powerLabel }}</dd></div>
          <div v-if="result.draft.popupMessage"><dt>{{ t('agent.field.popup') }}</dt><dd>{{ result.draft.popupMessage }}</dd></div>
        </dl>
        <div class="agent-result-title">
          <strong>{{ result.draft.title }}</strong>
          <button class="button button-primary" type="button" @click="addReminder">
            <Plus :size="15" />{{ t('agent.addReminder') }}
          </button>
        </div>
      </div>

      <div class="agent-result agent-advice-card">
        <h3 class="card-label">{{ t('agent.adviceTitle') }}</h3>
        <p class="agent-advice">{{ adviceLoading ? t('agent.adviceLoading') : (advice?.advice ?? '') }}</p>
        <button class="button" type="button" :disabled="adviceLoading" @click="refreshAdvice">
          <Sparkles :size="15" />{{ t('agent.advice') }}
        </button>
      </div>
    </div>

    <div v-show="panel === 'config'" class="agent-config">
      <div class="setting-card stacked-setting">
        <div><strong>{{ t('agent.configTitle') }}</strong><span>{{ t('agent.configHint') }}</span></div>
        <label class="agent-switch-row">
          <span>{{ t('agent.enabled') }}</span>
          <button :class="['switch', { on: config.enabled }]" type="button" role="switch" :aria-checked="config.enabled" @click="config.enabled = !config.enabled"><i></i></button>
        </label>
        <div class="agent-config-grid">
          <label class="field"><span>{{ t('agent.endpoint') }}</span><input v-model="config.endpoint" type="text" /></label>
          <label class="field"><span>{{ t('agent.model') }}</span><input v-model="config.model" type="text" /></label>
          <label class="field"><span>{{ t('agent.apiKey') }}</span><input v-model="config.apiKey" type="password" /></label>
        </div>
        <p class="setting-note">{{ t('agent.apiKeyHint') }}</p>
        <div class="action-row">
          <button class="button button-primary" type="button" @click="saveConfig"><Save :size="15" />{{ t('agent.save') }}</button>
        </div>
        <p v-if="configMessage" class="form-message" role="status">{{ configMessage }}</p>
      </div>
    </div>
  </section>
</template>
