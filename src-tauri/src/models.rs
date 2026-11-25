use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const REGION_ORDER: [&str; 7] = ["hk", "tw", "jp", "sg", "kr", "fr", "us"];

/// 首选地区之外的全球地区。顺序用于排名页“全球其他地区”分组展示。
pub const GLOBAL_REGION_ORDER: [&str; 31] = [
    "ca", "de", "gb", "au", "nl", "ch", "se", "fi", "no", "dk", "es", "it", "pl", "at", "cz",
    "tr", "ae", "il", "za", "br", "ar", "cl", "co", "pe", "mx", "my", "th", "vn", "ph", "in",
    "nz",
];

pub fn region_name(key: &str) -> &'static str {
    match key {
        "hk" => "香港",
        "tw" => "台湾",
        "jp" => "日本",
        "sg" => "新加坡",
        "kr" => "韩国",
        "fr" => "法国",
        "us" => "美国",
        "ca" => "加拿大",
        "de" => "德国",
        "gb" => "英国",
        "au" => "澳大利亚",
        "nl" => "荷兰",
        "ch" => "瑞士",
        "se" => "瑞典",
        "fi" => "芬兰",
        "no" => "挪威",
        "dk" => "丹麦",
        "es" => "西班牙",
        "it" => "意大利",
        "pl" => "波兰",
        "at" => "奥地利",
        "cz" => "捷克",
        "tr" => "土耳其",
        "ae" => "阿联酋",
        "il" => "以色列",
        "za" => "南非",
        "br" => "巴西",
        "ar" => "阿根廷",
        "cl" => "智利",
        "co" => "哥伦比亚",
        "pe" => "秘鲁",
        "mx" => "墨西哥",
        "my" => "马来西亚",
        "th" => "泰国",
        "vn" => "越南",
        "ph" => "菲律宾",
        "in" => "印度",
        "ru" => "俄罗斯",
        "nz" => "新西兰",
        "global" => "全球其他地区",
        "unknown" => "未知地区",
        "all" => "全部地区",
        _ => "未知地区",
    }
}

pub fn region_flag(key: &str) -> &'static str {
    match key {
        "hk" => "🇭🇰",
        "tw" => "🇹🇼",
        "jp" => "🇯🇵",
        "sg" => "🇸🇬",
        "kr" => "🇰🇷",
        "fr" => "🇫🇷",
        "us" => "🇺🇸",
        "ca" => "🇨🇦",
        "de" => "🇩🇪",
        "gb" => "🇬🇧",
        "au" => "🇦🇺",
        "nl" => "🇳🇱",
        "ch" => "🇨🇭",
        "se" => "🇸🇪",
        "fi" => "🇫🇮",
        "no" => "🇳🇴",
        "dk" => "🇩🇰",
        "es" => "🇪🇸",
        "it" => "🇮🇹",
        "pl" => "🇵🇱",
        "at" => "🇦🇹",
        "cz" => "🇨🇿",
        "tr" => "🇹🇷",
        "ae" => "🇦🇪",
        "il" => "🇮🇱",
        "za" => "🇿🇦",
        "br" => "🇧🇷",
        "ar" => "🇦🇷",
        "cl" => "🇨🇱",
        "co" => "🇨🇴",
        "pe" => "🇵🇪",
        "mx" => "🇲🇽",
        "my" => "🇲🇾",
        "th" => "🇹🇭",
        "vn" => "🇻🇳",
        "ph" => "🇵🇭",
        "in" => "🇮🇳",
        "ru" => "🇷🇺",
        "nz" => "🇳🇿",
        _ => "🌐",
    }
}

pub fn region_colos(key: &str) -> &'static [&'static str] {
    match key {
        "hk" => &["HKG"],
        "tw" => &["TPE", "KHH", "RMQ"],
        "jp" => &["NRT", "KIX", "FUK", "OKA", "UKB", "ITM"],
        "sg" => &["SIN"],
        "kr" => &["ICN", "SEL"],
        "fr" => &["CDG", "MRS"],
        "us" => &[
            "LAX", "SJC", "SEA", "ORD", "IAD", "ATL", "DFW", "MIA", "EWR", "BOS", "DEN", "PHX",
            "IAH", "LAS", "MSP", "DTW", "PHL", "SAN", "CLT", "SLC", "PDX", "AUS",
        ],
        _ => &[],
    }
}

