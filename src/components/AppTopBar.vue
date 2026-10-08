<script setup lang="ts">
import { CalendarClock, Coffee, Info, SlidersHorizontal, Sparkles } from '@lucide/vue'
import brandIcon from '../assets/logo.svg'
import { translate } from '../i18n'
import type { MessageKey } from '../i18n'
import type { Language } from '../types'

type View = 'events' | 'rest' | 'settings' | 'about' | 'agent'

const props = defineProps<{
  currentView: View
  language: Language
  appVersion: string
}>()

const emit = defineEmits<{ navigate: [view: View] }>()

const items: { id: View; label: MessageKey; icon: typeof Coffee }[] = [
  { id: 'rest', label: 'nav.rest', icon: Coffee },
  { id: 'events', label: 'nav.events', icon: CalendarClock },
  { id: 'agent', label: 'nav.agent', icon: Sparkles },
  { id: 'settings', label: 'nav.settings', icon: SlidersHorizontal },
  { id: 'about', label: 'nav.about', icon: Info },
]

function t(key: MessageKey) {
  return translate(props.language, key)
}

function select(view: View) {
  emit('navigate', view)
}
</script>

<template>
  <header class="topbar">
    <div class="topbar-brand">
      <img class="topbar-mark" :src="brandIcon" alt="" />
      <div class="topbar-wordmark">
        <strong>知歇</strong>
        <span>{{ t('app.tagline') }}</span>
      </div>
    </div>

    <nav class="topbar-nav" aria-label="主导航">
      <button
        v-for="item in items"
        :key="item.id"
        :class="['topbar-pill', { active: currentView === item.id }]"
        type="button"
        :aria-current="currentView === item.id ? 'page' : undefined"
        @click="select(item.id)"
      >
        <component :is="item.icon" :size="16" />
        <span>{{ t(item.label) }}</span>
      </button>
    </nav>

    <div class="topbar-meta">
      <span class="topbar-version">v{{ appVersion }}</span>
    </div>
  </header>
</template>
