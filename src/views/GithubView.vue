<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useMessage } from 'naive-ui'
import {
  Check,
  CircleCheck,
  Copy,
  ExternalLink,
  FileCode2,
  FileText,
  Filter,
  Gauge,
  GitBranch,
  KeyRound,
  Layers,
  RefreshCw,
  Rocket,
  ShieldCheck,
  SlidersHorizontal,
  UploadCloud,
  X,
} from '@lucide/vue'
import { api, isTauri } from '../api'
import { state, refreshDashboard } from '../store'
import type { ExportOptions, GithubStatus, GithubUploadOutcome, ScanResult } from '../types'

const message = useMessage()
const repo = ref('')
const token = ref('')
const branch = ref('main')
const remoteDir = ref('mikasa-results')
const status = ref<GithubStatus>({ configured: false, masked: '', repo: '', branch: 'main', remoteDir: 'mikasa-results' })
const busy = ref(false)
const outcome = ref<GithubUploadOutcome | null>(null)
const history = ref<Array<Record<string, unknown>>>([])
const check = ref<Record<string, unknown> | null>(null)
const checkError = ref('')

const uploadOpen = ref(false)
const uploadRows = ref<ScanResult[]>([])
const loadingRows = ref(false)
const uploadMode = ref<'all' | 'split'>('all')
const uploadFormat = ref<'txt' | 'csv'>('txt')
const minSpeed = ref(0)
const maxLatency = ref(0)
const perRegionLimit = ref(0)
const includeCity = ref(true)
const includeSpeed = ref(true)
const includeLatency = ref(false)
const includeColo = ref(false)
const includeSource = ref(false)

const speedPresets = [0, 1, 3, 5, 10]
const latencyPresets = [0, 300, 500, 800, 1000]

function currentScanId() {
  return state.selectedScanId || state.dashboard.latestScanId || ''
}

const canOpenUpload = computed(() => repo.value.trim().length > 0 && status.value.configured && !busy.value)
const normalizedMinSpeed = computed(() => Math.max(0, Number(minSpeed.value) || 0))
const normalizedMaxLatency = computed(() => Math.max(0, Number(maxLatency.value) || 0))
const normalizedPerRegion = computed(() => Math.max(0, Math.floor(Number(perRegionLimit.value) || 0)))

function numberLabel(value: number) {
  return Number.isInteger(value) ? String(value) : value.toFixed(1).replace(/\.0$/, '')
}

const filterSummary = computed(() => {
  const parts: string[] = []
  if (normalizedMinSpeed.value > 0) parts.push(`速度 ≥ ${numberLabel(normalizedMinSpeed.value)} MB/s`)
  if (normalizedMaxLatency.value > 0) parts.push(`延迟 ≤ ${numberLabel(normalizedMaxLatency.value)} ms`)
  if (normalizedPerRegion.value > 0) parts.push(`每地区速度前 ${normalizedPerRegion.value} 个`)
  return parts.length ? parts.join(' · ') : '无额外阈值'
})

const passedRows = computed(() => uploadRows.value.filter((row) => row.status === 'passed'))
const eligibleRows = computed(() =>
  limitPerRegion(
    passedRows.value.filter((row) => {
      if (normalizedMinSpeed.value > 0 && row.speedMbS < normalizedMinSpeed.value) return false
      if (normalizedMaxLatency.value > 0 && row.latency > normalizedMaxLatency.value) return false
      return true
    }),
  ),
)

function fastestFirst(a: ScanResult, b: ScanResult) {
  return b.speedMbS - a.speedMbS || a.latency - b.latency || a.rank - b.rank
}

function limitPerRegion(list: ScanResult[]) {
  const sorted = [...list].sort(fastestFirst)
  const limit = normalizedPerRegion.value
  if (!limit) return sorted
  const grouped = new Map<string, ScanResult[]>()
  for (const row of sorted) {
    const group = grouped.get(row.region) || []
    group.push(row)
    grouped.set(row.region, group)
  }
  return [...grouped.values()].flatMap((group) => group.slice(0, limit)).sort(fastestFirst)
}

const regionCounts = computed<Record<string, number>>(() => {
  const counts: Record<string, number> = {}
  for (const region of state.regions) counts[region.key] = 0
  for (const row of eligibleRows.value) counts[row.region] = (counts[row.region] || 0) + 1
  return counts
})

const extraRegionCounts = computed(() => {
  let global = 0
  let unknown = 0
  for (const row of eligibleRows.value) {
    if (row.region === 'unknown') unknown += 1
    else if (!state.regions.some((region) => region.key === row.region)) global += 1
  }
  return { global, unknown }
})

const expectedFiles = computed(() => {
  if (!eligibleRows.value.length) return []
  const ext = uploadFormat.value
  if (uploadMode.value === 'split') {
    const files = state.regions
      .filter((region) => (regionCounts.value[region.key] || 0) > 0)
      .map((region) => `${region.key.toUpperCase()}/all.${ext}`)
    if (extraRegionCounts.value.global > 0) files.push(`GLOBAL/all.${ext}`)
    if (extraRegionCounts.value.unknown > 0) files.push(`UNKNOWN/all.${ext}`)
    return [...files, `all.${ext}`]
  }
  return [`all.${ext}`]
})

