<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useMessage } from 'naive-ui'
import {
  Activity,
  ArrowUpRight,
  ChevronRight,
  CircleCheck,
  Clock3,
  Gauge,
  Globe,
  Play,
  RefreshCw,
  Rocket,
  Sparkles,
  Zap,
  ListChecks,
} from '@lucide/vue'
import ChartBox from '../components/ChartBox.vue'
import RegionFlag from '../components/RegionFlag.vue'
import earthHero from '../assets/earth-hero.png'
import sakuraBranch from '../assets/sakura-branch.svg'
import {
  state,
  regionStatsFor,
  startScan,
  toggleRegion,
  selectAllRegions,
  isAllSelected,
  refreshDashboard,
} from '../store'
import type { RegionInfo, RegionStats } from '../types'
import { api, isTauri } from '../api'

const message = useMessage()

const sourceLabel = ref('')
const sourceUrl = ref('')

async function loadSourceLabel() {
  if (!isTauri()) return
  try {
    const info = await api.speedSourceInfo()
    sourceLabel.value = info.effectiveLabel
    sourceUrl.value = info.sourceUrl
  } catch {
    sourceLabel.value = ''
  }
}

onMounted(loadSourceLabel)
watch(() => state.settings.speedSource, loadSourceLabel)

const displayRegions = computed<RegionInfo[]>(() => state.regions)
const preferredKeys = computed(() => new Set(state.regions.map((region) => region.key)))
const selectedCount = computed(() => state.selectedRegions.length)
const allSelected = computed(() => isAllSelected())
const busy = computed(() => state.scan?.status === 'running')
const scanActive = computed(
  () => state.scan?.status === 'running' || state.scan?.status === 'paused' || state.scan?.status === 'cancelling',
)
const scanCompleted = computed(() => state.scan?.status === 'completed')
const hasResults = computed(() => state.dashboard.usable > 0)

/**
 * 非首选地区（以及未知地区）只要本轮有达标结果，也必须出现在总览里。
 * 后端 dashboard 会按真实 colo 国家返回这些地区；实时扫描结束瞬间则用 global 汇总兜底。
 */
const extraRegionStats = computed<RegionStats[]>(() => {
  const preferred = preferredKeys.value
  const fromDashboard = (state.dashboard.regionStats || [])
    .filter((row) => !preferred.has(row.key) && Number(row.finalCount || 0) > 0)
    .sort((a, b) => b.finalCount - a.finalCount)
  if (fromDashboard.length) return fromDashboard
  return (state.scan?.regionStats || [])
    .filter((row) => !preferred.has(row.key) && Number(row.finalCount || 0) > 0)
    .sort((a, b) => b.finalCount - a.finalCount)
})

const chartStats = computed(() => {
  const base = displayRegions.value.map((region) => regionStatsFor(region.key))
  const extras = extraRegionStats.value
  if (!extras.length) return base
  const extraKnown = extras.filter((row) => row.key !== 'unknown')
  const unknown = extras.find((row) => row.key === 'unknown')
  const result = [...base]
  if (extraKnown.length) {
    const total = extraKnown.reduce((sum, row) => sum + row.finalCount, 0)
    const latency = total ? extraKnown.reduce((sum, row) => sum + row.avgLatency * row.finalCount, 0) / total : 0
    const speed = total ? extraKnown.reduce((sum, row) => sum + row.avgSpeed * row.finalCount, 0) / total : 0
    result.push({
      key: 'global',
      name: '全球其他',
      flag: '🌐',
      candidates: extraKnown.reduce((sum, row) => sum + row.candidates, 0),
      tcpOk: 0,
      httpsOk: 0,
      coloOk: 0,
      latencyOk: 0,
      downloadOk: total,
      speedOk: total,
      finalCount: total,
      avgLatency: latency,
      avgSpeed: speed,
      latencyEliminated: 0,
    })
  }
  if (unknown) result.push(unknown)
  return result
})

const chartShortNames: Record<string, string> = {
  香港: '港',
  台湾: '台',
  日本: '日',
  新加坡: '新',
  韩国: '韩',
  法国: '法',
  美国: '美',
  全球其他: '其他',
  全球其他地区: '其他',
  未知地区: '未知',
}

function chartShortName(name: string) {
  return chartShortNames[name] || name.slice(0, 2)
}

// 平均延迟 / 下载图表只画本轮真正有结果的地区；没有结果的地区显示 0
// 会在折线或柱状图里造成误导。IP 数量图表仍保留全部地区，方便看出缺口。
const dataChartStats = computed(() => chartStats.value.filter((row) => row.finalCount > 0))

const statCards = computed(() => {
  const stats = chartStats.value
  return [
    {
      label: '测试 IP 数量',
      value: state.dashboard.totalTested.toLocaleString(),
      suffix: '个',
      delta: state.dashboard.totalCandidates > 0 ? `候选池 ${state.dashboard.totalCandidates.toLocaleString()}` : '等待导入',
      tone: 'blue',
      icon: Rocket,
      spark: stats.map((row) => row.candidates || row.finalCount),
    },
    {
      label: '平均延迟',
      value: state.dashboard.avgLatency > 0 ? state.dashboard.avgLatency.toFixed(0) : '—',
      suffix: 'ms',
      delta: state.dashboard.avgLatency > 0 ? '最近扫描均值' : '尚无测速',
      tone: 'cyan',
      icon: Zap,
      spark: stats.map((row) => row.avgLatency),
    },
    {
      label: '平均下载速度',
      value: state.dashboard.avgSpeed > 0 ? state.dashboard.avgSpeed.toFixed(2) : '—',
      suffix: 'MB/s',
      delta: state.dashboard.avgSpeed > 0 ? `${(state.dashboard.avgSpeed * 8).toFixed(1)} Mbps` : '尚无测速',
      tone: 'violet',
      icon: Activity,
      spark: stats.map((row) => row.avgSpeed),
    },
    {
      label: '可用 IP 数量',
      value: state.dashboard.usable.toLocaleString(),
      suffix: '个',
      delta: state.dashboard.usable > 0 ? '完整测速通过' : '等待优选',
      tone: 'green',
      icon: CircleCheck,
      spark: stats.map((row) => row.finalCount),
    },
  ]
})

