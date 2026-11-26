import { reactive } from 'vue'
import { listen } from '@tauri-apps/api/event'
import { api, isTauri } from './api'
import type {
  AppSettings,
  DashboardData,
  NetworkInfo,
  RegionInfo,
  RegionStats,
  ScanLogEntry,
  ScanProgress,
  ScanRequest,
  ScanResult,
  ScanHistoryRow,
  SourceOverview,
} from './types'

export type ViewKey =
  | 'overview'
  | 'sources'
  | 'scan'
  | 'rankings'
  | 'export'
  | 'github'
  | 'history'
  | 'settings'
  | 'about'

const defaultSettings: AppSettings = {
  defaultPort: 443,
  concurrency: 64,
  perRegionTop: 20,
  downloadTopPerRegion: 30,
  latencyLimitMs: 1500,
  speedLimitMbS: 1,
  downloadSeconds: 10,
  minDownloadBytes: 3145728,
  tcpTimeoutMs: 2500,
  httpsTimeoutMs: 8000,
  latencyRounds: 4,
  stabilityRounds: 0,
  downloadThreads: 4,
  smartSupplement: true,
  autoDetectNetwork: true,
  autoRefreshSources: true,
  sourceRefreshMinutes: 60,
  downloadUrl: 'https://speed.cloudflare.com/__down?bytes=104857600',
  backupDownloadUrl: 'https://speed.cloudflare.com/__down?bytes=52428800',
  speedSource: 'auto',
  backupSpeedSource: 'cloudflare',
  autoSwitchSpeedSource: true,
  customSpeedUrl: '',
  maxCandidates: 1024,
  exportDir: '',
  logLevel: 'info',
  uiScale: 'auto',
  proxy: '',
  fofaQueryTemplate: 'country="{country}" && port="{port}" && org="Cloudflare, Inc."',
  fofaEmail: '',
  githubDefaultBranch: 'main',
  githubRemoteDir: 'mikasa-results',
  customColoMap: '',
  autoUploadAfterScan: false,
}

const defaultDashboard: DashboardData = {
  totalCandidates: 0,
  totalTested: 0,
  avgLatency: 0,
  avgSpeed: 0,
  usable: 0,
  latestScanId: '',
  latestScanAt: '',
  regionStats: [],
  recentResults: [],
  network: null,
}

export const state = reactive({
  initialized: false,
  loading: true,
  initError: '',
  settings: { ...defaultSettings } as AppSettings,
  regions: [] as RegionInfo[],
  dashboard: { ...defaultDashboard } as DashboardData,
  sourceCounts: {} as Record<string, number>,
  sourceStats: [] as SourceOverview[],
  dataDir: '',
  version: '1.0.7',
  network: null as NetworkInfo | null,
  networkLoading: false,
  activeView: 'overview' as ViewKey,
  selectedRegions: ['hk', 'tw', 'jp', 'sg', 'kr', 'fr', 'us'] as string[],
  enabledSources: [
    'local', 'url', 'fofa', 'cloudflare', 'cm', 'github', 'censys',
    'cf_best_ip', 'bestcf', 'best_cf_ips', 'ipdb', 'cloudflare_ip', 'free_nodes_bestcf', 'xiu2',
  ] as string[],
  scan: null as ScanProgress | null,
  logs: [] as ScanLogEntry[],
  results: [] as ScanResult[],
  selectedScanId: '',
  history: [] as ScanHistoryRow[],
})

let listenersReady = false

function emptyRegionStats(region: RegionInfo): RegionStats {
  return {
    key: region.key,
    name: region.name,
    flag: region.flag,
    candidates: 0,
    tcpOk: 0,
    httpsOk: 0,
    coloOk: 0,
    latencyOk: 0,
    downloadOk: 0,
    speedOk: 0,
    finalCount: 0,
    avgLatency: 0,
    avgSpeed: 0,
    latencyEliminated: 0,
  }
}

export function regionStatsFor(key: string): RegionStats {
  const fromScan = state.scan?.regionStats?.find((r) => r.key === key)
  if (fromScan) return fromScan
  const fromDashboard = state.dashboard.regionStats?.find((r) => r.key === key)
  if (fromDashboard) return fromDashboard
  const region = state.regions.find((r) => r.key === key)
  return region ? emptyRegionStats(region) : {
    key, name: key, flag: '🌐', candidates: 0, tcpOk: 0, httpsOk: 0, coloOk: 0,
    latencyOk: 0, downloadOk: 0, speedOk: 0, finalCount: 0, avgLatency: 0, avgSpeed: 0,
    latencyEliminated: 0,
  }
}

export async function initApp() {
  state.loading = true
  state.initError = ''
  try {
    const data = await api.bootstrap()
    state.settings = data.settings
    state.regions = data.regions
    state.dashboard = data.dashboard
    state.sourceCounts = data.sourceCounts || {}
    state.sourceStats = data.sourceStats || []
    state.dataDir = data.dataDir
    state.version = data.version
    state.network = data.dashboard.network
    state.selectedScanId = data.dashboard.latestScanId || ''
    state.initialized = true
    if (state.settings.autoDetectNetwork) {
      void refreshNetwork(false)
    }
    await attachListeners()
  } catch (error) {
    state.initError = String(error)
  } finally {
    state.loading = false
  }
}

