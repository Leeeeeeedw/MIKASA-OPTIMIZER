import { invoke } from '@tauri-apps/api/core'
import type {
  AppSettings,
  BootstrapData,
  CmFetchOutcome,
  CmRawPreview,
  CmSourceStatus,
  DashboardData,
  SpeedSourceCompare,
  SpeedSourceInfo,
  SpeedSourceProbe,
  ExportOutcome,
  ExportOptions,
  FilePreview,
  FofaQueryPreview,
  FofaStatus,
  CensysStatus,
  CensysFetchOutcome,
  CommunitySourceInfo,
  RegistryDiscovery,
  RegistrySource,
  SourceQuality,
  GithubStatus,
  GithubUploadOutcome,
  ImportResult,
  NetworkInfo,
  ScanHistoryRow,
  ScanProgress,
  ScanRequest,
  ScanResult,
  RetestOutcome,
  SourceOverview,
  UrlSource,
} from './types'

export const isTauri = () => typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

export const api = {
  bootstrap: () => invoke<BootstrapData>('get_bootstrap'),
  detectNetwork: (force = false) => invoke<NetworkInfo>('detect_network', { force }),
  getDashboard: () => invoke<DashboardData>('get_dashboard'),
  getSettings: () => invoke<AppSettings>('get_settings'),
  saveSettings: (settings: AppSettings) => invoke<AppSettings>('save_settings', { settings }),
  listCandidates: (params: {
    page?: number
    pageSize?: number
    search?: string
    region?: string
    source?: string
  }) => invoke<{ items: import('./types').Candidate[]; total: number; page: number; pageSize: number }>('list_candidates', params),
  previewFile: (path: string) => invoke<FilePreview>('preview_file', { path }),
  importText: (params: { text: string; defaultPort?: number; region?: string; source?: string }) =>
    invoke<ImportResult>('import_text', params),
  importFile: (params: { path: string; defaultPort?: number; region?: string; source?: string }) =>
    invoke<ImportResult>('import_file', params),
  importUrls: (params: { urls: string[]; defaultPort?: number; region?: string; source?: string }) =>
    invoke<ImportResult>('import_urls', params),
  communitySourceList: () => invoke<CommunitySourceInfo[]>('community_source_list'),
  communitySourceStatus: () => invoke<SourceOverview[]>('community_source_status'),
  communityFetch: (source: string, maxResults?: number) =>
    invoke<ImportResult>('community_fetch', { source, maxResults }),
  sourceQuality: () => invoke<SourceQuality[]>('source_quality'),
  registrySources: () => invoke<RegistrySource[]>('registry_sources'),
  registryDiscover: () => invoke<RegistryDiscovery>('registry_discover'),
  cloudflareFetch: (params: { ipv4?: boolean; ipv6?: boolean; perCidr?: number }) =>
    invoke<ImportResult>('cloudflare_fetch', params),
  fofaPreview: (regions: string[]) => invoke<FofaQueryPreview[]>('fofa_preview', { regions }),
  fofaStatus: () => invoke<FofaStatus>('fofa_status'),
  saveFofaConfig: (email: string, apiKey: string) => invoke<FofaStatus>('save_fofa_config', { email, apiKey }),
  clearFofaConfig: () => invoke<void>('clear_fofa_config'),
  fofaFetch: (regions: string[], maxResults?: number) => invoke<ImportResult>('fofa_fetch', { regions, maxResults }),
  censysStatus: () => invoke<CensysStatus>('censys_status'),
  saveCensysConfig: (token: string) => invoke<CensysStatus>('save_censys_config', { token }),
  clearCensysConfig: () => invoke<void>('clear_censys_config'),
  censysFetch: (maxResults?: number) => invoke<CensysFetchOutcome>('censys_fetch', { maxResults }),
  cmStatus: () => invoke<CmSourceStatus>('cm_status'),
  cmFetch: (format = 'auto') => invoke<CmFetchOutcome>('cm_fetch', { format }),
  cmSaveConfig: (autoUpdate: boolean, intervalMinutes: number) =>
    invoke<CmSourceStatus>('cm_save_config', { autoUpdate, intervalMinutes }),
  cmPreview: (format = 'txt') => invoke<CmRawPreview>('cm_preview', { format }),
  cmClear: () => invoke<CmSourceStatus>('cm_clear'),
  listUrlSources: () => invoke<UrlSource[]>('list_url_sources'),
  addUrlSource: (url: string, intervalMinutes?: number) => invoke<UrlSource[]>('add_url_source', { url, intervalMinutes }),
  removeUrlSource: (id: number) => invoke<UrlSource[]>('remove_url_source', { id }),
  fetchUrlSource: (id: number) => invoke<ImportResult>('fetch_url_source', { id }),
  fetchAllUrlSources: () => invoke<{ added: number; updated: number; failed: number; messages: string[] }>('fetch_all_url_sources'),
  startScan: (request: ScanRequest) => invoke<string>('start_scan', { request }),
  cancelScan: () => invoke<boolean>('cancel_scan'),
  pauseScan: () => invoke<boolean>('pause_scan'),
  resumeScan: () => invoke<boolean>('resume_scan'),
  getCurrentScan: () => invoke<ScanProgress | null>('get_current_scan'),
  getResults: (scanId?: string, region?: string) => invoke<ScanResult[]>('get_results', { scanId, region }),
  retestIp: (ip: string, port: number) => invoke<RetestOutcome>('retest_ip', { ip, port }),
  getHistory: (limit?: number) => invoke<ScanHistoryRow[]>('get_history', { limit }),
  deleteScan: (scanId: string) => invoke<void>('delete_scan', { scanId }),
  clearHistory: (scope: string) => invoke<void>('clear_history', { scope }),
  backupDatabase: (directory?: string) => invoke<string>('backup_database', { directory }),
  exportResults: (options: ExportOptions) => invoke<ExportOutcome>('export_results', { options }),
  githubStatus: () => invoke<GithubStatus>('github_status'),
  saveGithubConfig: (params: { token?: string; repo?: string; branch?: string; remoteDir?: string }) =>
    invoke<GithubStatus>('save_github_config', params),
  clearGithubToken: () => invoke<void>('clear_github_token'),
  githubCheck: (repo: string, branch?: string) => invoke<Record<string, unknown>>('github_check', { repo, branch }),
  githubUpload: (params: { repo: string; branch?: string; remoteDir?: string; scanId?: string; options?: ExportOptions; splitRegions?: boolean }) =>
    invoke<GithubUploadOutcome>('github_upload', params),
  getGithubHistory: () => invoke<Array<Record<string, unknown>>>('get_github_history'),
  getDonationQr: (kind: 'wechat' | 'alipay') => invoke<string>('get_donation_qr', { kind }),
  openExternalUrl: (url: string) => invoke<void>('open_external_url', { url }),
  openDataDir: () => invoke<string>('open_data_dir'),
  speedSourceInfo: () => invoke<SpeedSourceInfo>('speed_source_info'),
  speedSourceProbe: () => invoke<SpeedSourceProbe[]>('speed_source_probe'),
  speedSourceCompare: (ip: string, port: number) =>
    invoke<SpeedSourceCompare>('speed_source_compare', { ip, port }),
  appInfo: () => invoke<{ version: string; dataDir: string; dbPath: string }>('app_info'),
}