function sparkPoints(values: number[]) {
  const max = Math.max(...values)
  const min = Math.min(...values)
  return values
    .map((value, index) => {
      const x = (index / (values.length - 1)) * 100
      const y = 34 - ((value - min) / Math.max(1, max - min)) * 25
      return `${x.toFixed(1)},${y.toFixed(1)}`
    })
    .join(' ')
}

const latencyOption = computed(() => {
  const labels = dataChartStats.value.map((r) => r.name)
  const values = dataChartStats.value.map((r) => r.avgLatency)
  return {
    animationDuration: 500,
    grid: { left: 38, right: 12, top: 20, bottom: 28 },
    tooltip: {
      trigger: 'axis',
      backgroundColor: 'rgba(10, 17, 35, 0.96)',
      borderColor: 'rgba(112, 139, 255, 0.3)',
      textStyle: { color: '#e8edff', fontSize: 11 },
      formatter: (params: Array<{ name: string; value: number }>) =>
        `${params[0].name}<br/>平均延迟：${Number(params[0].value).toFixed(1)} ms`,
    },
    xAxis: {
      type: 'category',
      data: labels,
      axisLine: { lineStyle: { color: 'rgba(112, 139, 255, 0.15)' } },
      axisTick: { show: false },
      axisLabel: { color: '#6f7c9e', fontSize: 9, interval: 0, formatter: chartShortName },
    },
    yAxis: {
      type: 'value',
      splitLine: { lineStyle: { color: 'rgba(112, 139, 255, 0.09)' } },
      axisLabel: { color: '#5d6b8c', fontSize: 9 },
    },
    series: [
      {
        type: 'bar',
        data: values.map((value, index) => ({
          value,
          itemStyle: {
            color:
              index === 0
                ? '#6f8dff'
                : index === 1
                  ? '#5c87ff'
                  : index === 2
                    ? '#6b75ff'
                    : index === 3
                      ? '#8069ff'
                      : index === 4
                        ? '#9b63ff'
                        : index === 5
                          ? '#a75cff'
                          : '#5d8cff',
          },
        })),
        barMaxWidth: 12,
        itemStyle: { borderRadius: [5, 5, 0, 0] },
      },
    ],
  }
})

const speedOption = computed(() => {
  const labels = dataChartStats.value.map((r) => r.name)
  const values = dataChartStats.value.map((r) => r.avgSpeed)
  return {
    animationDuration: 500,
    grid: { left: 38, right: 12, top: 20, bottom: 28 },
    tooltip: {
      trigger: 'axis',
      backgroundColor: 'rgba(10, 17, 35, 0.96)',
      borderColor: 'rgba(112, 139, 255, 0.3)',
      textStyle: { color: '#e8edff', fontSize: 11 },
      formatter: (params: Array<{ name: string; value: number }>) =>
        `${params[0].name}<br/>平均下载：${Number(params[0].value).toFixed(2)} MB/s`,
    },
    xAxis: {
      type: 'category',
      data: labels,
      axisLine: { lineStyle: { color: 'rgba(112, 139, 255, 0.15)' } },
      axisTick: { show: false },
      axisLabel: { color: '#6f7c9e', fontSize: 9, interval: 0, formatter: chartShortName },
    },
    yAxis: {
      type: 'value',
      splitLine: { lineStyle: { color: 'rgba(112, 139, 255, 0.09)' } },
      axisLabel: { color: '#5d6b8c', fontSize: 9 },
    },
    series: [
      {
        type: 'line',
        data: values,
        smooth: true,
        symbol: 'circle',
        symbolSize: 7,
        lineStyle: { width: 2.5, color: '#9d6eff' },
        itemStyle: { color: '#b58aff', borderColor: '#0b1224', borderWidth: 2 },
        areaStyle: {
          color: {
            type: 'linear',
            x: 0, y: 0, x2: 0, y2: 1,
            colorStops: [
              { offset: 0, color: 'rgba(155, 92, 255, 0.28)' },
              { offset: 1, color: 'rgba(155, 92, 255, 0)' },
            ],
          },
        },
      },
    ],
  }
})

const countOption = computed(() => {
  const labels = chartStats.value.map((r) => r.name)
  // 图表统计的是本轮正式保留的 IP，而不是候选池里的原始条数；
  // 候选池数量已在“测试 IP 数量”卡片单独展示，混用会让地区数量看起来失真。
  const values = chartStats.value.map((r) => r.finalCount)
  return {
    animationDuration: 500,
    grid: { left: 34, right: 8, top: 16, bottom: 24 },
    tooltip: {
      trigger: 'axis',
      backgroundColor: 'rgba(10, 17, 35, 0.96)',
      borderColor: 'rgba(112, 139, 255, 0.3)',
      textStyle: { color: '#e8edff', fontSize: 11 },
      formatter: (params: Array<{ name: string; value: number }>) => `${params[0].name}<br/>正式保留：${params[0].value} 个`,
    },
    xAxis: {
      type: 'category',
      data: labels,
      axisLine: { lineStyle: { color: 'rgba(112, 139, 255, 0.15)' } },
      axisTick: { show: false },
      axisLabel: { color: '#6f7c9e', fontSize: 9, interval: 0, formatter: chartShortName },
    },
    yAxis: {
      type: 'value',
      splitLine: { lineStyle: { color: 'rgba(112, 139, 255, 0.09)' } },
      axisLabel: { color: '#5d6b8c', fontSize: 8 },
    },
    series: [
      {
        type: 'bar',
        data: values,
        barMaxWidth: 8,
        itemStyle: {
          borderRadius: [4, 4, 0, 0],
          color: {
            type: 'linear',
            x: 0, y: 0, x2: 0, y2: 1,
            colorStops: [
              { offset: 0, color: '#43d9ff' },
              { offset: 1, color: '#6d8cff' },
            ],
          },
        },
      },
    ],
  }
})

const logs = computed(() => state.logs.slice(-16).reverse())

const flowSteps = [
  { title: 'TCP 连通性', desc: '批量探测端口与握手' },
  { title: 'HTTPS / colo', desc: '识别边缘节点与地区' },
  { title: '延迟测试', desc: '多次采样取稳定值' },
  { title: '下载测速', desc: '真实拉流验证带宽' },
]

