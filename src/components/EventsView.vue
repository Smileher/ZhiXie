<script setup lang="ts">
import { CalendarClock, Check, Pencil, Play, Plus, Trash2, X } from '@lucide/vue'
import { translate } from '../i18n'
import type { MessageKey } from '../i18n'
import type { Language, PowerAction, Reminder, ReminderForm, ReminderType } from '../types'

const props = defineProps<{
  language: Language
  subtitle: string
  showForm: boolean
  editingId: string | null
  form: ReminderForm
  powerActionOptions: { value: PowerAction; label: string }[]
  frequencyOptions: { value: ReminderForm['type']; label: string }[]
  weekdayOptions: { value: number; label: string }[]
  typeLabels: Record<ReminderType, string>
  reminders: Reminder[]
  actionMessage: string
  formatNext: (reminder: Reminder) => string
}>()

const emit = defineEmits<{
  testNotification: []
  testReminder: [reminder: Reminder]
  add: []
  closeForm: []
  save: []
  'update:form': [patch: Partial<ReminderForm>]
  toggleWeekday: [day: number]
  toggleMonthDay: [day: number]
  toggleReminder: [reminder: Reminder]
  editReminder: [reminder: Reminder]
  removeReminder: [id: string]
}>()

function t(key: MessageKey, params: Record<string, string | number> = {}) {
  return translate(props.language, key, params)
}
</script>

<template>
  <section class="page-section">
    <header class="page-header reminder-header">
      <div>
        <h1>{{ t('events.title') }}</h1>
        <p class="page-subtitle">{{ subtitle }}</p>
      </div>
      <button class="button test-notification" type="button" @click="emit('testNotification')"><Play :size="14" />{{ t('settings.testNotification') }}</button>
    </header>

    <div class="reminder-toolbar">
      <span>{{ t('events.title') }} <em>{{ reminders.length }}</em></span>
      <button class="button button-primary" type="button" @click="emit('add')"><Plus :size="15" />{{ t('events.add') }}</button>
    </div>

    <div v-if="showForm" class="form-panel">
      <div class="form-heading">
        <div>
          <h2>{{ editingId ? t('events.editTitle') : t('events.addTitle') }}</h2>
        </div>
        <button class="icon-button" type="button" :aria-label="t('common.close')" :title="t('common.close')" @click="emit('closeForm')"><X :size="18" /></button>
      </div>
      <label class="field">
        <span>{{ t('events.content') }}</span>
        <input :value="form.title" type="text" maxlength="120" :placeholder="t('events.contentPlaceholder')" @input="emit('update:form', { title: ($event.target as HTMLInputElement).value })" />
      </label>
      <div class="field">
        <span>{{ t('events.frequency') }}</span>
        <div class="segmented frequency-segments">
          <button v-for="option in frequencyOptions" :key="option.value" :class="{ selected: form.type === option.value }" type="button" @click="emit('update:form', { type: option.value })">{{ option.label }}</button>
        </div>
      </div>
      <div class="field-row">
        <label v-if="form.type === 'once'" class="field">
          <span>{{ t('events.reminderTime') }}</span>
          <input :value="form.triggerAt" type="datetime-local" @input="emit('update:form', { triggerAt: ($event.target as HTMLInputElement).value })" />
        </label>
        <label v-else class="field">
          <span>{{ t('events.exactTime') }}</span>
          <input :value="form.time" type="time" @input="emit('update:form', { time: ($event.target as HTMLInputElement).value })" />
        </label>
      </div>
      <div v-if="form.type === 'weekly'" class="field">
        <span>{{ t('events.selectWeekday') }}</span>
        <div class="choice-grid weekday-grid">
          <button v-for="day in weekdayOptions" :key="day.value" :class="{ selected: form.weekdays.includes(day.value) }" type="button" @click="emit('toggleWeekday', day.value)">{{ t('rule.weekPrefix') }}{{ day.label }}</button>
        </div>
      </div>
      <div v-if="form.type === 'monthly'" class="field">
        <span>{{ t('events.selectDate') }}</span>
        <div class="choice-grid month-grid">
          <button v-for="day in 31" :key="day" :class="{ selected: form.monthDays.includes(day) }" type="button" @click="emit('toggleMonthDay', day)">{{ day }}</button>
        </div>
        <small>{{ t('events.missingDateHint') }}</small>
      </div>
      <small v-if="actionMessage" class="status-message form-message">{{ actionMessage }}</small>
      <label class="field">
        <span>{{ t('events.action') }}</span>
        <select :value="form.powerAction || ''" @change="emit('update:form', { powerAction: (($event.target as HTMLSelectElement).value || null) as PowerAction | null })">
          <option value="">{{ t('events.noAction') }}</option>
          <option v-for="option in powerActionOptions" :key="option.value" :value="option.value">{{ option.label }}</option>
        </select>
        <small v-if="form.powerAction">{{ t('events.actionHint') }}</small>
      </label>
      <div class="form-actions">
        <button class="button" type="button" @click="emit('closeForm')">{{ t('common.cancel') }}</button>
        <button class="button button-primary" type="button" @click="emit('save')"><Check :size="15" />{{ t('events.save') }}</button>
      </div>
    </div>

    <div v-if="reminders.length" class="reminder-list">
      <article v-for="reminder in reminders" :key="reminder.id" :class="['reminder-row', { disabled: !reminder.enabled }]">
        <div :class="['reminder-status', { enabled: reminder.enabled }]"></div>
        <div class="reminder-main">
          <div class="reminder-title"><strong>{{ reminder.title }}</strong><span class="type-chip">{{ typeLabels[reminder.type] }}</span></div>
          <span v-if="reminder.powerAction" class="type-chip">{{ powerActionOptions.find(option => option.value === reminder.powerAction)?.label }}</span>
          <span>{{ formatNext(reminder) }}</span>

        </div>
        <button class="switch" :class="{ on: reminder.enabled }" type="button" :aria-label="reminder.enabled ? t('common.disabled') : t('common.enabled')" @click="emit('toggleReminder', reminder)"><span></span></button>
        <button class="icon-button row-action" type="button" :aria-label="t('settings.testNotification')" :title="t('settings.testNotification')" @click="emit('testReminder', reminder)"><Play :size="15" /></button>
        <button class="icon-button row-action" type="button" :aria-label="t('events.edit')" :title="t('events.edit')" @click="emit('editReminder', reminder)"><Pencil :size="15" /></button>
        <button class="icon-button row-action danger" type="button" :aria-label="t('events.delete')" :title="t('events.delete')" @click="emit('removeReminder', reminder.id)"><Trash2 :size="15" /></button>
      </article>
    </div>
    <div v-else-if="!showForm" class="empty-state">
      <div class="empty-icon"><CalendarClock :size="22" /></div>
      <h2>{{ t('events.emptyTitle') }}</h2>
      <p>{{ t('events.emptyBody') }}</p>
      <button class="button" type="button" @click="emit('add')"><Plus :size="15" />{{ t('events.addFirst') }}</button>
    </div>
  </section>
</template>
