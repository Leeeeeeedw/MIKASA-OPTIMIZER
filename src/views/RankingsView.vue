<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useDialog, useMessage } from 'naive-ui'
import { ArrowDownWideNarrow, Download, Gauge, Medal, RefreshCw, Search, Trophy, Zap } from '@lucide/vue'
import { api } from '../api'
import { state, refreshDashboard } from '../store'
import RegionFlag from '../components/RegionFlag.vue'
import type { ScanResult, TestRound } from '../types'

const message = useMessage()
const dialog = useDialog()
const region = ref('all')
const sortMode = ref<'score' | 'latency' | 'speed' | 'loss'>('score')
const search = ref('')
const loading = ref(false)
const rows = ref<ScanResult[]>([])
const detail = ref<ScanResult | null>(null)
const detailRounds = ref<TestRound[]>([])
const retesting = ref('')
const batchRunning = ref(false)

const currentRegion = computed(() => state.regions.find((item) => item.key === region.value))
const regionLabel = computed(() => {
  if (region.value === 'all') return '全部地区'
  if (region.value === 'global') return '全球其他地区'
  if (region.value === 'unknown') return '未知地区'
  return currentRegion.value?.name || '地区'
})
function inRegionGroup(row: ScanResult) {
  if (region.value === 'all') return true
  if (region.value === 'global') {
    return row.region !== 'unknown' && !state.regions.some((item) => item.key === row.region)
  }
  if (region.value === 'unknown') return row.region === 'unknown'
  return row.region === region.value
}
const filtered = computed(() => {
  const query = search.value.trim().toLowerCase()
  const list = rows.value.filter((row) =>
    (!query ||
      row.ip.includes(query) ||
      row.colo.toLowerCase().includes(query) ||
      row.regionName.toLowerCase().includes(query) ||
      row.source.toLowerCase().includes(query)) &&
    inRegionGroup(row),
  )
  return list.sort((a, b) => {
    // 正式通过的结果永远排在参考结果之前，再按当前排序方式从最好到最差。
    const passA = a.status === 'passed' ? 0 : 1
    const passB = b.status === 'passed' ? 0 : 1
    if (passA !== passB) return passA - passB
    if (sortMode.value === 'latency') return a.latency - b.latency
    if (sortMode.value === 'speed') return b.speedMbS - a.speedMbS
    if (sortMode.value === 'loss') return a.loss - b.loss
    const byScore = score(b) - score(a)
    if (Math.abs(byScore) > 0.0001) return byScore
    return b.speedMbS - a.speedMbS || a.latency - b.latency
  })
})

function score(row: ScanResult) {
  const speedScore = Math.min(100, row.speedMbS * 3.2)
  const latencyScore = Math.max(0, 100 - row.latency / 3)
  return speedScore * 0.60 + latencyScore * 0.30 + Math.max(0, 100 - row.loss * 5) * 0.10
}

const passedCount = computed(() => filtered.value.filter((row) => row.status === 'passed').length)

async function load() {
  loading.value = true
  try {
    const scanId = state.selectedScanId || state.dashboard.latestScanId || undefined
    rows.value = await api.getResults(scanId, undefined)
  } catch (error) {
    message.error(String(error))
  } finally {
    loading.value = false
  }
}

async function refreshAll() {
  await refreshDashboard()
  await load()
}

function openDetail(row: ScanResult) {
  detail.value = row
  detailRounds.value = row.rounds?.length ? row.rounds : [
    {
      phase: 'latency',
      round: 1,
      latencyMs: row.latency,
      speedMbS: null,
      ok: true,
      message: `数据库记录：平均延迟 ${row.latency.toFixed(1)} ms，抖动 ${row.jitter.toFixed(1)} ms，丢包 ${row.loss.toFixed(1)}%`,
    },
    {
      phase: 'download',
      round: 1,
      latencyMs: null,
      speedMbS: row.speedMbS,
      ok: true,
      message: `数据库记录：实际下载 ${row.speedMbS.toFixed(2)} MB/s（${row.speedMbps.toFixed(1)} Mbps）`,
    },
  ]
}

async function retest(row: ScanResult) {
  retesting.value = row.ip
  try {
    const outcome = await api.retestIp(row.ip, row.port)
    row.rounds = outcome.rounds
    if (outcome.result) {
      Object.assign(row, outcome.result, { rounds: outcome.rounds })
      message.success(outcome.message)
    } else {
      message.warning(outcome.message)
    }
    openDetail(row)
  } catch (error) {
    message.error(String(error))
  } finally {
    retesting.value = ''
  }
}

