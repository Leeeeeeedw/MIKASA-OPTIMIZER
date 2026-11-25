<script setup lang="ts">
import { RefreshCw, Wifi, Globe, CircleCheck, LoaderCircle, Sun, Moon } from '@lucide/vue'
import { computed } from 'vue'
import { state, refreshNetwork } from '../store'
import { themeMode, setTheme } from '../theme'

const label = computed(() => {
  if (state.networkLoading) return '正在检测网络环境…'
  if (state.network?.ok && state.network.display) return state.network.display
  return '网络环境未知'
})

async function redetect() {
  await refreshNetwork(true)
}
</script>

<template>
  <div class="status-strip">
    <div class="status-left">
      <div class="globe-wrap">
        <Globe :size="16" :stroke-width="1.8" />
        <span class="globe-pulse"></span>
      </div>
      <span class="status-kicker">本地网络环境</span>
      <span class="status-value" :class="{ unknown: !state.network?.ok }">{{ label }}</span>
      <span v-if="state.network?.ok" class="status-done">
        <CircleCheck :size="13" />
        检测完成
      </span>
      <span v-else class="status-done unknown">
        <Wifi :size="13" />
        仅显示省份与运营商
      </span>
    </div>
    <div class="status-right">
      <div class="theme-switch" aria-label="主题切换">
        <button class="theme-btn" :class="{ active: themeMode === 'light' }" title="明亮主题" @click="setTheme('light')">
          <Sun :size="13" />
        </button>
        <button class="theme-btn" :class="{ active: themeMode === 'dark' }" title="深色主题" @click="setTheme('dark')">
          <Moon :size="13" />
        </button>
      </div>
      <span class="status-hint">网络环境仅用于测速参考</span>
      <button class="redetect" :disabled="state.networkLoading" @click="redetect">
        <LoaderCircle v-if="state.networkLoading" :size="13" class="spin" />
        <RefreshCw v-else :size="13" />
        {{ state.networkLoading ? '检测中' : '重新检测' }}
      </button>
    </div>
  </div>
</template>

<style scoped>
.status-strip {
  height: 48px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding: 0 24px;
  border-bottom: 1px solid rgba(108, 133, 255, 0.1);
  background: rgba(6, 11, 22, 0.56);
}

.status-left {
  display: flex;
  align-items: center;
  gap: 9px;
  min-width: 0;
}

.globe-wrap {
  position: relative;
  display: grid;
  place-items: center;
  width: 26px;
  height: 26px;
  color: #91b9ff;
  border: 1px solid rgba(99, 149, 255, 0.22);
  border-radius: 8px;
  background: rgba(65, 108, 224, 0.12);
}

.globe-pulse {
  position: absolute;
  inset: -4px;
  border: 1px solid rgba(92, 144, 255, 0.16);
  border-radius: 11px;
  animation: breathe 3s ease-in-out infinite;
}

@keyframes breathe {
  0%, 100% { opacity: 0.35; transform: scale(0.96); }
  50% { opacity: 0.8; transform: scale(1.03); }
}

.status-kicker {
  font-size: 11px;
  color: #6e7b9f;
  letter-spacing: 0.04em;
}

.status-value {
  color: #e7edff;
  font-size: 13px;
  font-weight: 700;
  letter-spacing: 0.02em;
}

.status-value.unknown {
  color: #8995b5;
}

.status-done {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  margin-left: 4px;
  padding: 3px 8px;
  border-radius: 999px;
  color: #63e394;
  font-size: 10.5px;
  background: rgba(34, 197, 94, 0.09);
  border: 1px solid rgba(34, 197, 94, 0.18);
}

.status-done.unknown {
  color: #7d8aac;
  background: rgba(102, 120, 168, 0.08);
  border-color: rgba(102, 120, 168, 0.14);
}

.status-right {
  display: flex;
  align-items: center;
  gap: 14px;
}

.theme-switch {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 3px;
  border: 1px solid rgba(108, 133, 255, 0.16);
  border-radius: 999px;
  background: rgba(7, 13, 28, 0.5);
}

.theme-btn {
  display: grid;
  place-items: center;
  width: 25px;
  height: 25px;
  border: none;
  border-radius: 50%;
  background: transparent;
  color: #66769b;
  cursor: pointer;
  transition: 0.18s ease;
}

.theme-btn.active {
  color: #e7edff;
  background: rgba(105, 134, 255, 0.25);
  box-shadow: 0 0 10px rgba(109, 140, 255, 0.22);
}

.theme-btn:hover {
  color: #c7d4f7;
}

.status-hint {
  font-size: 10.5px;
  color: #536184;
}

.redetect {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 28px;
  padding: 0 10px;
  border: 1px solid rgba(108, 133, 255, 0.16);
  border-radius: 7px;
  background: rgba(89, 116, 210, 0.07);
  color: #8f9ec1;
  font-size: 11px;
  cursor: pointer;
  transition: 0.18s ease;
}

.redetect:hover {
  color: #dbe4ff;
  border-color: rgba(108, 133, 255, 0.32);
}

.spin {
  animation: rotate 0.9s linear infinite;
}

@keyframes rotate {
  to { transform: rotate(360deg); }
}
</style>