const stageStats = computed(() => {
  const scan = state.scan
  const regionTotals = (state.dashboard.regionStats || []).reduce(
    (totals, row) => {
      totals.colo += row.coloOk || 0
      totals.download += row.speedOk || 0
      return totals
    },
    { colo: 0, download: 0 },
  )
  // 重启后没有实时 scan 状态，不能把未知阶段显示成 0，避免误判为测速失败。
  if (!scan) {
    return [
      { label: 'TCP 通过', value: '—' },
      { label: 'HTTPS 通过', value: '—' },
      { label: 'colo 命中', value: regionTotals.colo > 0 ? regionTotals.colo : '—' },
      {
        label: '下载通过',
        value: regionTotals.download > 0 ? regionTotals.download : state.dashboard.usable || '—',
      },
    ]
  }
  return [
    { label: 'TCP 通过', value: scan.tcpPassed ?? scan.tcpOk ?? 0 },
    { label: 'HTTPS 通过', value: scan.httpsPassed ?? scan.httpsOk ?? 0 },
    { label: 'colo 命中', value: scan.coloOk ?? regionTotals.colo },
    { label: '下载通过', value: scan.downloadPassed ?? (regionTotals.download || state.dashboard.usable) },
  ]
})

async function onStart() {
  try {
    await refreshDashboard()
    await startScan()
    message.success('扫描任务已启动')
  } catch (error) {
    message.error(String(error))
  }
}

function progressPercent(stats: { tested: number; total: number } | null) {
  if (!stats || !stats.total) return 0
  return Math.min(100, Math.round((stats.tested / stats.total) * 100))
}

const overallPercent = computed(() => {
  if (!state.scan || !scanActive.value) return hasResults.value ? 100 : 0
  return progressPercent(state.scan)
})

const coverageRows = computed(() => {
  const preferred = displayRegions.value.map((region) => {
    const stat = regionStatsFor(region.key)
    return {
      key: region.key,
      name: region.name,
      colo: region.colos?.[0] || '',
      final: stat.finalCount || 0,
    }
  })
  const extras = extraRegionStats.value.map((row) => ({
    key: row.key,
    name: row.name,
    colo: row.avgLatency > 0 ? `${row.avgLatency.toFixed(0)} ms` : '',
    final: row.finalCount || 0,
  }))
  if (extras.length) return [...preferred, ...extras]
  const fallback = state.scan?.regionStats?.find((row) => row.key === 'global')
  if (fallback && fallback.finalCount > 0) {
    preferred.push({ key: 'global', name: fallback.name || '全球其他地区', colo: '', final: fallback.finalCount })
  }
  return preferred
})

const coveredCount = computed(() => coverageRows.value.filter((row) => row.final > 0).length)
</script>

