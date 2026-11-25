<script setup lang="ts">
import { computed, onMounted } from 'vue'
import TitleBar from '../components/TitleBar.vue'
import Sidebar from '../components/Sidebar.vue'
import StatusStrip from '../components/StatusStrip.vue'
import OverviewView from '../views/OverviewView.vue'
import SourcesView from '../views/SourcesView.vue'
import ScanView from '../views/ScanView.vue'
import RankingsView from '../views/RankingsView.vue'
import ExportView from '../views/ExportView.vue'
import GithubView from '../views/GithubView.vue'
import HistoryView from '../views/HistoryView.vue'
import SettingsView from '../views/SettingsView.vue'
import AboutView from '../views/AboutView.vue'
import { ChevronRight, ScanLine } from '@lucide/vue'
import { state, initApp, attachListeners, isScanActive } from '../store'

const viewMap = {
  overview: OverviewView,
  sources: SourcesView,
  scan: ScanView,
  rankings: RankingsView,
  export: ExportView,
  github: GithubView,
  history: HistoryView,
  settings: SettingsView,
  about: AboutView,
}

const currentView = computed(() => viewMap[state.activeView] || OverviewView)
const scanActive = computed(() => isScanActive())

onMounted(async () => {
  await initApp()
  await attachListeners()
})
</script>

<template>
  <div class="app-shell">
    <TitleBar />
    <div class="shell-body">
      <Sidebar />
      <main class="shell-main">
        <div class="sakura-haze" aria-hidden="true">
          <i class="blossom b1"></i><i class="blossom b2"></i><i class="blossom b3"></i>
          <i class="drift d1"></i><i class="drift d2"></i><i class="drift d3"></i><i class="drift d4"></i>
        </div>
        <StatusStrip />
        <div class="content-scroll" :class="{ 'overview-active': state.activeView === 'overview' }">
          <div v-if="scanActive && state.activeView !== 'scan' && state.activeView !== 'overview'" class="scan-lock-banner">
            <ScanLine :size="14" />
            <span>优选任务正在运行。当前页面仅供查看，请勿保存设置、修改来源或启动新任务。</span>
            <button @click="state.activeView = 'scan'">返回扫描任务<ChevronRight :size="12" /></button>
          </div>
          <component :is="currentView" />
        </div>
      </main>
    </div>
  </div>
</template>

<style scoped>
.app-shell {
  width: 100%;
  height: 100vh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background: var(--bg);
}

.shell-body {
  flex: 1;
  min-height: 0;
  display: flex;
}

