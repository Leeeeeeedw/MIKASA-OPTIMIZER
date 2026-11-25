<script setup lang="ts">
import wingsOfFreedom from '../assets/wings-of-freedom.svg'

withDefaults(defineProps<{ size?: number }>(), { size: 24 })
</script>

<template>
  <span
    class="brand-mark"
    :style="{ width: `${size}px`, height: `${size}px`, borderRadius: `${Math.max(7, Math.round(size * 0.25))}px` }"
    aria-hidden="true"
  >
    <img :src="wingsOfFreedom" alt="" />
  </span>
</template>

<style scoped>
.brand-mark {
  display: block;
  overflow: hidden;
  flex-shrink: 0;
  box-shadow: 0 0 14px rgba(103, 124, 255, 0.4);
}

.brand-mark img {
  display: block;
  width: 100%;
  height: 100%;
  object-fit: cover;
}
</style>