<template>
  <div class="page overview-page">
    <section class="hero glass-card">
      <div class="hero-stars"></div>
      <div class="hero-orbit orbit-one"></div>
      <div class="hero-orbit orbit-two"></div>
      <div class="hero-content">
        <div class="hero-icon">
          <Globe :size="35" :stroke-width="1.2" />
          <span class="hero-icon-ring"></span>
        </div>
        <div class="hero-copy">
          <div class="hero-brand-row">
            <h1>三笠优选</h1>
            <span class="hero-en">MIKASA OPTIMIZER</span>
          </div>
          <p>更优的连接，更好的体验 · 实际延迟 + 真实下载测速</p>
          <div class="hero-badges">
            <span class="pill green"><CircleCheck :size="11" />真实测速</span>
            <span class="pill"><ListChecks :size="11" />七地区独立筛选</span>
            <span class="pill"><Sparkles :size="11" />综合质量排名</span>
          </div>
        </div>
      </div>
      <div class="hero-quote">
        <span>智能发现</span><i></i><span>七地区覆盖</span><i></i><span>历史记忆</span>
      </div>
      <img class="hero-sakura" :src="sakuraBranch" alt="" aria-hidden="true" />
      <div class="hero-earth-img" :style="{ backgroundImage: `url(${earthHero})` }"></div>
      <div class="hero-earth"></div>
    </section>

    <section class="section-block">
      <div class="block-head">
        <div class="section-title"><Globe :size="16" />选择优选地区</div>
        <div class="block-meta">
          {{ selectedCount }} / {{ state.regions.length }} 个地区已选择
          <button class="text-btn" :disabled="scanActive" @click="selectAllRegions">
            {{ allSelected ? '取消全选' : '全部选择' }}
          </button>
        </div>
      </div>
      <div class="region-grid">
        <button
          v-for="region in displayRegions"
          :key="region.key"
          class="region-card"
          :class="{ selected: state.selectedRegions.includes(region.key) }"
          :disabled="scanActive"
          :title="`${region.name} · ${region.colos.join(' · ')}`"
          @click="toggleRegion(region.key)"
        >
          <RegionFlag :code="region.key" :width="24" />
          <span class="region-copy">
            <strong>{{ region.name }}</strong>
            <em>{{ region.colos.slice(0, 3).join(' · ') }}</em>
          </span>
          <span class="region-check"><CircleCheck :size="12" /></span>
        </button>
        <button class="region-card all-region" :class="{ selected: allSelected }" :disabled="scanActive" @click="selectAllRegions">
          <span class="region-globe"><Globe :size="19" /></span>
          <span class="region-copy">
            <strong>全部地区</strong>
            <em>智能优选</em>
          </span>
          <span class="region-check"><CircleCheck :size="12" /></span>
        </button>
      </div>
    </section>

    <section class="param-bar glass-card">
      <div class="param-title"><Gauge :size="15" /><span>优选参数</span></div>
      <div class="param-items">
        <label class="param-item">
          <span>每地区保留</span>
          <select v-model.number="state.settings.perRegionTop" :disabled="scanActive">
            <option :value="10">10 个</option>
            <option :value="20">20 个</option>
            <option :value="50">50 个</option>
            <option :value="100">100 个</option>
          </select>
        </label>
        <label class="param-item">
          <span>测试端口</span>
          <select v-model.number="state.settings.defaultPort" :disabled="scanActive">
            <option :value="443">443</option>
            <option :value="80">80</option>
            <option :value="8443">8443</option>
          </select>
        </label>
        <label class="param-item">
          <span>并发数量</span>
          <select v-model.number="state.settings.concurrency" :disabled="scanActive">
            <option :value="32">32</option>
            <option :value="64">64</option>
            <option :value="128">128</option>
            <option :value="256">256</option>
          </select>
        </label>
        <label class="param-item">
          <span>延迟上限</span>
          <select v-model.number="state.settings.latencyLimitMs" :disabled="scanActive">
            <option :value="300">300 ms</option>
            <option :value="500">500 ms</option>
            <option :value="800">800 ms</option>
            <option :value="1000">1000 ms</option>
            <option :value="1500">1500 ms</option>
            <option :value="2000">2000 ms</option>
          </select>
        </label>
        <label class="param-item">
          <span>测速地址</span>
          <select v-model="state.settings.speedSource" :disabled="scanActive">
            <option value="auto">自动选择</option>
            <option value="cloudflare">Cloudflare</option>
            <option value="cm">CM 提供</option>
            <option value="mobile">移动专属</option>
            <option value="custom">手动输入</option>
          </select>
        </label>
        <span v-if="sourceLabel" class="source-inline" :title="sourceUrl">· {{ sourceLabel }}</span>


        <span class="smart-pill"><Zap :size="11" />智能补充模式</span>
        <span class="param-note">下载 {{ state.settings.downloadSeconds }}s / 最低 {{ (state.settings.minDownloadBytes / 1024 / 1024).toFixed(0) }}MB</span>
      </div>
      <button class="btn-primary start-btn" :disabled="scanActive || selectedCount === 0" @click="onStart">
        <Play :size="15" fill="currentColor" />
        {{ busy ? '扫描进行中' : '开始优选' }}
      </button>
    </section>

    <section class="dashboard-grid">
      <div class="stats-grid">
        <div v-for="card in statCards" :key="card.label" class="stat-card glass-card" :class="`stat-${card.tone}`">
          <div class="stat-top">
            <div class="stat-icon"><component :is="card.icon" :size="17" /></div>
            <span class="stat-label">{{ card.label }}</span>
          </div>
          <div class="stat-value-row">
            <span class="stat-value">{{ card.value }}</span>
            <span class="stat-unit">{{ card.suffix }}</span>
          </div>
          <div class="stat-bottom">
            <span class="stat-delta">{{ card.delta }}</span>
            <svg viewBox="0 0 100 36" preserveAspectRatio="none" class="spark">
              <polyline :points="sparkPoints(card.spark)" fill="none" stroke="currentColor" stroke-width="1.4" />
            </svg>
          </div>
        </div>
      </div>

      <div class="charts-grid">
        <div class="chart-card glass-card">
          <div class="card-head">
            <div class="section-title"><Activity :size="14" />地区平均延迟</div>
            <span class="mini-legend"><i class="latency"></i>ms</span>
          </div>
          <ChartBox :option="latencyOption" height="104px" />
        </div>
        <div class="chart-card glass-card">
          <div class="card-head">
            <div class="section-title"><Zap :size="14" />地区平均下载</div>
            <span class="mini-legend"><i class="speed"></i>MB/s</span>
          </div>
          <ChartBox :option="speedOption" height="104px" />
        </div>
        <div class="chart-card glass-card">
          <div class="card-head">
            <div class="section-title"><ListChecks :size="14" />地区保留 IP 数量</div>
            <span class="mini-legend"><i class="count"></i>个</span>
          </div>
          <ChartBox :option="countOption" height="104px" />
        </div>
      </div>
    </section>

    <section class="bottom-grid">
      <div class="logs-card glass-card">
        <div class="logs-head">
          <div class="section-title"><Clock3 :size="15" />实时日志</div>
          <div class="logs-tabs">
            <span class="active">测试详情</span>
            <span>历史记录</span>
            <button class="icon-btn" title="刷新状态" @click="refreshDashboard"><RefreshCw :size="13" /></button>
          </div>
        </div>
        <div class="log-list" :class="{ empty: logs.length === 0 }">
          <template v-if="logs.length">
            <div v-for="(log, index) in logs" :key="index" class="log-row">
              <span class="log-dot" :class="log.level"></span>
              <span class="log-time mono">{{ log.time }}</span>
              <span class="log-message">{{ log.message }}</span>
            </div>
          </template>
          <div v-else class="log-empty">
            <div class="log-empty-title">等待开始扫描 · 真实测试流程</div>
            <div class="log-flow">
              <div v-for="(step, index) in flowSteps" :key="step.title" class="log-flow-step">
                <span class="flow-index">0{{ index + 1 }}</span>
                <div class="flow-copy">
                  <strong>{{ step.title }}</strong>
                  <em>{{ step.desc }}</em>
                </div>
              </div>
            </div>
            <p class="log-empty-note">导入候选 IP 后点击“开始优选”，这里会实时显示各阶段日志。</p>
          </div>
        </div>
        <div class="stage-strip">
          <div v-for="stage in stageStats" :key="stage.label" class="stage-cell">
            <span>{{ stage.label }}</span>
            <strong>{{ stage.value }}</strong>
          </div>
        </div>
      </div>

      <div class="summary-card glass-card">
        <div class="summary-head">
          <div class="summary-icon" :class="{ done: state.dashboard.usable > 0 }">
            <CircleCheck :size="20" />
          </div>
          <div>
            <div class="summary-title">
              {{
                busy
                  ? '正在测速'
                  : scanActive
                    ? '任务已暂停'
                    : scanCompleted
                      ? '本轮优选完成'
                      : hasResults
                        ? '最近一次优选结果'
                        : '等待优选'
              }}
            </div>
            <div class="summary-sub">
              {{
                busy
                  ? state.scan?.message
                  : scanActive
                    ? '任务已暂停，可在扫描任务页继续或取消'
                    : scanCompleted
                      ? `本轮找到 ${state.dashboard.usable} 个优质 IP`
                      : hasResults
                        ? `已找到 ${state.dashboard.usable} 个优质 IP（上次结果）`
                        : '完成后此处显示本轮结果'
              }}
            </div>
          </div>
        </div>
        <div class="summary-progress">
          <div class="summary-progress-head">
            <span>{{ state.scan ? '整体进度' : hasResults ? '结果状态' : '整体进度' }}</span>
            <strong>{{ state.scan ? `${overallPercent}%` : hasResults ? '已完成' : '0%' }}</strong>
          </div>
          <div class="progress-track"><span :style="{ width: `${overallPercent}%` }"></span></div>
        </div>
        <div class="coverage-list">
          <div class="coverage-head">
            <span>地区覆盖</span>
            <em>{{ coveredCount }} / {{ coverageRows.length }}</em>
          </div>
          <div class="coverage-chips">
            <div
              v-for="row in coverageRows"
              :key="row.key"
              class="coverage-chip"
              :class="{ zero: row.final === 0 }"
              :title="`${row.name} · ${row.colo || '未知 colo'} · ${row.final} 个`"
            >
              <RegionFlag :code="row.key" :width="13" />
              <span class="coverage-name">{{ row.name }}</span>
              <em>{{ row.final }}</em>
            </div>
          </div>
        </div>
        <div class="summary-actions">
          <button class="btn-ghost" @click="state.activeView = 'github'"><ArrowUpRight :size="13" />上传 GitHub</button>
          <button class="btn-ghost" @click="state.activeView = 'export'">导出结果<ChevronRight :size="13" /></button>
        </div>
      </div>
    </section>

    <footer class="overview-footer">
      <span><span class="online-dot"></span>网络状态：{{ state.network?.ok ? '正常' : '待检测' }} · {{ state.network?.display || '网络环境未知' }}</span>
      <span class="footer-note">所有结果均来自本地真实测速 · 不使用模拟数据</span>
    </footer>
  </div>