export async function attachListeners() {
  if (!isTauri() || listenersReady) return
  listenersReady = true
  await listen<ScanProgress>('scan-progress', (event) => {
    state.scan = event.payload
  })
  await listen<ScanLogEntry>('scan-log', (event) => {
    state.logs.push(event.payload)
    if (state.logs.length > 800) state.logs.splice(0, state.logs.length - 800)
  })
  await listen<{ scanId: string; status: string; count: number; error?: string }>('scan-finished', async (event) => {
    if (state.scan && state.scan.scanId === event.payload.scanId) {
      state.scan.status = event.payload.status
      state.scan.finishedAt = new Date().toISOString()
    }
    if (event.payload.status === 'completed') {
      state.selectedScanId = event.payload.scanId
      await Promise.all([refreshDashboard(), loadResults(event.payload.scanId)])
    }
  })
  await listen('url-sources-updated', () => {
    void refreshDashboard()
  })
}

export async function refreshNetwork(force = true) {
  if (!isTauri()) return
  state.networkLoading = true
  try {
    state.network = await api.detectNetwork(force)
  } finally {
    state.networkLoading = false
  }
}

export async function refreshDashboard() {
  if (!isTauri()) return
  state.dashboard = await api.getDashboard()
  if (!state.network && state.dashboard.network) state.network = state.dashboard.network
}

export async function refreshSourceStats() {
  if (!isTauri()) return
  const data = await api.bootstrap()
  state.sourceCounts = data.sourceCounts || {}
  state.sourceStats = data.sourceStats || []
}

export async function loadResults(scanId?: string, region?: string) {
  if (!isTauri()) return
  state.results = await api.getResults(scanId || state.selectedScanId || undefined, region)
}

export function buildScanRequest(): ScanRequest {
  return {
    regions: state.selectedRegions.length === 0 ? ['all'] : [...state.selectedRegions],
    perRegionTop: state.settings.perRegionTop,
    downloadTopPerRegion: state.settings.downloadTopPerRegion,
    port: state.settings.defaultPort,
    concurrency: state.settings.concurrency,
    latencyLimitMs: state.settings.latencyLimitMs,
    speedLimitMbS: state.settings.speedLimitMbS,
    downloadSeconds: state.settings.downloadSeconds,
    minDownloadBytes: state.settings.minDownloadBytes,
    latencyRounds: state.settings.latencyRounds,
    stabilityRounds: state.settings.stabilityRounds,
    downloadThreads: state.settings.downloadThreads,
    smartSupplement: state.settings.smartSupplement,
    maxCandidates: state.settings.maxCandidates,
    sourceFilter: state.enabledSources.length ? [...state.enabledSources] : ["__none__"],
    downloadUrl: state.settings.downloadUrl || null,
  }
}

export async function startScan() {
  if (!isTauri()) throw new Error('请在桌面应用中运行扫描')
  const id = await api.startScan(buildScanRequest())
  state.selectedScanId = id
  state.logs = []
  state.scan = {
    scanId: id,
    status: 'running',
    phase: 'starting',
    message: '正在准备扫描任务',
    tested: 0,
    total: 0,
    tcpOk: 0,
    httpsOk: 0,
    coloOk: 0,
    latencyOk: 0,
    downloadOk: 0,
    speedOk: 0,
    finalCount: 0,
    regionStats: state.regions.map(emptyRegionStats),
    startedAt: new Date().toISOString(),
    finishedAt: '',
    tcpTested: 0,
    tcpPassed: 0,
    httpsTested: 0,
    httpsPassed: 0,
    latencyTested: 0,
    latencyPassed: 0,
    latencyEliminated: 0,
    downloadTested: 0,
    downloadPassed: 0,
    finalSelected: 0,
  }
  state.activeView = 'scan'
}

export async function cancelScan() {
  await api.cancelScan()
}

export async function pauseScan() {
  await api.pauseScan()
  if (state.scan) {
    state.scan.status = 'paused'
    state.scan.message = '任务已暂停，点击「继续」恢复扫描'
  }
}

export async function resumeScan() {
  await api.resumeScan()
  if (state.scan) {
    state.scan.status = 'running'
    state.scan.message = '任务已继续'
  }
}

export function toggleRegion(key: string) {
  const index = state.selectedRegions.indexOf(key)
  if (index >= 0) state.selectedRegions.splice(index, 1)
  else state.selectedRegions.push(key)
}

export function selectAllRegions() {
  if (state.selectedRegions.length === state.regions.length) state.selectedRegions = []
  else state.selectedRegions = state.regions.map((r) => r.key)
}

export function isAllSelected() {
  return state.regions.length > 0 && state.selectedRegions.length === state.regions.length
}

export function isScanActive() {
  return (
    state.scan?.status === 'running' ||
    state.scan?.status === 'paused' ||
    state.scan?.status === 'cancelling'
  )
}
