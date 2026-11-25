<script setup lang="ts">
import { ref } from 'vue'
import { useMessage } from 'naive-ui'
import {
  House,
  Radar,
  ScanLine,
  Trophy,
  FileDown,
  GitBranch,
  Clock3,
  Settings,
  Info,
  ShieldCheck,
  Heart,
  ChevronRight,
} from '@lucide/vue'
import { state, isScanActive, type ViewKey } from '../store'
import mikasaChild from '../assets/young-mikasa-rail.png'
import sakuraBranch from '../assets/sakura-branch.svg'
import BrandMark from './BrandMark.vue'
import DonateDialog from './DonateDialog.vue'

const donateOpen = ref(false)
const message = useMessage()
let warnedScanId = ''

const navItems: Array<{ key: ViewKey; label: string; icon: unknown }> = [
  { key: 'overview', label: '总览', icon: House },
  { key: 'sources', label: 'IP 来源', icon: Radar },
  { key: 'scan', label: '扫描任务', icon: ScanLine },
  { key: 'rankings', label: '地区排名', icon: Trophy },
  { key: 'export', label: '结果导出', icon: FileDown },
  { key: 'github', label: 'GitHub 上传', icon: GitBranch },
  { key: 'history', label: '历史记录', icon: Clock3 },
  { key: 'settings', label: '设置', icon: Settings },
  { key: 'about', label: '关于软件', icon: Info },
]

function go(key: ViewKey) {
  if (
    key !== state.activeView &&
    key !== 'scan' &&
    isScanActive() &&
    warnedScanId !== (state.scan?.scanId || '')
  ) {
    warnedScanId = state.scan?.scanId || ''
    message.warning('优选任务进行中：可以查看其它页面，但请勿修改来源、保存设置或启动新任务。', {
      duration: 5200,
      closable: true,
    })
  }
  state.activeView = key
}
</script>

<template>
  <aside class="sidebar">
    <img class="sakura-crown" :src="sakuraBranch" alt="" aria-hidden="true" />
    <div class="character-wrap">
      <img :src="mikasaChild" class="character" alt="童年三笠" />
      <div class="character-shade"></div>
      <div class="petal-layer">
        <i class="petal p1"></i>
        <i class="petal p2"></i>
        <i class="petal p3"></i>
        <i class="petal p4"></i>
      </div>
    </div>

    <div class="brand">
      <BrandMark :size="32" />
      <div class="brand-copy">
        <div class="brand-title">三笠优选</div>
        <div class="brand-sub">MIKASA OPTIMIZER</div>
      </div>
    </div>

    <nav class="nav-list">
      <button
        v-for="item in navItems"
        :key="item.key"
        class="nav-item"
        :class="{ active: state.activeView === item.key }"
        @click="go(item.key)"
      >
        <span class="nav-glow"></span>
        <component :is="item.icon" :size="17" :stroke-width="1.9" class="nav-icon" />
        <span class="nav-label">{{ item.label }}</span>
        <span class="nav-mark"></span>
      </button>
    </nav>

    <div class="sidebar-footer">
      <div class="castle"></div>
      <div class="footer-quote">
        <ShieldCheck :size="13" />
        <span>让每一次连接<br />都经过认真筛选。</span>
      </div>
      <div class="footer-version">
        <span>v{{ state.version }}</span>
        <span class="footer-dot"></span>
        <span>LOCAL ENGINE</span>
      </div>
      <button class="donate-entry" @click="donateOpen = true">
        <span class="donate-icon"><Heart :size="14" /></span>
        <span class="donate-copy">
          <strong>打赏作者</strong>
          <em>微信 / 支付宝</em>
        </span>
        <ChevronRight :size="13" class="donate-arrow" />
      </button>
    </div>
    <DonateDialog :open="donateOpen" @close="donateOpen = false" />
  </aside>
</template>

<style scoped>
.sidebar {
  position: relative;
  width: 224px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  border-right: 1px solid rgba(108, 133, 255, 0.18);
  background:
    radial-gradient(520px 280px at 50% 8%, rgba(91, 96, 215, 0.32), transparent 68%),
    radial-gradient(360px 260px at 12% 96%, rgba(97, 55, 170, 0.22), transparent 74%),
    linear-gradient(180deg, rgba(12, 19, 42, 0.58) 0%, rgba(6, 11, 28, 0.94) 45%, rgba(5, 9, 22, 0.99) 100%);
}

.character-wrap {
  position: relative;
  height: 196px;
  flex-shrink: 0;
  overflow: hidden;
  isolation: isolate;
  background: #070b17;
}

