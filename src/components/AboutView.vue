<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { BadgeCheck, Sparkles, UserCheck } from '@lucide/vue'
import brandIcon from '../assets/logo.svg'
import schoolLogo from '../assets/school-badge.png'
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

// 团队信息默认隐藏：连续点击版本号 5 次，或按 Ctrl/Cmd + Shift + K 切换显示。
const TAP_COUNT = 5
const TAP_INTERVAL = 2000
const teamVisible = ref(false)
let tapCount = 0
let lastTapAt = 0

function toggleTeam() {
  teamVisible.value = !teamVisible.value
  tapCount = 0
}

function handleVersionTap() {
  const now = Date.now()
  tapCount = now - lastTapAt > TAP_INTERVAL ? 1 : tapCount + 1
  lastTapAt = now
  if (tapCount >= TAP_COUNT) toggleTeam()
}

function handleShortcut(event: KeyboardEvent) {
  if (!event.shiftKey || !(event.ctrlKey || event.metaKey) || event.key.toLowerCase() !== 'k') return
  event.preventDefault()
  toggleTeam()
}

onMounted(() => window.addEventListener('keydown', handleShortcut))
onBeforeUnmount(() => window.removeEventListener('keydown', handleShortcut))
</script>

<template>
  <section class="page-section about-section">
    <div class="about-overview">
      <img :src="brandIcon" alt="" />
      <div class="about-product"><h1>知歇</h1><p>{{ t('about.tagline') }}</p></div>
      <span class="about-version" @click="handleVersionTap">{{ t('about.version', { version: appVersion }) }}</span>
    </div>

    <div class="about-card">
      <span class="about-card-icon"><Sparkles :size="18" /></span>
      <div>
        <strong>{{ t('about.agentTitle') }}</strong>
        <p>{{ t('about.agentBody') }}</p>
      </div>
    </div>

    <div class="about-card">
      <span class="about-card-icon"><BadgeCheck :size="18" /></span>
      <div>
        <strong>{{ t('about.stackTitle') }}</strong>
        <p>{{ t('about.stackBody') }}</p>
      </div>
    </div>

    <div v-if="teamVisible" class="about-team">
      <div class="about-card about-school-card">
        <span class="about-school-logo"><img :src="schoolLogo" alt="" /></span>
        <div class="about-school-copy">
          <strong class="about-school-name">{{ t('about.schoolName') }}</strong>
          <span class="about-school-role">{{ t('about.schoolRole') }}</span>
          <p>{{ t('about.schoolBody') }}</p>
        </div>
      </div>
      <div class="about-card about-mentor">
        <span class="about-card-icon"><UserCheck :size="20" /></span>
        <div>
          <span class="about-mentor-label">{{ t('about.mentorLabel') }}</span>
          <strong class="about-mentor-name">{{ t('about.mentorName') }}</strong>
          <span class="about-mentor-school">{{ t('about.schoolName') }}</span>
        </div>
      </div>
    </div>

    <footer class="about-footer"><p class="about-copyright">{{ t('about.copyright') }}</p></footer>
  </section>
</template>
