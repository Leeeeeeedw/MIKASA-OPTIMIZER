<script setup lang="ts">
import { computed } from 'vue'
import { useMessage } from 'naive-ui'
import {
  Ban,
  ChevronRight,
  CircleCheck,
  Clock3,
  Download,
  Gauge,
  LoaderCircle,
  Pause,
  Play,
  Radar,
  ShieldCheck,
  Zap,
} from '@lucide/vue'
import { state, startScan, cancelScan, pauseScan, resumeScan, refreshDashboard } from '../store'
import RegionFlag from '../components/RegionFlag.vue'

const message = useMessage()
const scan = computed(() => state.scan)
const percent = computed(() => {
  if (!scan.value?.total) return 0
  return Math.min(100, Math.round((scan.value.tested / scan.value.total) * 100))
})
const running = computed(() => scan.value?.status === 'running')
const paused = computed(() => scan.value?.status === 'paused')
const phases = [
  { key: 'tcp', title: 'TCP 连通性', icon: Radar, desc: '默认 443 端口，筛选可连接目标' },
  { key: 'https', title: 'HTTPS / colo', icon: ShieldCheck, desc: '正确 SNI + CF-RAY 实际机房识别' },
  { key: 'latency', title: 'TCP 延迟', icon: Gauge, desc: '多轮握手 RTT / 波动 / 丢包率' },
  { key: 'download', title: '实际下载', icon: Download, desc: '真实字节流，MB/s 与 Mbps 同显' },
]
const phaseIndex = computed(() => {
  const map: Record<string, number> = { tcp: 0, https: 1, latency: 2, download: 3, finished: 4 }
  return map[scan.value?.phase || ''] ?? 0
})
const allStats = computed(() => scan.value?.regionStats || [])

function barWidth(row: { candidates: number; coloOk: number }): number {
  if (!row.candidates) return 0
  const ratio = (row.coloOk / row.candidates) * 100
  return Math.max(0, Math.min(100, Math.round(ratio)))
}
const logs = computed(() => state.logs.slice(-14).reverse())

async function onStart() {
  try {
    await startScan()
    message.success('扫描已启动')
  } catch (error) {
    message.error(String(error))
  }
}

async function onCancel() {
  try {
    await cancelScan()
    message.info('已请求取消扫描')
  } catch (error) {
    message.error(String(error))
  }
}

async function onPause() {
  try {
    await pauseScan()
    message.info('已请求暂停，当前测试项完成后生效')
  } catch (error) {
    message.error(String(error))
  }
}

async function onResume() {
  try {
    await resumeScan()
    message.success('扫描已继续')
  } catch (error) {
    message.error(String(error))
  }
}

async function refresh() {
  await refreshDashboard()
}
</script>

