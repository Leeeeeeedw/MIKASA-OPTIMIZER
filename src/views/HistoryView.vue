<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useMessage } from 'naive-ui'
import { open } from '@tauri-apps/plugin-dialog'
import { Clock3, Database, Download, RefreshCw, Trash2, History, RotateCcw, Save } from '@lucide/vue'
import { api, isTauri } from '../api'
import { state } from '../store'
import type { ScanHistoryRow } from '../types'

const message = useMessage()
const rows = ref<ScanHistoryRow[]>([])
const busy = ref(false)
const selected = ref<ScanHistoryRow | null>(null)

function selectRow(row: ScanHistoryRow) {
  selected.value = row
}

async function load() {
  busy.value = true
  try {
    rows.value = await api.getHistory(200)
  } catch (error) {
    message.error(String(error))
  } finally {
    busy.value = false
  }
}

async function remove(scanId: string) {
  try {
    await api.deleteScan(scanId)
    await load()
    message.success('历史记录已删除')
  } catch (error) {
    message.error(String(error))
  }
}

async function clearAll() {
  try {
    await api.clearHistory('all')
    await load()
    message.success('历史记录已清空')
  } catch (error) {
    message.error(String(error))
  }
}

async function backup() {
  let directory: string | undefined
  if (isTauri()) {
    const selected = await open({ directory: true, multiple: false })
    if (!selected || Array.isArray(selected)) return
    directory = selected
  }
  try {
    const path = await api.backupDatabase(directory)
    message.success(`数据库已备份：${path}`)
  } catch (error) {
    message.error(String(error))
  }
}

function openScan(scanId: string) {
  state.selectedScanId = scanId
  state.activeView = 'rankings'
}

onMounted(load)
</script>

<template>
  <div class="page history-page">
    <header class="page-header">
      <div>
        <h2>历史记录</h2>
        <p>SQLite 保存每次扫描、真实测速指标和 IP 状态，支持复测与数据备份。</p>
      </div>
      <div class="header-actions">
        <button class="btn-ghost" @click="load"><RefreshCw :size="14" />刷新</button>
        <button class="btn-ghost" @click="backup"><Save :size="14" />备份数据库</button>
        <button class="btn-ghost danger" :disabled="!rows.length" @click="clearAll"><Trash2 :size="14" />清空历史</button>
      </div>
    </header>

    <section class="history-summary">
      <div class="glass-card summary-item"><Clock3 :size="18" /><div><span>扫描记录</span><strong>{{ rows.length }}</strong></div></div>
      <div class="glass-card summary-item"><Database :size="18" /><div><span>累计测试</span><strong>{{ rows.reduce((a, r) => a + Number(r.tested || 0), 0).toLocaleString() }}</strong></div></div>
      <div class="glass-card summary-item"><History :size="18" /><div><span>最近扫描</span><strong class="small">{{ rows[0]?.startedAt || '暂无' }}</strong></div></div>
    </section>

    <section class="history-grid">
      <div class="glass-card history-list">
        <div class="history-row head"><span>扫描时间</span><span>状态</span><span>测试数量</span><span>优选结果</span><span>平均延迟</span><span>平均下载</span><span>地区</span><span>操作</span></div>
        <div
          v-for="row in rows"
          :key="row.scanId"
          class="history-row"
          :class="{ active: selected?.scanId === row.scanId }"
          @click="selectRow(row)"
        >
          <span>{{ row.startedAt || '—' }}</span>
          <span class="status" :class="row.status">{{ row.status === 'completed' ? '已完成' : row.status === 'running' ? '运行中' : row.status }}</span>
          <span>{{ row.tested }}</span>
          <span class="green">{{ row.finalCount }}</span>
          <span>{{ row.avgLatency ? row.avgLatency.toFixed(1) + ' ms' : '—' }}</span>
          <span>{{ row.avgSpeed ? row.avgSpeed.toFixed(2) + ' MB/s' : '—' }}</span>
          <span class="regions">{{ row.regions?.join(' · ') || '—' }}</span>
          <span class="row-actions">
            <button class="link-btn" @click.stop="openScan(row.scanId)"><RotateCcw :size="11" />查看结果</button>
            <button class="link-btn danger" @click.stop="remove(row.scanId)"><Trash2 :size="11" />删除</button>
          </span>
        </div>
        <div v-if="!rows.length" class="empty-state">
          <Database :size="30" />
          <span>暂无历史扫描记录</span>
          <small>完成一次真实测速后，记录会自动保存到这里。</small>
        </div>
      </div>

      <aside class="glass-card detail-card">
        <div class="panel-head"><History :size="15" /><span>任务详情</span></div>
        <template v-if="selected">
          <div class="detail-region">{{ selected.scanId }}</div>
          <div class="detail-metrics">
            <div><span>地区覆盖</span><strong>{{ selected.regions?.length || 0 }} / 7</strong></div>
            <div><span>优选 IP</span><strong class="green">{{ selected.finalCount }}</strong></div>
            <div><span>测试数量</span><strong>{{ selected.tested }}</strong></div>
            <div><span>平均延迟</span><strong>{{ selected.avgLatency ? selected.avgLatency.toFixed(1) + ' ms' : '—' }}</strong></div>
            <div><span>平均速度</span><strong>{{ selected.avgSpeed ? selected.avgSpeed.toFixed(2) + ' MB/s' : '—' }}</strong></div>
            <div><span>开始时间</span><strong class="small">{{ selected.startedAt || '—' }}</strong></div>
          </div>
          <p class="detail-rule">历史数据仅用于排序与选择复测对象，不会当作本次实测结果。</p>
          <div class="detail-actions">
            <button class="btn-primary" @click="openScan(selected.scanId)">查看结果</button>
            <button class="btn-ghost danger" @click="remove(selected.scanId)">删除记录</button>
          </div>
        </template>
        <div v-else class="empty-state small"><Clock3 :size="24" /><span>选择左侧任意一条记录查看详情</span></div>
      </aside>
    </section>

    <section class="glass-card backup-card">
      <div class="backup-icon"><Download :size="19" /></div>
      <div><strong>数据安全</strong><span>可随时将 SQLite 数据库备份到本地目录；历史 IP 会优先复测，但不会沿用旧速度。</span></div>
      <button class="btn-ghost" @click="backup">立即备份</button>
    </section>
  </div>
