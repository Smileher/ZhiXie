<script setup lang="ts">
import { computed } from 'vue'
import type { AppSettings } from '../types'

type BackgroundSettings = Pick<AppSettings, 'popupBackgroundFit' | 'popupBackgroundOffsetX' | 'popupBackgroundOffsetY' | 'popupBackgroundScale' | 'popupOverlayOpacity'>
const props = defineProps<{ settings: BackgroundSettings; url: string }>()

// 图片盒与弹窗等大，先缩放再平移，保证位移比例不受缩放影响。
const imageStyle = computed(() => props.settings.popupBackgroundFit === 'stretch'
  ? { objectFit: 'fill' as const, transform: 'none' }
  : {
    objectFit: 'contain' as const,
    transform: `translate(${props.settings.popupBackgroundOffsetX}%, ${props.settings.popupBackgroundOffsetY}%) scale(${props.settings.popupBackgroundScale / 100})`,
  })
</script>

<template>
  <div class="popup-background" aria-hidden="true">
    <img v-if="url" :src="url" :style="imageStyle" alt="" />
    <div v-if="url" class="popup-background-overlay" :style="{ opacity: settings.popupOverlayOpacity / 100 }"></div>
  </div>
</template>
