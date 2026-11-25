<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useMessage } from 'naive-ui'
import { open } from '@tauri-apps/plugin-dialog'
import {
  Link2,
  FileText,
  Globe,
  Cloud,
  UploadCloud,
  ClipboardPaste,
  RefreshCw,
  Trash2,
  Plus,
  Search,
  ShieldCheck,
  Database,
  LoaderCircle,
  CircleCheck,
  Eye,
  X,
  ListChecks,
  GitBranch,
  Boxes,
  Download,
  Sparkles,
  ExternalLink,
  Activity,
  Radio,
} from '@lucide/vue'
import { api, isTauri } from '../api'
import { state, refreshDashboard, refreshSourceStats, isScanActive } from '../store'
import RegionFlag from '../components/RegionFlag.vue'
import type { CensysStatus, CmRawPreview, CmSourceStatus, CommunitySourceInfo, FilePreview, FofaQueryPreview, FofaStatus, ImportResult, RegistryDiscovery, RegistrySource, SourceOverview, SourceQuality, UrlSource } from '../types'

const message = useMessage()
const tab = ref<'local' | 'url' | 'fofa' | 'cloudflare' | 'cm' | 'github' | 'censys' | 'community'>('local')
const busy = ref(false)
const pasteText = ref('')
const importRegion = ref('')
const preview = ref<FilePreview | null>(null)
const selectedPath = ref('')
const dragOver = ref(false)
const importResult = ref<ImportResult | null>(null)

const urlInput = ref('')
const urlInterval = ref(60)
const urlSources = ref<UrlSource[]>([])
const urlReport = ref('')

const fofaEmail = ref('')
const fofaKey = ref('')
const fofaStatus = ref<FofaStatus>({ configured: false, email: '', keyMasked: '' })
const fofaRegions = ref<string[]>(['hk', 'tw', 'jp', 'sg', 'kr', 'fr', 'us'])
const fofaMax = ref(500)
const fofaPreviewRows = ref<FofaQueryPreview[]>([])

const useIpv4 = ref(true)
const useIpv6 = ref(false)
const perCidr = ref(3)

const cmStatus = ref<CmSourceStatus | null>(null)
const cmFormat = ref<'auto' | 'txt' | 'json'>('auto')
const cmAutoUpdate = ref(true)
const cmInterval = ref(1440)
const cmRaw = ref<CmRawPreview | null>(null)
const cmRawVisible = ref(false)

const githubUrl = ref('https://raw.githubusercontent.com/XIU2/CloudflareSpeedTest/master/ip.txt')
const githubPresets = [
  { label: 'XIU2 · IPv4 CIDR', url: 'https://raw.githubusercontent.com/XIU2/CloudflareSpeedTest/master/ip.txt' },
  { label: 'XIU2 · IPv6 CIDR', url: 'https://raw.githubusercontent.com/XIU2/CloudflareSpeedTest/master/ipv6.txt' },
]

const censysToken = ref('')
const censysMax = ref(100)
const censysStatus = ref<CensysStatus>({ configured: false, tokenMasked: '', lastFetch: '', lastCount: 0, lastMessage: '' })

const communitySources = ref<CommunitySourceInfo[]>([])
const communityStatus = ref<SourceOverview[]>([])
const communityBusy = ref('')
const communityMax = ref(5000)
const communityMap = computed(() => Object.fromEntries(communityStatus.value.map((row) => [row.key, row])))

const qualityRows = ref<SourceQuality[]>([])
const registrySources = ref<RegistrySource[]>([])
const discovery = ref<RegistryDiscovery | null>(null)
const discovering = ref(false)

async function loadCommunity() {
  try {
    communitySources.value = await api.communitySourceList()
    communityStatus.value = await api.communitySourceStatus()
    qualityRows.value = await api.sourceQuality()
  } catch (error) {
    message.error(String(error))
  }
}

async function loadRegistry() {
  try {
    registrySources.value = await api.registrySources()
  } catch (error) {
    message.error(String(error))
  }
}

async function discoverRegistry() {
  discovering.value = true
  try {
    discovery.value = await api.registryDiscover()
    message.success(`上游发现完成：${discovery.value.totalSources} 个注册源，${discovery.value.accepted} 个白名单端点`)
  } catch (error) {
    message.error(String(error))
  } finally {
    discovering.value = false
  }
}

async function fetchCommunity(key: string) {
  communityBusy.value = key
  try {
    const result = await api.communityFetch(key, communityMax.value)
    setImport(result, communitySources.value.find((item) => item.key === key)?.name || '社区源')
    await loadCommunity()
  } catch (error) {
    message.error(String(error))
    await loadCommunity()
  } finally {
    communityBusy.value = ''
  }
}

async function loadCensys() {
  try {
    censysStatus.value = await api.censysStatus()
  } catch (error) {
    message.error(String(error))
  }
}

async function saveCensys() {
  if (!censysToken.value.trim()) {
    message.warning('请填写 Censys Personal Access Token')
    return
  }
  try {
    censysStatus.value = await api.saveCensysConfig(censysToken.value.trim())
    censysToken.value = ''
    message.success('Censys Token 已保存到 Windows 凭据管理器')
  } catch (error) {
    message.error(String(error))
  }
}

async function fetchCensys() {
  if (!censysStatus.value.configured) {
    message.warning('请先配置并保存 Censys Personal Access Token')
    return
  }
  busy.value = true
  try {
    const outcome = await api.censysFetch(censysMax.value)
    setImport(outcome.result, 'Censys 导入')
    message.success(outcome.message)
    await loadCensys()
  } catch (error) {
    message.error(String(error))
  } finally {
    busy.value = false
  }
}

async function clearCensys() {
  try {
    await api.clearCensysConfig()
    censysStatus.value = { configured: false, tokenMasked: '', lastFetch: '', lastCount: 0, lastMessage: '' }
    message.success('Censys Token 已从 Windows 凭据管理器删除')
  } catch (error) {
    message.error(String(error))
  }
}

async function loadCm() {
  try {
    cmStatus.value = await api.cmStatus()
    cmAutoUpdate.value = cmStatus.value.autoUpdate
    cmInterval.value = cmStatus.value.intervalMinutes
  } catch (error) {
    message.error(String(error))
  }
}

async function fetchCm(format: 'auto' | 'txt' | 'json' = cmFormat.value) {
  cmFormat.value = format
  busy.value = true
  try {
    const result = await api.cmFetch(format)
    message.success(result.message)
    await loadCm()
    await refreshDashboard()
    await refreshSourceStats()
  } catch (error) {
    message.error(String(error))
    await loadCm()
  } finally {
    busy.value = false
  }
}

async function saveCmConfig() {
  try {
    cmStatus.value = await api.cmSaveConfig(cmAutoUpdate.value, cmInterval.value)
    message.success('CM IP 库自动更新设置已保存')
  } catch (error) {
    message.error(String(error))
  }
}

async function showCmRaw() {
  try {
    cmRaw.value = await api.cmPreview(cmStatus.value?.format === 'json' ? 'json' : 'txt')
    cmRawVisible.value = true
  } catch (error) {
    message.error(String(error))
  }
}

async function clearCm() {
  try {
    cmStatus.value = await api.cmClear()
    message.success('CM 来源配置与记录已清除，候选池中的其他来源不受影响')
    await refreshDashboard()
    await refreshSourceStats()
  } catch (error) {
    message.error(String(error))
  }
}

const sourceCounts = computed(() => state.sourceCounts || {})
const totalCandidates = computed(() => {
  const uniqueTotal = Number(state.dashboard?.totalCandidates || 0)
  if (uniqueTotal > 0) return uniqueTotal
  return Object.values(sourceCounts.value).reduce((a, b) => a + Number(b || 0), 0)
})

function toggleSource(key: string) {
  const i = state.enabledSources.indexOf(key)
  if (i >= 0) state.enabledSources.splice(i, 1)
  else state.enabledSources.push(key)
}

function setImport(result: ImportResult, label: string) {
  importResult.value = result
  message.success(`${label}：新增 ${result.added}，更新 ${result.updated}，候选池共 ${result.total} 条`)
  void refreshDashboard()
  void refreshSourceStats()
}

async function pickFile() {
  if (!isTauri()) {
    message.warning('文件选择仅在桌面应用中可用')
    return
  }
  const selected = await open({ multiple: false, filters: [{ name: 'IP 列表', extensions: ['txt', 'csv', 'list'] }] })
  if (!selected || Array.isArray(selected)) return
  busy.value = true
  try {
    preview.value = await api.previewFile(selected)
    selectedPath.value = selected
  } catch (error) {
    message.error(String(error))
  } finally {
    busy.value = false
  }
}

function onDrop(event: DragEvent) {
  dragOver.value = false
  const file = event.dataTransfer?.files?.[0]
  if (!file) return
  const reader = new FileReader()
  reader.onload = () => {
    preview.value = { name: file.name, content: String(reader.result || ''), bytes: file.size }
  }
  reader.readAsText(file)
}

async function importPickedFile() {
  if (!preview.value) return
  if (!isTauri()) {
    message.warning('桌面应用中才能读取本地路径')
    return
  }
  busy.value = true
  try {
    const result = selectedPath.value
      ? await api.importFile({ path: selectedPath.value, defaultPort: state.settings.defaultPort, region: importRegion.value })
      : await api.importText({ text: preview.value.content, defaultPort: state.settings.defaultPort, region: importRegion.value })
    setImport(result, '文件导入')
    preview.value = null
    selectedPath.value = ''
  } catch (error) {
    message.error(String(error))
  } finally {
    busy.value = false
  }
}

async function importPaste() {
  if (!pasteText.value.trim()) {
    message.warning('请先粘贴 IP / IP:PORT / CSV 内容')
    return
  }
  busy.value = true
  try {
    setImport(await api.importText({ text: pasteText.value, defaultPort: state.settings.defaultPort, region: importRegion.value }), '文本导入')
    pasteText.value = ''
  } catch (error) {
    message.error(String(error))
  } finally {
    busy.value = false
  }
}