function formatLine(row: ScanResult) {
  let line = `${row.ip}:${row.port}#三笠优选`
  if (includeCity.value && row.regionName) line += ` | ${row.regionName}`
  if (includeSpeed.value) line += ` | 下载 ${row.speedMbS.toFixed(2)}MB/s`
  if (includeLatency.value) line += ` | 延迟 ${row.latency.toFixed(0)}ms`
  if (includeColo.value && row.colo) line += ` | ${row.colo}`
  if (includeSource.value && row.source) line += ` | ${row.source}`
  return line
}

const previewLine = computed(() => (eligibleRows.value.length ? formatLine(eligibleRows.value[0]) : '筛选后没有可上传节点'))

const uploadOptions = computed<ExportOptions>(() => ({
  scanId: currentScanId(),
  directory: '',
  format: uploadFormat.value,
  region: 'all',
  perRegionLimit: normalizedPerRegion.value,
  minSpeedMbS: normalizedMinSpeed.value,
  maxLatencyMs: normalizedMaxLatency.value,
  includeSpeed: includeSpeed.value,
  includeLatency: includeLatency.value,
  includeCity: includeCity.value,
  includeColo: includeColo.value,
  includeSource: includeSource.value,
}))

const canConfirm = computed(
  () => canOpenUpload.value && check.value?.canPush === true && eligibleRows.value.length > 0,
)

const outcomePrimaryFile = computed(() => {
  const files = outcome.value?.files || []
  return files.find((file) => file.toLowerCase() === 'all.txt' || file.toLowerCase() === 'all.csv') || files[0] || ''
})

const outcomeRawUrl = computed(() => (outcomePrimaryFile.value ? uploadedFileUrl(outcomePrimaryFile.value) : ''))

function uploadPath(file: string) {
  const dir = remoteDir.value.trim().replace(/^\/+|\/+$/g, '')
  return [...(dir ? dir.split('/') : []), ...file.split('/')].filter(Boolean).map(encodeURIComponent).join('/')
}

function displayPath(file: string) {
  const dir = remoteDir.value.trim().replace(/^\/+|\/+$/g, '')
  return [...(dir ? dir.split('/') : []), ...file.split('/')].filter(Boolean).join('/')
}

