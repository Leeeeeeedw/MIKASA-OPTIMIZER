<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import { useMessage } from 'naive-ui'
import { open } from '@tauri-apps/plugin-dialog'
import {
  Check,
  Download,
  FileCode2,
  FileText,
  Filter,
  FolderOpen,
  Gauge,
  Layers,
  PackageOpen,
  RefreshCw,
  SlidersHorizontal,
} from '@lucide/vue'
import { api, isTauri } from '../api'
import { state } from '../store'
import type { ExportOutcome, ScanResult } from '../types'
import RegionFlag from '../components/RegionFlag.vue'

const message = useMessage()
const format = ref<'txt' | 'csv'>('txt')
const regionChoice = ref<string>('all')
const outputDir = ref(state.settings.exportDir || '')
const minSpeed = ref(0)
const maxLatency = ref(0)
const includeCity = ref(true)
const includeSpeed = ref(true)
const includeLatency = ref(false)
const includeColo = ref(false)
const includeSource = ref(false)
const busy = ref(false)
const outcome = ref<ExportOutcome | null>(null)
const rows = ref<ScanResult[]>([])
const loadingRows = ref(false)

const speedPresets = [0, 1, 3, 5, 10]
const latencyPresets = [0, 300, 500, 800, 1000]

const selectedRegion = computed(() => state.regions.find((region) => region.key === regionChoice.value))
const selectedName = computed(() => {
  if (regionChoice.value === 'all') return '全部地区'
  if (regionChoice.value === 'global') return '全球其他地区'
  if (regionChoice.value === 'unknown') return '未知地区'
  return selectedRegion.value?.name || '全部地区'
})
const normalizedMinSpeed = computed(() => Math.max(0, Number(minSpeed.value) || 0))
const normalizedMaxLatency = computed(() => Math.max(0, Number(maxLatency.value) || 0))

function numberLabel(value: number) {
  return Number.isInteger(value) ? String(value) : value.toFixed(1).replace(/\.0$/, '')
}

const filterSuffix = computed(() => {
  const parts: string[] = []
  if (normalizedMinSpeed.value > 0) parts.push(`${numberLabel(normalizedMinSpeed.value)}MBs以上`)
  if (normalizedMaxLatency.value > 0) parts.push(`${numberLabel(normalizedMaxLatency.value)}ms内`)
  return parts.length ? `_${parts.join('_')}` : ''
})

const fileName = computed(() => `${selectedName.value}${filterSuffix.value}.${format.value}`)
const filterSummary = computed(() => {
  const parts: string[] = []
  if (normalizedMinSpeed.value > 0) parts.push(`速度 ≥ ${numberLabel(normalizedMinSpeed.value)} MB/s`)
  if (normalizedMaxLatency.value > 0) parts.push(`延迟 ≤ ${numberLabel(normalizedMaxLatency.value)} ms`)
  return parts.length ? parts.join(' · ') : '无额外阈值'
})

const passedRows = computed(() => rows.value.filter((row) => row.status === 'passed'))
function matchesRegion(row: ScanResult) {
  if (regionChoice.value === 'all') return true
  if (regionChoice.value === 'global') {
    return row.region !== 'unknown' && !state.regions.some((region) => region.key === row.region)
  }
  if (regionChoice.value === 'unknown') return row.region === 'unknown'
  return row.region === regionChoice.value
}
const eligibleRows = computed(() =>
  passedRows.value.filter((row) => {
    if (!matchesRegion(row)) return false
    if (normalizedMinSpeed.value > 0 && row.speedMbS < normalizedMinSpeed.value) return false
    if (normalizedMaxLatency.value > 0 && row.latency > normalizedMaxLatency.value) return false
    return true
  }),
)

