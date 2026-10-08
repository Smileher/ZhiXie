<script setup lang="ts">
import { Sparkles } from '@lucide/vue'
import brandIcon from '../assets/logo.svg'
import { translate } from '../i18n'
import type { MessageKey } from '../i18n'
import type { Language } from '../types'

const props = defineProps<{
  language: Language
  appVersion: string
}>()

function t(key: MessageKey, params: Record<string, string | number> = {}) {
  return translate(props.language, key, params)
}
</script>

<template>
  <section class="page-section about-section">
    <div class="about-overview">
      <img :src="brandIcon" alt="" />
      <div class="about-product"><h1>知歇</h1><p>{{ t('about.tagline') }}</p></div>
      <span class="about-version">{{ t('about.version', { version: appVersion }) }}</span>
    </div>

    <div class="about-card">
      <span class="about-card-icon"><Sparkles :size="18" /></span>
      <div>
        <strong>{{ t('about.agentTitle') }}</strong>
        <p>{{ t('about.agentBody') }}</p>
      </div>
    </div>

    <div class="about-card">
      <div>
        <strong>{{ t('about.stackTitle') }}</strong>
        <p>{{ t('about.stackBody') }}</p>
      </div>
    </div>

    <footer class="about-footer"><p class="about-copyright">{{ t('about.copyright') }}</p></footer>
  </section>
</template>