</template>

<style scoped>
.overview-page {
  flex: 1;
  gap: 7px;
  padding: 8px 14px 4px;
  min-height: 0;
  height: auto;
  overflow: hidden;
}

.hero {
  position: relative;
  flex: 0 0 92px;
  height: 92px;
  min-height: 92px;
  overflow: hidden;
  padding: 8px 24px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  background:
    linear-gradient(100deg, rgba(7, 15, 36, 0.94) 0%, rgba(8, 17, 42, 0.8) 45%, rgba(15, 20, 56, 0.54) 100%),
    radial-gradient(700px 260px at 83% 36%, rgba(39, 80, 200, 0.46), transparent 62%);
}

.hero-stars {
  position: absolute;
  inset: 0;
  opacity: 0.6;
  background-image:
    radial-gradient(circle at 18% 25%, rgba(197, 218, 255, 0.8) 0 1px, transparent 1.5px),
    radial-gradient(circle at 43% 14%, rgba(145, 180, 255, 0.5) 0 1px, transparent 1.5px),
    radial-gradient(circle at 72% 24%, rgba(233, 241, 255, 0.7) 0 1px, transparent 1.5px),
    radial-gradient(circle at 88% 60%, rgba(170, 194, 255, 0.55) 0 1px, transparent 1.5px),
    radial-gradient(circle at 31% 78%, rgba(161, 186, 255, 0.45) 0 1px, transparent 1.5px),
    radial-gradient(circle at 56% 50%, rgba(181, 204, 255, 0.5) 0 1px, transparent 1.5px);
  background-size: 340px 220px;
}

.hero-orbit {
  position: absolute;
  border: 1px solid rgba(95, 144, 255, 0.2);
  border-radius: 50%;
  pointer-events: none;
}

.orbit-one {
  width: 560px;
  height: 560px;
  right: 130px;
  top: -260px;
  transform: rotate(-18deg);
}

.orbit-two {
  width: 760px;
  height: 760px;
  right: -30px;
  top: -370px;
  border-color: rgba(111, 94, 255, 0.12);
}

.hero-content {
  position: relative;
  z-index: 2;
  display: flex;
  align-items: center;
  gap: 22px;
  min-width: 0;
}

.hero-copy {
  min-width: 0;
}

.hero-brand-row {
  display: flex;
  align-items: baseline;
  gap: 12px;
  min-width: 0;
}

.hero-en {
  color: #8ea4d8;
  font-size: 9px;
  font-weight: 700;
  letter-spacing: 0.24em;
  white-space: nowrap;
}

.hero-icon {
  position: relative;
  width: 56px;
  height: 56px;
  display: grid;
  place-items: center;
  color: #b9d1ff;
  border: 1px solid rgba(92, 147, 255, 0.47);
  border-radius: 50%;
  background: radial-gradient(circle, rgba(35, 87, 221, 0.55), rgba(11, 30, 82, 0.15));
  box-shadow: 0 0 40px rgba(52, 113, 255, 0.32), inset 0 0 22px rgba(74, 132, 255, 0.22);
}

.hero-icon::before,
.hero-icon::after {
  content: "";
  position: absolute;
  border: 1px solid rgba(94, 144, 255, 0.35);
  border-radius: 50%;
}

.hero-icon::before { inset: 7px; }
.hero-icon::after { inset: -7px; opacity: 0.4; }

.hero-icon-ring {
  position: absolute;
  width: 1px;
  height: 100%;
  background: linear-gradient(transparent, rgba(125, 178, 255, 0.75), transparent);
}

.hero h1 {
  margin: 0;
  color: #f2f6ff;
  font-size: 29px;
  line-height: 1.08;
  letter-spacing: 0.06em;
  font-weight: 800;
  text-shadow: 0 0 30px rgba(119, 159, 255, 0.3);
}

.hero p {
  margin-top: 4px;
  color: #8397c7;
  font-size: 10.5px;
  letter-spacing: 0.04em;
}

.hero-badges {
  display: flex;
  gap: 6px;
  margin-top: 7px;
}

.hero-quote {
  position: relative;
  z-index: 3;
  display: flex;
  align-items: center;
  gap: 16px;
  margin-right: 92px;
  color: #cedcff;
  font-size: 17px;
  font-weight: 500;
  letter-spacing: 0.16em;
  opacity: 0.8;
  text-shadow: 0 0 20px rgba(123, 161, 255, 0.5);
}

.hero-quote i {
  width: 3px;
  height: 3px;
  border-radius: 50%;
  background: #7294ff;
  box-shadow: 0 0 9px #7294ff;
}

.hero-sakura {
  position: absolute;
  right: -20px;
  top: -34px;
  width: 310px;
  height: 132px;
  object-fit: contain;
  object-position: right top;
  pointer-events: none;
  opacity: 0.5;
  filter: brightness(1.2) saturate(1.1) drop-shadow(0 0 14px rgba(255, 160, 205, 0.22));
  z-index: 2;
}