/// Cloudflare colo → 机房所在城市（中文）。
/// 只有真实 CF-RAY / trace 返回的 colo 才会进入这里，不用 GeoIP 猜城市。
pub fn colo_city(colo: &str) -> Option<&'static str> {
    let city = match colo.trim().to_ascii_uppercase().as_str() {
        "HKG" => "香港",
        "TPE" => "台北",
        "KHH" => "高雄",
        "RMQ" => "台中",
        "NRT" | "HND" => "东京",
        "KIX" | "ITM" => "大阪",
        "UKB" => "神户",
        "FUK" => "福冈",
        "OKA" => "冲绳",
        "CTS" => "札幌",
        "SIN" => "新加坡",
        "ICN" | "SEL" => "首尔",
        "CDG" => "巴黎",
        "MRS" => "马赛",
        "LAX" => "洛杉矶",
        "SJC" => "圣何塞",
        "SFO" => "旧金山",
        "SEA" => "西雅图",
        "ORD" => "芝加哥",
        "IAD" => "阿什本",
        "ATL" => "亚特兰大",
        "DFW" => "达拉斯",
        "MIA" => "迈阿密",
        "EWR" => "纽瓦克",
        "BOS" => "波士顿",
        "DEN" => "丹佛",
        "PHX" => "凤凰城",
        "IAH" => "休斯敦",
        "LAS" => "拉斯维加斯",
        "MSP" => "明尼阿波利斯",
        "DTW" => "底特律",
        "PHL" => "费城",
        "SAN" => "圣迭戈",
        "CLT" => "夏洛特",
        "SLC" => "盐湖城",
        "PDX" => "波特兰",
        "AUS" => "奥斯汀",
        "YYZ" => "多伦多",
        "YVR" => "温哥华",
        "YUL" => "蒙特利尔",
        "YYC" => "卡尔加里",
        "FRA" => "法兰克福",
        "DUS" => "杜塞尔多夫",
        "BER" => "柏林",
        "MUC" => "慕尼黑",
        "HAM" => "汉堡",
        "LHR" | "LGW" => "伦敦",
        "MAN" => "曼彻斯特",
        "EDI" => "爱丁堡",
        "SYD" => "悉尼",
        "MEL" => "墨尔本",
        "BNE" => "布里斯班",
        "PER" => "珀斯",
        "ADL" => "阿德莱德",
        "AMS" => "阿姆斯特丹",
        "ZRH" => "苏黎世",
        "ARN" => "斯德哥尔摩",
        "HEL" => "赫尔辛基",
        "OSL" => "奥斯陆",
        "CPH" => "哥本哈根",
        "MAD" => "马德里",
        "BCN" => "巴塞罗那",
        "MXP" => "米兰",
        "FCO" => "罗马",
        "WAW" => "华沙",
        "VIE" => "维也纳",
        "PRG" => "布拉格",
        "IST" => "伊斯坦布尔",
        "DXB" => "迪拜",
        "TLV" => "特拉维夫",
        "JNB" => "约翰内斯堡",
        "CPT" => "开普敦",
        "GRU" => "圣保罗",
        "GIG" => "里约热内卢",
        "EZE" => "布宜诺斯艾利斯",
        "SCL" => "圣地亚哥",
        "BOG" => "波哥大",
        "LIM" => "利马",
        "MEX" => "墨西哥城",
        "KUL" => "吉隆坡",
        "BKK" => "曼谷",
        "SGN" => "胡志明市",
        "HAN" => "河内",
        "MNL" => "马尼拉",
        "BOM" => "孟买",
        "MAA" => "金奈",
        "DEL" => "新德里",
        "BLR" => "班加罗尔",
        "SVO" | "DME" => "莫斯科",
        "AKL" => "奥克兰",
        _ => return None,
    };
    Some(city)
}