function repoSlug() {
  if (outcome.value?.repo) return outcome.value.repo
  const value = repo.value.trim().replace(/^https?:\/\/github\.com\//, '').replace(/^git@github\.com:/, '').replace(/\.git$/, '')
  return value.replace(/^\/+|\/+$/g, '')
}

function uploadedFileUrl(file: string, kind: 'raw' | 'blob' = 'raw') {
  const slug = repoSlug()
  const branchName = encodeURIComponent(branch.value.trim() || 'main')
  const path = uploadPath(file)
  return kind === 'raw'
    ? `https://raw.githubusercontent.com/${slug}/${branchName}/${path}`
    : `https://github.com/${slug}/blob/${branchName}/${path}`
}

function jsdelivrUrl(file: string) {
  const slug = repoSlug()
  const branchName = encodeURIComponent(branch.value.trim() || 'main')
  return `https://cdn.jsdelivr.net/gh/${slug}@${branchName}/${uploadPath(file)}`
}

async function openExternal(url: string) {
  try {
    await api.openExternalUrl(url)
  } catch (error) {
    message.error(String(error))
  }
}

async function copyLink(url: string, label: string) {
  try {
    await navigator.clipboard.writeText(url)
    message.success(`${label}已复制，可直接粘贴到脚本或浏览器`)
  } catch (error) {
    message.error(`复制失败：${String(error)}`)
  }
}

async function openUploadedFile(file: string, kind: 'raw' | 'blob') {
  await openExternal(uploadedFileUrl(file, kind))
}

async function load() {
  try {
    await refreshDashboard()
    status.value = await api.githubStatus()
    repo.value = status.value.repo || ''
    branch.value = status.value.branch || 'main'
    remoteDir.value = status.value.remoteDir || 'mikasa-results'
    history.value = await api.getGithubHistory()
    await loadRowsForPreview()
  } catch (error) {
    message.error(String(error))
  }
}

async function saveConfig() {
  if (!repo.value.trim()) {
    message.warning('请输入 GitHub 仓库地址')
    return
  }
  if (!token.value.trim() && !status.value.configured) {
    message.warning('请输入 GitHub Token')
    return
  }
  busy.value = true
  try {
    status.value = await api.saveGithubConfig({
      token: token.value.trim() || undefined,
      repo: repo.value.trim(),
      branch: branch.value,
      remoteDir: remoteDir.value,
    })
    token.value = ''
    check.value = null
    checkError.value = ''
    message.success('GitHub Token 已保存到 Windows 凭据管理器')
  } catch (error) {
    message.error(String(error))
  } finally {
    busy.value = false
  }
}

async function checkRepo() {
  check.value = null
  checkError.value = ''
  busy.value = true
  try {
    check.value = await api.githubCheck(repo.value.trim(), branch.value.trim() || undefined)
    message.success(check.value.canPush ? '仓库、分支和推送权限检查通过' : '仓库可访问，但当前 Token 没有推送权限')
    try {
      status.value = await api.saveGithubConfig({
        repo: repo.value.trim(),
        branch: branch.value,
        remoteDir: remoteDir.value,
      })
    } catch {
      /* 配置持久化失败不影响本次检查结果 */
    }
  } catch (error) {
    checkError.value = String(error)
    message.error(checkError.value)
  } finally {
    busy.value = false
  }
}

async function clearToken() {
  try {
    await api.clearGithubToken()
    await load()
    message.success('已清除 GitHub Token')
  } catch (error) {
    message.error(String(error))
  }
}

async function loadRowsForPreview() {
  if (!isTauri()) return
  const scanId = currentScanId()
  if (!scanId) {
    uploadRows.value = []
    return
  }
  loadingRows.value = true
  try {
    uploadRows.value = await api.getResults(scanId)
  } catch {
    uploadRows.value = []
  } finally {
    loadingRows.value = false
  }
}

async function openUploadDialog() {
  if (!repo.value.trim()) {
    message.warning('请先填写 GitHub 仓库地址')
    return
  }
  if (!status.value.configured) {
    message.warning('请先保存 GitHub Token')
    return
  }
  await refreshDashboard()
  uploadOpen.value = true
  if (!check.value) void checkRepo()
  await loadRowsForPreview()
  if (!eligibleRows.value.length) {
    message.warning('当前没有完整通过并符合筛选条件的节点，请先完成一轮优选')
  }
}

function closeUpload() {
  if (busy.value) return
  uploadOpen.value = false
}

function onKey(event: KeyboardEvent) {
  if (event.key === 'Escape' && uploadOpen.value) closeUpload()
}

watch(uploadOpen, (open) => {
  if (open) window.addEventListener('keydown', onKey)
  else window.removeEventListener('keydown', onKey)
})

onBeforeUnmount(() => window.removeEventListener('keydown', onKey))

async function performUpload() {
  if (!check.value?.canPush) {
    message.warning('请先检查仓库与权限')
    return
  }
  if (!eligibleRows.value.length) {
    message.warning('当前筛选没有可上传的节点')
    return
  }
  busy.value = true
  try {
    outcome.value = await api.githubUpload({
      repo: repo.value.trim(),
      branch: branch.value || undefined,
      remoteDir: remoteDir.value || undefined,
      scanId: currentScanId() || undefined,
      options: uploadOptions.value,
      splitRegions: uploadMode.value === 'split',
    })
    uploadOpen.value = false
    message.success(outcome.value.verified ? '上传完成，已通过 commit 校验' : '上传完成，请在 GitHub 检查结果')
    try {
      status.value = await api.saveGithubConfig({
        repo: repo.value.trim(),
        branch: branch.value,
        remoteDir: remoteDir.value,
      })
    } catch {
      /* 上传成功后配置保存失败时，仍保留本次上传结果 */
    }
    await load()
  } catch (error) {
    message.error(String(error))
  } finally {
    busy.value = false
  }
}

onMounted(load)
</script>

<template>
  <div class="page github-page">
    <header class="page-header">
      <div>
        <h2>GitHub 上传</h2>
        <p>先筛选节点，再上传为脚本可直接读取的 ASCII 文本；Token 仅保存在 Windows 凭据管理器。</p>
      </div>
      <div class="header-actions">
        <button class="btn-ghost" @click="load"><RefreshCw :size="14" />刷新状态</button>
        <button class="btn-primary" :disabled="!canOpenUpload" @click="openUploadDialog">
          <UploadCloud :size="14" />开始上传
        </button>
      </div>
    </header>

    <section class="github-grid">
      <div class="glass-card panel">
        <div class="panel-head"><GitBranch :size="16" /><span>仓库与凭据</span><span class="panel-note">不会使用旧仓库或旧 Token</span></div>
        <div class="form-column">
          <label>仓库地址<input v-model="repo" placeholder="https://github.com/owner/repo 或 git@github.com:owner/repo.git" /></label>
          <div class="two-col">
            <label>分支<input v-model="branch" placeholder="main" /></label>
            <label>远程目录<input v-model="remoteDir" placeholder="mikasa-results" /></label>
          </div>
          <label>GitHub Token<input v-model="token" type="password" placeholder="ghp_… 仅保存到系统凭据" /></label>
          <div class="status-line" :class="{ ok: status.configured }">
            <ShieldCheck :size="14" />
            {{ status.configured ? `已配置 Token ${status.masked}` : '尚未配置 GitHub Token' }}
          </div>
          <div class="button-row">
            <button class="btn-primary" :disabled="busy" @click="saveConfig"><KeyRound :size="13" />保存配置</button>
            <button class="btn-ghost" :disabled="busy || !status.configured || !repo.trim()" @click="checkRepo"><ShieldCheck :size="13" />检查仓库与权限</button>
            <button class="btn-ghost" :disabled="busy || !status.configured" @click="clearToken">清除 Token</button>
          </div>
          <div v-if="check" class="status-line" :class="{ ok: check.canPush === true }">
            <CircleCheck :size="14" />
            {{ check.repo }} · 默认分支 {{ check.defaultBranch }} · 分支 {{ check.branch }} {{ check.branchExists ? '已存在' : '将新建' }} · {{ check.canPush ? '具备推送权限' : '无推送权限' }}
          </div>
          <div v-if="checkError" class="status-line error">{{ checkError }}</div>
        </div>
      </div>

      <div class="glass-card panel">
        <div class="panel-head"><GitBranch :size="16" /><span>上传路径</span><span class="panel-note">{{ currentScanId() || '暂无扫描' }}</span></div>
        <div class="upload-summary">
          <div><span>目标仓库</span><strong>{{ repo || '尚未填写仓库地址' }}</strong></div>
          <div><span>分支 / 目录</span><strong>{{ branch }} / {{ remoteDir }}</strong></div>
          <div><span>结果来源</span><strong>{{ currentScanId() || '最新扫描结果' }}</strong></div>
          <div><span>可上传节点</span><strong>{{ loadingRows ? '读取中…' : `${eligibleRows.length} 个完整通过节点` }}</strong></div>
        </div>
        <div class="path-preview">
          <div class="path-row"><span class="path-key">合并</span><code>{{ displayPath(`all.${uploadFormat}`) }}</code></div>
          <div class="path-row"><span class="path-key">分地区</span><code>{{ displayPath('JP/all.txt') }}</code></div>
          <div class="path-row"><span class="path-key">其他地区</span><code>{{ displayPath('GLOBAL/all.txt') }}</code></div>
        </div>
        <p class="hint">
          GitHub 上传统一使用英文路径，避免中文文件名变成很长的 URL 编码。点击「开始上传」会先弹出筛选窗口，按速度、延迟和备注字段生成订阅文件。
        </p>
      </div>
    </section>

    <section v-if="outcome" class="glass-card outcome-card">
      <div class="outcome-icon"><CircleCheck :size="24" /></div>
      <div class="outcome-main">
        <strong>GitHub 上传{{ outcome.verified ? '已完成并校验' : '已提交' }}</strong>
        <span>{{ outcome.repo }} · {{ outcome.branch }} · {{ outcome.files.length }} 个文件</span>
        <code>{{ outcome.commitSha }}</code>
        <div class="outcome-links">
          <button class="btn-ghost" @click="openUploadedFile(outcomePrimaryFile, 'raw')"><FileText :size="12" />直接打开 TXT</button>
          <button class="btn-ghost" @click="copyLink(outcomeRawUrl, 'raw 直链')"><Copy :size="12" />复制脚本直链</button>
          <button class="btn-ghost" @click="copyLink(jsdelivrUrl(outcomePrimaryFile), 'jsDelivr 直链')"><Copy :size="12" />复制 CDN 直链</button>
          <button class="btn-ghost" @click="openUploadedFile(outcomePrimaryFile, 'blob')"><ExternalLink :size="12" />GitHub 页面</button>
        </div>
        <code class="raw-url">{{ outcomeRawUrl }}</code>
        <div class="outcome-files">
          <div v-for="file in outcome.files" :key="file" class="outcome-file">
            <code>{{ displayPath(file) }}</code>
            <button class="mini-link" @click="openUploadedFile(file, 'raw')">打开</button>
            <button class="mini-link" @click="copyLink(uploadedFileUrl(file), `${file} 直链`)">复制</button>
          </div>
        </div>
        <p class="private-note" :class="{ ok: check?.private === false }">
          <template v-if="check?.private === false">
            当前仓库为 Public，raw 直链和 jsDelivr 直链都可以被脚本匿名读取。
          </template>
          <template v-else>
            如果目标仓库是 Private，raw 链接需要登录或 Token，脚本无法匿名读取。
            要得到可以直接放进脚本的链接，请把目标仓库改成 Public。
          </template>
        </p>
      </div>
    </section>

    <section class="glass-card panel history-panel">
      <div class="panel-head"><Rocket :size="16" /><span>上传历史</span></div>
      <div class="history-table">
        <div class="h-row head"><span>时间</span><span>仓库</span><span>分支</span><span>提交</span><span>状态</span><span>说明</span></div>
        <div v-for="(row, index) in history" :key="index" class="h-row">
          <span>{{ row.createdAt || row.created_at || '—' }}</span>
          <span>{{ row.repo || '—' }}</span>
          <span>{{ row.branch || '—' }}</span>
          <span class="mono">{{ String(row.commitSha || row.commit_sha || '').slice(0, 12) || '—' }}</span>
          <span :class="row.status === '成功' ? 'ok-text' : 'error-text'">{{ row.status || '—' }}</span>
          <span class="history-message" :title="String(row.message || '')">{{ row.message || '—' }}</span>
        </div>
        <div v-if="!history.length" class="empty-note">暂无上传历史</div>
      </div>
    </section>

    <Teleport to="body">
      <Transition name="upload-fade">
        <div v-if="uploadOpen" class="upload-backdrop" @click.self="closeUpload">
          <section class="upload-dialog" role="dialog" aria-modal="true" aria-label="GitHub 上传设置">
            <header class="dialog-head">
              <div>
                <span class="dialog-kicker">SCRIPT SUBSCRIPTION</span>
                <h2>GitHub 上传设置</h2>
                <p>筛选本次要上传的节点，生成可直接给脚本读取的 ASCII 文本。</p>
              </div>
              <button class="dialog-close" title="关闭" @click="closeUpload"><X :size="16" /></button>
            </header>

            <div class="dialog-body">
              <section class="dialog-section">
                <div class="section-title"><Layers :size="14" /><span>上传范围</span><em>文件路径固定为英文</em></div>
                <div class="mode-grid">
                  <button :class="{ active: uploadMode === 'all' }" @click="uploadMode = 'all'">
                    <FileText :size="17" />
                    <span><strong>合并全部地区</strong><em>只生成一个 all.txt</em></span>
                    <Check v-if="uploadMode === 'all'" :size="13" class="choice-check" />
                  </button>
                  <button :class="{ active: uploadMode === 'split' }" @click="uploadMode = 'split'">
                    <GitBranch :size="17" />
                    <span><strong>按地区拆分</strong><em>all.txt + JP/all.txt …</em></span>
                    <Check v-if="uploadMode === 'split'" :size="13" class="choice-check" />
                  </button>
                </div>
              </section>

              <section class="dialog-section">
                <div class="section-title"><FileCode2 :size="14" /><span>文件格式</span><em>脚本直链使用 .txt 最稳</em></div>
                <div class="format-grid">
                  <button :class="{ active: uploadFormat === 'txt' }" @click="uploadFormat = 'txt'">
                    <FileText :size="16" /><span><strong>TXT</strong><em>IP:端口#备注，直接给客户端或脚本</em></span>
                  </button>
                  <button :class="{ active: uploadFormat === 'csv' }" @click="uploadFormat = 'csv'">
                    <FileCode2 :size="16" /><span><strong>CSV</strong><em>完整指标，便于表格分析</em></span>
                  </button>
                </div>
              </section>

              <section class="dialog-section">
                <div class="section-title"><Filter :size="14" /><span>结果筛选</span><em>{{ filterSummary }}</em></div>
                <div class="filter-grid">
                  <div class="filter-block">
                    <span class="filter-title">最低下载速度</span>
                    <div class="preset-row">
                      <button v-for="preset in speedPresets" :key="`speed-${preset}`" :class="{ active: normalizedMinSpeed === preset }" @click="minSpeed = preset">
                        {{ preset === 0 ? '不限' : `${preset}+` }}
                      </button>
                    </div>
                    <label class="number-input"><input v-model.number="minSpeed" type="number" min="0" step="0.5" /><span>MB/s</span></label>
                  </div>
                  <div class="filter-block">
                    <span class="filter-title">最高 TCP 延迟</span>
                    <div class="preset-row">
                      <button v-for="preset in latencyPresets" :key="`latency-${preset}`" :class="{ active: normalizedMaxLatency === preset }" @click="maxLatency = preset">
                        {{ preset === 0 ? '不限' : `${preset}ms` }}
                      </button>
                    </div>
                    <label class="number-input"><input v-model.number="maxLatency" type="number" min="0" step="10" /><span>ms</span></label>
                  </div>
                </div>
              </section>

              <section class="dialog-section">
                <div class="section-title"><Layers :size="14" /><span>每地区上传数量</span><em>按真实下载速度从快到慢</em></div>
                <div class="preset-row region-limit-row">
                  <button
                    v-for="preset in [5, 10, 0]"
                    :key="`region-${preset}`"
                    :class="{ active: normalizedPerRegion === preset }"
                    @click="perRegionLimit = preset"
                  >{{ preset === 0 ? '全部过测 IP' : `每地区前 ${preset} 个` }}</button>
                </div>
              </section>

              <section class="dialog-section">
                <div class="section-title"><SlidersHorizontal :size="14" /><span>备注字段</span><em>只影响 TXT，CSV 始终完整</em></div>
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
              </section>

              <section class="dialog-section preview-section">
                <div class="section-title"><Gauge :size="14" /><span>上传预览</span><em>{{ loadingRows ? '读取数据库…' : `${eligibleRows.length} 个节点` }}</em></div>
                <div class="preview-stats">
                  <div><span>完整通过</span><strong>{{ passedRows.length }}</strong><em>条</em></div>
                  <div><span>筛选保留</span><strong class="accent">{{ eligibleRows.length }}</strong><em>条</em></div>
                  <div><span>上传文件</span><strong>{{ expectedFiles.length }}</strong><em>个</em></div>
                </div>
                <div class="expected-files">
                  <div v-for="file in expectedFiles" :key="file" class="expected-file">
                    <FileText :size="13" /><code>{{ displayPath(file) }}</code>
                  </div>
                  <div v-if="!expectedFiles.length" class="empty-files">当前筛选没有可上传节点</div>
                </div>
                <pre class="line-preview">{{ previewLine }}</pre>
              </section>

              <div v-if="!check" class="permission-warning">
                <ShieldCheck :size="15" />
                <span>上传前需要先检查仓库、分支和 Token 推送权限。</span>
                <button class="btn-ghost" :disabled="busy" @click="checkRepo">立即检查</button>
              </div>
              <div v-else-if="check.canPush !== true" class="permission-warning error">
                <ShieldCheck :size="15" />
                <span>当前 Token 没有推送权限，请检查 Token 的 Contents 权限或仓库协作者权限。</span>
              </div>
              <div v-else class="permission-warning ok">
                <CircleCheck :size="15" />
                <span>权限检查通过，可以上传。</span>
              </div>
            </div>

            <footer class="dialog-foot">
              <span>{{ repo || '未填写仓库' }} · {{ branch }} / {{ remoteDir }}</span>
              <div>
                <button class="btn-ghost" :disabled="busy" @click="closeUpload">取消</button>
                <button class="btn-primary" :disabled="!canConfirm" @click="performUpload">
                  <UploadCloud :size="14" />{{ busy ? '上传中…' : `上传 ${expectedFiles.length} 个文件` }}
                </button>
              </div>
            </footer>
          </section>
        </div>
      </Transition>
    </Teleport>
  </div>
</template>

<style scoped>
.github-page { gap: 14px; }
.header-actions { display: flex; gap: 8px; }
.github-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 14px; min-width: 0; }
.github-grid > * { min-width: 0; }
.panel { padding: 17px; }
.panel-head { display: flex; align-items: center; gap: 8px; color: var(--text-0); font-size: 12px; font-weight: 700; margin-bottom: 14px; }
.panel-head svg { color: #8ea9ff; }
.panel-note { margin-left: auto; color: var(--text-3); font-size: 9px; font-weight: 400; }
.form-column { display: flex; flex-direction: column; gap: 11px; }
.form-column label { display: flex; flex-direction: column; gap: 5px; color: var(--text-2); font-size: 10px; }
.form-column input {
  height: 33px;
  padding: 0 9px;
  border: 1px solid var(--line);
  border-radius: 7px;
  outline: none;
  background: var(--panel-2);
  color: var(--text-1);
  font-size: 10px;
}
.form-column input:focus { border-color: var(--line-strong); }
.two-col { display: grid; grid-template-columns: 1fr 1fr; gap: 9px; }
.status-line { display: flex; align-items: center; gap: 6px; color: var(--text-2); font-size: 9.5px; }
.status-line.ok { color: #2f9d68; }
.status-line.error { color: #cf6f52; }
.button-row { display: flex; flex-wrap: wrap; gap: 8px; }
.upload-summary { display: flex; flex-direction: column; gap: 9px; padding: 10px 11px; border: 1px solid var(--line); border-radius: 8px; background: var(--panel-2); }
.upload-summary div { display: flex; align-items: center; justify-content: space-between; gap: 10px; }
.upload-summary span { color: var(--text-2); font-size: 9.5px; }
.upload-summary strong { color: var(--text-1); font-size: 10px; font-weight: 500; text-align: right; word-break: break-all; }
.path-preview { display: flex; flex-direction: column; gap: 6px; margin-top: 12px; }
.path-row { display: flex; align-items: center; gap: 8px; min-width: 0; }
.path-key { flex-shrink: 0; width: 38px; color: var(--text-3); font-size: 9px; }
.path-row code { overflow: hidden; color: #6f8ed8; font-family: var(--mono); font-size: 9px; text-overflow: ellipsis; white-space: nowrap; }
.hint { color: var(--text-3); font-size: 9px; line-height: 1.7; margin-top: 12px; }
.outcome-card { display: flex; align-items: flex-start; gap: 12px; padding: 15px 17px; border-color: rgba(65, 202, 135, 0.25); }
.outcome-icon { display: grid; place-items: center; width: 38px; height: 38px; flex-shrink: 0; border-radius: 50%; color: #5cdfa0; background: rgba(52, 191, 123, 0.14); }
.outcome-main { display: flex; flex-direction: column; gap: 4px; min-width: 0; flex: 1; }
.outcome-main strong { color: var(--text-0); font-size: 12px; }
.outcome-main > span { color: var(--text-2); font-size: 9.5px; }
.outcome-main > code { color: #6f8ed8; font-family: var(--mono); font-size: 9px; }
.outcome-links { display: flex; flex-wrap: wrap; gap: 7px; margin-top: 7px; }
.raw-url { display: block; max-width: min(880px, 80vw); overflow: hidden; margin-top: 2px; color: var(--text-2); text-overflow: ellipsis; white-space: nowrap; }
.outcome-files { display: flex; flex-direction: column; gap: 4px; margin-top: 9px; }
.outcome-file { display: flex; align-items: center; gap: 8px; min-width: 0; padding: 5px 7px; border: 1px solid var(--line); border-radius: 6px; background: var(--panel-2); }
.outcome-file code { flex: 1; overflow: hidden; color: var(--text-1); font-family: var(--mono); font-size: 9px; text-overflow: ellipsis; white-space: nowrap; }
.mini-link { flex-shrink: 0; border: 0; background: none; color: #6f8ed8; font-size: 8.5px; cursor: pointer; }
.mini-link:hover { color: var(--text-0); }
.private-note { margin-top: 9px; padding: 9px 10px; border: 1px dashed rgba(221, 157, 82, 0.35); border-radius: 7px; color: var(--text-2); font-size: 9px; line-height: 1.7; }
.private-note.ok { border-color: rgba(47, 157, 104, 0.36); }
.history-panel { margin-top: 0; }
.history-table { border-top: 1px solid var(--line); overflow-x: auto; }
.h-row { display: grid; grid-template-columns: minmax(96px, 1.1fr) minmax(140px, 1.6fr) minmax(56px, 0.7fr) minmax(84px, 1fr) minmax(58px, 0.7fr) minmax(160px, 1.8fr); min-width: 700px; min-height: 32px; align-items: center; border-bottom: 1px solid var(--line); color: var(--text-2); font-size: 9.5px; }
.h-row.head { min-height: 27px; color: var(--text-3); font-size: 8.5px; }
.h-row span { padding: 0 5px; min-width: 0; white-space: nowrap; }
.history-message { display: block; overflow: hidden; text-overflow: ellipsis; color: var(--text-2); }
.mono { font-family: var(--mono); }
.error-text { color: #cf6f52; }
.ok-text { color: #2f9d68; }
.empty-note { padding: 20px; color: var(--text-3); text-align: center; font-size: 9.5px; }

/* Upload settings dialog */
.upload-backdrop {
  position: fixed;
  inset: 0;
  z-index: 95;
  display: grid;
  place-items: center;
  padding: 22px;
  background: rgba(3, 7, 18, 0.68);
  backdrop-filter: blur(7px);
}
.upload-dialog {
  width: min(860px, calc(100vw - 44px));
  max-height: calc(100vh - 44px);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  border: 1px solid var(--line-strong);
  border-radius: 18px;
  background:
    radial-gradient(520px 260px at 92% 0%, rgba(155, 92, 255, 0.13), transparent 68%),
    var(--panel-solid);
  box-shadow: 0 30px 90px rgba(2, 6, 20, 0.55), inset 0 1px 0 rgba(255, 255, 255, 0.12);
}
.dialog-head { display: flex; align-items: flex-start; justify-content: space-between; gap: 16px; padding: 20px 22px 14px; border-bottom: 1px solid var(--line); }
.dialog-kicker { display: block; margin-bottom: 5px; color: #8fa9ff; font-family: var(--mono); font-size: 8px; letter-spacing: 0.18em; }
.dialog-head h2 { margin: 0; color: var(--text-0); font-size: 18px; letter-spacing: 0.04em; }
.dialog-head p { margin: 5px 0 0; color: var(--text-2); font-size: 9.5px; }
.dialog-close { display: grid; place-items: center; width: 30px; height: 30px; flex-shrink: 0; border: 1px solid var(--line); border-radius: 8px; background: transparent; color: var(--text-2); cursor: pointer; }
.dialog-close:hover { border-color: var(--line-strong); color: var(--text-0); }
.dialog-body { display: flex; flex-direction: column; gap: 10px; padding: 14px 22px; overflow-y: auto; }
.dialog-section { padding: 12px; border: 1px solid var(--line); border-radius: 11px; background: var(--panel-2); }
.section-title { display: flex; align-items: center; gap: 6px; margin-bottom: 10px; color: var(--text-1); font-size: 10px; font-weight: 650; }
.section-title svg { color: #8ea9ff; }
.section-title em { margin-left: auto; color: var(--text-3); font-size: 8.5px; font-style: normal; font-weight: 400; }
.mode-grid, .format-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; }
.mode-grid button, .format-grid button { display: flex; align-items: center; gap: 9px; min-height: 48px; padding: 9px 10px; border: 1px solid var(--line); border-radius: 8px; background: var(--panel-solid); color: var(--text-2); text-align: left; cursor: pointer; }
.mode-grid button:hover, .format-grid button:hover { border-color: rgba(109, 140, 255, 0.34); }
.mode-grid button.active, .format-grid button.active { border-color: rgba(109, 140, 255, 0.56); background: rgba(61, 91, 203, 0.14); color: var(--text-0); }
.mode-grid button > svg, .format-grid button > svg { flex-shrink: 0; color: #8ea9ff; }
.mode-grid button span, .format-grid button span { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
.mode-grid strong, .format-grid strong { font-size: 10px; font-weight: 600; white-space: nowrap; }
.mode-grid em, .format-grid em { color: var(--text-3); font-size: 8px; font-style: normal; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.choice-check { margin-left: auto; flex-shrink: 0; color: #7ea4ff; }
.filter-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; }
.filter-block { padding: 9px; border: 1px solid var(--line); border-radius: 8px; background: var(--panel-solid); }
.filter-title { display: block; margin-bottom: 8px; color: var(--text-2); font-size: 9px; }
.preset-row { display: flex; flex-wrap: wrap; gap: 4px; }
.region-limit-row button { flex: 1; height: 30px; font-size: 10px; }
.preset-row button { min-width: 38px; height: 23px; padding: 0 7px; border: 1px solid var(--line); border-radius: 5px; background: var(--panel-2); color: var(--text-2); font-size: 8.5px; cursor: pointer; }
.preset-row button:hover { border-color: rgba(109, 140, 255, 0.4); color: var(--text-0); }
.preset-row button.active { border-color: rgba(109, 140, 255, 0.58); background: rgba(70, 99, 213, 0.2); color: #6f8ed8; }
.number-input { display: flex; align-items: center; gap: 6px; margin-top: 7px; }
.number-input input { width: 100%; height: 27px; padding: 0 8px; border: 1px solid var(--line); border-radius: 6px; outline: none; background: var(--panel-2); color: var(--text-1); font-size: 10px; }
.number-input span { color: var(--text-3); font-size: 8.5px; white-space: nowrap; }
.suffix-grid { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 6px; }
.suffix-toggle { display: flex; align-items: center; gap: 7px; min-height: 37px; padding: 6px 8px; border: 1px solid var(--line); border-radius: 7px; background: var(--panel-solid); color: var(--text-2); text-align: left; cursor: pointer; }
.suffix-toggle:hover { border-color: rgba(109, 140, 255, 0.36); }
.suffix-toggle.active { border-color: rgba(109, 140, 255, 0.46); background: rgba(61, 91, 203, 0.12); color: var(--text-1); }
.toggle-check { display: grid; place-items: center; width: 15px; height: 15px; flex-shrink: 0; border: 1px solid var(--line-strong); border-radius: 4px; color: #fff; }
.suffix-toggle.active .toggle-check { border-color: #6d8cff; background: linear-gradient(135deg, #6d8cff, #9b5cff); }
.toggle-copy { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
.toggle-copy strong { font-size: 9px; font-weight: 600; white-space: nowrap; }
.toggle-copy em { color: var(--text-3); font-size: 7.5px; font-style: normal; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.preview-section { padding-bottom: 11px; }
.preview-stats { display: grid; grid-template-columns: repeat(3, 1fr); gap: 6px; margin-bottom: 8px; }
.preview-stats div { padding: 8px 9px; border: 1px solid var(--line); border-radius: 7px; background: var(--panel-solid); }
.preview-stats span { display: block; color: var(--text-3); font-size: 8.5px; }
.preview-stats strong { display: inline-block; margin-top: 3px; color: var(--text-0); font-size: 18px; line-height: 1; }
.preview-stats strong.accent { color: #2f9d68; }
.preview-stats em { margin-left: 3px; color: var(--text-3); font-size: 8px; font-style: normal; }
.expected-files { display: flex; flex-wrap: wrap; gap: 5px; max-height: 92px; overflow-y: auto; }
.expected-file { display: flex; align-items: center; gap: 5px; padding: 4px 6px; border: 1px solid var(--line); border-radius: 5px; background: var(--panel-solid); color: #6f8ed8; }
.expected-file code { font-family: var(--mono); font-size: 8.5px; }
.empty-files { padding: 9px; color: var(--text-3); font-size: 8.5px; }
.line-preview { margin: 8px 0 0; padding: 8px 9px; min-height: 30px; overflow: auto; border: 1px solid var(--line); border-radius: 6px; background: var(--bg-1); color: var(--text-2); font-family: var(--mono); font-size: 8.5px; line-height: 1.6; white-space: pre-wrap; word-break: break-all; }
.permission-warning { display: flex; align-items: center; gap: 8px; padding: 9px 11px; border: 1px dashed rgba(221, 157, 82, 0.4); border-radius: 8px; color: var(--text-2); font-size: 9px; }
.permission-warning svg { flex-shrink: 0; color: #c99445; }
.permission-warning span { flex: 1; }
.permission-warning.error { border-color: rgba(207, 111, 82, 0.45); }
.permission-warning.error svg { color: #cf6f52; }
.permission-warning.ok { border-color: rgba(47, 157, 104, 0.38); }
.permission-warning.ok svg { color: #2f9d68; }
.dialog-foot { display: flex; align-items: center; justify-content: space-between; gap: 12px; padding: 13px 22px 17px; border-top: 1px solid var(--line); }
.dialog-foot > span { overflow: hidden; color: var(--text-3); font-size: 8.5px; text-overflow: ellipsis; white-space: nowrap; }
.dialog-foot > div { display: flex; flex-shrink: 0; gap: 7px; }
.upload-fade-enter-active, .upload-fade-leave-active { transition: opacity 0.18s ease; }
.upload-fade-enter-from, .upload-fade-leave-to { opacity: 0; }

html[data-theme="light"] .upload-backdrop { background: rgba(23, 35, 61, 0.28); }
html[data-theme="light"] .upload-dialog { box-shadow: 0 24px 70px rgba(45, 66, 110, 0.22); }
html[data-theme="light"] .path-row code, html[data-theme="light"] .outcome-main > code, html[data-theme="light"] .expected-file { color: #3f62b6; }
html[data-theme="light"] .preset-row button.active { color: #334fc4; }
html[data-theme="light"] .private-note { background: rgba(255, 242, 221, 0.5); }

@media (max-width: 1310px) { .github-grid { grid-template-columns: 1fr; } }
@media (max-width: 980px) {
  .suffix-grid { grid-template-columns: 1fr 1fr; }
}
@media (max-width: 680px) {
  .filter-grid, .mode-grid, .format-grid { grid-template-columns: 1fr; }
  .dialog-foot { align-items: flex-start; flex-direction: column; }
}
</style>
