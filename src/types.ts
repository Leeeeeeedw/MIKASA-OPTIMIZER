export interface RegionInfo {
  key: string
  name: string
  flag: string
  colos: string[]
}

export interface AppSettings {
  defaultPort: number
  concurrency: number
  perRegionTop: number
  downloadTopPerRegion: number
  latencyLimitMs: number
  speedLimitMbS: number
  downloadSeconds: number
  minDownloadBytes: number
  tcpTimeoutMs: number
  httpsTimeoutMs: number
  latencyRounds: number
  stabilityRounds: number
  downloadThreads: number
  smartSupplement: boolean
  autoDetectNetwork: boolean
  autoRefreshSources: boolean
  sourceRefreshMinutes: number
  downloadUrl: string
  backupDownloadUrl: string
  speedSource: string
  backupSpeedSource: string
  autoSwitchSpeedSource: boolean
  customSpeedUrl: string
  maxCandidates: number
  exportDir: string
  logLevel: string
  uiScale: string
  proxy: string
  fofaQueryTemplate: string
  fofaEmail: string
  githubDefaultBranch: string
  githubRemoteDir: string
  customColoMap: string
  autoUploadAfterScan: boolean
}

export interface NetworkInfo {
  province: string
  carrier: string
  display: string
  proxyHint: string
  detectedAt: string
  source: string
  ok: boolean
}

export interface Candidate {
  id: number
  ip: string
  port: number
  sources: string[]
  initialRegion: string
  firstSeen: string
  lastSeen: string
  lastColo: string
  lastLatency: number | null
  lastDownloadMbS: number | null
  lastLoss: number | null
  lastStability: number | null
  lastStatus: string
  lastTested: string
}

export interface CandidatePreview {
  ip: string
  port: number
  region: string
  source: string
}

export interface ImportResult {
  added: number
  updated: number
  total: number
  preview: CandidatePreview[]
  sourceCounts: Record<string, number>
}

export interface ScanRequest {
  regions: string[]
  perRegionTop: number
  downloadTopPerRegion: number
  port: number
  concurrency: number
  latencyLimitMs: number
  speedLimitMbS: number
  downloadSeconds: number
  minDownloadBytes: number
  latencyRounds: number
  stabilityRounds: number
  downloadThreads: number
  smartSupplement: boolean
  maxCandidates: number
  sourceFilter: string[]
  downloadUrl: string | null
}

export interface RegionStats {
  key: string
  name: string
  flag: string
  candidates: number
  tcpOk: number
  httpsOk: number
  coloOk: number
  latencyOk: number
  downloadOk: number
  speedOk: number
  finalCount: number
  avgLatency: number
  avgSpeed: number
  latencyEliminated: number
}

export interface ScanProgress {
  scanId: string
  status: string
  phase: string
  message: string
  tested: number
  total: number
  tcpOk: number
  httpsOk: number
  coloOk: number
  latencyOk: number
  downloadOk: number
  speedOk: number
  finalCount: number
  regionStats: RegionStats[]
  startedAt: string
  finishedAt: string
  tcpTested: number
  tcpPassed: number
  httpsTested: number
  httpsPassed: number
  latencyTested: number
  latencyPassed: number
  latencyEliminated: number
  downloadTested: number
  downloadPassed: number
  finalSelected: number
}

export interface ScanResult {
  id: number
  scanId: string
  rank: number
  ip: string
  port: number
  region: string
  regionName: string
  colo: string
  latency: number
  jitter: number
  speedMbS: number
  speedMbps: number
  loss: number
  stability: number
  source: string
  status: string
  testedAt: string
  rounds?: TestRound[]
}

export interface TestRound {
  phase: string
  round: number
  latencyMs: number | null
  speedMbS: number | null
  ok: boolean
  message: string
}

export interface RetestOutcome {
  ok: boolean
  message: string
  tcpOk: boolean
  colo: string
  result: ScanResult | null
  rounds: TestRound[]
}

export interface DashboardData {
  totalCandidates: number
  totalTested: number
  avgLatency: number
  avgSpeed: number
  usable: number
  latestScanId: string
  latestScanAt: string
  regionStats: RegionStats[]
  recentResults: ScanResult[]
  network: NetworkInfo | null
}

export interface ScanHistoryRow {
  scanId: string
  startedAt: string
  finishedAt: string
  status: string
  tested: number
  finalCount: number
  avgLatency: number
  avgSpeed: number
  regions: string[]
}

export interface FilePreview {
  name: string
  content: string
  bytes: number
}

export interface ExportOutcome {
  directory: string
  files: string[]
  count: number
  format: string
}

