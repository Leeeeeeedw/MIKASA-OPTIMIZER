<script setup lang="ts">
import { getCurrentWindow } from '@tauri-apps/api/window'
import { Minus, Square, X } from '@lucide/vue'
import BrandMark from './BrandMark.vue'
import { state } from '../store'

async function minimize() {
  try { await getCurrentWindow().minimize() } catch { /* browser preview */ }
}

async function toggleMaximize() {
  try { await getCurrentWindow().toggleMaximize() } catch { /* browser preview */ }
}

async function closeWindow() {
  try { await getCurrentWindow().close() } catch { /* browser preview */ }
}
</script>

<template>
  <header class="titlebar" data-tauri-drag-region>
    <div class="title-left" data-tauri-drag-region>
      <BrandMark :size="23" />
      <span class="app-name">MIKASA OPTIMIZER</span>
      <span class="app-version">v{{ state.version }}</span>
      <span class="title-divider"></span>
      <span class="title-caption">Cloudflare 多地区 IP 优选工具</span>
    </div>
    <div class="window-controls">
      <button class="win-btn" title="最小化" @click="minimize"><Minus :size="15" /></button>
      <button class="win-btn" title="最大化" @click="toggleMaximize"><Square :size="12" /></button>
      <button class="win-btn close" title="关闭" @click="closeWindow"><X :size="15" /></button>
    </div>
  </header>
</template>

<style scoped>
.titlebar {
  height: 38px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-shrink: 0;
  padding-left: 14px;
  background: linear-gradient(90deg, rgba(7, 11, 26, 0.98), rgba(17, 18, 48, 0.96) 58%, rgba(24, 18, 48, 0.95));
  border-bottom: 1px solid rgba(108, 133, 255, 0.2);
}

.title-left {
  display: flex;
  align-items: center;
  gap: 9px;
  height: 100%;
  min-width: 0;
}

.app-name {
  font-size: 10.5px;
  font-weight: 800;
  letter-spacing: 0.17em;
  color: #becbf2;
}

.app-version {
  font-size: 10px;
  color: #586587;
  padding-top: 1px;
}

.title-divider {
  width: 1px;
  height: 12px;
  background: rgba(114, 136, 209, 0.25);
}

.title-caption {
  font-size: 10.5px;
  color: #606c8d;
  letter-spacing: 0.02em;
}

.window-controls {
  display: flex;
  height: 100%;
}

.win-btn {
  width: 46px;
  height: 100%;
  border: none;
  background: transparent;
  color: #7e8aab;
  display: grid;
  place-items: center;
  cursor: pointer;
  transition: 0.15s ease;
}

.win-btn:hover {
  background: rgba(103, 124, 255, 0.14);
  color: #dbe4ff;
}

.win-btn.close:hover {
  background: #c42b1c;
  color: #fff;
}
</style>