.character {
  position: absolute;
  inset: 0;
  z-index: 0;
  display: block;
  width: 100%;
  height: 100%;
  object-fit: cover;
  object-position: center top;
  filter: saturate(0.96) brightness(0.92) contrast(1.03);
  transform: translateZ(0);
  backface-visibility: hidden;
  mask-image: none;
  -webkit-mask-image: none;
}

.character-shade {
  position: absolute;
  inset: 0;
  z-index: 1;
  pointer-events: none;
  background:
    linear-gradient(100deg, rgba(6, 10, 22, 0.34), transparent 52%),
    linear-gradient(0deg, #070b17 0%, rgba(7, 11, 23, 0.32) 22%, transparent 55%);
}

.character-wrap::after {
  content: "";
  position: absolute;
  left: 0;
  right: 0;
  bottom: 0;
  height: 30%;
  z-index: 2;
  pointer-events: none;
  background: linear-gradient(to bottom, rgba(7, 11, 23, 0), rgba(7, 11, 23, 0.98) 94%);
}

.sakura-crown {
  position: absolute;
  right: -38px;
  top: -24px;
  width: 202px;
  height: 108px;
  object-fit: contain;
  object-position: right top;
  opacity: .55;
  pointer-events: none;
  z-index: 3;
}

.petal-layer {
  position: absolute;
  inset: 0;
  z-index: 3;
  pointer-events: none;
}

.petal {
  position: absolute;
  width: 6px;
  height: 5px;
  border-radius: 60% 40% 55% 45%;
  background: linear-gradient(140deg, rgba(255, 196, 222, 0.92), rgba(214, 138, 190, 0.55));
  box-shadow: 0 0 8px rgba(255, 170, 208, 0.5);
  animation: petal-fall 11s linear infinite;
  opacity: 0;
}

.petal.p1 { left: 18%; animation-delay: 0s; }
.petal.p2 { left: 44%; animation-delay: 3.2s; animation-duration: 13s; }
.petal.p3 { left: 72%; animation-delay: 6.1s; animation-duration: 12s; }
.petal.p4 { left: 88%; animation-delay: 8.4s; animation-duration: 15s; }

@keyframes petal-fall {
  0% { transform: translate3d(0, -10px, 0) rotate(0deg); opacity: 0; }
  18% { opacity: 0.9; }
  78% { opacity: 0.7; }
  100% { transform: translate3d(-26px, 196px, 0) rotate(220deg); opacity: 0; }
}

@media (prefers-reduced-motion: reduce) {
  .petal { animation: none; opacity: 0; }
}

.brand {
  position: relative;
  display: flex;
  align-items: center;
  gap: 11px;
  padding: 0 20px 14px;
  margin-top: -12px;
  z-index: 2;
}

.brand-title {
  font-size: 19px;
  font-weight: 800;
  letter-spacing: 0.09em;
  line-height: 1.1;
  background: linear-gradient(94deg, #ffffff 6%, #d6e2ff 52%, #f3c7e4 100%);
  -webkit-background-clip: text;
  background-clip: text;
  -webkit-text-fill-color: transparent;
}

.brand-sub {
  margin-top: 3px;
  font-size: 7.5px;
  letter-spacing: 0.245em;
  color: #7e91bd;
}

.nav-list {
  position: relative;
  z-index: 3;
  display: flex;
  flex-direction: column;
  gap: 3px;
  padding: 0 12px;
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  scrollbar-width: none;
}

.nav-list::-webkit-scrollbar {
  width: 0;
}

.nav-item {
  position: relative;
  display: flex;
  align-items: center;
  gap: 12px;
  height: 42px;
  padding: 0 14px;
  border: 1px solid transparent;
  border-radius: 9px;
  color: #8b97b8;
  background: transparent;
  cursor: pointer;
  font-size: 12.5px;
  text-align: left;
  transition: 0.16s ease;
}

.nav-item:hover {
  color: #dbe4ff;
  background: rgba(90, 118, 255, 0.08);
}

.nav-item.active {
  color: #ffffff;
  border-color: rgba(136, 170, 255, 0.82);
  background:
    linear-gradient(96deg, rgba(69, 105, 239, 0.96) 0%, rgba(73, 77, 214, 0.9) 64%, rgba(81, 49, 165, 0.62) 100%);
  box-shadow:
    0 0 0 1px rgba(115, 158, 255, 0.2),
    0 10px 26px rgba(39, 72, 220, 0.4),
    inset 0 1px 0 rgba(255, 255, 255, 0.18);
}

.nav-glow {
  position: absolute;
  left: -1px;
  top: 8px;
  width: 2px;
  height: 25px;
  border-radius: 3px;
  background: #a8bbff;
  box-shadow: 0 0 10px rgba(145, 168, 255, 0.95);
  opacity: 0;
}

.nav-item.active .nav-glow {
  opacity: 1;
}

.nav-icon {
  flex-shrink: 0;
  opacity: 0.92;
}

.nav-label {
  letter-spacing: 0.035em;
  font-weight: 600;
  white-space: nowrap;
}

.nav-mark {
  margin-left: auto;
  width: 4px;
  height: 4px;
  border-radius: 50%;
  background: #6f84bc;
  opacity: 0;
}

.sidebar-footer {
  position: relative;
  margin-top: auto;
  min-height: 148px;
  padding: 20px 20px 13px;
  overflow: hidden;
}

.castle {
  position: absolute;
  inset: auto 0 32px;
  height: 60px;
  opacity: 0.22;
  background:
    linear-gradient(90deg, transparent 3%, #34508d 3%, #34508d 8%, transparent 8%, transparent 12%, #34508d 12%, #34508d 19%, transparent 19%, transparent 22%, #34508d 22%, #34508d 27%, transparent 27%, transparent 31%, #34508d 31%, #34508d 38%, transparent 38%, transparent 45%, #34508d 45%, #34508d 51%, transparent 51%, transparent 56%, #34508d 56%, #34508d 63%, transparent 63%, transparent 70%, #34508d 70%, #34508d 75%, transparent 75%, transparent 82%, #34508d 82%, #34508d 91%, transparent 91%),
    linear-gradient(180deg, transparent 20%, #253d70 21%, #253d70 100%);
  mask-image: linear-gradient(180deg, transparent, #000 30%, #000 100%);
}

.footer-quote {
  position: relative;
  display: flex;
  gap: 8px;
  align-items: flex-start;
  color: #7c8bb5;
  font-size: 11.5px;
  line-height: 1.65;
}

.footer-quote svg {
  margin-top: 2px;
  color: #7e93dd;
  flex-shrink: 0;
}

.footer-version {
  position: relative;
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 16px;
  color: #536184;
  font-size: 9px;
  letter-spacing: 0.12em;
}

.footer-dot {
  width: 3px;
  height: 3px;
  border-radius: 50%;
  background: #43507a;
}

.donate-entry {
  position: relative;
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  margin-top: 11px;
  padding: 7px 9px;
  border: 1px solid rgba(255, 154, 205, 0.22);
  border-radius: 9px;
  background: linear-gradient(100deg, rgba(125, 72, 160, 0.18), rgba(60, 45, 110, 0.1));
  color: #d7c6e8;
  cursor: pointer;
  text-align: left;
  transition: 0.16s ease;
}

.donate-entry:hover {
  border-color: rgba(255, 154, 205, 0.46);
  background: linear-gradient(100deg, rgba(145, 82, 180, 0.26), rgba(74, 54, 128, 0.16));
  transform: translateY(-1px);
}

.donate-icon {
  display: grid;
  place-items: center;
  width: 26px;
  height: 26px;
  flex-shrink: 0;
  border-radius: 8px;
  background: rgba(255, 154, 205, 0.15);
  color: #ff9fca;
}

.donate-copy {
  display: flex;
  flex-direction: column;
  gap: 1px;
  min-width: 0;
}

.donate-copy strong {
  color: #f2e6ff;
  font-size: 10.5px;
  letter-spacing: 0.04em;
}

.donate-copy em {
  color: #a392bb;
  font-size: 8px;
  font-style: normal;
}

.donate-arrow {
  margin-left: auto;
  color: #8c7ba8;
}

/* 窗口高度变化时压缩插画与导航，避免底部信息被挤出或导航被裁切。 */
@media (max-height: 840px) {
  .character-wrap { height: 150px; }
  .nav-item { height: 37px; }
  .sidebar-footer { min-height: 122px; padding: 10px 20px 10px; }
  .footer-quote { display: none; }
  .footer-version { margin-top: 8px; }
  .donate-entry { margin-top: 7px; padding: 5px 8px; }
}

@media (max-height: 720px) {
  .character-wrap { height: 108px; }
  .brand { margin-top: -6px; padding-bottom: 8px; }
  .brand-title { font-size: 17px; }
  .nav-item { height: 32px; font-size: 12px; }
  .sidebar-footer { display: flex; flex-direction: column; min-height: 0; padding: 4px 14px 8px; }
  .castle, .footer-quote { display: none; }
  .footer-version { margin-top: 4px; }
  .donate-entry { margin-top: 5px; padding: 4px 7px; }
  .donate-icon { width: 22px; height: 22px; }
}
</style>