export interface ExportOptions {
  scanId?: string
  directory?: string
  format?: string
  region?: string
  perRegionLimit?: number
  minSpeedMbS?: number
  maxLatencyMs?: number
  includeSpeed?: boolean
  includeLatency?: boolean
  includeCity?: boolean
  includeColo?: boolean
  includeSource?: boolean
}

export interface UrlSource {
  id: number
  url: string
  enabled: boolean
  intervalMinutes: number
  lastFetch: string
  lastCount: number
  lastStatus: string
}

export interface FofaStatus {
  configured: boolean
  email: string
  keyMasked: string
}

export interface CensysStatus {
  configured: boolean
  tokenMasked: string
  lastFetch: string
  lastCount: number
  lastMessage: string
}

export interface CensysFetchOutcome {
  fetched: number
  total: number
  message: string
  result: ImportResult
}

export interface FofaQueryPreview {
  region: string
  regionName: string
  country: string
  query: string
}

export interface GithubUploadOutcome {
  repo: string
  branch: string
  files: string[]
  commitSha: string
  verified: boolean
}

export interface CmSourceStatus {
  status: string
  count: number
  lastFetch: string
  lastAttempt: string
  dataUpdatedAt: string
  lastAdded: number
  lastUpdated: number
  lastDuplicates: number
  lastInvalid: number
  format: string
  autoUpdate: boolean
  intervalMinutes: number
  cacheUsed: boolean
  error: string
  formatDiff: string
  etag: string
  sourceUrlTxt: string
  sourceUrlJson: string
}

export interface CmFetchOutcome {
  added: number
  updated: number
  total: number
  duplicates: number
  invalid: number
  count: number
  format: string
  dataUpdatedAt: string
  fetchedAt: string
  cacheUsed: boolean
  message: string
  formatDiff: string
}

export interface CmRawPreview {
  format: string
  bytes: number
  lines: number
  content: string
  truncated: boolean
}

export interface GithubStatus {
  configured: boolean
  masked: string
  repo: string
  branch: string
  remoteDir: string
}

export interface SourceOverview {
  key: string
  name: string
  count: number
  fetched: number
  added: number
  duplicates: number
  invalid: number
  lastUpdate: string
  status: string
  message?: string
}

export interface CommunitySourceInfo {
  key: string
  name: string
  repo: string
  license: string
  frequency: string
  note: string
}

export interface SourceQuality {
  key: string
  name: string
  rawCount: number
  uniqueCount: number
  duplicateRate: number
  validConnections: number
  highSpeedCount: number
  regionCoverage: number
  updateFrequency: string
  lastSuccess: string
  score: number
}

export interface RegistrySource {
  key: string
  name: string
  repo: string
  url: string
  license: string
  branch: string
  kind: string
  note: string
  readmeUrl: string
}

export interface DiscoveredEndpoint {
  url: string
  kind: string
  source: string
  allowed: boolean
  reason: string
}

export interface RegistryDiscovery {
  generatedAt: string
  sources: RegistrySource[]
  endpoints: DiscoveredEndpoint[]
  totalSources: number
  totalEndpoints: number
  accepted: number
  rejected: number
  warnings: string[]
}

export interface BootstrapData {
  settings: AppSettings
  regions: RegionInfo[]
  dashboard: DashboardData
  sourceCounts: Record<string, number>
  sourceStats: SourceOverview[]
  dataDir: string
  version: string
}

export interface ScanLogEntry {
  scanId: string
  time: string
  level: string
  message: string
}

export interface SpeedSourceProbe {
  key: string
  label: string
  url: string
  ok: boolean
  dnsOk: boolean
  tcpOk: boolean
  tlsOk: boolean
  status: number
  firstChunkBytes: number
  elapsedMs: number
  redirected: boolean
  finalUrl: string
  resolvedIp: string
  connectedIp: string
  error: string
}

export interface IspInfo {
  asn: number | null
  org: string
  isp: string
  isChinaMobile: boolean
  probeOk: boolean
  error: string
}

export interface SpeedSourceCompareRow {
  key: string
  label: string
  url: string
  targetIp: string
  connectedIp: string
  status: number
  bytes: number
  seconds: number
  mbS: number
  mbps: number
  ok: boolean
  error: string
}

export interface SpeedSourceCompare {
  ip: string
  port: number
  rows: SpeedSourceCompareRow[]
  recommended: string
  note: string
}

export interface SpeedSourceInfo {
  source: string
  effectiveSource: string
  effectiveLabel: string
  sourceUrl: string
  backupSource: string
  autoSwitch: boolean
  customUrl: string
  isp: IspInfo
  probes: SpeedSourceProbe[] | null
}
