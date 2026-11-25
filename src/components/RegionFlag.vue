<script setup lang="ts">
import 'flag-icons/css/flag-icons.min.css'

withDefaults(defineProps<{ code: string; width?: number }>(), { width: 28 })
</script>

<template>
  <span
    v-if="code === 'global' || code === 'unknown'"
    class="region-emoji"
    :style="{ width: `${width}px`, height: `${Math.round(width * 0.72)}px` }"
  >{{ code === 'global' ? '🌐' : '❓' }}</span>
  <span
    v-else
    class="fi region-flag-icon"
    :class="`fi-${code}`"
    :style="{ width: `${width}px`, height: `${Math.round(width * 0.72)}px` }"
  />
</template>

<style scoped>
.region-flag-icon {
  display: inline-block;
  flex: 0 0 auto;
  border-radius: 3px;
  box-shadow: 0 0 0 1px rgba(255, 255, 255, 0.14), 0 2px 6px rgba(0, 0, 0, 0.25);
  background-size: cover;
}
</style>