function batchRetest() {
  const targets = filtered.value.slice(0, 20)
  if (!targets.length) return
  dialog.warning({
    title: '批量复测确认',
    content: `将对当前列表前 ${targets.length} 个 IP 重新执行 TCP、HTTPS、多轮延迟和真实下载测试。该过程可能持续数分钟。`,
    positiveText: '开始复测',
    negativeText: '取消',
    onPositiveClick: async () => {
      batchRunning.value = true
      let passed = 0
      let failed = 0
      try {
        for (const row of targets) {
          try {
            const outcome = await api.retestIp(row.ip, row.port)
            row.rounds = outcome.rounds
            if (outcome.result) {
              Object.assign(row, outcome.result, { rounds: outcome.rounds })
              passed += 1
            } else {
              failed += 1
            }
          } catch {
            failed += 1
          }
        }
        message.success(`批量复测完成：通过 ${passed}，未通过 ${failed}`)
      } finally {
        batchRunning.value = false
      }
    },
  })
}

onMounted(load)
watch(
  () => [state.selectedScanId, state.dashboard.latestScanId],
  () => void load(),
)
</script>

<template>
  <div class="page rankings-page">
    <header class="page-header">
      <div>
        <h2>地区排名</h2>
        <p>按实际 colo 归到国家与城市；日本显示东京/大阪，美国显示具体机房城市。</p>
      </div>
      <div class="header-actions">
        <button class="btn-ghost" :disabled="batchRunning || retesting !== ''" @click="batchRetest">
          <RefreshCw :size="14" :class="{ spinning: batchRunning }" />{{ batchRunning ? '批量复测中' : '批量复测' }}
        </button>
        <button class="btn-ghost" @click="refreshAll"><RefreshCw :size="14" />刷新</button>
        <button class="btn-primary" @click="state.activeView = 'export'"><Download :size="14" />导出结果</button>
      </div>
    </header>

    <section class="rank-top">
      <div class="glass-card rank-summary">
        <div class="rank-medal"><Trophy :size="24" /></div>
        <div><span>当前结果</span><strong>{{ filtered.length }} <small>条（正式 {{ passedCount }} 条）</small></strong></div>
      </div>
      <div class="glass-card rank-summary">
        <div class="rank-medal blue"><Gauge :size="24" /></div>
        <div><span>最低延迟</span><strong>{{ filtered.length ? Math.min(...filtered.map(r => r.latency)).toFixed(1) : '—' }} <small>ms</small></strong></div>
      </div>
      <div class="glass-card rank-summary">
        <div class="rank-medal violet"><Zap :size="24" /></div>
        <div><span>最高下载</span><strong>{{ filtered.length ? Math.max(...filtered.map(r => r.speedMbS)).toFixed(2) : '—' }} <small>MB/s</small></strong></div>
      </div>
      <div class="glass-card rank-summary">
        <div class="rank-medal green"><Medal :size="24" /></div>
        <div><span>平均丢包率</span><strong>{{ filtered.length ? (filtered.reduce((a, r) => a + r.loss, 0) / filtered.length).toFixed(1) : '—' }} <small>%</small></strong></div>
      </div>
    </section>

    <section class="glass-card rank-body">
      <div class="rank-toolbar">
        <div class="region-tabs">
          <button
            class="region-tab"
            :class="{ active: region === 'all' }"
            @click="region = 'all'"
          >全部</button>
          <button
            v-for="item in state.regions"
            :key="item.key"
            class="region-tab"
            :class="{ active: region === item.key }"
            @click="region = item.key"
          ><RegionFlag :code="item.key" :width="17" /> {{ item.name }}</button>
          <button
            class="region-tab"
            :class="{ active: region === 'global' }"
            @click="region = 'global'"
          ><RegionFlag code="global" :width="17" /> 全球其他</button>
          <button
            class="region-tab"
            :class="{ active: region === 'unknown' }"
            @click="region = 'unknown'"
          ><RegionFlag code="unknown" :width="17" /> 未知地区</button>
        </div>
        <div class="rank-tools">
          <div class="search-box"><Search :size="13" /><input v-model="search" placeholder="搜索 IP / 城市 / colo" /></div>
          <label class="sort-select"><ArrowDownWideNarrow :size="13" />
            <select v-model="sortMode">
              <option value="score">综合评分</option>
              <option value="latency">延迟优先</option>
              <option value="speed">速度优先</option>
              <option value="loss">丢包率优先</option>
            </select>
          </label>
        </div>
      </div>

      <div class="rank-caption">
        <span>{{ regionLabel }} · Top {{ filtered.length }} · 正式 {{ passedCount }} / 参考 {{ filtered.length - passedCount }}</span>
        <span class="caption-note">排序：{{ sortMode === 'score' ? '综合质量（速度 60% / TCP 延迟 30% / 丢包 10%）' : sortMode === 'latency' ? 'TCP 平均延迟升序' : sortMode === 'speed' ? '下载速度降序' : '丢包率升序' }} · 延迟为 TCP 握手 RTT（对齐 v2rayN）；1.0.3 之前的历史结果为旧 HTTPS 口径，可复测刷新 · 国家/城市由真实 CF-RAY colo 推导，不按 IP 段猜测 · 未达标 IP 仅展示，不会进入正式导出</span>
      </div>

      <div class="table-wrap">
        <div class="rank-row table-head">
          <span>#</span><span>IP</span><span>端口</span><span>国家 / 城市</span><span>colo</span><span>延迟</span><span>下载 MB/s</span><span>下载 Mbps</span><span>丢包率</span><span>来源</span><span>状态</span><span>操作</span>
        </div>
        <div v-for="(row, index) in filtered" :key="row.id" class="rank-row">
          <span class="rank-index" :class="{ top: index < 3 }">{{ String(index + 1).padStart(2, '0') }}</span>
          <span class="ip mono">{{ row.ip }}</span>
          <span class="mono">{{ row.port }}</span>
          <span><span class="region-tag" :title="row.regionName">{{ row.regionName }}</span></span>
          <span class="colo">{{ row.colo || '—' }}</span>
          <span class="latency">{{ row.latency.toFixed(1) }} ms</span>
          <span class="speed">{{ row.speedMbS.toFixed(2) }}</span>
          <span class="mbps">{{ row.speedMbps.toFixed(1) }}</span>
          <span :class="{ warn: row.loss > 5 }">{{ row.loss.toFixed(1) }}%</span>
          <span class="source">{{ row.source }}</span>
          <span class="status" :class="{ ref: row.status !== 'passed' }">
            <i></i>{{ row.status === 'passed' ? '完整通过' : row.status === 'latency_below' ? '未达延迟' : row.status === 'speed_below' ? '速度偏低' : row.status || '参考' }}
          </span>
          <span class="row-actions">
            <button class="link-btn" @click="openDetail(row)">详情</button>
            <button class="link-btn accent" :disabled="retesting !== '' || batchRunning" @click="retest(row)">
              {{ retesting === row.ip ? '测速中' : '复测' }}
            </button>
          </span>
        </div>
        <div v-if="!filtered.length" class="empty-state">
          <Trophy :size="30" />
          <span>{{ loading ? '加载中…' : '该地区暂无完整测速通过的结果' }}</span>
          <small>只有真实完成下载测速并达到阈值的 IP 才会出现在这里。</small>
        </div>
      </div>
    </section>

    <div v-if="detail" class="detail-mask" @click.self="detail = null">
      <section class="detail-card glass-card">
        <header class="detail-head">
          <div>
            <div class="detail-kicker">IP DETAIL · 真实测速记录</div>
            <h3 class="mono">{{ detail.ip }}:{{ detail.port }}</h3>
            <p>{{ detail.regionName }} · {{ detail.colo || 'colo 未知' }} · 来源 {{ detail.source }}</p>
          </div>
          <button class="detail-close" @click="detail = null">×</button>
        </header>
        <div class="detail-metrics">
          <div><span>平均延迟</span><strong>{{ detail.latency.toFixed(1) }} ms</strong></div>
          <div><span>抖动</span><strong>{{ detail.jitter.toFixed(1) }} ms</strong></div>
          <div><span>实际下载</span><strong>{{ detail.speedMbS.toFixed(2) }} MB/s</strong></div>
          <div><span>换算 Mbps</span><strong>{{ detail.speedMbps.toFixed(1) }} Mbps</strong></div>
          <div><span>丢包率</span><strong>{{ detail.loss.toFixed(1) }} %</strong></div>
          <div><span>综合评分</span><strong>{{ score(detail).toFixed(1) }}</strong></div>
        </div>
        <div class="detail-section-title">逐轮测试记录</div>
        <div class="round-list">
          <div v-for="(round, index) in detailRounds" :key="index" class="round-row" :class="{ fail: !round.ok }">
            <span class="round-phase">{{ round.phase }}</span>
            <span class="round-no">#{{ round.round }}</span>
            <span class="round-value">{{ round.latencyMs != null ? `${round.latencyMs.toFixed(1)} ms` : round.speedMbS != null ? `${round.speedMbS.toFixed(2)} MB/s` : '—' }}</span>
            <span class="round-message">{{ round.message }}</span>
          </div>
          <div v-if="!detailRounds.length" class="empty-note">暂无逐轮记录，请点击“复测”重新执行完整测试。</div>
        </div>
        <div class="detail-rule">
          <strong>综合评分规则</strong>
          <span>速度 60% · 延迟 30% · 丢包 10%。原始指标全部保留，不用综合分替代真实延迟与下载速度。</span>
        </div>
        <footer class="detail-actions">
          <button class="btn-ghost" @click="detail = null">关闭</button>
          <button class="btn-primary" :disabled="retesting !== '' || batchRunning" @click="detail && retest(detail)">
            {{ retesting === detail.ip ? '正在重新测速…' : '重新测速' }}
          </button>
        </footer>
      </section>
    </div>
  </div>