/// 界面与导出使用的节点地理位置名。
/// 香港 / 台湾 / 新加坡只显示自身；其余国家在 colo 能识别城市时显示「国家 + 城市」。
pub fn node_display_name(region: &str, colo: &str) -> String {
    let region = region.trim();
    if region == "unknown" || region == "global" {
        return region_name(region).to_string();
    }
    if matches!(region, "hk" | "tw" | "sg") {
        return region_name(region).to_string();
    }
    match colo_city(colo) {
        Some(city) => format!("{}{}", region_name(region), city),
        None => region_name(region).to_string(),
    }
}

pub fn region_for_colo(colo: &str) -> Option<&'static str> {
    let colo = colo.to_ascii_uppercase();
    for key in REGION_ORDER {
        if region_colos(key).iter().any(|c| *c == colo) {
            return Some(key);
        }
    }
    None
}

/// Cloudflare 实际 colo → 国家/地区。映射来自 LeilaoMi/cf-best-ip（MIT）
/// 的 COLO_TO_COUNTRY 表，并补充本项目已有的 KHH/FUK/OKA 等代码。
/// 这里不使用 GeoIP：只有真实 CF-RAY/trace 返回的 colo 才进入此表。
pub fn global_region_for_colo(colo: &str) -> Option<&'static str> {
    let key = match colo.trim().to_ascii_uppercase().as_str() {
        "HKG" => "hk",
        "TPE" | "KHH" | "RMQ" => "tw",
        "NRT" | "KIX" | "FUK" | "OKA" | "UKB" | "ITM" => "jp",
        "SIN" => "sg",
        "ICN" | "SEL" => "kr",
        "CDG" | "MRS" => "fr",
        "LAX" | "SJC" | "SEA" | "ORD" | "IAD" | "DFW" | "ATL" | "EWR" | "MIA" | "DEN"
        | "BOS" | "SFO" | "PHX" | "IAH" | "LAS" | "MSP" | "DTW" | "PHL" | "SAN" | "CLT"
        | "SLC" | "PDX" | "AUS" => "us",
        "YYZ" | "YVR" | "YUL" | "YYC" => "ca",
        "FRA" | "DUS" | "BER" | "MUC" | "HAM" => "de",
        "LHR" | "LGW" | "MAN" | "EDI" => "gb",
        "SYD" | "MEL" | "BNE" | "PER" | "ADL" => "au",
        "AMS" => "nl",
        "ZRH" => "ch",
        "ARN" => "se",
        "HEL" => "fi",
        "OSL" => "no",
        "CPH" => "dk",
        "MAD" | "BCN" => "es",
        "MXP" | "FCO" => "it",
        "WAW" => "pl",
        "VIE" => "at",
        "PRG" => "cz",
        "IST" => "tr",
        "DXB" => "ae",
        "TLV" => "il",
        "JNB" | "CPT" => "za",
        "GRU" | "GIG" => "br",
        "EZE" => "ar",
        "SCL" => "cl",
        "BOG" => "co",
        "LIM" => "pe",
        "MEX" => "mx",
        "KUL" => "my",
        "BKK" => "th",
        "SGN" | "HAN" => "vn",
        "MNL" => "ph",
        "BOM" | "MAA" | "DEL" | "BLR" => "in",
        "SVO" | "DME" => "ru",
        "AKL" => "nz",
        _ => {
            if colo.trim().is_empty() {
                return None;
            }
            return Some("unknown");
        }
    };
    Some(key)
}

/// 首选七地区优先；其余有效 colo 归入全球地区；空 colo 返回 None。
pub fn region_for_any_colo(colo: &str) -> Option<&'static str> {
    region_for_colo(colo).or_else(|| global_region_for_colo(colo))
}