<template>
  <div class="page scan-page">
    <header class="page-header">
      <div>
        <h2>扫描任务</h2>
        <p>真实 TCP / HTTPS / 延迟 / 下载全流程，不使用推算速度。</p>
      </div>
      <div class="header-actions">
        <button class="btn-ghost" @click="refresh">刷新状态</button>
        <button v-if="running" class="btn-ghost" @click="onPause"><Pause :size="14" />暂停</button>
        <button v-if="paused" class="btn-primary" @click="onResume"><Play :size="14" />继续</button>
        <button v-if="running || paused" class="btn-ghost danger" @click="onCancel"><Ban :size="14" />取消扫描</button>
        <button v-if="!running && !paused" class="btn-primary" :disabled="!state.selectedRegions.length" @click="onStart"><Play :size="14" />开始优选</button>
      </div>
    </header>

    <section class="scan-hero glass-card">
      <div class="scan-orbit">
        <div class="radar-circle"></div>
        <LoaderCircle v-if="running" :size="30" class="spin" />
        <Pause v-else-if="paused" :size="30" />
        <Radar v-else :size="30" />
      </div>
      <div class="scan-copy">
        <div class="scan-state">
          <span class="state-dot" :class="{ live: running, paused }"></span>
          {{ paused ? '扫描已暂停' : running ? '扫描进行中' : scan?.status === 'completed' ? '扫描完成' : '等待开始扫描' }}
        </div>
        <h3>{{ scan?.message || '等待开始：选择地区后点击开始优选' }}</h3>
        <div class="progress-track big"><span :style="{ width: `${percent}%` }"></span></div>
        <div class="progress-meta">
          <span>已测试 {{ scan?.tested || 0 }} / {{ scan?.total || 0 }} 个 IP</span>
          <strong>{{ percent }}%</strong>
        </div>
      </div>
      <div class="scan-counts">
        <div><span>TCP 通过</span><strong>{{ scan?.tcpPassed ?? scan?.tcpOk ?? 0 }}<em v-if="scan?.tcpTested"> / {{ scan.tcpTested }}</em></strong></div>
        <div><span>HTTPS 通过</span><strong>{{ scan?.httpsPassed ?? scan?.httpsOk ?? 0 }}<em v-if="scan?.httpsTested"> / {{ scan.httpsTested }}</em></strong></div>
        <div><span>colo 命中</span><strong>{{ scan?.coloOk || 0 }}</strong></div>
        <div><span>延迟通过</span><strong>{{ scan?.latencyPassed ?? scan?.latencyOk ?? 0 }}<em v-if="scan?.latencyEliminated"> · 超限 {{ scan.latencyEliminated }}</em></strong></div>
        <div><span>下载通过</span><strong>{{ scan?.downloadPassed ?? scan?.downloadOk ?? 0 }}<em v-if="scan?.downloadTested"> / {{ scan.downloadTested }}</em></strong></div>
        <div><span>正式保留</span><strong class="green">{{ scan?.finalSelected ?? scan?.finalCount ?? 0 }}</strong></div>
      </div>
      <p class="stage-note">
        延迟上限只用于最终筛选：超过上限的 IP 仍会完成下载测速，但只作参考展示，不进入正式导出。
      </p>
    </section>

    <section class="phase-flow glass-card">
      <div v-for="(phase, index) in phases" :key="phase.key" class="phase-item" :class="{ active: phaseIndex === index, done: phaseIndex > index }">
        <div class="phase-icon"><component :is="phase.icon" :size="16" /></div>
        <div><strong>{{ phase.title }}</strong><span>{{ phase.desc }}</span></div>
        <ChevronRight v-if="index < phases.length - 1" :size="13" class="phase-arrow" />
      </div>
    </section>

    <section class="scan-main-grid">
      <div class="glass-card panel region-panel">
        <div class="panel-head"><Gauge :size="16" /><span>首选地区 + 全球扫描统计</span><span class="panel-note">每个地区独立统计，全球其他地区与未知地区单独保留</span></div>
        <div class="region-stat-table">
          <div class="rs-row head">
            <span>地区</span><span>候选</span><span>TCP</span><span>HTTPS</span><span>colo</span><span>延迟通过</span><span>超限</span><span>下载成功</span><span>正式</span><span>平均延迟</span><span>平均速度</span>
          </div>
          <div v-for="row in allStats" :key="row.key" class="rs-row">
            <span class="region-cell">
              <RegionFlag :code="row.key" :width="18" />
              <span class="region-meta">
                <em>{{ row.name }}</em>
                <i class="region-bar"><b :style="{ width: `${barWidth(row)}%` }"></b></i>
              </span>
            </span>
            <span>{{ row.candidates }}</span><span>{{ row.tcpOk }}</span><span>{{ row.httpsOk }}</span><span>{{ row.coloOk }}</span>
            <span>{{ row.latencyOk }}</span><span class="warn">{{ row.latencyEliminated || 0 }}</span>
            <span>{{ row.downloadOk }}</span><span class="green">{{ row.finalCount }}</span>
            <span>{{ row.avgLatency ? row.avgLatency.toFixed(1) + ' ms' : '—' }}</span>
            <span>{{ row.avgSpeed ? row.avgSpeed.toFixed(2) + ' MB/s' : '—' }}</span>
          </div>
          <div v-if="!allStats.length" class="empty-state">选择地区并开始扫描后，这里显示每个地区的真实阶段统计。</div>
        </div>
      </div>

      <div class="glass-card panel log-panel">
        <div class="panel-head"><Clock3 :size="16" /><span>任务日志</span><span class="panel-note">实时事件</span></div>
        <div class="terminal">
          <div v-for="(log, index) in logs" :key="index" class="terminal-line">
            <span class="t-time">{{ log.time }}</span>
            <span class="t-dot" :class="log.level"></span>
            <span class="t-msg">{{ log.message }}</span>
          </div>
          <div v-if="!logs.length" class="terminal-placeholder">等待扫描事件…</div>
        </div>
      </div>
    </section>

    <section class="glass-card panel tips-panel">
      <div class="tip"><Zap :size="15" /><span><strong>真实测速：</strong>每个正式结果都完成 TCP、HTTPS、多轮延迟和实际下载，下载失败或速度不达标不会进入优选名单。</span></div>
      <div class="tip"><CircleCheck :size="15" /><span><strong>地区判定：</strong>以本地 HTTPS 请求返回的 CF-RAY 中实际 colo 为准，FOFA country 只用于初筛。</span></div>
    </section>
  </div>