function parseUrlInput() {
  const found = new Set<string>()
  for (const raw of urlInput.value.split(/[\s,，;；]+/)) {
    const value = raw.trim()
    if (!/^https?:\/\//i.test(value)) continue
    try {
      const parsed = new URL(value)
      if (parsed.protocol === 'http:' || parsed.protocol === 'https:') found.add(value)
    } catch {
      /* 忽略不完整或非法的地址 */
    }
  }
  return [...found]
}

async function saveUrlSources(urls: string[]) {
  for (const url of urls) {
    urlSources.value = await api.addUrlSource(url, urlInterval.value)
  }
}

async function importUrlText() {
  const urls = parseUrlInput()
  if (!urls.length) {
    message.warning('请输入至少一个以 http:// 或 https:// 开头的链接')
    return
  }
  busy.value = true
  try {
    await saveUrlSources(urls)
    setImport(await api.importUrls({ urls, defaultPort: state.settings.defaultPort, region: importRegion.value }), 'URL 导入')
    urlInput.value = ''
    await loadUrlSources()
    message.success(`${urls.length} 个链接已保存，下次可直接单独更新`, { duration: 5000 })
  } catch (error) {
    message.error(String(error))
  } finally {
    busy.value = false
  }
}

async function importGithub() {
  const urls = githubUrl.value.split(/\s+/).filter(Boolean)
  if (!urls.length) {
    message.warning('请输入至少一个 GitHub Raw 地址')
    return
  }
  busy.value = true
  try {
    setImport(
      await api.importUrls({ urls, defaultPort: state.settings.defaultPort, region: importRegion.value, source: 'github' }),
      'GitHub Raw 导入',
    )
  } catch (error) {
    message.error(String(error))
  } finally {
    busy.value = false
  }
}

function useGithubPreset(url: string) {
  githubUrl.value = url
}

async function loadUrlSources() {
  try {
    urlSources.value = await api.listUrlSources()
  } catch (error) {
    message.error(String(error))
  }
}

async function addUrlSource() {
  const urls = parseUrlInput()
  if (!urls.length) {
    message.warning('请输入以 http:// 或 https:// 开头的订阅链接，每行一个')
    return
  }
  busy.value = true
  try {
    await saveUrlSources(urls)
    message.success(`已保存 ${urls.length} 个订阅链接，下次打开仍会保留`)
    urlInput.value = ''
  } catch (error) {
    message.error(String(error))
  } finally {
    busy.value = false
  }
}

async function fetchUrl(id: number) {
  busy.value = true
  try {
    const result = await api.fetchUrlSource(id)
    setImport(result, '订阅拉取')
    await loadUrlSources()
  } catch (error) {
    message.error(String(error))
  } finally {
    busy.value = false
  }
}

async function removeUrl(id: number) {
  try {
    urlSources.value = await api.removeUrlSource(id)
    message.success('已删除')
  } catch (error) {
    message.error(String(error))
  }
}

async function fetchAllUrls() {
  busy.value = true
  urlReport.value = ''
  try {
    const result = await api.fetchAllUrlSources()
    if (result.failed > 0) {
      message.warning(`拉取结束：新增 ${result.added}，更新 ${result.updated}，失败 ${result.failed} 个订阅`, { duration: 6500 })
      const failures = result.messages.filter((item) => item.startsWith('[失败]'))
      if (failures.length) {
        urlReport.value = failures.join('\n')
      }
    } else {
      message.success(`全部拉取完成：新增 ${result.added}，更新 ${result.updated}`)
    }
    await loadUrlSources()
    await refreshDashboard()
  } catch (error) {
    message.error(String(error))
  } finally {
    busy.value = false
  }
}

async function loadFofa() {
  try {
    fofaStatus.value = await api.fofaStatus()
    fofaEmail.value = fofaStatus.value.email || ''
  } catch (error) {
    message.error(String(error))
  }
}

async function previewFofa() {
  if (!fofaRegions.value.length) {
    message.warning('请选择至少一个地区')
    return []
  }
  try {
    const rows = await api.fofaPreview(fofaRegions.value)
    fofaPreviewRows.value = rows
    return rows
  } catch (error) {
    message.error(String(error))
    return []
  }
}

function base64Utf8(value: string) {
  const bytes = new TextEncoder().encode(value)
  let binary = ''
  for (const byte of bytes) binary += String.fromCharCode(byte)
  return btoa(binary)
}

async function openFofaWeb() {
  if (!fofaRegions.value.length) {
    message.warning('请至少选择一个地区')
    return
  }
  const rows = fofaPreviewRows.value.length ? fofaPreviewRows.value : await previewFofa()
  const query = rows.find((row) => fofaRegions.value.includes(row.region))?.query
  if (!query) {
    message.warning('暂无可用的 FOFA 查询语句')
    return
  }
  try {
    await api.openExternalUrl(`https://fofa.info/result?qbase64=${encodeURIComponent(base64Utf8(query))}`)
  } catch (error) {
    message.error(String(error))
  }
}

async function saveFofa() {
  if (!fofaEmail.value || !fofaKey.value) {
    message.warning('请填写 FOFA 邮箱和 API Key')
    return
  }
  busy.value = true
  try {
    fofaStatus.value = await api.saveFofaConfig(fofaEmail.value.trim(), fofaKey.value.trim())
    fofaKey.value = ''
    message.success('FOFA 凭据已保存到 Windows 凭据管理器')
    await previewFofa()
  } catch (error) {
    message.error(String(error))
  } finally {
    busy.value = false
  }
}

async function runFofa() {
  if (!fofaStatus.value.configured) {
    message.warning('请先保存 FOFA API 配置')
    return
  }
  busy.value = true
  try {
    setImport(await api.fofaFetch(fofaRegions.value, fofaMax.value), 'FOFA 获取')
  } catch (error) {
    message.error(String(error))
  } finally {
    busy.value = false
  }
}

async function fetchCloudflare() {
  if (!useIpv4.value && !useIpv6.value) {
    message.warning('请至少选择一个 IP 协议版本')
    return
  }
  busy.value = true
  try {
    setImport(await api.cloudflareFetch({ ipv4: useIpv4.value, ipv6: useIpv6.value, perCidr: perCidr.value }), 'Cloudflare 官方段抽样')
  } catch (error) {
    message.error(String(error))
  } finally {
    busy.value = false
  }
}

function toggleFofaRegion(key: string) {
  const index = fofaRegions.value.indexOf(key)
  if (index >= 0) fofaRegions.value.splice(index, 1)
  else fofaRegions.value.push(key)
}

const sourceCards = [
  { key: 'local', label: '本地文件', desc: 'TXT / CSV / 粘贴文本', icon: FileText, tone: 'blue' },
  { key: 'url', label: '网络链接', desc: '远程订阅与自动更新', icon: Link2, tone: 'cyan' },
  { key: 'fofa', label: 'FOFA 接入', desc: 'API 查询，凭据本地保存', icon: Search, tone: 'green' },
  { key: 'cloudflare', label: 'Cloudflare 官方段', desc: '官方 CIDR 安全抽样', icon: Cloud, tone: 'orange' },
  { key: 'cm', label: 'CM IP 库', desc: 'TXT / JSON 双格式', icon: Database, tone: 'violet' },
  { key: 'github', label: 'GitHub Raw', desc: '公开 IP 清单直读', icon: GitBranch, tone: 'slate' },
  { key: 'censys', label: 'Censys 平台', desc: 'ASN 13335 主机查询', icon: ShieldCheck, tone: 'teal' },
  { key: 'community', label: '社区聚合', desc: '七个公开源统一去重', icon: Boxes, tone: 'pink' },
] as const

const activeSource = computed(() => sourceCards.find((item) => item.key === tab.value) || sourceCards[0])
const communitySourceKeys = computed(() =>
  communitySources.value.length
    ? communitySources.value.map((item) => item.key)
    : ['cf_best_ip', 'bestcf', 'best_cf_ips', 'ipdb', 'cloudflare_ip', 'free_nodes_bestcf', 'xiu2'],
)

function sourceCount(key: string) {
  if (key === 'cm') return cmStatus.value?.count || sourceCounts.value.cm || 0
  if (key === 'community') {
    return communitySourceKeys.value.reduce(
      (sum, item) => sum + Number(communityMap.value[item]?.count || sourceCounts.value[item] || 0),
      0,
    )
  }
  return Number(sourceCounts.value[key] || 0)
}

function sourceEnabled(key: string) {
  if (key === 'community') return communitySourceKeys.value.some((item) => state.enabledSources.includes(item))
  return state.enabledSources.includes(key)
}

function toggleSourceCard(key: string) {
  if (key === 'community') {
    const keys = communitySourceKeys.value
    const allEnabled = keys.length > 0 && keys.every((item) => state.enabledSources.includes(item))
    if (allEnabled) {
      state.enabledSources = state.enabledSources.filter((item) => !keys.includes(item) && item !== 'community')
    } else {
      for (const item of keys) {
        if (!state.enabledSources.includes(item)) state.enabledSources.push(item)
      }
    }
    return
  }
  toggleSource(key)
}

function sourceStatus(key: string) {
  const row = state.sourceStats.find((item) => item.key === key)
  if (key === 'cm') return cmStatus.value?.status || row?.status || '未连接'
  if (key === 'url') return urlSources.value.length ? `${urlSources.value.length} 个订阅` : '未添加订阅'
  if (key === 'fofa') return fofaStatus.value.configured ? '已配置' : '未配置'
  if (key === 'censys') return censysStatus.value.configured ? '已配置' : '未配置'
  if (key === 'community') return `${communitySourceKeys.value.length} 个公开源`
  if (row?.status && row.status !== '待获取') return row.status
  return sourceCount(key) > 0 ? '已导入' : '待获取'
}

function sourceLastUpdate(key: string) {
  const row = state.sourceStats.find((item) => item.key === key)
  if (key === 'cm') return cmStatus.value?.lastFetch || cmStatus.value?.dataUpdatedAt || row?.lastUpdate || '—'
  if (key === 'censys') return censysStatus.value.lastFetch || row?.lastUpdate || '—'
  if (key === 'url') {
    const latest = urlSources.value
      .map((item) => item.lastFetch)
      .filter(Boolean)
      .sort()
      .at(-1)
    return latest || row?.lastUpdate || '—'
  }
  if (key === 'community') {
    const latest = communityStatus.value
      .map((item) => item.lastUpdate)
      .filter(Boolean)
      .sort()
      .at(-1)
    return latest || '—'
  }
  return row?.lastUpdate || '—'
}

function sourceStateTone(key: string) {
  const status = sourceStatus(key)
  if (/失败|未配置|未连接/.test(status)) return 'bad'
  if (/待获取|未添加|未配置/.test(status)) return 'warn'
  return 'ok'
}

function localDateKey() {
  const now = new Date()
  return `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}-${String(now.getDate()).padStart(2, '0')}`
}

const todayAdded = computed(() => {
  const today = localDateKey()
  return state.sourceStats
    .filter((row) => String(row.lastUpdate || '').slice(0, 10) === today)
    .reduce((sum, row) => sum + Number(row.added || 0), 0)
})

const enabledSourceCount = computed(() => sourceCards.filter((item) => sourceEnabled(item.key)).length)
const sparkBars = computed(() => {
  const values = state.sourceStats.slice(0, 12).map((row) => Number(row.added || 0))
  const max = Math.max(1, ...values)
  return values.map((value) => Math.max(8, Math.round((value / max) * 100)))
})

async function refreshSourceStatus() {
  busy.value = true
  try {
    await Promise.all([
      refreshDashboard(),
      refreshSourceStats(),
      loadCommunity(),
      loadUrlSources(),
      loadCm(),
      loadCensys(),
      loadFofa(),
    ])
    message.success('已刷新来源状态')
  } catch (error) {
    message.error(String(error))
  } finally {
    busy.value = false
  }
}

type SyncTally = { sources: number; added: number; updated: number; failed: number; errors: string[] }

/**
 * 右侧“一键刷新”不是刷新界面，而是对所有已经有候选记录的数据源执行真实拉取。
 * 网络链接、社区源、CM、Cloudflare、FOFA、Censys 各自独立，单源失败不影响其他源。
 */
async function syncAllSources() {
  if (isScanActive()) {
    message.warning('优选任务正在运行，请先暂停或结束任务后再同步来源')
    return
  }
  busy.value = true
  const tally: SyncTally = { sources: 0, added: 0, updated: 0, failed: 0, errors: [] }
  try {
    await Promise.all([loadCommunity(), loadUrlSources(), loadCm(), loadCensys(), loadFofa(), refreshSourceStats()])

    const jobs: Array<Promise<void>> = []
    const countOf = (key: string) => Number(sourceCounts.value[key] || 0)

    if (urlSources.value.length) {
      const total = urlSources.value.length
      jobs.push((async () => {
        try {
          const res = await api.fetchAllUrlSources()
          tally.sources += total
          tally.added += res.added
          tally.updated += res.updated
          tally.failed += res.failed
          tally.errors.push(...res.messages.filter((item) => item.startsWith('[失败]')))
        } catch (error) {
          tally.sources += total
          tally.failed += total
          tally.errors.push(`网络链接：${String(error)}`)
        }
      })())
    }

    for (const key of communitySourceKeys.value) {
      if (Number(communityMap.value[key]?.count || countOf(key)) <= 0) continue
      jobs.push((async () => {
        try {
          const result = await api.communityFetch(key, communityMax.value)
          tally.sources += 1
          tally.added += result.added
          tally.updated += result.updated
        } catch (error) {
          tally.sources += 1
          tally.failed += 1
          tally.errors.push(`${communityMap.value[key]?.name || key}：${String(error)}`)
        }
      })())
    }

    if (Number(cmStatus.value?.count || countOf('cm')) > 0) {
      jobs.push((async () => {
        try {
          const result = await api.cmFetch('auto')
          tally.sources += 1
          tally.added += result.added
          tally.updated += result.updated
        } catch (error) {
          tally.sources += 1
          tally.failed += 1
          tally.errors.push(`CM IP 库：${String(error)}`)
        }
      })())
    }

    if (countOf('cloudflare') > 0) {
      jobs.push((async () => {
        try {
          const result = await api.cloudflareFetch({ ipv4: useIpv4.value, ipv6: useIpv6.value, perCidr: perCidr.value })
          tally.sources += 1
          tally.added += result.added
          tally.updated += result.updated
        } catch (error) {
          tally.sources += 1
          tally.failed += 1
          tally.errors.push(`Cloudflare：${String(error)}`)
        }
      })())
    }

    if (fofaStatus.value.configured && countOf('fofa') > 0) {
      jobs.push((async () => {
        try {
          const result = await api.fofaFetch(fofaRegions.value, fofaMax.value)
          tally.sources += 1
          tally.added += result.added
          tally.updated += result.updated
        } catch (error) {
          tally.sources += 1
          tally.failed += 1
          tally.errors.push(`FOFA：${String(error)}`)
        }
      })())
    }

    if (censysStatus.value.configured && countOf('censys') > 0) {
      jobs.push((async () => {
        try {
          const result = await api.censysFetch(censysMax.value)
          tally.sources += 1
          tally.added += result.result.added
          tally.updated += result.result.updated
        } catch (error) {
          tally.sources += 1
          tally.failed += 1
          tally.errors.push(`Censys：${String(error)}`)
        }
      })())
    }

    if (!jobs.length) {
      message.warning('当前没有已保存或已有候选记录的远端来源，请先添加网络链接或获取来源')
      return
    }

    await Promise.all(jobs)
    await Promise.all([
      refreshDashboard(),
      refreshSourceStats(),
      loadCommunity(),
      loadUrlSources(),
      loadCm(),
      loadCensys(),
      loadFofa(),
    ])

    const summary = `已同步 ${tally.sources} 个来源 · 新增 ${tally.added} · 更新 ${tally.updated}${tally.failed ? ` · 失败 ${tally.failed}` : ''}`
    if (tally.failed) {
      message.warning(summary, { duration: 9000 })
      if (tally.errors.length) urlReport.value = tally.errors.slice(0, 12).join('\n')
    } else {
      message.success(summary, { duration: 6000 })
    }
  } catch (error) {
    message.error(String(error))
  } finally {
    busy.value = false
  }
}

onMounted(async () => {
  await refreshDashboard()
  await loadUrlSources()
  await loadFofa()
  await previewFofa()
  await loadCm()
  await loadCensys()
  await loadCommunity()
  await loadRegistry()
})
</script>

<template>
  <div class="page sources-page">
    <header class="page-header">
      <div>
        <h2>IP 来源管理</h2>
        <p>点选来源卡配置数据源；开关只影响下一次扫描，已导入的候选不会被删除。</p>
      </div>
      <div class="header-actions">
        <div class="pool-badge">
          <Database :size="15" />
          <span>候选池 <strong>{{ totalCandidates.toLocaleString() }}</strong> 条</span>
        </div>
        <button class="btn-ghost" :disabled="busy" @click="refreshSourceStatus">
          <RefreshCw :size="13" :class="{ spin: busy }" />刷新状态
        </button>
      </div>
    </header>

    <section class="source-dashboard">
      <div class="glass-card source-selector">
        <div class="panel-head">
          <Boxes :size="16" /><span>数据源矩阵</span>
          <span class="panel-note">点选卡片切换配置 · 右侧开关控制扫描启用</span>
        </div>
        <div class="source-card-list">
          <div
            v-for="item in sourceCards"
            :key="item.key"
            class="source-card"
            :class="{ selected: tab === item.key, off: !sourceEnabled(item.key) }"
            role="button"
            tabindex="0"
            @click="tab = item.key"
            @keydown.enter="tab = item.key"
          >
            <span class="source-icon" :class="`tone-${item.tone}`"><component :is="item.icon" :size="18" /></span>
            <span class="source-card-main">
              <span class="source-card-title">
                <strong>{{ item.label }}</strong>
                <em class="source-state" :class="sourceStateTone(item.key)">{{ sourceStatus(item.key) }}</em>
              </span>
              <span class="source-card-desc">{{ item.desc }}</span>
              <span class="source-card-meta">{{ sourceCount(item.key).toLocaleString() }} 条候选 · {{ sourceLastUpdate(item.key) }}</span>
            </span>
            <span
              class="source-switch"
              :class="{ on: sourceEnabled(item.key) }"
              role="switch"
              :aria-checked="sourceEnabled(item.key)"
              tabindex="0"
              @click.stop="toggleSourceCard(item.key)"
              @keydown.enter.stop.prevent="toggleSourceCard(item.key)"
            ><i></i></span>
          </div>
        </div>
        <div class="source-selector-foot">
          <span><Radio :size="12" />{{ enabledSourceCount }} / {{ sourceCards.length }} 个来源已启用</span>
          <span>统一候选池按 IP:端口 去重</span>
        </div>
      </div>

      <aside class="source-aside">
        <div class="glass-card update-card">
          <button class="update-button" :disabled="busy" @click="syncAllSources">
            <RefreshCw :size="16" :class="{ spin: busy }" />
            <span><strong>{{ busy ? '正在同步…' : '一键刷新' }}</strong><em>真实同步所有已有候选的来源</em></span>
          </button>
          <div class="candidate-total">
            <span>候选池总数</span>
            <strong>{{ totalCandidates.toLocaleString() }}</strong>
            <em>去重后的 IP:端口 端点</em>
          </div>
          <div class="aside-metrics">
            <div>
              <span>今日新增</span>
              <strong>{{ todayAdded.toLocaleString() }}</strong>
              <div class="spark-bars">
                <i v-for="(height, index) in sparkBars" :key="index" :style="{ height: `${height}%` }"></i>
              </div>
            </div>
            <div>
              <span>启用来源</span>
              <strong>{{ enabledSourceCount }}<small> / {{ sourceCards.length }}</small></strong>
              <em>开关即时生效</em>
            </div>
          </div>
          <div v-if="importResult" class="recent-import">
            <CircleCheck :size="15" />
            <span>最近导入 · 新增 {{ importResult.added }} / 更新 {{ importResult.updated }}</span>
          </div>
        </div>

        <div class="glass-card aside-status">
          <div class="panel-head"><Activity :size="15" /><span>来源状态</span><span class="panel-note">真实记录</span></div>
          <div class="aside-status-list">
            <div v-for="row in state.sourceStats.slice(0, 7)" :key="row.key" class="aside-status-row">
              <i :class="row.status === '更新失败' ? 'bad' : row.status === '待获取' ? 'warn' : 'ok'"></i>
              <span>{{ row.name }}</span>
              <em>{{ row.count }}</em>
            </div>
            <div v-if="!state.sourceStats.length" class="empty-note">尚无来源获取记录</div>
          </div>
        </div>
      </aside>
    </section>

    <div class="config-heading">
      <span class="config-heading-icon"><component :is="activeSource.icon" :size="16" /></span>
      <div><strong>{{ activeSource.label }} · 配置</strong><em>当前卡片对应的真实接口与导入设置</em></div>
      <span class="config-heading-state" :class="sourceStateTone(activeSource.key)">{{ sourceStatus(activeSource.key) }}</span>
    </div>

    <details class="glass-card source-stat-card">
      <summary><ListChecks :size="16" /><span>来源真实统计</span><em>展开查看每个来源的获取、新增、重复与错误明细</em></summary>
      <table class="source-stat-table">
        <thead><tr><th>来源</th><th>当前候选</th><th>获取数量</th><th>新增</th><th>重复</th><th>无效</th><th>最后更新</th><th>状态</th></tr></thead>
        <tbody>
          <tr v-for="row in state.sourceStats" :key="row.key">
            <td>{{ row.name }}</td>
            <td class="num">{{ row.count }}</td>
            <td class="num">{{ row.fetched }}</td>
            <td class="num">{{ row.added }}</td>
            <td class="num">{{ row.duplicates }}</td>
            <td class="num">{{ row.invalid }}</td>
            <td>{{ row.lastUpdate || '—' }}</td>
            <td><span class="stat-pill" :class="{ ok: row.status === '正常' || row.status === '已导入' }">{{ row.status }}</span></td>
          </tr>
        </tbody>
      </table>
    </details>

    <section v-if="tab === 'local'" class="source-grid">
      <div class="glass-card panel">
        <div class="panel-head"><UploadCloud :size="16" /><span>拖拽文件或选择本地文件</span></div>
        <div
          class="drop-zone"
          :class="{ over: dragOver }"
          @dragover.prevent="dragOver = true"
          @dragleave.prevent="dragOver = false"
          @drop.prevent="onDrop"
          @click="pickFile"
        >
          <FileText :size="34" />
          <strong>TXT / CSV 文件</strong>
          <span>支持 IPv4、IPv6、IP:PORT、[IPv6]:PORT，自动识别逗号与空格分隔</span>
          <button class="btn-ghost" :disabled="busy">选择文件</button>
        </div>
        <div v-if="preview" class="file-preview">
          <div class="file-preview-head">
            <span><FileText :size="13" />{{ preview.name }} · {{ preview.bytes }} B</span>
            <button class="icon-btn" @click="preview = null"><X :size="13" /></button>
          </div>
          <pre>{{ preview.content.slice(0, 1600) }}</pre>
          <button class="btn-primary" :disabled="busy" @click="importPickedFile">导入该文件</button>
        </div>
      </div>

      <div class="glass-card panel">
        <div class="panel-head"><ClipboardPaste :size="16" /><span>粘贴文本导入</span></div>
        <textarea v-model="pasteText" class="ip-textarea" placeholder="104.16.1.1:443&#10;2606:4700::6810:1&#10;# 支持注释、CSV、空格分隔" />
        <div class="inline-row">
          <label>默认端口 <input v-model.number="state.settings.defaultPort" type="number" /></label>
          <label>初始地区
            <select v-model="importRegion">
              <option value="">不标注</option>
              <option v-for="region in state.regions" :key="region.key" :value="region.key">{{ region.name }}</option>
            </select>
          </label>
          <button class="btn-primary" :disabled="busy" @click="importPaste">导入文本</button>
        </div>
      </div>
    </section>

    <section v-if="tab === 'url'" class="source-grid">
      <div class="glass-card panel">
        <div class="panel-head"><Link2 :size="16" /><span>在线 IP 列表（TXT / CSV）</span></div>
        <textarea v-model="urlInput" class="ip-textarea small" placeholder="https://example.com/ips.txt&#10;https://example.com/cloudflare.csv&#10;每行一个链接" />
        <div class="inline-row">
          <label>自动更新间隔 <input v-model.number="urlInterval" type="number" min="5" /> 分钟</label>
          <button class="btn-ghost" :disabled="busy" @click="addUrlSource"><Plus :size="13" />添加订阅</button>
          <button class="btn-primary" :disabled="busy" @click="importUrlText">立即导入链接</button>
        </div>
        <p class="hint">链接会自动保存到本地数据库，下次打开仍会出现在下方，可单独点击「更新」拉取最新 IP。</p>
        <div class="source-list">
          <div v-for="item in urlSources" :key="item.id" class="source-row">
            <div class="source-row-main">
              <span class="url-text">{{ item.url }}</span>
              <span class="muted">每 {{ item.intervalMinutes }} 分钟 · 最近 {{ item.lastCount || 0 }} 条 · {{ item.lastStatus || '待拉取' }}</span>
            </div>
            <div class="row-actions">
              <button class="row-update" title="立即拉取该链接的最新 IP" :disabled="busy" @click="fetchUrl(item.id)">
                <RefreshCw :size="12" />更新
              </button>
              <button class="icon-btn danger" title="删除" @click="removeUrl(item.id)"><Trash2 :size="13" /></button>
            </div>
          </div>
          <div v-if="!urlSources.length" class="empty-note">尚未添加订阅链接</div>
        </div>
        <button class="btn-ghost" :disabled="busy || !urlSources.length" @click="fetchAllUrls">全部立即更新</button>
        <pre v-if="urlReport" class="url-report">{{ urlReport }}</pre>
      </div>
    </section>

    <section v-if="tab === 'fofa'" class="source-grid fofa-grid">
      <div class="glass-card panel">
        <div class="panel-head"><Search :size="16" /><span>FOFA API 配置</span></div>
        <div class="form-column">
          <label>FOFA 邮箱<input v-model="fofaEmail" placeholder="name@example.com" /></label>
          <label>API Key<input v-model="fofaKey" type="password" placeholder="仅保存在 Windows 凭据管理器" /></label>
          <div class="status-line">
            <ShieldCheck :size="14" />
            <span>{{ fofaStatus.configured ? `已配置 ${fofaStatus.email} · ${fofaStatus.keyMasked}` : '尚未配置 FOFA 凭据' }}</span>
          </div>
          <div class="button-row">
            <button class="btn-primary" :disabled="busy" @click="saveFofa">保存配置并生成查询</button>
            <button class="btn-ghost" :disabled="busy" @click="openFofaWeb"><ExternalLink :size="13" />打开网页搜索</button>
          </div>
          <p class="hint">免费 FOFA 账号通常可使用网页搜索，但 API 查询需要单独权限。网页搜索按钮会带上当前地区与端口的查询语句。</p>
        </div>
      </div>
      <div class="glass-card panel">
        <div class="panel-head"><Globe :size="16" /><span>地区选择与查询预览</span></div>
        <div class="region-checks">
          <button v-for="region in state.regions" :key="region.key" class="region-check" :class="{ active: fofaRegions.includes(region.key) }" @click="toggleFofaRegion(region.key)">
            <RegionFlag :code="region.key" :width="16" /> {{ region.name }}
          </button>
        </div>
        <div class="query-list">
          <div v-for="row in fofaPreviewRows" :key="row.region" class="query-row">
            <span class="query-region">{{ row.regionName }}</span>
            <code>{{ row.query }}</code>
          </div>
        </div>
        <div class="inline-row">
          <label>每地区最多 <input v-model.number="fofaMax" type="number" min="10" max="10000" /> 条</label>
          <button class="btn-primary" :disabled="busy || !fofaStatus.configured" @click="runFofa">通过 API 获取并导入</button>
        </div>
        <p class="hint">FOFA 的 country 字段只作为候选来源标签，最终地区必须经过本地 HTTPS 请求的 CF-RAY colo 验证。</p>
      </div>
    </section>

    <section v-if="tab === 'cloudflare'" class="source-grid">
      <div class="glass-card panel">
        <div class="panel-head"><Cloud :size="16" /><span>Cloudflare 官方公开 IP 段</span></div>
        <p class="hint">从 Cloudflare 官方 ips-v4 / ips-v6 清单获取 CIDR，按段随机抽样，不进行无边界扫描。</p>
        <div class="switch-list">
          <label><input v-model="useIpv4" type="checkbox" />IPv4 官方段</label>
          <label><input v-model="useIpv6" type="checkbox" />IPv6 官方段（可选）</label>
          <label>每个 CIDR 抽样 <input v-model.number="perCidr" type="number" min="1" max="100" /> 个地址</label>
        </div>
        <button class="btn-primary" :disabled="busy" @click="fetchCloudflare">
          <LoaderCircle v-if="busy" :size="14" class="spin" />
          <Cloud v-else :size="14" />
          抓取并加入候选池
        </button>
        <p class="hint">抽样只产生候选，所有候选仍需 TCP / HTTPS / 延迟 / 下载全流程测试。</p>
      </div>
    </section>

    <section v-if="tab === 'github'" class="source-grid">
      <div class="glass-card panel">
        <div class="panel-head"><GitBranch :size="16" /><span>GitHub Raw 公开订阅</span></div>
        <p class="hint">
          直接读取公开仓库的 TXT / CSV / JSON 文本清单，保留原始地区标签。CIDR 行会按每个网段取 1 个样本，避免无边界扫描。
          参考仓库仅使用公开原始文件，不复制其代码；许可证与更新时间在下方注明。
        </p>
        <div class="github-presets">
          <button v-for="preset in githubPresets" :key="preset.url" class="preset-chip" @click="useGithubPreset(preset.url)">
            <GitBranch :size="12" />{{ preset.label }}
          </button>
        </div>
        <textarea v-model="githubUrl" class="ip-textarea small" placeholder="https://raw.githubusercontent.com/owner/repo/main/ip.txt" />
        <div class="inline-row">
          <label>初始地区
            <select v-model="importRegion">
              <option value="">不标注</option>
              <option v-for="region in state.regions" :key="region.key" :value="region.key">{{ region.name }}</option>
            </select>
          </label>
          <button class="btn-primary" :disabled="busy" @click="importGithub">
            <LoaderCircle v-if="busy" :size="14" class="spin" />
            <GitBranch v-else :size="14" />
            获取并导入
          </button>
        </div>
        <div class="repo-notes">
          <div><span>XIU2/CloudflareSpeedTest</span><em>GPL-3.0 · master · ip.txt / ipv6.txt</em></div>
          <div><span>PoemMisty/CFData-WEB</span><em>GPL-3.0 · main · 测速源参考</em></div>
        </div>
      </div>
      <div class="glass-card panel">
        <div class="panel-head"><ShieldCheck :size="16" /><span>来源边界</span></div>
        <ul class="source-rules">
          <li>GitHub 清单只产生候选，不代表 IP 当前可用。</li>
          <li>来源中的国家/地区标签只作线索，最终地区以本地 CF-RAY colo 实测为准。</li>
          <li>导入后仍需完成 TCP、HTTPS、延迟与真实下载测试。</li>
          <li>不会把仓库中的示例数字写入本地统计。</li>
        </ul>
      </div>
    </section>


    <section v-if="tab === 'censys'" class="source-grid">
      <div class="glass-card panel">
        <div class="panel-head"><ShieldCheck :size="16" /><span>Censys 平台 API</span><span class="cm-status-pill" :class="{ bad: !censysStatus.configured }">{{ censysStatus.configured ? '已配置' : '未配置' }}</span></div>
        <p class="hint">
          使用 Censys Platform Global Data Search 查询 Cloudflare ASN 13335 的 443 端口主机。只把 host.ip 作为候选，绝不把 Censys 地理位置当作 Cloudflare colo。
          免费/Starter 套餐按 credits 计费；拉取前可在 Censys 控制台查看余额。
        </p>
        <div class="inline-row censys-config">
          <label>Personal Access Token
            <input v-model="censysToken" type="password" placeholder="输入 Censys Platform Token" autocomplete="off" />
          </label>
          <button class="btn-ghost" @click="saveCensys">保存 Token</button>
          <button class="btn-ghost danger" :disabled="!censysStatus.configured" @click="clearCensys">清除</button>
        </div>
        <div class="inline-row">
          <label>本次最多导入
            <select v-model.number="censysMax">
              <option :value="25">25 条</option>
              <option :value="50">50 条</option>
              <option :value="100">100 条</option>
            </select>
          </label>
          <button class="btn-primary" :disabled="busy || !censysStatus.configured" @click="fetchCensys">
            <LoaderCircle v-if="busy" :size="14" class="spin" />
            <ShieldCheck v-else :size="14" />
            查询并导入
          </button>
        </div>
        <div class="repo-notes">
          <div><span>API 端点</span><em>api.platform.censys.io/v3/global/search/query</em></div>
          <div><span>查询</span><em>host.services.port: 443 and host.autonomous_system.asn: 13335</em></div>
          <div><span>当前状态</span><em>{{ censysStatus.lastFetch || '尚未拉取' }} · 上次 {{ censysStatus.lastCount }} 条 · {{ censysStatus.lastMessage || '—' }}</em></div>
        </div>
      </div>
      <div class="glass-card panel">
        <div class="panel-head"><ShieldCheck :size="16" /><span>凭据与配额</span></div>
        <ul class="source-rules">
          <li>Token 仅保存在 Windows 凭据管理器，不写入设置、数据库或日志。</li>
          <li>未配置或额度不足时只显示明确错误，不影响 CM、GitHub、Cloudflare 等其他来源。</li>
          <li>Censys 只提供候选 IP，实际地区和可用性必须由本机 TCP / HTTPS / CF-RAY / 下载测速验证。</li>
          <li>免费额度不是无限查询；每次只取前 25/50/100 条，避免无边界消耗 credits。</li>
        </ul>
      </div>
    </section>

    <section v-if="tab === 'community'" class="community-layout">
      <div class="glass-card panel">
        <div class="panel-head"><Boxes :size="16" /><span>社区公开源聚合</span><span class="cm-status-pill">{{ communitySources.length }} 个来源</span></div>
        <p class="hint">每个来源独立抓取、独立记录状态；单源失败不会影响其他来源。采集阶段只扩大候选池，不按延迟提前淘汰。</p>
        <div class="inline-row">
          <label>每个来源最多保留
            <select v-model.number="communityMax">
              <option :value="1000">1,000 条</option>
              <option :value="3000">3,000 条</option>
              <option :value="5000">5,000 条</option>
              <option :value="10000">10,000 条</option>
            </select>
          </label>
          <button class="btn-ghost" :disabled="!!communityBusy" @click="loadCommunity">
            <RefreshCw :size="14" :class="{ spin: !!communityBusy }" />刷新状态
          </button>
        </div>
        <div class="community-grid">
          <article v-for="item in communitySources" :key="item.key" class="community-card">
            <div class="community-head">
              <div><strong>{{ item.name }}</strong><span>{{ item.repo }}</span></div>
              <span class="source-state" :class="{ bad: communityMap[item.key]?.status === '更新失败', warn: communityMap[item.key]?.status === '部分成功' }">{{ communityMap[item.key]?.status || '待获取' }}</span>
            </div>
            <div class="community-metrics">
              <span>候选 <b>{{ communityMap[item.key]?.count || 0 }}</b></span>
              <span>获取 <b>{{ communityMap[item.key]?.fetched || 0 }}</b></span>
              <span>新增 <b>{{ communityMap[item.key]?.added || 0 }}</b></span>
              <span>重复 <b>{{ communityMap[item.key]?.duplicates || 0 }}</b></span>
            </div>
            <div class="community-meta">
              <span>{{ item.license }} · {{ item.frequency }}</span>
              <span>最后更新：{{ communityMap[item.key]?.lastUpdate || '—' }}</span>
              <span v-if="communityMap[item.key]?.message" class="community-msg">{{ communityMap[item.key]?.message }}</span>
            </div>
            <button class="btn-primary" :disabled="!!communityBusy" @click="fetchCommunity(item.key)">
              <LoaderCircle v-if="communityBusy === item.key" :size="14" class="spin" />
              <Download v-else :size="14" />
              获取并导入
            </button>
          </article>
        </div>
      </div>
      <div class="glass-card panel">
        <div class="panel-head"><ShieldCheck :size="16" /><span>聚合规则</span></div>
        <ul class="source-rules">
          <li>七个社区源只提供公开候选 IP，不代表当前可用，也不直接成为优选结果。</li>
          <li>所有 IP 统一去重并保留来源、首次发现、最后更新时间与历史测速记录。</li>
          <li>采集与测速分离：采集不按延迟淘汰，测速由本机真实 TCP / HTTPS / CF-RAY / 下载完成。</li>
          <li>非首选地区的达标高速 IP 会归入「全球其他地区」保留，不因地区被丢弃。</li>
          <li>未知 colo 标记为未知地区，不猜测国家，仍允许参与排名。</li>
        </ul>
      </div>
    </section>

    <section v-if="tab === 'community'" class="glass-card panel registry-panel">
      <div class="panel-head">
        <Sparkles :size="16" /><span>数据源注册与上游发现</span>
        <span class="cm-status-pill">{{ registrySources.length }} 个登记源</span>
      </div>
      <div class="registry-actions">
        <button class="btn-primary" :disabled="discovering" @click="discoverRegistry">
          <LoaderCircle v-if="discovering" :size="14" class="spin" />
          <Sparkles v-else :size="14" />
          扫描 README 发现上游
        </button>
        <span v-if="discovery" class="registry-summary">
          {{ discovery.totalSources }} 个源 · {{ discovery.totalEndpoints }} 个端点 · 白名单通过 {{ discovery.accepted }} · 拒绝 {{ discovery.rejected }} · {{ discovery.generatedAt }}
        </span>
      </div>
      <div class="registry-body">
        <div class="registry-list">
          <article v-for="source in registrySources" :key="source.key" class="registry-row">
            <div><strong>{{ source.name }}</strong><span>{{ source.repo }} · {{ source.license }}</span></div>
            <em>{{ source.branch }} · {{ source.kind }}</em>
            <a class="registry-link" :href="source.url" target="_blank" rel="noreferrer"><ExternalLink :size="13" /></a>
          </article>
        </div>
        <div class="quality-table">
          <div class="quality-head"><span>来源质量</span><span>独立贡献</span><span>有效连接</span><span>高速 IP</span><span>质量分</span></div>
          <div v-for="row in qualityRows" :key="row.key" class="quality-row">
            <span>{{ row.name }}</span>
            <b>{{ row.uniqueCount }}</b>
            <b>{{ row.validConnections }}</b>
            <b>{{ row.highSpeedCount }}</b>
            <b :class="{ good: row.score >= 60 }">{{ row.score.toFixed(1) }}</b>
          </div>
          <p v-if="!qualityRows.length" class="hint">尚无来源质量数据，先执行「获取并导入」。</p>
        </div>
      </div>
      <div v-if="discovery" class="registry-endpoints">
        <span v-for="endpoint in discovery.endpoints.slice(0, 16)" :key="endpoint.url" :class="{ rejected: !endpoint.allowed }">
          {{ endpoint.allowed ? '✓' : '×' }} {{ endpoint.kind }} · {{ endpoint.url }}
        </span>
        <span v-if="discovery.endpoints.length > 16" class="registry-summary">另有 {{ discovery.endpoints.length - 16 }} 个端点未展开</span>
      </div>
      <div v-if="discovery?.warnings?.length" class="registry-warnings">
        <span v-for="warning in discovery.warnings" :key="warning">{{ warning }}</span>
      </div>
    </section>

    <section v-if="tab === 'cm'" class="source-grid cm-grid">
      <div class="glass-card panel">
        <div class="panel-head">
          <Database :size="16" /><span>CM IP 库 · 实时来源</span>
          <span class="cm-status-pill" :class="{ bad: cmStatus?.status === '获取失败' }">{{ cmStatus?.status || '未连接' }}</span>
        </div>
        <p class="hint">TXT 与 JSON 均为公开 IP 数据源，获取后进入统一候选池；来源中的国家/colo 仅作参考标签，最终地区必须重新实测。</p>
        <div class="cm-url-list">
          <div><span>TXT</span><code>{{ cmStatus?.sourceUrlTxt || 'https://zip.cm.edu.kg/all.txt' }}</code></div>
          <div><span>JSON</span><code>{{ cmStatus?.sourceUrlJson || 'https://zip.cm.edu.kg/all.json' }}</code></div>
        </div>
        <div class="cm-metrics">
          <div><span>获取数量</span><strong>{{ cmStatus?.count || 0 }}</strong></div>
          <div><span>有效 IP</span><strong>{{ cmStatus?.count || 0 }}</strong></div>
          <div><span>本次重复</span><strong>{{ cmStatus?.lastDuplicates || 0 }}</strong></div>
          <div><span>本次无效</span><strong>{{ cmStatus?.lastInvalid || 0 }}</strong></div>
        </div>
        <div class="cm-times">
          <span>数据更新时间：{{ cmStatus?.dataUpdatedAt || '—' }}</span>
          <span>最后获取：{{ cmStatus?.lastFetch || '—' }}</span>
          <span>最后尝试：{{ cmStatus?.lastAttempt || '—' }}</span>
        </div>
        <div v-if="cmStatus?.formatDiff" class="cm-diff">{{ cmStatus.formatDiff }}</div>
        <div v-if="cmStatus?.error" class="cm-error">{{ cmStatus.error }}</div>
        <div v-if="cmStatus?.cacheUsed" class="cm-cache">网络获取失败，当前显示的是已明确标注的本地缓存数据（{{ cmStatus.count }} 条）。</div>
      </div>

      <div class="glass-card panel">
        <div class="panel-head"><RefreshCw :size="16" /><span>更新与缓存策略</span></div>
        <div class="switch-list">
          <label><input v-model="cmAutoUpdate" type="checkbox" />启用自动更新</label>
          <label>更新间隔 <input v-model.number="cmInterval" type="number" min="30" max="10080" /> 分钟</label>
        </div>
        <button class="btn-ghost" :disabled="busy" @click="saveCmConfig">保存自动更新设置</button>
        <div class="cm-actions">
          <button class="btn-primary" :disabled="busy" @click="fetchCm('auto')">
            <LoaderCircle v-if="busy" :size="14" class="spin" /><RefreshCw v-else :size="14" />自动获取（TXT）
          </button>
          <button class="btn-ghost" :disabled="busy" @click="fetchCm('txt')">仅 TXT</button>
          <button class="btn-ghost" :disabled="busy" @click="fetchCm('json')">仅 JSON</button>
        </div>
        <div class="cm-actions">
          <button class="btn-ghost" @click="showCmRaw"><Eye :size="13" />查看原始数据</button>
          <button class="btn-ghost danger" @click="clearCm"><Trash2 :size="13" />清除 CM 来源配置</button>
        </div>
        <p class="hint">获取失败时保留上次成功缓存，并在页面明确标注；不会用缓存数据冒充本次实时结果。TXT 与 JSON 都获取过后会自动记录两种格式的数量差异。</p>
      </div>
    </section>

    <div v-if="cmRawVisible" class="cm-modal" @click.self="cmRawVisible = false">
      <div class="cm-modal-card">
        <div class="cm-modal-head">
          <strong>CM 原始数据 · {{ cmRaw?.format?.toUpperCase() }}</strong>
          <span>{{ cmRaw?.bytes || 0 }} B · {{ cmRaw?.lines || 0 }} 行</span>
          <button class="icon-btn" @click="cmRawVisible = false"><X :size="14" /></button>
        </div>
        <pre>{{ cmRaw?.content }}</pre>
      </div>
    </div>
  </div>
</template>

<style scoped>
.sources-page { gap: 14px; }
.header-actions { display: flex; align-items: center; gap: 8px; }
.source-dashboard { display: grid; grid-template-columns: minmax(0, 1.62fr) minmax(280px, .72fr); gap: 14px; align-items: start; }
.source-selector { display: flex; flex-direction: column; gap: 11px; padding: 15px; }
.panel-head { display: flex; align-items: center; gap: 8px; color: var(--text-0); font-size: 12px; font-weight: 700; }
.panel-head svg { color: #8ea9ff; }
.panel-note { margin-left: auto; color: var(--text-3); font-size: 9px; font-weight: 400; }
.source-card-list { display: grid; grid-template-columns: 1fr; gap: 7px; }
.source-card {
  position: relative;
  display: grid;
  grid-template-columns: 38px minmax(0, 1fr) auto;
  align-items: center;
  gap: 10px;
  min-height: 56px;
  padding: 8px 11px;
  border: 1px solid rgba(109, 134, 198, 0.15);
  border-radius: 10px;
  background: linear-gradient(100deg, rgba(12, 22, 47, 0.72), rgba(8, 15, 32, 0.55));
  color: var(--text-2);
  cursor: pointer;
  transition: border-color .16s, background .16s, transform .16s, box-shadow .16s;
}
.source-card::before {
  content: "";
  position: absolute;
  left: 0;
  top: 9px;
  bottom: 9px;
  width: 2px;
  border-radius: 2px;
  background: transparent;
}
.source-card:hover { border-color: rgba(109, 140, 255, 0.38); background: linear-gradient(100deg, rgba(20, 34, 70, 0.8), rgba(9, 17, 37, 0.62)); transform: translateX(2px); }
.source-card.selected { border-color: rgba(109, 140, 255, 0.62); background: linear-gradient(100deg, rgba(33, 56, 118, 0.72), rgba(18, 29, 64, 0.62)); box-shadow: inset 0 0 0 1px rgba(129, 156, 255, 0.1), 0 8px 22px rgba(20, 42, 108, 0.18); }
.source-card.selected::before { background: linear-gradient(180deg, #6d8cff, #b06dff); box-shadow: 0 0 10px rgba(109, 140, 255, .6); }
.source-card.off { opacity: .56; }
.source-card.off:hover { opacity: .76; }
.source-icon { display: grid; place-items: center; width: 36px; height: 36px; border-radius: 10px; border: 1px solid rgba(120, 148, 230, .2); flex-shrink: 0; }
.tone-blue { color: #8fb2ff; background: linear-gradient(145deg, rgba(55, 103, 232, .36), rgba(24, 52, 127, .25)); }
.tone-cyan { color: #75e3ff; background: linear-gradient(145deg, rgba(31, 147, 199, .28), rgba(16, 66, 105, .2)); }
.tone-green { color: #75e4b0; background: linear-gradient(145deg, rgba(35, 160, 115, .28), rgba(16, 76, 61, .2)); }
.tone-orange { color: #ffc27a; background: linear-gradient(145deg, rgba(190, 118, 35, .3), rgba(92, 55, 20, .2)); }
.tone-violet { color: #c49bff; background: linear-gradient(145deg, rgba(125, 72, 210, .32), rgba(59, 31, 111, .2)); }
.tone-slate { color: #c2d1e9; background: linear-gradient(145deg, rgba(91, 112, 150, .28), rgba(38, 50, 74, .2)); }
.tone-teal { color: #70dfe0; background: linear-gradient(145deg, rgba(29, 143, 145, .28), rgba(13, 69, 78, .2)); }
.tone-pink { color: #ff9fca; background: linear-gradient(145deg, rgba(199, 72, 130, .3), rgba(100, 31, 68, .2)); }
.source-card-main { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
.source-card-title { display: flex; align-items: center; gap: 8px; min-width: 0; }
.source-card-title strong { color: var(--text-0); font-size: 11px; font-weight: 650; white-space: nowrap; }
.source-state { flex-shrink: 0; padding: 2px 6px; border-radius: 99px; font-size: 8px; font-style: normal; font-weight: 500; }
.source-state.ok { color: #6fe0a8; background: rgba(38, 190, 118, .12); }
.source-state.warn { color: #e7bd76; background: rgba(205, 155, 57, .12); }
.source-state.bad { color: #f0948e; background: rgba(210, 77, 71, .12); }
.source-card-desc { color: var(--text-2); font-size: 9px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.source-card-meta { color: var(--text-3); font-size: 8.5px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.source-switch { position: relative; width: 36px; height: 20px; border: 1px solid rgba(109, 134, 198, .28); border-radius: 99px; background: rgba(40, 55, 90, .55); cursor: pointer; transition: .16s; flex-shrink: 0; }
.source-switch i { position: absolute; left: 2px; top: 2px; width: 14px; height: 14px; border-radius: 50%; background: #7c8aa8; transition: .16s; }
.source-switch.on { border-color: rgba(109, 140, 255, .65); background: linear-gradient(90deg, #5a7dff, #7b5cff); box-shadow: 0 0 12px rgba(92, 124, 255, .32); }
.source-switch.on i { left: 18px; background: #fff; box-shadow: 0 1px 4px rgba(0, 0, 0, .28); }
.source-selector-foot { display: flex; align-items: center; justify-content: space-between; gap: 10px; padding-top: 9px; border-top: 1px solid rgba(109, 134, 198, .1); color: var(--text-3); font-size: 8.5px; }
.source-selector-foot span { display: flex; align-items: center; gap: 4px; }
.source-selector-foot svg { color: #78dfab; }
.source-aside { display: flex; flex-direction: column; gap: 12px; min-width: 0; }
.update-card { position: relative; overflow: hidden; padding: 14px; display: flex; flex-direction: column; gap: 13px; background: linear-gradient(155deg, rgba(26, 51, 113, .82), rgba(9, 18, 41, .9)); }
.update-card::before { content: ""; position: absolute; width: 210px; height: 210px; right: -100px; top: -120px; border-radius: 50%; background: radial-gradient(circle, rgba(100, 132, 255, .28), transparent 66%); pointer-events: none; }
.update-button { position: relative; z-index: 1; display: flex; align-items: center; gap: 10px; width: 100%; min-height: 48px; padding: 9px 13px; border: 1px solid rgba(142, 169, 255, .45); border-radius: 9px; background: linear-gradient(100deg, rgba(74, 109, 236, .9), rgba(102, 82, 222, .85)); color: #fff; text-align: left; cursor: pointer; box-shadow: 0 10px 24px rgba(47, 77, 196, .25); }
.update-button:hover { background: linear-gradient(100deg, rgba(88, 123, 245, .96), rgba(120, 96, 238, .92)); }
.update-button:disabled { opacity: .65; cursor: wait; }
.update-button > span { display: flex; flex-direction: column; gap: 2px; }
.update-button strong { color: #fff; font-size: 12px; letter-spacing: .04em; }
.update-button em { color: rgba(234, 241, 255, .74); font-size: 8.5px; font-style: normal; }
.candidate-total { position: relative; z-index: 1; display: flex; flex-direction: column; gap: 3px; padding: 4px 2px 8px; border-bottom: 1px solid rgba(109, 140, 255, .14); }
.candidate-total span { color: var(--text-2); font-size: 9px; }
.candidate-total strong { color: #f0f5ff; font-size: 36px; line-height: 1; letter-spacing: .02em; font-variant-numeric: tabular-nums; }
.candidate-total em { color: var(--text-3); font-size: 8.5px; font-style: normal; }
.aside-metrics { position: relative; z-index: 1; display: grid; grid-template-columns: 1fr 1fr; gap: 8px; }
.aside-metrics > div { display: flex; flex-direction: column; gap: 3px; min-width: 0; padding: 9px 10px; border: 1px solid rgba(109, 140, 255, .13); border-radius: 8px; background: rgba(8, 16, 35, .46); }
.aside-metrics span { color: var(--text-3); font-size: 8.5px; }
.aside-metrics strong { color: var(--text-0); font-size: 20px; line-height: 1; }
.aside-metrics small { color: var(--text-3); font-size: 9px; font-weight: 400; }
.aside-metrics em { color: var(--text-3); font-size: 7.5px; font-style: normal; }
.spark-bars { display: flex; align-items: flex-end; gap: 2px; height: 22px; margin-top: 3px; }
.spark-bars i { flex: 1; min-width: 2px; border-radius: 2px 2px 0 0; background: linear-gradient(180deg, #62e0a7, rgba(49, 178, 182, .45)); opacity: .8; }
.recent-import { display: flex; align-items: center; gap: 6px; padding: 7px 8px; border-radius: 6px; background: rgba(46, 190, 126, .09); color: #6fdaa6; font-size: 8.5px; }
.aside-status { padding: 13px 14px; }
.aside-status .panel-head { margin-bottom: 8px; }
.aside-status-list { display: flex; flex-direction: column; gap: 4px; }
.aside-status-row { display: flex; align-items: center; gap: 7px; padding: 4px 2px; color: var(--text-2); font-size: 8.5px; border-bottom: 1px solid rgba(109, 134, 198, .06); }
.aside-status-row i { width: 5px; height: 5px; border-radius: 50%; flex-shrink: 0; }
.aside-status-row i.ok { background: #4fd895; box-shadow: 0 0 6px rgba(79, 216, 149, .5); }
.aside-status-row i.warn { background: #e4b866; box-shadow: 0 0 6px rgba(228, 184, 102, .45); }
.aside-status-row i.bad { background: #eb7f7a; box-shadow: 0 0 6px rgba(235, 127, 122, .45); }
.aside-status-row span { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.aside-status-row em { color: var(--text-3); font-size: 8px; font-style: normal; }
.config-heading { display: flex; align-items: center; gap: 10px; padding: 1px 2px 0; }
.config-heading-icon { display: grid; place-items: center; width: 29px; height: 29px; border-radius: 8px; background: rgba(75, 107, 225, .16); color: #93b0ff; }
.config-heading > div { display: flex; flex-direction: column; gap: 2px; }
.config-heading strong { color: var(--text-0); font-size: 12px; }
.config-heading em { color: var(--text-3); font-size: 8.5px; font-style: normal; }
.config-heading-state { margin-left: auto; padding: 3px 8px; border-radius: 99px; font-size: 8.5px; }
.config-heading-state.ok { color: #6fe0a8; background: rgba(38, 190, 118, .12); }
.config-heading-state.warn { color: #e7bd76; background: rgba(205, 155, 57, .12); }
.config-heading-state.bad { color: #f0948e; background: rgba(210, 77, 71, .12); }
.source-stat-card { padding: 0; overflow: hidden; }
.source-stat-card summary { display: flex; align-items: center; gap: 8px; padding: 12px 15px; color: var(--text-1); font-size: 11px; cursor: pointer; list-style: none; }
.source-stat-card summary::-webkit-details-marker { display: none; }
.source-stat-card summary svg { color: #8ea9ff; }
.source-stat-card summary em { margin-left: auto; color: var(--text-3); font-size: 8.5px; font-style: normal; }
.source-stat-card[open] summary { border-bottom: 1px solid rgba(109, 134, 198, .1); }
.source-stat-card .source-stat-table { padding: 0 8px 8px; width: calc(100% - 16px); }
.source-stat-table { width: 100%; border-collapse: collapse; font-size: 10.5px; }
.source-stat-table th { text-align: left; color: #7183aa; font-weight: 500; padding: 5px 8px; border-bottom: 1px solid rgba(109, 140, 255, .12); }
.source-stat-table td { padding: 6px 8px; border-bottom: 1px solid rgba(109, 140, 255, .07); color: #c8d4ee; }
.source-stat-table .num { font-family: "Cascadia Mono", monospace; color: #e8eeff; }
.stat-pill { display: inline-flex; padding: 2px 7px; border-radius: 99px; color: #98a7c8; background: rgba(120, 138, 176, .12); font-size: 9.5px; }
.stat-pill.ok { color: #55d995; background: rgba(34, 197, 94, .12); }
.source-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 13px; min-width: 0; }
.source-grid > * { min-width: 0; }
.fofa-grid { grid-template-columns: 0.92fr 1.08fr; }
.panel { padding: 16px; display: flex; flex-direction: column; gap: 12px; }
.drop-zone { min-height: 218px; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 9px; border: 1px dashed rgba(109, 140, 255, 0.35); border-radius: 12px; background: rgba(26, 43, 86, 0.12); color: #8195c5; cursor: pointer; transition: 0.18s; }
.drop-zone:hover, .drop-zone.over { border-color: rgba(125, 155, 255, 0.8); background: rgba(48, 79, 174, 0.18); }
.drop-zone strong { color: #cbd7f6; font-size: 13px; }
.drop-zone span { max-width: 280px; text-align: center; color: #627293; font-size: 10px; line-height: 1.6; }
.file-preview { border: 1px solid var(--line); border-radius: 9px; overflow: hidden; }
.file-preview-head { display: flex; align-items: center; justify-content: space-between; padding: 8px 10px; background: rgba(41, 58, 107, 0.25); color: #9cadd5; font-size: 10px; }
.file-preview-head span { display: flex; align-items: center; gap: 6px; }
.file-preview pre { max-height: 130px; overflow: auto; margin: 0; padding: 9px 11px; background: rgba(3, 7, 18, 0.55); color: #8ea3cb; font-size: 9.5px; line-height: 1.5; }
.file-preview .btn-primary { margin: 9px 11px; }
.ip-textarea { width: 100%; min-height: 218px; resize: vertical; border: 1px solid var(--line); border-radius: 10px; outline: none; padding: 12px; background: rgba(4, 8, 19, 0.65); color: #c4d3f0; font-family: var(--mono); font-size: 11px; line-height: 1.7; }
.ip-textarea.small { min-height: 115px; }
.inline-row { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; }
.inline-row label { display: flex; align-items: center; gap: 6px; color: #7180a1; font-size: 10px; }
.inline-row input, .form-column input { height: 29px; border: 1px solid var(--line); border-radius: 7px; outline: none; background: rgba(5, 10, 23, 0.75); color: #d5e1fa; padding: 0 8px; }
.inline-row input { width: 65px; }
.source-list { display: flex; flex-direction: column; gap: 7px; }
.source-row { display: flex; align-items: center; gap: 8px; padding: 9px 10px; border: 1px solid var(--line); border-radius: 8px; background: rgba(11, 20, 40, 0.5); }
.source-row-main { display: flex; flex-direction: column; gap: 3px; min-width: 0; flex: 1; }
.url-text { color: #aebdde; font-size: 10.5px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.muted { color: #586888; font-size: 9.5px; }
.row-actions { display: flex; gap: 4px; }
.row-update { display: inline-flex; align-items: center; gap: 4px; height: 25px; padding: 0 8px; border: 1px solid rgba(109, 140, 255, 0.28); border-radius: 6px; background: rgba(71, 101, 207, 0.12); color: #9fb6ff; font-size: 9.5px; cursor: pointer; }
.row-update:hover { border-color: rgba(109, 140, 255, 0.5); background: rgba(71, 101, 207, 0.22); color: #d3e0ff; }
.row-update:disabled { opacity: .45; cursor: default; }
.empty-note { padding: 24px; text-align: center; color: #566382; font-size: 10.5px; }
.form-column { display: flex; flex-direction: column; gap: 10px; }
.form-column label { display: flex; flex-direction: column; gap: 5px; color: #7f8dad; font-size: 10px; }
.form-column input { width: 100%; height: 34px; }
.status-line { display: flex; align-items: center; gap: 7px; color: #6fd4a7; font-size: 10px; }
.region-checks { display: flex; flex-wrap: wrap; gap: 5px; }
.region-check { height: 27px; border: 1px solid var(--line); border-radius: 6px; background: transparent; color: #7483a4; font-size: 10px; cursor: pointer; }
.region-check.active { border-color: rgba(109, 140, 255, 0.5); background: rgba(78, 109, 232, 0.17); color: #c8d7ff; }
.query-list { display: flex; flex-direction: column; gap: 5px; max-height: 170px; overflow-y: auto; }
.query-row { display: flex; gap: 8px; padding: 7px 8px; border-radius: 6px; background: rgba(6, 12, 26, 0.55); }
.query-region { width: 34px; flex-shrink: 0; color: #95a9e4; font-size: 10px; }
.query-row code { color: #7186b7; font-size: 9px; white-space: pre-wrap; word-break: break-all; }
.hint { color: #5c6b8c; font-size: 9.5px; line-height: 1.65; }
.url-report { max-height: 120px; overflow: auto; margin: 9px 0 0; padding: 8px 10px; border: 1px solid rgba(220, 120, 90, 0.2); border-radius: 7px; background: rgba(64, 25, 25, 0.2); color: #df9d89; font-family: var(--mono); font-size: 8.5px; line-height: 1.6; white-space: pre-wrap; word-break: break-all; }
.switch-list { display: flex; flex-direction: column; gap: 12px; color: #92a1c1; font-size: 11px; }
.switch-list label { display: flex; align-items: center; gap: 8px; }
.switch-list input[type="number"] { width: 65px; height: 28px; border: 1px solid var(--line); border-radius: 6px; background: rgba(5, 10, 23, 0.75); color: #d5e1fa; padding: 0 7px; }
.spin { animation: spin 0.9s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }
.cm-status-pill { margin-left: auto; padding: 3px 9px; border-radius: 99px; border: 1px solid rgba(67, 217, 255, 0.35); background: rgba(67, 217, 255, 0.1); color: #8ce8ff; font-size: 9.5px; }
.cm-status-pill.bad { border-color: rgba(255, 96, 96, 0.45); background: rgba(255, 70, 70, 0.12); color: #ff9c9c; }
.cm-url-list { display: flex; flex-direction: column; gap: 7px; }
.cm-url-list div { display: grid; grid-template-columns: 40px 1fr; gap: 8px; align-items: center; }
.cm-url-list span { color: #7f90b7; font-size: 9.5px; }
.cm-url-list code { overflow: hidden; text-overflow: ellipsis; padding: 7px 9px; border-radius: 7px; background: rgba(2, 7, 18, 0.55); color: #8fb0ea; font-size: 9.5px; }
.cm-metrics { display: grid; grid-template-columns: repeat(4, 1fr); gap: 8px; }
.cm-metrics div { display: flex; flex-direction: column; gap: 4px; padding: 10px 11px; border: 1px solid var(--line); border-radius: 9px; background: rgba(20, 34, 65, 0.32); }
.cm-metrics span { color: #7182a8; font-size: 9px; }
.cm-metrics strong { color: #e3ebff; font-size: 20px; }
.cm-times { display: flex; flex-direction: column; gap: 4px; color: #7384a8; font-size: 9.5px; }
.cm-diff { padding: 9px 10px; border-radius: 8px; background: rgba(109, 140, 255, 0.1); color: #a9bcf2; font-size: 9.5px; line-height: 1.6; }
.cm-error { padding: 9px 10px; border: 1px solid rgba(255, 95, 95, 0.3); border-radius: 8px; background: rgba(255, 60, 60, 0.08); color: #ff9d9d; font-size: 9.5px; line-height: 1.6; }
.cm-cache { padding: 9px 10px; border: 1px solid rgba(255, 193, 7, 0.3); border-radius: 8px; background: rgba(255, 180, 0, 0.08); color: #ffd77a; font-size: 9.5px; line-height: 1.6; }
.cm-actions { display: flex; gap: 8px; flex-wrap: wrap; }
.btn-ghost.danger { color: #ff9a9a; border-color: rgba(255, 90, 90, 0.3); }
.cm-modal { position: fixed; inset: 0; z-index: 1000; display: flex; align-items: center; justify-content: center; background: rgba(1, 4, 12, 0.72); backdrop-filter: blur(8px); }
.cm-modal-card { width: min(880px, 84vw); max-height: 76vh; display: flex; flex-direction: column; border: 1px solid rgba(109, 140, 255, 0.3); border-radius: 14px; background: #080f22; box-shadow: 0 30px 90px rgba(0, 0, 0, 0.55); }
.cm-modal-head { display: flex; align-items: center; gap: 12px; padding: 13px 16px; border-bottom: 1px solid var(--line); color: #dce6ff; font-size: 11px; }
.cm-modal-head span { color: #7587ad; font-size: 9.5px; }
.cm-modal-head .icon-btn { margin-left: auto; }
.cm-modal pre { flex: 1; overflow: auto; margin: 0; padding: 14px 16px; color: #91a9d6; font-size: 9.5px; line-height: 1.65; white-space: pre-wrap; word-break: break-all; }
@media (max-width: 1360px) {
  .community-layout { grid-template-columns: 1fr; }
  .registry-body { grid-template-columns: 1fr; }
}

@media (max-width: 1310px) {
  .source-grid, .fofa-grid { grid-template-columns: 1fr; }
  .cm-metrics { grid-template-columns: repeat(2, minmax(0, 1fr)); }
}

@media (max-width: 1180px) {
  .censys-config input { min-width: 220px; }
}

.github-presets { display: flex; flex-wrap: wrap; gap: 8px; margin: 10px 0; }
.preset-chip {
  display: inline-flex; align-items: center; gap: 6px; padding: 7px 10px;
  border: 1px solid rgba(110, 150, 255, .26); border-radius: 8px;
  background: rgba(48, 72, 132, .2); color: #a9c6ff; font-size: 10.5px; cursor: pointer;
}
.preset-chip:hover { border-color: rgba(148, 180, 255, .55); background: rgba(68, 96, 172, .28); }
.repo-notes { display: grid; gap: 6px; margin-top: 14px; }
.repo-notes div { display: flex; justify-content: space-between; gap: 10px; padding: 8px 10px; border: 1px solid rgba(92, 120, 190, .14); border-radius: 7px; background: rgba(9, 16, 34, .32); }
.repo-notes span { color: #c8d8ff; font-size: 10.5px; }
.repo-notes em { color: #6f85ab; font-size: 9.5px; font-style: normal; }
.source-rules { display: grid; gap: 8px; margin: 0; padding: 0; list-style: none; }
.source-rules li { position: relative; padding-left: 15px; color: #8fa8d8; font-size: 11px; line-height: 1.55; }
.source-rules li::before { content: ""; position: absolute; left: 0; top: 6px; width: 5px; height: 5px; border-radius: 50%; background: #8f6bff; box-shadow: 0 0 8px #8f6bff; }
.censys-config { align-items: flex-end; }
.censys-config input { min-width: 300px; }
.community-layout { display: grid; grid-template-columns: minmax(0, 1.65fr) minmax(260px, 0.7fr); gap: 12px; align-items: start; }
.community-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 9px; margin-top: 14px; }
.community-card { display: flex; flex-direction: column; gap: 9px; padding: 12px; border: 1px solid rgba(110, 145, 230, .2); border-radius: 10px; background: rgba(9, 18, 39, .55); }
.community-head { display: flex; align-items: flex-start; gap: 8px; }
.community-head > div { display: flex; flex-direction: column; gap: 3px; min-width: 0; flex: 1; }
.community-head strong { color: #dce8ff; font-size: 11px; }
.community-head span { color: #7286ad; font-size: 9px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.source-state { flex: 0 0 auto; padding: 3px 7px; border-radius: 99px; background: rgba(70, 211, 158, .12); color: #7ce2b1; font-size: 9px; }
.source-state.bad { background: rgba(255, 79, 79, .13); color: #ff9c9c; }
.source-state.warn { background: rgba(255, 187, 64, .13); color: #ffd07e; }
.community-metrics { display: grid; grid-template-columns: repeat(4, 1fr); gap: 5px; }
.community-metrics span { display: flex; flex-direction: column; gap: 2px; padding: 6px 7px; border-radius: 7px; background: rgba(23, 38, 72, .55); color: #6e81a8; font-size: 8.5px; }
.community-metrics b { color: #dce7ff; font-size: 14px; font-weight: 600; }
.community-meta { display: flex; flex-direction: column; gap: 3px; color: #7283a8; font-size: 9px; line-height: 1.45; }
.community-msg { color: #e0b06a; word-break: break-all; }
.community-card .btn-primary { align-self: flex-start; margin-top: 2px; }
.registry-panel { margin-top: 12px; }
.registry-actions { display: flex; align-items: center; gap: 12px; flex-wrap: wrap; margin: 12px 0; }
.registry-summary { color: #7f93b9; font-size: 10px; }
.registry-body { display: grid; grid-template-columns: minmax(0, 1fr) minmax(360px, 1.15fr); gap: 14px; }
.registry-list { display: grid; gap: 6px; }
.registry-row { display: flex; align-items: center; gap: 9px; padding: 7px 9px; border: 1px solid rgba(92, 120, 190, .14); border-radius: 7px; background: rgba(9, 16, 34, .32); }
.registry-row > div { display: flex; flex-direction: column; gap: 2px; min-width: 0; flex: 1; }
.registry-row strong { color: #cfdcff; font-size: 10.5px; }
.registry-row span, .registry-row em { color: #7084aa; font-size: 9px; font-style: normal; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.registry-link { color: #8fa9ff; }
.quality-table { display: grid; align-content: start; gap: 3px; overflow-x: auto; }
.quality-head, .quality-row { display: grid; grid-template-columns: minmax(120px, 1.5fr) repeat(4, minmax(58px, 0.65fr)); gap: 6px; align-items: center; padding: 5px 7px; min-width: 420px; }
.quality-head { color: #7084aa; font-size: 9px; border-bottom: 1px solid rgba(92, 120, 190, .18); }
.quality-row { color: #a9bad9; font-size: 9.5px; border-radius: 5px; background: rgba(23, 38, 72, .28); }
.quality-row b { color: #dce7ff; font-weight: 500; }
.quality-row b.good { color: #7ce2b1; }
.registry-endpoints { display: grid; gap: 3px; max-height: 150px; overflow: auto; margin-top: 10px; padding: 8px; border: 1px solid rgba(92, 120, 190, .14); border-radius: 7px; background: rgba(9, 16, 34, .26); }
.registry-endpoints span { color: #8fa8d8; font-size: 9px; line-height: 1.4; word-break: break-all; }
.registry-endpoints span.rejected { color: #d58b8b; }
.registry-warnings { display: grid; gap: 4px; margin-top: 10px; color: #d8a66a; font-size: 9px; }
@media (max-width: 1150px) { .community-layout { grid-template-columns: 1fr; } .registry-body { grid-template-columns: 1fr; } }
@media (max-width: 760px) { .community-grid { grid-template-columns: 1fr; } }
</style>