function formatLine(row: ScanResult) {
  let line = `${row.ip}:${row.port}#三笠优选`
  if (includeCity.value && row.regionName) line += ` | ${row.regionName}`
  if (includeSpeed.value) line += ` | 下载 ${row.speedMbS.toFixed(2)}MB/s`
  if (includeLatency.value) line += ` | 延迟 ${row.latency.toFixed(0)}ms`
  if (includeColo.value && row.colo) line += ` | ${row.colo}`
  if (includeSource.value && row.source) line += ` | ${row.source}`
  return line
}

const previewRows = computed(() => eligibleRows.value.slice(0, 6))
const previewLines = computed(() => previewRows.value.map(formatLine))
const fileNote = computed(() =>
  eligibleRows.value.length
    ? `当前筛选后 ${eligibleRows.value.length} 个完整通过节点 · ${filterSummary.value}`
    : `当前筛选没有可导出的节点 · ${filterSummary.value}`,
)

watch(
  [regionChoice, format, outputDir, minSpeed, maxLatency, includeCity, includeSpeed, includeLatency, includeColo, includeSource],
  () => {
    outcome.value = null
  },
)

async function loadRows() {
  if (!isTauri()) return
  loadingRows.value = true
  try {
    rows.value = await api.getResults(state.selectedScanId || state.dashboard.latestScanId || undefined, undefined)
  } catch {
    rows.value = []
  } finally {
    loadingRows.value = false
  }
}

watch(() => state.selectedScanId, () => void loadRows())
onMounted(loadRows)

async function chooseDir() {
  if (!isTauri()) {
    message.warning('目录选择仅在桌面应用中可用')
    return
  }
  const selected = await open({ directory: true, multiple: false })
  if (selected && !Array.isArray(selected)) outputDir.value = selected
}