.hero-earth-img {
  position: absolute;
  right: 118px;
  top: -4px;
  width: 600px;
  height: 118px;
  background-size: cover;
  background-position: center right;
  opacity: 0.62;
  mix-blend-mode: screen;
  mask-image: linear-gradient(100deg, transparent 0%, #000 34%, #000 76%, transparent 100%);
  pointer-events: none;
  z-index: 1;
}

.hero-earth {
  position: absolute;
  right: 38px;
  bottom: -84px;
  width: 250px;
  height: 250px;
  border-radius: 50%;
  background:
    radial-gradient(circle at 37% 30%, rgba(118, 173, 255, 0.55), transparent 8%),
    radial-gradient(circle at 50% 50%, rgba(19, 66, 177, 0.52), rgba(3, 12, 38, 0.92) 62%, transparent 64%);
  box-shadow: 0 0 78px rgba(43, 96, 255, 0.3), inset 18px 10px 48px rgba(77, 140, 255, 0.22);
  opacity: 0.55;
}

.section-block {
  flex: 0 0 auto;
  padding-top: 0;
}

.block-head,
.card-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  margin-bottom: 6px;
}

.block-meta {
  display: flex;
  align-items: center;
  gap: 11px;
  color: #62708f;
  font-size: 10.5px;
}

.text-btn {
  border: none;
  background: transparent;
  color: #8fa6ed;
  font-size: 10.5px;
  cursor: pointer;
}

.region-grid {
  display: grid;
  grid-template-columns: repeat(8, minmax(0, 1fr));
  gap: 7px;
}

.region-card {
  position: relative;
  display: flex;
  align-items: center;
  gap: 7px;
  height: 50px;
  padding: 0 8px;
  border: 1px solid rgba(108, 133, 255, 0.14);
  border-radius: 11px;
  background: linear-gradient(145deg, rgba(18, 29, 57, 0.7), rgba(8, 14, 28, 0.72));
  color: var(--text-1);
  text-align: left;
  cursor: pointer;
  transition: 0.18s ease;
}

.region-card:hover {
  border-color: rgba(116, 152, 255, 0.32);
  background: linear-gradient(145deg, rgba(24, 40, 78, 0.8), rgba(10, 18, 36, 0.8));
  transform: translateY(-1px);
}

.region-card.selected {
  border-color: rgba(109, 140, 255, 0.62);
  background: linear-gradient(120deg, rgba(48, 77, 176, 0.5), rgba(47, 37, 118, 0.28));
  box-shadow: 0 0 0 1px rgba(123, 152, 255, 0.08), 0 9px 22px rgba(25, 48, 135, 0.2);
}

.region-flag {
  font-size: 23px;
  line-height: 1;
  filter: saturate(0.9);
}

.region-copy {
  display: flex;
  flex-direction: column;
  gap: 3px;
  min-width: 0;
}

.region-copy strong {
  color: #e3eaff;
  font-size: 11px;
  font-weight: 700;
  white-space: nowrap;
}