pub fn region_options() -> Vec<RegionInfo> {
    REGION_ORDER
        .iter()
        .map(|k| RegionInfo {
            key: (*k).to_string(),
            name: region_name(k).to_string(),
            flag: region_flag(k).to_string(),
            colos: region_colos(k).iter().map(|s| (*s).to_string()).collect(),
        })
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegionInfo {
    pub key: String,
    pub name: String,
    pub flag: String,
    pub colos: Vec<String>,
}

fn default_speed_source() -> String {
    "auto".to_string()
}

fn default_backup_speed_source() -> String {
    "cloudflare".to_string()
}

fn default_true_setting() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppSettings {
    pub default_port: u16,
    pub concurrency: usize,
    pub per_region_top: usize,
    pub latency_limit_ms: f64,
    pub speed_limit_mb_s: f64,
    pub download_seconds: u64,
    pub min_download_bytes: u64,
    pub tcp_timeout_ms: u64,
    pub https_timeout_ms: u64,
    pub latency_rounds: u8,
    pub stability_rounds: u8,
    pub download_threads: usize,
    pub smart_supplement: bool,
    pub auto_detect_network: bool,
    pub auto_refresh_sources: bool,
    pub source_refresh_minutes: u64,
    pub download_url: String,
    pub backup_download_url: String,
    #[serde(default = "default_speed_source")]
    pub speed_source: String,
    #[serde(default = "default_backup_speed_source")]
    pub backup_speed_source: String,
    #[serde(default = "default_true_setting")]
    pub auto_switch_speed_source: bool,
    #[serde(default)]
    pub custom_speed_url: String,
    pub max_candidates: usize,
    pub export_dir: String,
    pub log_level: String,
    pub proxy: String,
    pub fofa_query_template: String,
    pub fofa_email: String,
    pub github_repo: String,
    pub github_default_branch: String,
    pub github_remote_dir: String,
    pub custom_colo_map: String,
    pub auto_upload_after_scan: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            default_port: 443,
            concurrency: 64,
            per_region_top: 20,
            latency_limit_ms: 1500.0,
            speed_limit_mb_s: 1.0,
            download_seconds: 10,
            min_download_bytes: 3 * 1024 * 1024,
            tcp_timeout_ms: 2500,
            https_timeout_ms: 8000,
            latency_rounds: 4,
            stability_rounds: 0,
            download_threads: 4,
            smart_supplement: true,
            auto_detect_network: true,
            auto_refresh_sources: true,
            source_refresh_minutes: 60,
            download_url: "https://speed.cloudflare.com/__down?bytes=104857600".to_string(),
            backup_download_url: "https://speed.cloudflare.com/__down?bytes=52428800".to_string(),
            speed_source: default_speed_source(),
            backup_speed_source: default_backup_speed_source(),
            auto_switch_speed_source: true,
            custom_speed_url: String::new(),
            max_candidates: 1200,
            export_dir: String::new(),
            log_level: "info".to_string(),
            proxy: String::new(),
            fofa_query_template:
                "country=\"{country}\" && port=\"{port}\" && org=\"Cloudflare, Inc.\"".to_string(),
            fofa_email: String::new(),
            github_repo: String::new(),
            github_default_branch: "main".to_string(),
            github_remote_dir: "mikasa-results".to_string(),
            custom_colo_map: String::new(),
            auto_upload_after_scan: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NetworkInfo {
    pub province: String,
    pub carrier: String,
    pub display: String,
    pub proxy_hint: String,
    pub detected_at: String,
    pub source: String,
    pub ok: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub id: i64,
    pub ip: String,
    pub port: u16,
    pub sources: Vec<String>,
    pub initial_region: String,
    pub first_seen: String,
    pub last_seen: String,
    pub last_colo: String,
    pub last_latency: Option<f64>,
    pub last_download_mb_s: Option<f64>,
    pub last_loss: Option<f64>,
    pub last_stability: Option<f64>,
    pub last_status: String,
    pub last_tested: String,
    pub ip_type: String,
    pub ip_version: u8,
    pub success_count: usize,
    pub fail_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CandidatePreview {
    pub ip: String,
    pub port: u16,
    pub region: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub added: usize,
    pub updated: usize,
    pub total: usize,
    pub preview: Vec<CandidatePreview>,
    pub source_counts: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanRequest {
    pub regions: Vec<String>,
    pub per_region_top: usize,
    pub port: u16,
    pub concurrency: usize,
    pub latency_limit_ms: f64,
    pub speed_limit_mb_s: f64,
    pub download_seconds: u64,
    pub min_download_bytes: u64,
    pub latency_rounds: u8,
    pub stability_rounds: u8,
    pub download_threads: usize,
    pub smart_supplement: bool,
    pub max_candidates: usize,
    #[serde(default)]
    pub source_filter: Vec<String>,
    pub download_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RegionStats {
    pub key: String,
    pub name: String,
    pub flag: String,
    pub candidates: usize,
    pub tcp_ok: usize,
    pub https_ok: usize,
    pub colo_ok: usize,
    pub latency_ok: usize,
    pub download_ok: usize,
    pub speed_ok: usize,
    pub final_count: usize,
    pub avg_latency: f64,
    pub avg_speed: f64,
    /// 完成了延迟测试但因超过延迟上限被最终淘汰的数量
    #[serde(default)]
    pub latency_eliminated: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ScanProgress {
    pub scan_id: String,
    pub status: String,
    pub phase: String,
    pub message: String,
    pub tested: usize,
    pub total: usize,
    pub tcp_ok: usize,
    pub https_ok: usize,
    pub colo_ok: usize,
    pub latency_ok: usize,
    pub download_ok: usize,
    pub speed_ok: usize,
    pub final_count: usize,
    pub region_stats: Vec<RegionStats>,
    pub started_at: String,
    pub finished_at: String,
    // ---- 分阶段独立统计，阶段切换时不清零 ----
    #[serde(default)]
    pub tcp_tested: usize,
    #[serde(default)]
    pub tcp_passed: usize,
    #[serde(default)]
    pub https_tested: usize,
    #[serde(default)]
    pub https_passed: usize,
    #[serde(default)]
    pub latency_tested: usize,
    #[serde(default)]
    pub latency_passed: usize,
    #[serde(default)]
    pub latency_eliminated: usize,
    #[serde(default)]
    pub download_tested: usize,
    #[serde(default)]
    pub download_passed: usize,
    #[serde(default)]
    pub final_selected: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub id: i64,
    pub scan_id: String,
    pub rank: usize,
    pub ip: String,
    pub port: u16,
    pub region: String,
    pub region_name: String,
    pub colo: String,
    pub latency: f64,
    pub jitter: f64,
    pub speed_mb_s: f64,
    pub speed_mbps: f64,
    pub loss: f64,
    pub stability: f64,
    pub source: String,
    pub status: String,
    pub tested_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TestRound {
    pub phase: String,
    pub round: usize,
    pub latency_ms: Option<f64>,
    pub speed_mb_s: Option<f64>,
    pub ok: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RetestOutcome {
    pub ok: bool,
    pub message: String,
    pub tcp_ok: bool,
    pub colo: String,
    pub result: Option<ScanResult>,
    pub rounds: Vec<TestRound>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DashboardData {
    pub total_candidates: usize,
    pub total_tested: usize,
    pub avg_latency: f64,
    pub avg_speed: f64,
    pub usable: usize,
    pub latest_scan_id: String,
    pub latest_scan_at: String,
    pub region_stats: Vec<RegionStats>,
    pub recent_results: Vec<ScanResult>,
    pub network: Option<NetworkInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanHistoryRow {
    pub scan_id: String,
    pub started_at: String,
    pub finished_at: String,
    pub status: String,
    pub tested: usize,
    pub final_count: usize,
    pub avg_latency: f64,
    pub avg_speed: f64,
    pub regions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilePreview {
    pub name: String,
    pub content: String,
    pub bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportOutcome {
    pub directory: String,
    pub files: Vec<String>,
    pub count: usize,
    pub format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ExportOptions {
    pub scan_id: String,
    pub directory: String,
    pub format: String,
    pub region: String,
    /// 0 表示不限制；单位 MB/s。
    pub min_speed_mb_s: f64,
    /// 0 表示不限制；单位 ms。
    pub max_latency_ms: f64,
    pub include_speed: bool,
    pub include_latency: bool,
    pub include_city: bool,
    pub include_colo: bool,
    pub include_source: bool,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            scan_id: String::new(),
            directory: String::new(),
            format: "txt".to_string(),
            region: "all".to_string(),
            min_speed_mb_s: 0.0,
            max_latency_ms: 0.0,
            include_speed: true,
            include_latency: false,
            include_city: true,
            include_colo: false,
            include_source: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UrlSource {
    pub id: i64,
    pub url: String,
    pub enabled: bool,
    pub interval_minutes: u64,
    pub last_fetch: String,
    pub last_count: usize,
    pub last_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FofaStatus {
    pub configured: bool,
    pub email: String,
    pub key_masked: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FofaQueryPreview {
    pub region: String,
    pub region_name: String,
    pub country: String,
    pub query: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GithubUploadOutcome {
    pub repo: String,
    pub branch: String,
    pub files: Vec<String>,
    pub commit_sha: String,
    pub verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CensysStatus {
    pub configured: bool,
    pub token_masked: String,
    pub last_fetch: String,
    pub last_count: usize,
    pub last_message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CensysFetchOutcome {
    pub fetched: usize,
    pub total: u64,
    pub message: String,
    pub result: ImportResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultStatus {
    pub fofa_configured: bool,
    pub github_configured: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CmSourceStatus {
    pub status: String,
    pub count: usize,
    pub last_fetch: String,
    pub last_attempt: String,
    pub data_updated_at: String,
    pub last_added: usize,
    pub last_updated: usize,
    pub last_duplicates: usize,
    pub last_invalid: usize,
    pub format: String,
    pub auto_update: bool,
    pub interval_minutes: u64,
    pub cache_used: bool,
    pub error: String,
    pub format_diff: String,
    pub etag: String,
    pub source_url_txt: String,
    pub source_url_json: String,
}

impl Default for CmSourceStatus {
    fn default() -> Self {
        Self {
            status: "未连接".into(),
            count: 0,
            last_fetch: String::new(),
            last_attempt: String::new(),
            data_updated_at: String::new(),
            last_added: 0,
            last_updated: 0,
            last_duplicates: 0,
            last_invalid: 0,
            format: "auto".into(),
            auto_update: true,
            interval_minutes: 1440,
            cache_used: false,
            error: String::new(),
            format_diff: String::new(),
            etag: String::new(),
            source_url_txt: "https://zip.cm.edu.kg/all.txt".into(),
            source_url_json: "https://zip.cm.edu.kg/all.json".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CmFetchOutcome {
    pub added: usize,
    pub updated: usize,
    pub total: usize,
    pub duplicates: usize,
    pub invalid: usize,
    pub count: usize,
    pub format: String,
    pub data_updated_at: String,
    pub fetched_at: String,
    pub cache_used: bool,
    pub message: String,
    pub format_diff: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CmRawPreview {
    pub format: String,
    pub bytes: usize,
    pub lines: usize,
    pub content: String,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SourceOverview {
    pub key: String,
    pub name: String,
    pub count: usize,
    pub fetched: usize,
    pub added: usize,
    pub duplicates: usize,
    pub invalid: usize,
    pub last_update: String,
    pub status: String,
    #[serde(default)]
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SourceQuality {
    pub key: String,
    pub name: String,
    pub raw_count: usize,
    pub unique_count: usize,
    pub duplicate_rate: f64,
    pub valid_connections: usize,
    pub high_speed_count: usize,
    pub region_coverage: usize,
    pub update_frequency: String,
    pub last_success: String,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RegistrySource {
    pub key: String,
    pub name: String,
    pub repo: String,
    pub url: String,
    pub license: String,
    pub branch: String,
    pub kind: String,
    pub note: String,
    pub readme_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredEndpoint {
    pub url: String,
    pub kind: String,
    pub source: String,
    pub allowed: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RegistryDiscovery {
    pub generated_at: String,
    pub sources: Vec<RegistrySource>,
    pub endpoints: Vec<DiscoveredEndpoint>,
    pub total_sources: usize,
    pub total_endpoints: usize,
    pub accepted: usize,
    pub rejected: usize,
    pub warnings: Vec<String>,
}