async function exportNow() {
  busy.value = true
  try {
    outcome.value = await api.exportResults({
      scanId: state.selectedScanId || state.dashboard.latestScanId || undefined,
      directory: outputDir.value || undefined,
      format: format.value,
      region: regionChoice.value,
      minSpeedMbS: normalizedMinSpeed.value,
      maxLatencyMs: normalizedMaxLatency.value,
      includeSpeed: includeSpeed.value,
      includeLatency: includeLatency.value,
      includeCity: includeCity.value,
      includeColo: includeColo.value,
      includeSource: includeSource.value,
    })
    message.success(`已导出 ${outcome.value.count} 个节点到 ${outcome.value.directory}`)
  } catch (error) {
    message.error(String(error))
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <div class="page export-page">
    <header class="page-header">
      <div>
        <h2>结果导出</h2>
        <p>只导出完整测速通过的真实结果；按速度、延迟筛选，并自由组合客户端备注字段。</p>
      </div>
      <div class="header-actions">
        <button class="btn-ghost" @click="state.activeView = 'rankings'">查看排名</button>
        <button class="btn-primary" :disabled="busy" @click="exportNow"><Download :size="14" />立即导出</button>
      </div>
    </header>

    <section class="export-grid">
      <div class="glass-card panel settings-panel">
        <div class="panel-head"><PackageOpen :size="16" /><span>导出设置</span><span class="panel-note">筛选与后缀可自由组合</span></div>

        <div class="setting-block">
          <label class="field-label">输出目录</label>
          <div class="dir-picker">
            <input v-model="outputDir" placeholder="默认使用文档目录下的 三笠优选导出" />
            <button class="btn-ghost" @click="chooseDir"><FolderOpen :size="13" />选择</button>
          </div>
        </div>

        <div class="setting-block">
          <label class="field-label">导出范围</label>
          <div class="region-choice">
            <button :class="{ active: regionChoice === 'all' }" @click="regionChoice = 'all'">
              <Layers :size="15" />
              <span><strong>全部地区</strong><em>合并为一个文件</em></span>
              <Check v-if="regionChoice === 'all'" :size="13" class="choice-check" />
            </button>
            <button
              v-for="region in state.regions"
              :key="region.key"
              :class="{ active: regionChoice === region.key }"
              @click="regionChoice = region.key"
            >
              <RegionFlag :code="region.key" :width="16" />
              <span><strong>{{ region.name }}</strong><em>仅导出该地区</em></span>
              <Check v-if="regionChoice === region.key" :size="13" class="choice-check" />
            </button>
            <button :class="{ active: regionChoice === 'global' }" @click="regionChoice = 'global'">
              <RegionFlag code="global" :width="16" />
              <span><strong>全球其他</strong><em>非七地区达标节点</em></span>
              <Check v-if="regionChoice === 'global'" :size="13" class="choice-check" />
            </button>
            <button :class="{ active: regionChoice === 'unknown' }" @click="regionChoice = 'unknown'">
              <RegionFlag code="unknown" :width="16" />
              <span><strong>未知地区</strong><em>colo 未识别的节点</em></span>
              <Check v-if="regionChoice === 'unknown'" :size="13" class="choice-check" />
            </button>
          </div>
        </div>

        <div class="setting-block">
          <label class="field-label">文件格式</label>
          <div class="format-choice">
            <button :class="{ active: format === 'txt' }" @click="format = 'txt'"><FileText :size="16" /><span><strong>TXT</strong><em>行前缀 + 可选备注，直接粘贴客户端</em></span></button>
            <button :class="{ active: format === 'csv' }" @click="format = 'csv'"><FileCode2 :size="16" /><span><strong>CSV</strong><em>始终包含完整原始指标，便于分析</em></span></button>
          </div>
        </div>

        <div class="setting-block">
          <div class="block-label-row">
            <label class="field-label"><Filter :size="12" />结果筛选</label>
            <span>{{ filterSummary }}</span>
          </div>
          <div class="filter-grid">
            <div class="filter-block">
              <span class="filter-title">最低下载速度</span>
              <div class="preset-row">
                <button
                  v-for="preset in speedPresets"
                  :key="`speed-${preset}`"
                  :class="{ active: normalizedMinSpeed === preset }"
                  @click="minSpeed = preset"
                >{{ preset === 0 ? '不限' : `${preset}+` }}</button>
              </div>
              <label class="number-input">
                <input v-model.number="minSpeed" type="number" min="0" step="0.5" />
                <span>MB/s</span>
              </label>
            </div>
            <div class="filter-block">
              <span class="filter-title">最高 TCP 延迟</span>
              <div class="preset-row">
                <button
                  v-for="preset in latencyPresets"
                  :key="`latency-${preset}`"
                  :class="{ active: normalizedMaxLatency === preset }"
                  @click="maxLatency = preset"
                >{{ preset === 0 ? '不限' : `${preset}ms` }}</button>
              </div>
              <label class="number-input">
                <input v-model.number="maxLatency" type="number" min="0" step="10" />
                <span>ms</span>
              </label>
            </div>
          </div>
        </div>

        <div class="setting-block">
          <div class="block-label-row">
            <label class="field-label"><SlidersHorizontal :size="12" />TXT 备注后缀</label>
            <span>按顺序拼接，CSV 不受影响</span>
          </div>
          <div class="suffix-grid">
            <button class="suffix-toggle" :class="{ active: includeCity }" @click="includeCity = !includeCity">
              <span class="toggle-check"><Check v-if="includeCity" :size="10" /></span>
              <span class="toggle-copy"><strong>城市 / 国家</strong><em>日本东京 · 美国洛杉矶</em></span>
            </button>
            <button class="suffix-toggle" :class="{ active: includeSpeed }" @click="includeSpeed = !includeSpeed">
              <span class="toggle-check"><Check v-if="includeSpeed" :size="10" /></span>
              <span class="toggle-copy"><strong>下载速度</strong><em>下载 18.35MB/s</em></span>
            </button>
            <button class="suffix-toggle" :class="{ active: includeLatency }" @click="includeLatency = !includeLatency">
              <span class="toggle-check"><Check v-if="includeLatency" :size="10" /></span>
              <span class="toggle-copy"><strong>TCP 延迟</strong><em>延迟 42ms</em></span>
            </button>
            <button class="suffix-toggle" :class="{ active: includeColo }" @click="includeColo = !includeColo">
              <span class="toggle-check"><Check v-if="includeColo" :size="10" /></span>
              <span class="toggle-copy"><strong>colo 代码</strong><em>NRT · LAX · HKG</em></span>
            </button>
            <button class="suffix-toggle" :class="{ active: includeSource }" @click="includeSource = !includeSource">
              <span class="toggle-check"><Check v-if="includeSource" :size="10" /></span>
              <span class="toggle-copy"><strong>来源名称</strong><em>CM IP 库 / GitHub</em></span>
            </button>
          </div>
          <p class="hint">全部关闭时仅导出 <code>IP:端口#三笠优选</code>；关闭城市后即使筛选日本也不会显示东京/大阪。</p>
        </div>

        <div class="setting-block preview-format-block">
          <label class="field-label">行格式预览</label>
          <code class="format-code">{{ previewLines[0] || '等待真实扫描结果…' }}</code>
        </div>

        <button class="btn-primary export-button" :disabled="busy" @click="exportNow">
          <Download :size="15" />{{ busy ? '正在导出…' : `导出${selectedName}（1 个文件）` }}
        </button>
      </div>

      <div class="glass-card panel preview-panel">
        <div class="panel-head"><Gauge :size="16" /><span>导出预览</span><span class="panel-note">{{ loadingRows ? '匹配数据库…' : '全部来自真实扫描结果' }}</span></div>

        <div class="preview-stats">
          <div><span>完整通过</span><strong>{{ passedRows.length }}</strong><em>条</em></div>
          <div><span>筛选后保留</span><strong class="accent">{{ eligibleRows.length }}</strong><em>条</em></div>
          <div><span>输出文件</span><strong>1</strong><em>个</em></div>
        </div>

        <div class="file-item all">
          <span class="file-icon"><FileText v-if="format === 'txt'" :size="15" /><FileCode2 v-else :size="15" /></span>
          <div><strong>{{ fileName }}</strong><span>{{ fileNote }}</span></div>
          <Check :size="14" class="file-check" />
        </div>

        <div class="preview-head">
          <span>内容预览</span>
          <em>{{ eligibleRows.length > 6 ? `显示前 6 条 / 共 ${eligibleRows.length} 条` : `共 ${eligibleRows.length} 条` }}</em>
        </div>
        <div v-if="previewRows.length" class="preview-table">
          <div class="preview-row preview-head-row">
            <span>IP:端口</span><span>国家 / 城市</span><span>延迟</span><span>下载</span>
          </div>
          <div v-for="row in previewRows" :key="row.id" class="preview-row">
            <span class="preview-ip">{{ row.ip }}:{{ row.port }}</span>
            <span class="preview-region">{{ row.regionName }}</span>
            <span class="preview-latency">{{ row.latency.toFixed(0) }} ms</span>
            <span class="preview-speed">{{ row.speedMbS.toFixed(2) }} MB/s</span>
          </div>
        </div>
        <pre v-if="previewLines.length" class="preview-code">{{ previewLines.join('\n') }}</pre>
        <div v-else class="preview-empty">
          <Filter :size="24" />
          <strong>{{ loadingRows ? '正在读取当前扫描结果' : '当前筛选没有符合条件的 IP' }}</strong>
          <span>降低最低速度或提高延迟上限后重试；未完成下载测速的参考结果不会被导出。</span>
        </div>

        <div class="export-tip">
          <strong>单文件 · 真实筛选</strong>
          <span>选择「全部地区」时只生成一个 {{ fileName }}。低于速度阈值或超过延迟上限的 IP 会在导出前被排除，文件名会带上筛选条件，便于区分多次导出。</span>
        </div>
        <div class="rule-list">
          <span><i></i>只导出 status = passed 的完整通过节点</span>
          <span><i></i>CSV 始终输出国家/城市、colo、延迟、MB/s、Mbps、丢包、稳定性、来源与时间</span>
          <span><i></i>筛选阈值只影响本次导出，不修改数据库中的原始测速结果</span>
        </div>
      </div>
    </section>

    <section v-if="outcome" class="glass-card outcome-card">
      <div class="outcome-icon"><Check :size="23" /></div>
      <div>
        <strong>导出完成 · {{ outcome.count }} 个节点</strong>
        <span>{{ outcome.directory }}</span>
        <div class="outcome-files"><span v-for="file in outcome.files" :key="file">{{ file }}</span></div>
      </div>
      <button class="btn-ghost" @click="outcome = null"><RefreshCw :size="13" />继续导出</button>
    </section>
  </div>
</template>

<style scoped>
.export-page { gap: 14px; }
.header-actions { display: flex; gap: 8px; }
.export-grid { display: grid; grid-template-columns: minmax(0, 1.04fr) minmax(0, .96fr); gap: 14px; min-width: 0; align-items: start; }
.export-grid > * { min-width: 0; }
.panel { padding: 17px; }
.settings-panel, .preview-panel { display: flex; flex-direction: column; }
.panel-head { display: flex; align-items: center; gap: 8px; color: var(--text-0); font-size: 12px; font-weight: 700; margin-bottom: 15px; }
.panel-head svg { color: #8ea9ff; }
.panel-note { margin-left: auto; color: var(--text-3); font-size: 9px; font-weight: 400; }
.setting-block { margin-bottom: 16px; }
.field-label { display: flex; align-items: center; gap: 5px; margin-bottom: 7px; color: var(--text-2); font-size: 10px; }
.block-label-row { display: flex; align-items: center; justify-content: space-between; gap: 10px; margin-bottom: 7px; }
.block-label-row .field-label { margin-bottom: 0; }
.block-label-row > span { color: var(--text-3); font-size: 8.5px; }
.dir-picker { display: flex; gap: 7px; }
.dir-picker input { flex: 1; height: 33px; padding: 0 10px; border: 1px solid var(--line); border-radius: 7px; outline: none; background: rgba(5, 10, 23, 0.7); color: var(--text-1); font-size: 10px; }
.region-choice { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 6px; }
.region-choice button { position: relative; display: flex; align-items: center; gap: 7px; min-height: 44px; padding: 7px 8px; border: 1px solid var(--line); border-radius: 8px; background: rgba(9, 17, 34, 0.55); color: var(--text-2); text-align: left; cursor: pointer; transition: border-color .15s, background .15s; }
.region-choice button:hover { border-color: rgba(109, 140, 255, 0.34); background: rgba(35, 58, 118, 0.2); }
.region-choice button.active { border-color: rgba(109, 140, 255, 0.56); background: rgba(61, 91, 203, 0.2); color: var(--text-0); box-shadow: inset 0 0 0 1px rgba(128, 155, 255, 0.12); }
.region-choice button span { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
.region-choice button strong { font-size: 10px; font-weight: 600; white-space: nowrap; }
.region-choice button em { color: var(--text-3); font-size: 8px; font-style: normal; white-space: nowrap; }
.choice-check { margin-left: auto; color: #7ea4ff; flex-shrink: 0; }
.format-choice { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; }
.format-choice button { display: flex; align-items: center; gap: 9px; padding: 11px; border: 1px solid var(--line); border-radius: 8px; background: rgba(9, 17, 34, 0.55); color: var(--text-2); text-align: left; cursor: pointer; }
.format-choice button.active { border-color: rgba(109, 140, 255, 0.5); background: rgba(61, 91, 203, 0.18); color: var(--text-0); }
.format-choice button span { display: flex; flex-direction: column; gap: 3px; }
.format-choice strong { font-size: 11px; }
.format-choice em { color: var(--text-3); font-size: 8.5px; font-style: normal; }
.filter-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; }
.filter-block { padding: 10px; border: 1px solid var(--line); border-radius: 9px; background: rgba(8, 15, 30, 0.4); }
.filter-title { display: block; color: var(--text-2); font-size: 9.5px; margin-bottom: 8px; }
.preset-row { display: flex; flex-wrap: wrap; gap: 4px; }
.preset-row button { min-width: 38px; height: 24px; padding: 0 7px; border: 1px solid rgba(109, 134, 198, 0.16); border-radius: 5px; background: rgba(16, 28, 55, 0.55); color: var(--text-2); font-size: 8.5px; cursor: pointer; }
.preset-row button:hover { border-color: rgba(109, 140, 255, 0.4); color: var(--text-0); }
.preset-row button.active { border-color: rgba(109, 140, 255, 0.58); background: rgba(70, 99, 213, 0.24); color: #a9c0ff; }
.number-input { display: flex; align-items: center; gap: 6px; margin-top: 8px; }
.number-input input { width: 100%; height: 28px; padding: 0 8px; border: 1px solid var(--line); border-radius: 6px; outline: none; background: rgba(4, 9, 20, 0.55); color: var(--text-1); font-size: 10px; }
.number-input span { color: var(--text-3); font-size: 9px; white-space: nowrap; }
.suffix-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 6px; }
.suffix-toggle { display: flex; align-items: center; gap: 8px; min-height: 38px; padding: 7px 9px; border: 1px solid var(--line); border-radius: 8px; background: rgba(9, 17, 34, 0.5); color: var(--text-2); text-align: left; cursor: pointer; }
.suffix-toggle:hover { border-color: rgba(109, 140, 255, 0.36); }
.suffix-toggle.active { border-color: rgba(109, 140, 255, 0.46); background: rgba(61, 91, 203, 0.16); color: var(--text-1); }
.toggle-check { display: grid; place-items: center; width: 16px; height: 16px; border: 1px solid rgba(109, 134, 198, 0.35); border-radius: 4px; color: #fff; flex-shrink: 0; }
.suffix-toggle.active .toggle-check { border-color: #6d8cff; background: linear-gradient(135deg, #6d8cff, #9b5cff); }
.toggle-copy { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
.toggle-copy strong { font-size: 9.5px; font-weight: 600; white-space: nowrap; }
.toggle-copy em { color: var(--text-3); font-size: 8px; font-style: normal; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.format-code { display: block; padding: 10px; border: 1px solid var(--line); border-radius: 7px; background: rgba(3, 8, 18, 0.55); color: #8fa9db; font-family: var(--mono); font-size: 9.5px; line-height: 1.6; word-break: break-all; }
.hint { margin-top: 7px; color: var(--text-3); font-size: 9px; line-height: 1.6; }
.hint code { color: #8fa9db; }
.preview-format-block { margin-bottom: 13px; }
.export-button { width: 100%; height: 38px; margin-top: auto; }
.preview-stats { display: grid; grid-template-columns: repeat(3, 1fr); gap: 7px; margin-bottom: 11px; }
.preview-stats div { padding: 10px 11px; border: 1px solid var(--line); border-radius: 8px; background: rgba(8, 15, 30, 0.4); }
.preview-stats span { display: block; color: var(--text-3); font-size: 9px; }
.preview-stats strong { display: inline-block; margin-top: 4px; color: var(--text-0); font-size: 20px; line-height: 1; }
.preview-stats strong.accent { color: #6fe0a4; }
.preview-stats em { margin-left: 3px; color: var(--text-3); font-size: 8px; font-style: normal; }
.file-item { display: flex; align-items: center; gap: 9px; padding: 9px 10px; border: 1px solid var(--line); border-radius: 7px; background: rgba(8, 15, 30, 0.42); }
.file-item.all { border-color: rgba(109, 140, 255, 0.25); background: rgba(57, 82, 175, 0.12); }
.file-icon { display: grid; place-items: center; width: 28px; height: 28px; border-radius: 6px; background: rgba(91, 121, 220, 0.13); color: #89a5f1; flex-shrink: 0; }
.file-item div { display: flex; flex-direction: column; gap: 3px; flex: 1; min-width: 0; }
.file-item strong { color: var(--text-1); font-size: 10.5px; font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.file-item span { color: var(--text-3); font-size: 8.5px; }
.file-check { color: #56d591; opacity: 0.8; }
.preview-head { display: flex; align-items: center; justify-content: space-between; margin: 12px 0 6px; color: var(--text-2); font-size: 9.5px; }
.preview-head em { color: var(--text-3); font-size: 8.5px; font-style: normal; }
.preview-table { margin-bottom: 8px; overflow: hidden; border: 1px solid var(--line); border-radius: 8px; background: rgba(2, 6, 15, 0.42); }
.preview-row { display: grid; grid-template-columns: minmax(118px, 1.25fr) minmax(92px, 1fr) 58px 76px; align-items: center; min-height: 25px; padding: 0 8px; border-bottom: 1px solid rgba(108, 133, 255, 0.08); color: var(--text-2); font-size: 8.5px; }
.preview-row:last-child { border-bottom: 0; }
.preview-head-row { min-height: 22px; background: rgba(84, 112, 203, 0.08); color: var(--text-3); font-size: 8px; }
.preview-row span { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.preview-ip { color: #aec8ff; font-family: var(--mono); }
.preview-region { color: var(--text-1); }
.preview-latency { color: #73c9f0; }
.preview-speed { color: #65d89d; font-weight: 600; }
.preview-code { margin: 0; padding: 11px; min-height: 92px; max-height: 150px; overflow: auto; border: 1px solid var(--line); border-radius: 8px; background: rgba(2, 6, 15, 0.62); color: #8fa9db; font-family: var(--mono); font-size: 9px; line-height: 1.8; white-space: pre-wrap; word-break: break-all; }
.preview-empty { display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 6px; min-height: 118px; padding: 12px; border: 1px dashed var(--line-strong); border-radius: 8px; color: var(--text-3); text-align: center; }
.preview-empty svg { opacity: .5; }
.preview-empty strong { color: var(--text-2); font-size: 10.5px; }
.preview-empty span { max-width: 320px; font-size: 8.5px; line-height: 1.6; }
.export-tip { display: flex; flex-direction: column; gap: 5px; margin-top: 11px; padding: 12px; border: 1px dashed rgba(109, 140, 255, 0.2); border-radius: 8px; background: rgba(30, 48, 98, 0.12); color: var(--text-2); font-size: 9.5px; line-height: 1.6; }
.export-tip strong { color: var(--text-1); font-size: 10px; }
.rule-list { display: flex; flex-direction: column; gap: 5px; margin-top: 10px; color: var(--text-3); font-size: 8.5px; line-height: 1.5; }
.rule-list span { display: flex; gap: 6px; }
.rule-list i { width: 4px; height: 4px; margin-top: 4px; border-radius: 50%; background: #6d8cff; flex-shrink: 0; }
.outcome-card { display: flex; align-items: flex-start; gap: 12px; padding: 17px; border-color: rgba(65, 202, 135, 0.25); }
.outcome-icon { display: grid; place-items: center; width: 36px; height: 36px; border-radius: 50%; background: rgba(54, 193, 124, 0.15); color: #5fe0a0; }
.outcome-card > div:nth-child(2) { display: flex; flex-direction: column; gap: 4px; flex: 1; color: var(--text-0); font-size: 12px; }
.outcome-card span { color: var(--text-2); font-size: 9.5px; }
.outcome-files { display: flex; flex-wrap: wrap; gap: 5px; margin-top: 5px; }
.outcome-files span { padding: 2px 5px; border-radius: 4px; background: rgba(88, 117, 205, 0.12); color: #92a8df; font-size: 8.5px; }

@media (max-width: 1310px) {
  .export-grid { grid-template-columns: 1fr; }
  .export-button { margin-top: 4px; }
}

@media (max-width: 1180px) {
  .region-choice { grid-template-columns: repeat(4, minmax(0, 1fr)); }
  .suffix-grid { grid-template-columns: 1fr 1fr; }
}
</style>