</template>

<style scoped>
.history-page { gap: 14px; }
.header-actions { display: flex; gap: 7px; }
.history-summary { display: grid; grid-template-columns: repeat(3, 1fr); gap: 11px; }
.summary-item { display: flex; align-items: center; gap: 10px; padding: 13px 15px; color: #8ea9ff; }
.summary-item div { display: flex; flex-direction: column; gap: 4px; }
.summary-item span { color: #7181a2; font-size: 9.5px; }
.summary-item strong { color: #e5edff; font-size: 20px; }
.summary-item strong.small { font-size: 11px; font-weight: 500; }
.history-list { padding: 12px 14px; overflow-x: auto; }
.history-grid { display: grid; grid-template-columns: 1.62fr 0.86fr; gap: 12px; align-items: start; min-width: 0; }
.history-grid > * { min-width: 0; }
.history-row { display: grid; grid-template-columns: minmax(118px, 1.4fr) minmax(52px, 0.7fr) minmax(52px, 0.7fr) minmax(52px, 0.7fr) minmax(70px, 0.9fr) minmax(74px, 0.9fr) minmax(92px, 1.15fr) minmax(104px, 1.3fr); align-items: center; min-width: 640px; min-height: 36px; border-bottom: 1px solid rgba(108, 133, 255, 0.07); color: #8ea0c1; font-size: 9.5px; cursor: pointer; }
.history-row.active { background: rgba(96, 126, 233, 0.1); box-shadow: inset 2px 0 0 #7c93ff; }
.detail-card { padding: 12px 15px 14px; display: flex; flex-direction: column; gap: 9px; }
.detail-region { font-size: 9.5px; color: #7186ab; font-family: ui-monospace, Menlo, monospace; word-break: break-all; }
.detail-metrics { display: grid; grid-template-columns: 1fr 1fr; gap: 8px 12px; }
.detail-metrics div { display: flex; flex-direction: column; gap: 3px; }
.detail-metrics span { color: #6b7d9f; font-size: 9px; }
.detail-metrics strong { color: #dfe9ff; font-size: 13px; }
.detail-metrics strong.small { font-size: 9.5px; font-weight: 500; }
.detail-rule { margin: 0; padding: 7px 9px; border-radius: 6px; background: rgba(92, 120, 200, 0.1); color: #6f82a5; font-size: 9px; line-height: 1.6; }
.detail-actions { display: flex; gap: 7px; margin-top: 2px; }
.detail-actions .btn-primary, .detail-actions .btn-ghost { flex: 1; justify-content: center; font-size: 10px; padding: 7px 0; }
.empty-state.small { padding: 30px 8px; }
.history-row.head { min-height: 28px; color: #5e6f90; font-size: 8.5px; }
.history-row > span { padding: 0 4px; min-width: 0; white-space: nowrap; }
.status { display: inline-flex; align-items: center; width: fit-content; padding: 2px 6px; border-radius: 4px; background: rgba(92, 117, 185, 0.12); color: #93a8d3; }
.status.completed { color: #5dd596; background: rgba(48, 183, 116, 0.11); }
.status.running { color: #d3b05d; background: rgba(198, 150, 53, 0.12); }
.green { color: #5eda97; }
.regions { color: #697b9e; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.row-actions { display: flex; gap: 6px; }
.link-btn { display: inline-flex; align-items: center; gap: 3px; border: 0; background: transparent; color: #8ea9e8; font-size: 9px; cursor: pointer; padding: 2px 3px; }
.link-btn.danger { color: #c9828c; }
.empty-state { display: flex; flex-direction: column; align-items: center; gap: 8px; padding: 55px 10px; color: #596986; font-size: 11px; }

@media (max-width: 1310px) {
  .history-grid { grid-template-columns: 1fr; }
  .history-summary { gap: 9px; }
}

@media (max-width: 1000px) {
  .history-summary { grid-template-columns: 1fr; }
}
.empty-state svg { opacity: 0.55; }
.empty-state small { color: #4b5a76; font-size: 9px; }
.backup-card { display: flex; align-items: center; gap: 12px; padding: 14px 16px; }
.backup-icon { display: grid; place-items: center; width: 34px; height: 34px; border-radius: 8px; color: #8ca7f0; background: rgba(91, 123, 218, 0.13); }
.backup-card div:nth-child(2) { display: flex; flex-direction: column; gap: 3px; flex: 1; }
.backup-card strong { color: #c9d8f6; font-size: 11px; }
.backup-card span { color: #617191; font-size: 9px; }
</style>