</template>

<style scoped>
.rankings-page { gap: 14px; }
.header-actions { display: flex; gap: 8px; }
.rank-top { display: grid; grid-template-columns: repeat(4, 1fr); gap: 11px; }
.rank-summary { display: flex; align-items: center; gap: 13px; padding: 14px 16px; }
.rank-medal { display: grid; place-items: center; width: 39px; height: 39px; border-radius: 10px; color: #e7bd69; background: rgba(211, 165, 72, 0.13); }
.rank-medal.blue { color: #7e9dff; background: rgba(91, 123, 232, 0.14); }
.rank-medal.violet { color: #b083ff; background: rgba(145, 90, 237, 0.13); }
.rank-medal.green { color: #5bdc9b; background: rgba(46, 183, 120, 0.13); }
.rank-summary div:last-child { display: flex; flex-direction: column; gap: 4px; }
.rank-summary span { color: #7180a1; font-size: 10px; }
.rank-summary strong { color: #eaf0ff; font-size: 22px; line-height: 1; }
.rank-summary small { color: #607092; font-size: 9px; font-weight: 400; }
.rank-body { padding: 14px; }
.rank-toolbar { display: flex; align-items: center; justify-content: space-between; gap: 12px; flex-wrap: wrap; }
.region-tabs { display: flex; gap: 4px; flex-wrap: wrap; }
.region-tab { height: 28px; border: 1px solid transparent; border-radius: 6px; background: transparent; color: #7282a6; font-size: 10px; padding: 0 10px; cursor: pointer; }
.region-tab:hover { color: #b8c8ec; }
.region-tab.active { border-color: rgba(108, 139, 255, 0.35); background: rgba(70, 99, 213, 0.16); color: #cfdbfa; }
.rank-tools { display: flex; align-items: center; gap: 7px; }
.search-box { display: flex; align-items: center; gap: 5px; height: 29px; padding: 0 9px; border: 1px solid var(--line); border-radius: 6px; color: #667897; }
.search-box input { width: 120px; border: 0; outline: 0; background: transparent; color: #c8d5f1; font-size: 10px; }
.sort-select { display: flex; align-items: center; gap: 5px; height: 29px; padding: 0 7px; border: 1px solid var(--line); border-radius: 6px; color: #7b8cae; }
.sort-select select { border: 0; outline: 0; background: transparent; color: #b5c5e8; font-size: 10px; }
.rank-caption { display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: 4px 12px; margin: 14px 1px 7px; color: #9eafd1; font-size: 10.5px; }
.caption-note { color: #5e6d8d; font-size: 9px; text-align: right; }
.table-wrap { overflow-x: auto; border-top: 1px solid rgba(108, 133, 255, 0.1); }
.rank-row { display: grid; grid-template-columns: 36px minmax(118px, 0.55fr) 38px 82px 52px 62px 66px 66px 46px minmax(85px, 1.6fr) 72px 76px; align-items: center; min-width: 800px; min-height: 38px; border-bottom: 1px solid rgba(109, 134, 198, 0.07); color: #9aabd0; font-size: 10px; }
.rank-row.table-head { min-height: 30px; color: #5f7091; font-size: 8.5px; }
.rank-row > span { padding: 0 5px; min-width: 0; white-space: nowrap; }
.rank-index { color: #536384; font-family: var(--mono); }
.rank-index.top { color: #e0b56d; font-weight: 700; }
.ip { color: #dfe8ff; font-size: 10.5px; }
.colo { color: #94a8dc; font-family: var(--mono); }
.latency { color: #6fc0f5; }
.speed { color: #73dda5; font-weight: 600; }
.mbps { color: #7f90b0; }
.warn { color: #e2a167; }
.source { color: #69799d; font-size: 9px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.status { display: flex; align-items: center; gap: 4px; color: #5ed898; font-size: 9px; }
.status i { width: 4px; height: 4px; border-radius: 50%; background: #4ed48d; box-shadow: 0 0 5px rgba(78, 212, 141, 0.6); }
.status.ref { color: #e0b364; }
.status.ref i { background: #e0b364; box-shadow: 0 0 5px rgba(224, 179, 100, 0.6); }
.region-tag { display: inline-block; max-width: 100%; padding: 2px 5px; border-radius: 4px; background: rgba(96, 129, 239, 0.13); color: #9db4f2; font-size: 9px; overflow: hidden; text-overflow: ellipsis; vertical-align: middle; }
.empty-state { display: flex; flex-direction: column; align-items: center; gap: 8px; padding: 55px 10px; color: #5b6b8b; font-size: 11px; }
.empty-state svg { color: #52678e; opacity: 0.6; }
.empty-state small { color: #4c5b77; font-size: 9px; }
.row-actions { display: flex; gap: 6px; align-items: center; }
.link-btn { border: 0; background: transparent; color: #8195c5; font-size: 9px; padding: 3px 2px; cursor: pointer; }
.link-btn:hover { color: #d7e2ff; }
.link-btn.accent { color: #6fc0f5; }
.link-btn:disabled { opacity: .45; cursor: wait; }
.spinning { animation: spin 1s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }
.detail-mask { position: fixed; inset: 38px 0 0; z-index: 80; display: grid; place-items: center; padding: 30px; background: rgba(3, 7, 18, .62); backdrop-filter: blur(8px); }
.detail-card { width: min(720px, 92vw); max-height: 82vh; overflow: auto; padding: 20px 22px; background: linear-gradient(150deg, rgba(16, 25, 50, .98), rgba(9, 15, 32, .98)); border-color: rgba(112, 142, 255, .26); box-shadow: 0 28px 90px rgba(0,0,0,.45); }
.detail-head { display: flex; justify-content: space-between; gap: 16px; }
.detail-kicker { color: #7286b4; font-size: 8px; letter-spacing: .2em; }
.detail-head h3 { margin: 5px 0 4px; color: #eef4ff; font-size: 20px; letter-spacing: .02em; }
.detail-head p { margin: 0; color: #8192b7; font-size: 10px; }
.detail-close { width: 28px; height: 28px; border: 1px solid rgba(113, 139, 207, .2); border-radius: 7px; background: transparent; color: #8fa2ca; font-size: 19px; cursor: pointer; }
.detail-metrics { display: grid; grid-template-columns: repeat(3, 1fr); gap: 8px; margin-top: 17px; }
.detail-metrics div { padding: 10px 12px; border: 1px solid rgba(112, 140, 211, .14); border-radius: 9px; background: rgba(86, 112, 190, .08); }
.detail-metrics span { display: block; color: #7b8db4; font-size: 9px; }
.detail-metrics strong { display: block; margin-top: 5px; color: #e8efff; font-size: 14px; }
.detail-section-title { margin: 18px 0 8px; color: #b9c8e8; font-size: 11px; font-weight: 700; }
.round-list { border-top: 1px solid rgba(110, 137, 205, .12); }
.round-row { display: grid; grid-template-columns: 58px 30px 78px 1fr; gap: 8px; align-items: center; min-height: 31px; border-bottom: 1px solid rgba(110, 137, 205, .07); font-size: 9.5px; color: #9cabd0; }
.round-row.fail { color: #d79a82; }
.round-phase { color: #7f94c2; font-family: var(--mono); text-transform: uppercase; font-size: 8px; }
.round-value { color: #6fc0f5; font-family: var(--mono); }
.round-row.fail .round-value { color: #e29b7d; }
.detail-rule { display: flex; gap: 12px; margin-top: 16px; padding: 10px 12px; border: 1px solid rgba(94, 179, 255, .14); border-radius: 8px; background: rgba(45, 93, 175, .08); color: #8192b5; font-size: 9.5px; line-height: 1.55; }
.detail-rule strong { flex: 0 0 auto; color: #9db8ea; }
.detail-actions { display: flex; justify-content: flex-end; gap: 8px; margin-top: 18px; }
@media (max-width: 1200px) {
  .rank-top { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  .rank-body { padding: 12px; }
}

@media (max-width: 1180px) {
  .detail-metrics { grid-template-columns: repeat(2, minmax(0, 1fr)); }
}
</style>