.shell-main {
  position: relative;
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  background:
    radial-gradient(1060px 560px at 52% -14%, rgba(60, 82, 207, 0.34), transparent 64%),
    radial-gradient(720px 480px at 94% 2%, rgba(133, 70, 206, 0.24), transparent 68%),
    radial-gradient(520px 360px at 6% 100%, rgba(70, 47, 156, 0.2), transparent 72%),
    linear-gradient(180deg, #081029 0%, #060c20 58%, #050916 100%);
}

.sakura-haze {
  position: absolute;
  inset: 0;
  z-index: 0;
  overflow: hidden;
  pointer-events: none;
}

/* 设计稿中的科技网格底纹，只做极淡的环境层。 */
.sakura-haze::before {
  content: "";
  position: absolute;
  inset: 0;
  background-image:
    linear-gradient(rgba(124, 152, 255, 0.035) 1px, transparent 1px),
    linear-gradient(90deg, rgba(124, 152, 255, 0.035) 1px, transparent 1px);
  background-size: 46px 46px;
  mask-image: radial-gradient(ellipse at 52% 0%, #000 12%, transparent 74%);
  opacity: 0.9;
}

html[data-theme="light"] .sakura-haze::before {
  background-image:
    linear-gradient(rgba(58, 83, 140, 0.05) 1px, transparent 1px),
    linear-gradient(90deg, rgba(58, 83, 140, 0.05) 1px, transparent 1px);
  opacity: 0.6;
}

.blossom {
  position: absolute;
  width: 360px;
  height: 190px;
  opacity: .3;
  background:
    radial-gradient(ellipse at 18% 68%, rgba(255, 172, 211, .52) 0 4px, transparent 5px),
    radial-gradient(ellipse at 33% 42%, rgba(255, 192, 221, .44) 0 3px, transparent 4px),
    radial-gradient(ellipse at 55% 73%, rgba(236, 144, 195, .42) 0 3px, transparent 4px),
    radial-gradient(ellipse at 72% 37%, rgba(255, 174, 212, .5) 0 4px, transparent 5px),
    radial-gradient(ellipse at 88% 64%, rgba(255, 202, 226, .4) 0 3px, transparent 4px);
  filter: blur(.2px) drop-shadow(0 0 10px rgba(255, 139, 190, .18));
}
.blossom.b1 { right: -36px; top: 36px; transform: rotate(-8deg); }
.blossom.b2 { left: 8%; bottom: -46px; transform: rotate(12deg) scale(.9); opacity: .17; }
.blossom.b3 { right: 16%; bottom: 4%; transform: scale(.62); opacity: .14; }
.drift {
  position: absolute;
  width: 7px;
  height: 6px;
  border-radius: 64% 36% 62% 38%;
  background: linear-gradient(145deg, rgba(255, 208, 231, .92), rgba(207, 117, 176, .46));
  box-shadow: 0 0 10px rgba(255, 155, 206, .45);
  animation: drift-down 14s linear infinite;
  opacity: 0;
}
.drift.d1 { left: 72%; animation-delay: 2s; }
.drift.d2 { left: 86%; animation-delay: 5.5s; animation-duration: 18s; }
.drift.d3 { left: 62%; animation-delay: 9s; animation-duration: 16s; }
.drift.d4 { left: 94%; animation-delay: 11.5s; animation-duration: 21s; }
@keyframes drift-down {
  0% { transform: translate3d(0, -24px, 0) rotate(0deg); opacity: 0; }
  12% { opacity: .8; }
  55% { transform: translate3d(-36px, 43vh, 0) rotate(120deg); opacity: .65; }
  100% { transform: translate3d(18px, 88vh, 0) rotate(300deg); opacity: 0; }
}

.shell-main > :not(.sakura-haze) { position: relative; z-index: 1; }

.content-scroll {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  overflow-x: hidden;
  padding: 0 18px 16px;
}

/* 总览页固定为单屏布局：不使用整页滚动，由页面内部各区块分配高度。 */
.content-scroll.overview-active {
  display: flex;
  flex-direction: column;
  overflow: hidden;
  padding-bottom: 8px;
}

.content-scroll.overview-active > .page {
  flex: 1;
  min-height: 0;
}

.scan-lock-banner {
  position: sticky;
  top: 8px;
  z-index: 20;
  display: flex;
  align-items: center;
  gap: 9px;
  margin: 10px 0 12px;
  padding: 9px 12px;
  border: 1px solid rgba(244, 177, 96, 0.28);
  border-radius: 10px;
  background: rgba(38, 28, 25, 0.88);
  color: #f2c787;
  font-size: 10px;
  box-shadow: 0 8px 28px rgba(3, 7, 18, 0.22);
  backdrop-filter: blur(12px);
}

.scan-lock-banner span { flex: 1; min-width: 0; line-height: 1.55; }

.scan-lock-banner button {
  display: inline-flex;
  flex-shrink: 0;
  align-items: center;
  gap: 2px;
  padding: 5px 8px;
  border: 1px solid rgba(242, 199, 135, 0.24);
  border-radius: 7px;
  background: rgba(242, 199, 135, 0.08);
  color: #ffdda4;
  font-size: 9px;
  cursor: pointer;
}

html[data-theme="light"] .scan-lock-banner {
  border-color: rgba(191, 112, 28, 0.24);
  background: rgba(255, 248, 232, 0.94);
  color: #9a5b11;
  box-shadow: 0 8px 28px rgba(72, 49, 19, 0.08);
}

html[data-theme="light"] .scan-lock-banner button {
  border-color: rgba(191, 112, 28, 0.22);
  background: rgba(191, 112, 28, 0.07);
  color: #8a4d08;
}
</style>