.region-copy em {
  color: #7181a8;
  font-size: 8px;
  font-style: normal;
  letter-spacing: 0.05em;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.region-check {
  display: grid;
  place-items: center;
  margin-left: auto;
  width: 18px;
  height: 18px;
  border: 1px solid rgba(117, 142, 203, 0.3);
  border-radius: 50%;
  color: transparent;
  flex-shrink: 0;
}

.region-card.selected .region-check {
  color: #fff;
  border-color: #91a9ff;
  background: #6d8cff;
  box-shadow: 0 0 10px rgba(109, 140, 255, 0.6);
}

.all-region .region-globe {
  display: grid;
  place-items: center;
  flex-shrink: 0;
  width: 24px;
  height: 24px;
  border: 1px solid rgba(118, 148, 255, 0.25);
  border-radius: 50%;
  color: #90a9ff;
}

.param-bar {
  flex: 0 0 auto;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 6px 10px;
}

.param-title {
  display: flex;
  align-items: center;
  gap: 7px;
  padding-right: 13px;
  border-right: 1px solid var(--line);
  color: #d3dcf8;
  font-size: 12px;
  font-weight: 700;
  white-space: nowrap;
}

.param-title svg { color: #8ca4f4; }

.param-items {
  min-width: 0;
  flex-wrap: wrap;
  overflow: hidden;
  display: flex;
  align-items: center;
  gap: 13px;
  flex: 1;
  min-width: 0;
}

.param-item {
  display: flex;
  align-items: center;
  gap: 7px;
  color: #7f8cac;
  font-size: 10.5px;
  white-space: nowrap;
}

.param-item select {
  height: 27px;
  min-width: 72px;
  padding: 0 8px;
  border: 1px solid rgba(108, 133, 255, 0.18);
  border-radius: 7px;
  outline: none;
  background: rgba(7, 12, 26, 0.8);
  color: #dce5ff;
  font-size: 11px;
}

.smart-pill {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  height: 24px;
  padding: 0 8px;
  border: 1px solid rgba(69, 199, 152, 0.2);
  border-radius: 6px;
  background: rgba(45, 183, 137, 0.08);
  color: #72d7af;
  font-size: 10px;
  white-space: nowrap;
}

.source-inline {
  color: #6f80a8;
  font-size: 9px;
  letter-spacing: 0.02em;
  white-space: nowrap;
}

.source-pill {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  height: 27px;
  padding: 0 8px;
  border: 1px solid rgba(109, 140, 255, 0.22);
  border-radius: 6px;
  background: rgba(109, 140, 255, 0.08);
  color: #9db2ff;
  font-size: 10px;
  white-space: nowrap;
  max-width: 210px;
  overflow: hidden;
  text-overflow: ellipsis;
}

.param-note {
  color: #596583;
  font-size: 9.5px;
  white-space: nowrap;
}

.start-btn {
  height: 34px;
  padding: 0 24px;
  flex-shrink: 0;
}

.dashboard-grid {
  display: grid;
  grid-template-columns: 0.9fr 1.6fr;
  gap: 8px;
  flex: 0 0 168px;
  min-height: 0;
}

.stats-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 8px;
}

.stat-card {
  padding: 7px 9px 4px;
  min-height: 0;
  overflow: hidden;
}

.stat-card::before {
  content: "";
  position: absolute;
  width: 120px;
  height: 120px;
  right: -48px;
  top: -58px;
  border-radius: 50%;
  background: currentColor;
  opacity: 0.08;
  filter: blur(14px);
}

.stat-blue { color: #6d8cff; }
.stat-cyan { color: #43d9ff; }
.stat-violet { color: #a675ff; }
.stat-green { color: #40d387; }

.stat-top {
  display: flex;
  align-items: center;
  gap: 9px;
}

.stat-icon {
  display: grid;
  place-items: center;
  width: 26px;
  height: 26px;
  border-radius: 8px;
  background: rgba(109, 140, 255, 0.16);
  border: 1px solid rgba(109, 140, 255, 0.28);
  color: #cfe0ff;
  opacity: 0.98;
}

.stat-cyan .stat-icon {
  background: rgba(67, 217, 255, 0.14);
  border-color: rgba(67, 217, 255, 0.24);
  color: #c8f4ff;
}

.stat-violet .stat-icon {
  background: rgba(166, 117, 255, 0.15);
  border-color: rgba(166, 117, 255, 0.26);
  color: #e3d4ff;
}

.stat-green .stat-icon {
  background: rgba(64, 211, 135, 0.14);
  border-color: rgba(64, 211, 135, 0.24);
  color: #c9f7df;
}

.stat-label {
  color: #95a1c0;
  font-size: 11.5px;
}

.stat-value-row {
  display: flex;
  align-items: baseline;
  gap: 4px;
  margin-top: 4px;
}

.stat-value {
  color: #f1f5ff;
  font-size: 20px;
  font-weight: 800;
  line-height: 1;
  font-variant-numeric: tabular-nums;
}

.stat-unit {
  color: #657292;
  font-size: 9px;
}

.stat-bottom {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  height: 15px;
  margin-top: 2px;
}

.stat-delta {
  color: #65c58f;
  font-size: 8.5px;
}

.spark {
  width: 58px;
  height: 15px;
  color: currentColor;
  opacity: 0.7;
  overflow: visible;
}

.charts-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 8px;
}

.chart-card,
.summary-card {
  padding: 10px 10px 3px;
  min-width: 0;
  overflow: hidden;
}

.chart-card :deep(.chart-box) {
  min-width: 0;
}

.mini-legend {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  color: #647191;
  font-size: 9.5px;
}

.mini-legend i {
  width: 5px;
  height: 5px;
  border-radius: 50%;
}

.mini-legend .latency { background: #6d8cff; }
.mini-legend .speed { background: #a675ff; }

.bottom-grid {
  display: grid;
  grid-template-columns: 1.75fr 0.9fr;
  gap: 9px;
  flex: 1 1 auto;
  min-height: 0;
}

.coverage-list {
  margin-top: 8px;
  display: flex;
  flex-direction: column;
  gap: 2px;
  flex: 1;
  min-height: 0;
  overflow: hidden;
}

.coverage-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  color: #7c8bb2;
  font-size: 9.5px;
  letter-spacing: 0.05em;
  padding-bottom: 3px;
  border-bottom: 1px dashed rgba(108, 133, 255, 0.14);
}

.coverage-head em {
  font-style: normal;
  color: #9fb2e0;
}

.coverage-chips {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 5px 6px;
  margin-top: 6px;
  max-height: 134px;
  overflow-y: auto;
  scrollbar-width: thin;
}

.coverage-chip {
  display: flex;
  align-items: center;
  gap: 5px;
  min-width: 0;
  height: 26px;
  padding: 0 7px;
  border: 1px solid rgba(108, 133, 255, 0.1);
  border-radius: 5px;
  background: rgba(98, 124, 215, 0.06);
  color: #a8b6d6;
  font-size: 10.5px;
}

.coverage-name {
  flex: 1;
  min-width: 0;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.coverage-chip em {
  font-style: normal;
  color: #57e39b;
  font-size: 12px;
  font-weight: 700;
  font-variant-numeric: tabular-nums;
}

.coverage-chip.zero { color: #5d6a8c; }
.coverage-chip.zero em { color: #5d6a8c; font-weight: 400; }

.summary-card {
  padding: 10px 10px 7px;
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
}

.summary-head {
  display: flex;
  align-items: center;
  gap: 11px;
}

.summary-icon {
  display: grid;
  place-items: center;
  width: 34px;
  height: 34px;
  flex-shrink: 0;
  border: 1px solid rgba(108, 133, 255, 0.22);
  border-radius: 11px;
  color: #8091c6;
  background: rgba(87, 112, 214, 0.1);
}

.summary-icon.done {
  border-color: rgba(34, 197, 94, 0.32);
  color: #53e392;
  background: rgba(34, 197, 94, 0.1);
  box-shadow: 0 0 18px rgba(34, 197, 94, 0.14);
}

.summary-title {
  color: #e5ebff;
  font-size: 15px;
  font-weight: 700;
}

.summary-sub {
  color: #7180a5;
  font-size: 10.5px;
  margin-top: 4px;
}

.summary-progress {
  margin-top: 8px;
}

.summary-progress-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 7px;
  color: #7482a7;
  font-size: 10px;
}

.summary-progress-head strong {
  color: #b8c6ea;
  font-size: 11px;
}

.progress-track {
  height: 6px;
  overflow: hidden;
  border-radius: 999px;
  background: rgba(109, 140, 255, 0.12);
}

.progress-track span {
  display: block;
  height: 100%;
  border-radius: inherit;
  background: linear-gradient(90deg, #5c7cff, #a46bff);
  box-shadow: 0 0 12px rgba(124, 104, 255, 0.55);
  transition: width 0.35s ease;
}

.summary-actions {
  display: flex;
  gap: 8px;
  margin-top: auto;
  padding-top: 8px;
}

.summary-actions .btn-ghost {
  flex: 1;
  font-size: 10.5px;
}

.logs-card {
  display: flex;
  flex-direction: column;
  padding: 8px 11px 6px;
  height: 100%;
  min-height: 0;
}

.logs-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.logs-tabs {
  display: flex;
  align-items: center;
  gap: 16px;
  color: #657393;
  font-size: 10.5px;
}

.logs-tabs .active {
  color: #b8c8f5;
}

.logs-tabs .icon-btn {
  width: 26px;
  height: 26px;
}

.log-list {
  margin-top: 6px;
  border-top: 1px solid rgba(108, 133, 255, 0.1);
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  scrollbar-width: thin;
  scrollbar-color: rgba(109, 140, 255, 0.28) transparent;
}

.log-list.empty {
  display: flex;
  flex-direction: column;
  justify-content: center;
  overflow: hidden;
}

.log-empty {
  padding: 8px 10px 10px;
}

.log-empty-title {
  color: #8ea2d6;
  font-size: 10.5px;
  letter-spacing: 0.08em;
}

.log-flow {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 6px 8px;
  margin-top: 8px;
}

.log-flow-step {
  display: flex;
  align-items: center;
  gap: 9px;
  padding: 7px 9px;
  border: 1px solid rgba(108, 133, 255, 0.12);
  border-radius: 7px;
  background: rgba(98, 124, 215, 0.05);
}

.flow-index {
  color: rgba(122, 156, 255, 0.55);
  font-size: 15px;
  font-weight: 800;
  font-variant-numeric: tabular-nums;
}

.flow-copy {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.flow-copy strong {
  color: #aebde3;
  font-size: 10.5px;
  font-weight: 600;
}

.flow-copy em {
  color: #5f6e92;
  font-size: 9px;
  font-style: normal;
}

.log-empty-note {
  margin: 9px 0 0;
  color: #566488;
  font-size: 9.5px;
}

.log-row {
  display: flex;
  align-items: center;
  gap: 8px;
  min-height: 23px;
  color: #8896b8;
  font-size: 10.2px;
}

.log-dot {
  width: 5px;
  height: 5px;
  border-radius: 50%;
  background: #40c582;
  box-shadow: 0 0 6px rgba(64, 197, 130, 0.55);
}

.log-dot.error { background: #f87171; box-shadow: 0 0 6px rgba(248, 113, 113, 0.5); }
.log-dot.warn { background: #f5b945; }

.log-time {
  width: 58px;
  color: #5e6d91;
  font-size: 10px;
}

.log-message {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.stage-strip {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 5px;
  margin-top: 6px;
  padding-top: 6px;
  border-top: 1px solid rgba(108, 133, 255, 0.1);
}

.stage-cell {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 4px;
  min-width: 0;
  padding: 4px 7px;
  border: 1px solid rgba(108, 133, 255, 0.1);
  border-radius: 6px;
  background: rgba(98, 124, 215, 0.06);
}

.stage-cell span {
  color: #7483a8;
  font-size: 9px;
  white-space: nowrap;
}

.stage-cell strong {
  color: #a9c6ff;
  font-size: 12px;
  font-weight: 700;
  font-variant-numeric: tabular-nums;
}

.overview-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding-top: 1px;
  color: #647292;
  font-size: 10px;
}

.online-dot {
  display: inline-block;
  width: 5px;
  height: 5px;
  margin-right: 5px;
  border-radius: 50%;
  background: #37d27c;
  box-shadow: 0 0 6px rgba(55, 210, 124, 0.7);
}

.footer-note {
  color: #4f5d7e;
}

@media (max-width: 1560px) {
  .hero-quote { gap: 12px; margin-right: 62px; font-size: 15px; }
  .hero-earth-img { right: 90px; width: 520px; }
}

@media (max-width: 1400px) {
  .hero-quote { display: none; }
  .hero-earth-img { right: 0; width: 460px; opacity: 0.42; }
  .region-grid { grid-template-columns: repeat(8, minmax(0, 1fr)); }
  .charts-grid { grid-template-columns: repeat(3, minmax(0, 1fr)); }
  .charts-grid > .chart-card:last-child { grid-column: auto; }
}

@media (max-width: 1400px), (max-height: 860px) {
  .overview-page { gap: 6px; padding: 8px 14px 5px; }
  .hero { min-height: 88px; padding: 8px 22px; }
  .hero h1 { font-size: 25px; }
  .hero p { margin-top: 2px; }
  .hero-badges { margin-top: 4px; }
  .hero-icon { width: 48px; height: 48px; }
  .hero-sakura { width: 270px; height: 108px; top: -26px; }
  .block-head { margin-bottom: 6px; }
  .section-title { font-size: 13.5px; }
  .block-meta { font-size: 9.5px; }
  .region-card { height: 44px; padding: 0 7px; gap: 6px; }
  .region-copy strong { font-size: 10px; }
  .region-copy em { display: none; }
  .region-check { width: 16px; height: 16px; }
  .all-region .region-globe { width: 23px; height: 23px; }
  .param-bar { gap: 8px; padding: 4px 10px; }
  .param-title { gap: 5px; padding-right: 9px; font-size: 11px; }
  .param-items { flex-wrap: nowrap; gap: 8px; }
  .param-item { gap: 4px; font-size: 9.5px; }
  .param-item select { height: 27px; min-width: 62px; padding: 0 5px; font-size: 10px; }
  .source-inline { display: none; }
  .smart-pill { height: 25px; padding: 0 6px; font-size: 9px; }
  .param-note { font-size: 9px; }
  .start-btn { height: 34px; padding: 0 18px; font-size: 12px; }
  .stats-grid { gap: 6px; }
  .stat-card { padding: 7px 9px 5px; }
  .stat-icon { width: 26px; height: 26px; }
  .stat-label { font-size: 10.5px; }
  .stat-value { font-size: 20px; }
  .chart-card { padding: 8px 9px 2px; }
  .logs-card { min-height: 148px; padding: 8px 10px 5px; }
  .log-row { height: 19px; }
  .summary-card { min-height: 148px; padding: 9px 10px 7px; }
  .summary-icon { width: 32px; height: 32px; }
  .summary-title { font-size: 13px; }
  .summary-progress { margin-top: 7px; }
  .coverage-list { margin-top: 4px; }
  .summary-actions { padding-top: 8px; }
  .overview-footer { font-size: 9px; }
}

@media (max-width: 1199px) {
  .overview-page { padding-left: 14px; padding-right: 14px; }
  .hero { padding: 12px 20px; }
  .hero-en { display: none; }
  .dashboard-grid { grid-template-columns: 1fr; }
  .charts-grid { grid-template-columns: repeat(3, minmax(0, 1fr)); }
  .charts-grid > .chart-card:last-child { grid-column: auto; }
  .bottom-grid { grid-template-columns: 1fr; }
  .summary-card { min-height: 0; }
}

@media (max-width: 1320px) {
  .param-note { display: none; }
  .param-items { gap: 7px; }
  .param-item select { min-width: 58px; }
}

@media (max-height: 820px) {
  .coverage-chips { gap: 3px 5px; }
  .coverage-chip { height: 24px; font-size: 10px; }
}

@media (max-width: 1080px) {
  .region-grid, .stats-grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }
  .param-bar { flex-wrap: wrap; }
  .charts-grid { grid-template-columns: 1fr; }
  .charts-grid > .chart-card:last-child { grid-column: auto; }
}
</style>