</template>

<style scoped>
.scan-page { gap: 14px; }
.header-actions { display: flex; gap: 8px; }
.scan-hero { position: relative; overflow: hidden; display: flex; flex-wrap: wrap; align-items: center; gap: 14px 20px; padding: 18px 24px; background: linear-gradient(115deg, rgba(17, 33, 74, 0.82), rgba(8, 15, 33, 0.95)); }
.scan-orbit { position: relative; width: 70px; height: 70px; flex-shrink: 0; display: grid; place-items: center; border: 1px solid rgba(96, 143, 255, 0.4); border-radius: 50%; color: #a8c6ff; background: radial-gradient(circle, rgba(45, 95, 220, 0.24), transparent 65%); }
.scan-orbit::before { content: ""; position: absolute; inset: 9px; border: 1px dashed rgba(100, 148, 255, 0.28); border-radius: 50%; }
.radar-circle { position: absolute; width: 120%; height: 120%; border: 1px solid rgba(100, 148, 255, 0.08); border-radius: 50%; }
.scan-copy { flex: 1 1 420px; min-width: 0; }
.scan-state { display: flex; align-items: center; gap: 7px; color: #8da4d7; font-size: 11px; white-space: nowrap; }
.state-dot { width: 6px; height: 6px; border-radius: 50%; background: #647697; }
.state-dot.live { background: #46d78d; box-shadow: 0 0 9px rgba(70, 215, 141, 0.7); }
.state-dot.paused { background: #f5b945; box-shadow: 0 0 9px rgba(245, 185, 69, 0.65); }
.scan-copy h3 { margin: 7px 0 14px; color: #e4ecff; font-size: 15px; font-weight: 600; line-height: 1.45; overflow: hidden; display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; }
.progress-track.big { height: 8px; }
.progress-meta { display: flex; justify-content: space-between; margin-top: 7px; color: #637393; font-size: 10px; }
.progress-meta strong { color: #c2d0f2; font-size: 11px; }
.scan-counts { display: grid; grid-template-columns: repeat(6, minmax(0, 1fr)); gap: 12px; flex: 1 1 520px; min-width: 0; }
.scan-counts div { display: flex; flex-direction: column; gap: 5px; }
.scan-counts span { color: #6d7e9f; font-size: 9.5px; }
.scan-counts strong { color: #d6e2ff; font-size: 19px; font-weight: 700; }
.scan-counts strong em { font-style: normal; font-size: 10px; font-weight: 500; color: #7a8cae; margin-left: 2px; }
.scan-counts .green { color: #5ce09d; }
.stage-note { flex: 1 0 100%; margin: 0; font-size: 10.5px; color: #6d7e9f; line-height: 1.6; }
.rs-row .warn { color: #e8b766; }
.phase-flow { display: flex; align-items: stretch; padding: 11px 13px; gap: 4px; }
.phase-item { position: relative; flex: 1; display: flex; align-items: center; gap: 8px; padding: 7px 9px; border-radius: 8px; opacity: 0.5; }
.phase-item.done, .phase-item.active { opacity: 1; }
.phase-item.active { background: rgba(79, 112, 234, 0.16); }
.phase-icon { display: grid; place-items: center; width: 29px; height: 29px; border-radius: 7px; background: rgba(98, 126, 220, 0.15); color: #8fa9ed; }
.phase-item.done .phase-icon { color: #53dc94; background: rgba(52, 193, 126, 0.12); }
.phase-item div:nth-child(2) { display: flex; flex-direction: column; gap: 3px; }
.phase-item strong { color: #c5d2ed; font-size: 10.5px; }
.phase-item span { color: #5e6e8c; font-size: 8.5px; white-space: nowrap; }
.phase-arrow { color: #4f5d7b; flex-shrink: 0; }
.scan-main-grid { display: grid; grid-template-columns: 1.4fr 0.8fr; gap: 13px; min-width: 0; }
.scan-main-grid > * { min-width: 0; }
.panel { padding: 15px; }
.panel-head { display: flex; align-items: center; gap: 7px; color: #dbe5ff; font-size: 12px; font-weight: 700; margin-bottom: 11px; }
.panel-head svg { color: #8ea9ff; }
.panel-note { margin-left: auto; color: #5f6e8e; font-size: 9px; font-weight: 400; }
.region-stat-table { display: flex; flex-direction: column; overflow-x: auto; overflow-y: hidden; }
.rs-row { display: grid; grid-template-columns: minmax(100px, 1.05fr) repeat(8, minmax(40px, 0.62fr)) minmax(60px, 0.9fr) minmax(68px, 0.9fr); align-items: center; min-width: 660px; min-height: 30px; border-bottom: 1px solid rgba(111, 136, 200, 0.08); color: #92a2c4; font-size: 9.5px; }
.rs-row.head { color: #5f6f90; font-size: 8.5px; border-bottom-color: rgba(111, 136, 200, 0.17); }
.rs-row > span { padding: 0 3px; min-width: 0; white-space: nowrap; }
.region-cell { color: #c5d4f3; font-weight: 600; }
.green { color: #5ada96; }
.empty-state { padding: 42px 12px; color: #576684; font-size: 10.5px; text-align: center; }
.terminal { min-height: 265px; max-height: 310px; overflow-y: auto; padding: 9px 10px; border-radius: 8px; background: rgba(2, 6, 15, 0.64); border: 1px solid rgba(91, 117, 180, 0.12); }
.terminal-line { display: flex; align-items: flex-start; gap: 7px; padding: 3px 0; font-family: var(--mono); font-size: 9px; line-height: 1.5; }
.t-time { color: #52617f; white-space: nowrap; }
.t-dot { width: 4px; height: 4px; margin-top: 5px; border-radius: 50%; background: #59cf92; flex-shrink: 0; }
.t-dot.warn { background: #efba55; }
.t-dot.error { background: #f06d77; }
.t-msg { color: #8093b5; word-break: break-all; }
.terminal-placeholder { color: #4e5c78; font-size: 10px; padding: 20px 0; text-align: center; }
.tips-panel { gap: 7px; padding: 12px 15px; }
.tip { display: flex; align-items: flex-start; gap: 7px; color: #7282a3; font-size: 9.5px; line-height: 1.6; }
.tip svg { color: #8ea9ff; flex-shrink: 0; margin-top: 1px; }
.tip strong { color: #9aadd0; }
.spin { animation: spin 1.1s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }
@media (max-width: 1440px) {
  .phase-flow { flex-wrap: wrap; }
  .phase-item { min-width: 150px; }
}

@media (max-width: 1320px) {
  .scan-counts { grid-template-columns: repeat(3, minmax(0, 1fr)); flex-basis: 100%; width: 100%; }
  .scan-main-grid { grid-template-columns: 1fr; }
  .scan-hero { padding: 15px 18px; }
  .scan-counts strong { font-size: 17px; }
}
</style>




