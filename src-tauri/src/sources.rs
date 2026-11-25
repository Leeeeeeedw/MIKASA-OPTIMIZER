use crate::db::CandidateSeed;
use crate::models::*;
use base64::Engine;
use futures_util::stream::{self, StreamExt};
use ipnet::IpNet;
use rand::Rng;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::str::FromStr;
use std::time::Duration;

fn http_client(timeout_secs: u64) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .use_rustls_tls()
        .user_agent("MikasaOptimizer/1.0 (+local window utility)")
        .timeout(Duration::from_secs(timeout_secs))
        .connect_timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| format!("创建网络客户端失败: {e}"))
}

#[derive(Debug)]
struct FetchFailure {
    message: String,
    retryable: bool,
}

fn request_error(url: &str, stage: &str, error: reqwest::Error) -> FetchFailure {
    let kind = if error.is_timeout() {
        "请求超时"
    } else if error.is_connect() {
        "连接失败"
    } else if error.is_decode() {
        "响应正文读取或解压中断"
    } else {
        "网络请求失败"
    };
    let source = std::error::Error::source(&error)
        .map(|value| value.to_string())
        .filter(|value| !value.trim().is_empty() && value != &error.to_string());
    FetchFailure {
        message: format!(
            "{stage} {url} 失败（{kind}）: {error}{}",
            source
                .map(|value| format!("；底层原因: {value}"))
                .unwrap_or_default()
        ),
        retryable: true,
    }
}

fn is_public_candidate(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v) => {
            !(v.is_private()
                || v.is_loopback()
                || v.is_link_local()
                || v.is_broadcast()
                || v.is_unspecified()
                || v.is_documentation()
                || v.octets()[0] == 0)
        }
        IpAddr::V6(v) => {
            !(v.is_loopback()
                || v.is_unspecified()
                || (v.segments()[0] & 0xfe00) == 0xfc00
                || (v.segments()[0] & 0xffc0) == 0xfe80)
        }
    }
}

fn parse_host_port(value: &str, default_port: u16) -> Option<(String, u16)> {
    let token = value
        .trim()
        .trim_matches(|c| c == '"' || c == '\'' || c == ';' || c == ',')
        .split_whitespace()
        .next()?
        .to_string();
    if token.is_empty() {
        return None;
    }
    if let Some(rest) = token.strip_prefix('[') {
        let (inside, after) = rest.split_once(']')?;
        let ip = IpAddr::from_str(inside).ok()?;
        if !is_public_candidate(&ip) {
            return None;
        }
        let port = after
            .strip_prefix(':')
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(default_port);
        return Some((ip.to_string(), port));
    }
    if let Ok(ip) = IpAddr::from_str(&token) {
        if !is_public_candidate(&ip) {
            return None;
        }
        return Some((ip.to_string(), default_port));
    }
    if let Some((host, port)) = token.rsplit_once(':') {
        if let (Ok(ip), Ok(p)) = (IpAddr::from_str(host), port.parse::<u16>()) {
            if !is_public_candidate(&ip) {
                return None;
            }
            return Some((ip.to_string(), p));
        }
    }
    None
}

fn push_seed(
    out: &mut Vec<CandidateSeed>,
    seen: &mut HashSet<String>,
    ip: &str,
    port: u16,
    source: &str,
    region: &str,
) {
    let Ok(addr) = IpAddr::from_str(ip) else {
        return;
    };
    if !is_public_candidate(&addr) {
        return;
    }
    let key = format!("{}:{}", addr, port);
    if seen.insert(key) {
        out.push(CandidateSeed {
            ip: addr.to_string(),
            port,
            source: source.to_string(),
            region: region.to_string(),
        });
    }
}

fn collect_json_value(
    value: &Value,
    default_port: u16,
    source: &str,
    region: &str,
    out: &mut Vec<CandidateSeed>,
    seen: &mut HashSet<String>,
) {
    match value {
        Value::Object(map) => {
            let ip_text = ["ip", "host", "address", "server", "ip_address"]
                .iter()
                .find_map(|key| map.get(*key).and_then(Value::as_str));
            if let Some(ip) = ip_text {
                let port = map
                    .get("port")
                    .and_then(|value| value.as_u64())
                    .map(|value| value as u16)
                    .or_else(|| map.get("port").and_then(Value::as_str).and_then(|value| value.parse().ok()))
                    .unwrap_or(default_port);
                let token = if IpAddr::from_str(ip).map(|addr| addr.is_ipv6()).unwrap_or(false) {
                    format!("[{ip}]:{port}")
                } else {
                    format!("{ip}:{port}")
                };
                if let Some((parsed_ip, parsed_port)) = parse_host_port(&token, default_port) {
                    push_seed(out, seen, &parsed_ip, parsed_port, source, region);
                }
            }
            for (key, child) in map {
                if ip_text.is_some() && matches!(key.as_str(), "ip" | "host" | "address" | "server" | "ip_address" | "port") {
                    continue;
                }
                collect_json_value(child, default_port, source, region, out, seen);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_json_value(item, default_port, source, region, out, seen);
            }
        }
        Value::String(text) => {
            for token in text.split(|c: char| c.is_whitespace() || matches!(c, ',' | ';' | '|')) {
                if let Some((ip, port)) = parse_host_port(token, default_port) {
                    push_seed(out, seen, &ip, port, source, region);
                }
            }
        }
        _ => {}
    }
}

pub fn parse_candidate_text(text: &str, default_port: u16, source: &str, region: &str) -> Vec<CandidateSeed> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    // JSON / JSONL endpoints: parse structured values first. This covers API
    // responses such as `{"ip":"1.2.3.4","port":443}` and arrays of strings.
    if let Ok(value) = serde_json::from_str::<Value>(text.trim()) {
        collect_json_value(&value, default_port, source, region, &mut out, &mut seen);
        return out;
    }
    for raw_line in text.replace('\u{feff}', "").lines() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with('{') || trimmed.starts_with('[') {
            if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
                collect_json_value(&value, default_port, source, region, &mut out, &mut seen);
                continue;
            }
        }
        let content = trimmed.split('#').next().unwrap_or("").trim();
        if content.is_empty() {
            continue;
        }
        // Support CSV: only treat the second cell as a port when it is a plausible
        // listening port. IPDB's CSV uses columns like `IP,已发送,已接收,...`, where
        // the second cell is a small packet count (e.g. 4) and must not become `:4`.
        let cells: Vec<&str> = content.split(',').map(|s| s.trim()).collect();
        let csv_port = cells
            .get(1)
            .and_then(|value| value.trim_matches('"').parse::<u16>().ok())
            .filter(|port| (80..=65535).contains(port));
        let primary = if let Some(port) = csv_port {
            format!("{}:{}", cells[0].trim_matches('"'), port)
        } else {
            cells[0].trim_matches('"').to_string()
        };
        // GitHub Raw 的 ip.txt / ipv6.txt 常为 CIDR 段：每个段取 1 个可复现样本即可。
        if let Ok(net) = primary.parse::<IpNet>() {
            let samples: Vec<String> = match net {
                IpNet::V4(_) => sample_v4(&net, 1).into_iter().map(|ip| ip.to_string()).collect(),
                IpNet::V6(_) => sample_v6(&net, 1).into_iter().map(|ip| ip.to_string()).collect(),
            };
            for ip in samples {
                if IpAddr::from_str(&ip).map(|value| !is_public_candidate(&value)).unwrap_or(true) {
                    continue;
                }
                let key = format!("{ip}:{default_port}");
                if seen.insert(key) {
                    out.push(CandidateSeed {
                        ip,
                        port: default_port,
                        source: source.to_string(),
                        region: region.to_string(),
                    });
                }
            }
            continue;
        }
        if let Some((ip, port)) = parse_host_port(&primary, default_port) {
            let key = format!("{ip}:{port}");
            if seen.insert(key) {
                out.push(CandidateSeed {
                    ip,
                    port,
                    source: source.to_string(),
                    region: region.to_string(),
                });
            }
        }
    }
    out
}

async fn fetch_url_text_once(url: &str, max_bytes: usize) -> Result<String, FetchFailure> {
    let parsed = url::Url::parse(url).map_err(|_| FetchFailure {
        message: format!("链接格式无效: {url}"),
        retryable: false,
    })?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(FetchFailure {
            message: "仅支持 http/https 链接".to_string(),
            retryable: false,
        });
    }
    let client = http_client(25).map_err(|message| FetchFailure {
        message,
        retryable: false,
    })?;
    let resp = client
        .get(url)
        .header("Accept", "text/plain,text/csv,application/json,*/*")
        // 文本列表体积很小，关闭压缩可以规避代理或 HTTP/2 中途截断导致的
        // "error decoding response body" 假象。
        .header("Accept-Encoding", "identity")
        .send()
        .await
        .map_err(|error| request_error(url, "请求", error))?;
    let status = resp.status();
    if !status.is_success() {
        let retryable = status.as_u16() == 429 || status.is_server_error();
        let hint = match status.as_u16() {
            404 => "，链接不存在。BestCF 的 /random-region/IP/all.txt 是无效路径，请改用 /random-region/mix.txt 或 /random-region/US/all.txt 这类真实地区代码",
            401 | 403 => "，可能需要登录、Token 权限不足或被站点限制",
            429 => "，请求过于频繁，请稍后重试",
            _ => "",
        };
        return Err(FetchFailure {
            message: format!("请求 {url} 返回 HTTP {status}{hint}"),
            retryable,
        });
    }
    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();
    let bytes = resp
        .bytes()
        .await
        .map_err(|error| request_error(url, "读取", error))?;
    if bytes.len() > max_bytes {
        return Err(FetchFailure {
            message: format!("{url} 响应过大（{} 字节），已拒绝", bytes.len()),
            retryable: false,
        });
    }
    if bytes.is_empty() {
        return Err(FetchFailure {
            message: format!("{url} 返回空内容"),
            retryable: true,
        });
    }
    if content_type.contains("text/html") {
        return Err(FetchFailure {
            message: format!("{url} 返回 HTML 页面而不是 IP 文本，可能被登录页、验证页或错误页替代"),
            retryable: false,
        });
    }
    let text = String::from_utf8_lossy(&bytes).to_string();
    let head = text.trim_start().to_ascii_lowercase();
    if head.starts_with("<!doctype html") || head.starts_with("<html") {
        return Err(FetchFailure {
            message: format!("{url} 返回 HTML 页面而不是 IP 文本"),
            retryable: false,
        });
    }
    Ok(text)
}

pub async fn fetch_url_text(url: &str, max_bytes: usize) -> Result<String, String> {
    let mut last_error = None;
    for attempt in 0..3u64 {
        match fetch_url_text_once(url, max_bytes).await {
            Ok(text) => return Ok(text),
            Err(error) => {
                let should_retry = error.retryable && attempt < 2;
                last_error = Some(error);
                if !should_retry {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(350 + attempt * 550)).await;
            }
        }
    }
    Err(last_error
        .map(|error| error.message)
        .unwrap_or_else(|| format!("请求 {url} 失败")))
}

pub async fn fetch_url_candidates(urls: &[String], default_port: u16) -> Result<Vec<CandidateSeed>, String> {
    let mut out = Vec::new();
    for url in urls {
        let text = fetch_url_text(url, 8 * 1024 * 1024).await?;
        out.extend(parse_candidate_text(&text, default_port, "url", ""));
    }
    Ok(out)
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommunitySourceInfo {
    pub key: String,
    pub name: String,
    pub repo: String,
    pub license: String,
    pub frequency: String,
    pub note: String,
}

pub const COMMUNITY_SOURCE_KEYS: [&str; 7] = [
    "cf_best_ip",
    "bestcf",
    "best_cf_ips",
    "ipdb",
    "cloudflare_ip",
    "free_nodes_bestcf",
    "xiu2",
];

pub fn community_sources() -> Vec<CommunitySourceInfo> {
    [
        ("cf_best_ip", "cf-best-ip 上游聚合", "LeilaoMi/cf-best-ip", "MIT", "上游不定期更新", "Worker 项目无固定公开实例；读取其开源 Worker 中的公开上游文件集合"),
        ("bestcf", "BestCF（DustinWin）", "DustinWin/BestCF", "未标注（仅公开数据）", "随仓库更新", "读取 bestcf 分支的移动/联通/电信/统一/代理五个文件"),
        ("best_cf_ips", "best-cf-ips（LancelotRar）", "LancelotRar/best-cf-ips", "未标注（仅公开数据）", "每 6 小时", "读取完整扫描榜 top400，不读取只含前几十名的排名片段"),
        ("ipdb", "IPDB（ymyuuu）", "ymyuuu/IPDB", "MIT", "每日更新", "读取 bestcf 列表与 BestCF/ipv4.csv、ipv6.csv"),
        ("cloudflare_ip", "cloudflare_ip（svip-s）", "svip-s/cloudflare_ip", "未标注（仅公开数据）", "每日 02:00", "读取 full_ips.txt 与 best_ips.txt，并保留镜像回退"),
        ("free_nodes_bestcf", "free-nodes/bestcf 上游源", "free-nodes/bestcf", "未标注（仅公开数据）", "多子源不定时", "读取 bestcf.tools12345.com 下运营商与社区子源，过滤私网地址"),
        ("xiu2", "XIU2/CloudflareSpeedTest", "XIU2/CloudflareSpeedTest", "GPL-3.0", "随仓库更新", "读取官方示例 IPv4/IPv6 CIDR，并抽样为候选"),
    ]
    .into_iter()
    .map(|(key, name, repo, license, frequency, note)| CommunitySourceInfo {
        key: key.to_string(),
        name: name.to_string(),
        repo: repo.to_string(),
        license: license.to_string(),
        frequency: frequency.to_string(),
        note: note.to_string(),
    })
    .collect()
}

/// Each inner vector is one logical file with an ordered list of mirrors.
/// A mirror is tried only when the previous one fails, so a healthy primary does
/// not cause duplicate downloads.
fn community_urls(key: &str) -> Option<Vec<Vec<&'static str>>> {
    Some(match key {
        "cf_best_ip" => vec![
            vec!["https://raw.githubusercontent.com/joname1/BestCFip/main/ipv4.txt"],
            vec!["https://raw.githubusercontent.com/joname1/BestCFip/main/ipv6.txt"],
            vec!["https://raw.githubusercontent.com/KafeMars/best-ips-domains/main/cf-bestips.txt"],
            vec![
                "https://bestcf.pages.dev/uouin/all.txt",
                "https://bestcf.eu.cc/uouin/all.txt",
                "https://bestcf.tools12345.com/uouin/all.txt",
            ],
            vec![
                "https://bestcf.pages.dev/s5gy/all.txt",
                "https://bestcf.eu.cc/s5gy/all.txt",
                "https://bestcf.tools12345.com/s5gy/all.txt",
            ],
            vec![
                "https://bestcf.pages.dev/lzj/all.txt",
                "https://bestcf.eu.cc/lzj/all.txt",
                "https://bestcf.tools12345.com/lzj/all.txt",
            ],
            vec![
                "https://bestcf.pages.dev/nirevil/ipv4.txt",
                "https://bestcf.eu.cc/nirevil/ipv4.txt",
                "https://bestcf.tools12345.com/nirevil/ipv4.txt",
            ],
        ],
        "bestcf" => vec![
            vec![
                "https://cdn.jsdelivr.net/gh/DustinWin/BestCF@bestcf/bestcf-ip.txt",
                "https://raw.githubusercontent.com/DustinWin/BestCF/bestcf/bestcf-ip.txt",
            ],
            vec![
                "https://cdn.jsdelivr.net/gh/DustinWin/BestCF@bestcf/cmcc-ip.txt",
                "https://raw.githubusercontent.com/DustinWin/BestCF/bestcf/cmcc-ip.txt",
            ],
            vec![
                "https://cdn.jsdelivr.net/gh/DustinWin/BestCF@bestcf/cucc-ip.txt",
                "https://raw.githubusercontent.com/DustinWin/BestCF/bestcf/cucc-ip.txt",
            ],
            vec![
                "https://cdn.jsdelivr.net/gh/DustinWin/BestCF@bestcf/ctcc-ip.txt",
                "https://raw.githubusercontent.com/DustinWin/BestCF/bestcf/ctcc-ip.txt",
            ],
            vec![
                "https://cdn.jsdelivr.net/gh/DustinWin/BestCF@bestcf/proxy-ip.txt",
                "https://raw.githubusercontent.com/DustinWin/BestCF/bestcf/proxy-ip.txt",
            ],
        ],
        "best_cf_ips" => vec![vec![
            "https://cdn.jsdelivr.net/gh/LancelotRar/best-cf-ips@main/best-cf-ip-scanned-top400.txt",
            "https://raw.githubusercontent.com/LancelotRar/best-cf-ips/main/best-cf-ip-scanned-top400.txt",
        ]],
        "ipdb" => vec![
            vec!["https://ipdb.api.030101.xyz/?type=bestcf"],
            vec![
                "https://cdn.jsdelivr.net/gh/ymyuuu/IPDB@main/BestCF/ipv4.csv",
                "https://raw.githubusercontent.com/ymyuuu/IPDB/main/BestCF/ipv4.csv",
            ],
            vec![
                "https://cdn.jsdelivr.net/gh/ymyuuu/IPDB@main/BestCF/ipv6.csv",
                "https://raw.githubusercontent.com/ymyuuu/IPDB/main/BestCF/ipv6.csv",
            ],
        ],
        "cloudflare_ip" => vec![
            vec![
                "https://ips.gaoji.uk/full_ips.txt",
                "https://cdn.jsdelivr.net/gh/svip-s/cloudflare_ip@main/full_ips.txt",
                "https://raw.githubusercontent.com/svip-s/cloudflare_ip/main/full_ips.txt",
            ],
            vec![
                "https://ips.gaoji.uk/best_ips.txt",
                "https://cdn.jsdelivr.net/gh/svip-s/cloudflare_ip@main/best_ips.txt",
                "https://raw.githubusercontent.com/svip-s/cloudflare_ip/main/best_ips.txt",
            ],
        ],
        "free_nodes_bestcf" => vec![
            vec!["https://bestcf.tools12345.com/tiancheng/all.txt"],
            vec!["https://bestcf.tools12345.com/s5gy/all.txt"],
            vec!["https://bestcf.tools12345.com/wetest/ipv4.txt"],
            vec!["https://bestcf.tools12345.com/uouin/all.txt"],
            vec!["https://bestcf.tools12345.com/cfyes/ipv4.txt"],
            vec!["https://bestcf.tools12345.com/cfyes/ipv6.txt"],
            vec!["https://bestcf.tools12345.com/cmliu/all.txt"],
            vec!["https://bestcf.tools12345.com/cmliu2/all.txt"],
            vec!["https://bestcf.tools12345.com/vvhan/ipv4.txt"],
            vec!["https://bestcf.tools12345.com/vvhan/ipv6.txt"],
            vec!["https://bestcf.tools12345.com/nirevil/ipv4.txt"],
            vec!["https://bestcf.tools12345.com/nirevil/ipv6.txt"],
            vec!["https://bestcf.tools12345.com/gslege/Cfxyz.txt"],
            vec!["https://bestcf.tools12345.com/zhixuanwang/ipv4-onlyip.txt"],
            vec!["https://bestcf.tools12345.com/lajiao/all.txt"],
            vec!["https://bestcf.tools12345.com/moistr/all.txt"],
        ],
        "xiu2" => vec![
            vec![
                "https://cdn.jsdelivr.net/gh/XIU2/CloudflareSpeedTest@master/ip.txt",
                "https://raw.githubusercontent.com/XIU2/CloudflareSpeedTest/master/ip.txt",
            ],
            vec![
                "https://cdn.jsdelivr.net/gh/XIU2/CloudflareSpeedTest@master/ipv6.txt",
                "https://raw.githubusercontent.com/XIU2/CloudflareSpeedTest/master/ipv6.txt",
            ],
        ],
        _ => return None,
    })
}

/// 拉取一个社区聚合源。单文件失败只记录错误，其他文件继续；返回去重后的候选。
pub async fn community_candidates(
    source: &str,
    max_results: usize,
) -> Result<(Vec<CandidateSeed>, usize, Vec<String>), String> {
    let urls = community_urls(source)
        .ok_or_else(|| format!("未知社区源: {source}"))?;
    let cap = max_results.clamp(1, 20_000);
    let source_name = community_sources()
        .into_iter()
        .find(|s| s.key == source)
        .map(|s| s.name)
        .unwrap_or_else(|| source.to_string());
    let mut out: Vec<CandidateSeed> = Vec::new();
    let mut seen = HashSet::new();
    let mut errors = Vec::new();
    let mut files_ok = 0usize;
    let url_groups: Vec<Vec<String>> = urls
        .into_iter()
        .map(|mirrors| mirrors.into_iter().map(str::to_string).collect())
        .collect();
    let results = stream::iter(url_groups.into_iter().map(|mirrors: Vec<String>| async move {
        let mut last_error = String::new();
        for url in mirrors {
            match fetch_url_text(&url, 8 * 1024 * 1024).await {
                Ok(text) => return Ok(text),
                Err(error) => last_error = format!("{url}: {error}"),
            }
        }
        Err(last_error)
    }))
    .buffer_unordered(4)
    .collect::<Vec<Result<String, String>>>()
    .await;
    for result in results {
        match result {
            Ok(text) => {
                files_ok += 1;
                for seed in parse_candidate_text(&text, 443, source, "") {
                    let key = format!("{}:{}", seed.ip, seed.port);
                    if seen.insert(key) {
                        out.push(seed);
                        if out.len() >= cap {
                            break;
                        }
                    }
                }
            }
            Err(error) if !error.is_empty() => errors.push(error),
            Err(_) => {}
        }
    }
    if out.is_empty() {
        return Err(if errors.is_empty() {
            format!("{source_name} 未返回可用 IP")
        } else {
            format!("{source_name} 获取失败：{}", errors.join("；"))
        });
    }
    Ok((out, files_ok, errors))
}

pub fn build_fofa_query(template: &str, country: &str, port: u16) -> String {
    template
        .replace("{country}", country)
        .replace("{port}", &port.to_string())
        .replace("{org}", "Cloudflare, Inc.")
}

pub fn fofa_preview(settings: &AppSettings, regions: &[String]) -> Vec<FofaQueryPreview> {
    let selected: Vec<String> = if regions.iter().any(|r| r == "all") {
        REGION_ORDER.iter().map(|s| s.to_string()).collect()
    } else {
        regions.to_vec()
    };
    selected
        .into_iter()
        .filter_map(|key| {
            let country = match key.as_str() {
                "hk" => "HK",
                "tw" => "TW",
                "jp" => "JP",
                "sg" => "SG",
                "kr" => "KR",
                "fr" => "FR",
                "us" => "US",
                _ => return None,
            };
            let region_name = region_name(&key).to_string();
            Some(FofaQueryPreview {
                region: key.clone(),
                region_name,
                country: country.to_string(),
                query: build_fofa_query(&settings.fofa_query_template, country, settings.default_port),
            })
        })
        .collect()
}

pub async fn fofa_search(
    email: &str,
    api_key: &str,
    settings: &AppSettings,
    regions: &[String],
    max_results: usize,
) -> Result<(Vec<CandidateSeed>, usize), String> {
    if email.trim().is_empty() || api_key.trim().is_empty() {
        return Err("请先在「IP 来源 → FOFA」中配置邮箱与 API Key".to_string());
    }
    let selected: Vec<String> = if regions.iter().any(|r| r == "all") {
        REGION_ORDER.iter().map(|s| s.to_string()).collect()
    } else {
        regions.to_vec()
    };
    if selected.is_empty() {
        return Err("请至少选择一个地区".to_string());
    }
    let per_region = (max_results / selected.len().max(1)).clamp(20, 1000);
    let client = http_client(30)?;
    let mut all = Vec::new();
    let mut requests = 0usize;
    for key in selected {
        let country = match key.as_str() {
            "hk" => "HK",
            "tw" => "TW",
            "jp" => "JP",
            "sg" => "SG",
            "kr" => "KR",
            "fr" => "FR",
            "us" => "US",
            _ => continue,
        };
        let query = build_fofa_query(&settings.fofa_query_template, country, settings.default_port);
        let encoded = base64::engine::general_purpose::STANDARD.encode(query.as_bytes());
        let mut fetched = 0usize;
        let mut page = 1usize;
        while fetched < per_region && page <= 5 {
            let size = (per_region - fetched).min(1000);
            let resp = client
                .get("https://fofa.info/api/v1/search/all")
                .query(&[
                    ("email", email),
                    ("key", api_key),
                    ("qbase64", &encoded),
                    ("fields", "ip,port,country,country_name"),
                    ("page", &page.to_string()),
                    ("size", &size.to_string()),
                ])
                .send()
                .await
                .map_err(|e| format!("FOFA 请求失败: {e}"))?;
            let status = resp.status();
            let body: serde_json::Value = resp
                .json()
                .await
                .map_err(|e| format!("FOFA 响应解析失败（HTTP {status}）: {e}"))?;
            if body.get("error").and_then(|v| v.as_bool()).unwrap_or(false) {
                let msg = body
                    .get("errmsg")
                    .and_then(|v| v.as_str())
                    .unwrap_or("未知错误");
                return Err(format!("FOFA API 错误: {msg}"));
            }
            let results = body
                .get("results")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            if results.is_empty() {
                break;
            }
            for row in &results {
                let (ip, port) = if let Some(arr) = row.as_array() {
                    let ip = arr.first().and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let port = arr
                        .get(1)
                        .and_then(|v| v.as_str())
                        .and_then(|s| s.parse::<u16>().ok())
                        .or_else(|| arr.get(1).and_then(|v| v.as_u64()).map(|n| n as u16))
                        .unwrap_or(settings.default_port);
                    (ip, port)
                } else if let Some(obj) = row.as_object() {
                    let ip = obj.get("ip").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let port = obj
                        .get("port")
                        .and_then(|v| v.as_str())
                        .and_then(|s| s.parse::<u16>().ok())
                        .or_else(|| obj.get("port").and_then(|v| v.as_u64()).map(|n| n as u16))
                        .unwrap_or(settings.default_port);
                    (ip, port)
                } else {
                    continue;
                };
                if ip.parse::<IpAddr>().is_ok() {
                    all.push(CandidateSeed {
                        ip,
                        port,
                        source: "fofa".to_string(),
                        region: key.clone(),
                    });
                    fetched += 1;
                }
            }
            requests += 1;
            if results.len() < size {
                break;
            }
            page += 1;
        }
    }
    if all.is_empty() {
        return Err("FOFA 返回 0 条可用 IP。请检查账号权限、查询语法或配额".to_string());
    }
    Ok((all, requests))
}


/// Censys Platform API（Global Data Search）真实查询。
/// 文档：https://docs.censys.com/reference/v3-globaldata-search-query
/// 只把返回的 host.ip 当候选，绝不把 Censys 的 location 当 Cloudflare colo。
pub async fn censys_search(
    token: &str,
    max_results: usize,
) -> Result<(Vec<CandidateSeed>, u64), String> {
    if token.trim().is_empty() {
        return Err("请先配置 Censys Personal Access Token".to_string());
    }
    let page_size = max_results.clamp(1, 100);
    let body = json!({
        "query": "host.services.port: 443 and host.autonomous_system.asn: 13335",
        "page_size": page_size
    });
    let client = http_client(60)?;
    let resp = client
        .post("https://api.platform.censys.io/v3/global/search/query")
        .header("Authorization", format!("Bearer {}", token.trim()))
        .header("Accept", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Censys 请求失败: {e}"))?;
    let status = resp.status();
    let text = resp.text().await.map_err(|e| format!("Censys 响应读取失败: {e}"))?;
    if !status.is_success() {
        let hint = match status.as_u16() {
            401 | 403 => "Token 无效、无权限或套餐不支持 Platform API",
            402 => "Censys credits 余额不足",
            429 => "Censys 请求频率超限",
            _ => "Censys API 返回错误",
        };
        return Err(format!(
            "Censys API 返回 HTTP {}（{}）：{}",
            status.as_u16(),
            hint,
            text.chars().take(240).collect::<String>()
        ));
    }
    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("Censys JSON 解析失败: {e}"))?;
    let total = value
        .pointer("/result/total")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let hits = value
        .pointer("/result/hits")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let mut seen = HashSet::new();
    let mut seeds = Vec::new();
    for hit in hits {
        let ip = hit
            .pointer("/host/ip")
            .and_then(|v| v.as_str())
            .or_else(|| hit.get("ip").and_then(|v| v.as_str()))
            .unwrap_or("")
            .to_string();
        if ip.parse::<IpAddr>().is_err() || !seen.insert(ip.clone()) {
            continue;
        }
        seeds.push(CandidateSeed {
            ip,
            port: 443,
            source: "censys".to_string(),
            region: String::new(),
        });
    }
    if seeds.is_empty() {
        return Err(format!(
            "Censys 返回 0 条可用 host（total={}）。请检查 Token 套餐与查询权限",
            total
        ));
    }
    Ok((seeds, total))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ipv4_ipv6_ports_csv_and_dedupes() {
        let text = "\u{feff}1.1.1.1\n1.1.1.1:443\n2606:4700:4700::1111\n[2606:4700:4700::1111]:8443\n\"8.8.8.8\",2053\n# comment\ninvalid-value\n1.1.1.1:443#remark\n";
        let rows = parse_candidate_text(text, 443, "local", "hk");
        assert_eq!(rows.len(), 4, "expected 4 unique IP:port rows, got {rows:?}");
        assert!(rows.iter().any(|r| r.ip == "1.1.1.1" && r.port == 443));
        assert!(rows.iter().any(|r| r.ip == "2606:4700:4700::1111" && r.port == 8443));
        assert!(rows.iter().any(|r| r.ip == "8.8.8.8" && r.port == 2053));
        assert_eq!(rows.iter().filter(|r| r.ip == "1.1.1.1" && r.port == 443).count(), 1);
    }

    #[test]
    fn rejects_sent_count_as_port_and_private_addresses() {
        let text = "IP 地址,已发送,已接收,丢包率,平均延迟,下载速度 (MB/s)\n104.18.14.114,4,4,0.00,0.00,0.00\n192.168.1.1:443\n127.0.0.1:443\n1.1.1.1:8443#US\n";
        let rows = parse_candidate_text(text, 443, "ipdb", "");
        assert_eq!(rows.len(), 2, "expected public IPs only, got {rows:?}");
        assert!(rows.iter().any(|r| r.ip == "104.18.14.114" && r.port == 443));
        assert!(rows.iter().any(|r| r.ip == "1.1.1.1" && r.port == 8443));
    }

    #[test]
    fn samples_one_candidate_per_cidr() {
        let text = "173.245.48.0/20\n2606:4700::/32\n173.245.48.0/20\n";
        let rows = parse_candidate_text(text, 443, "github", "us");
        assert_eq!(rows.len(), 3, "expected one sample per CIDR line, got {rows:?}");
        assert!(rows.iter().all(|r| r.port == 443));
        assert!(rows.iter().any(|r| r.ip.starts_with("173.245.")));
        assert!(rows.iter().any(|r| r.ip.contains(':')));
    }

    #[test]
    fn parses_json_and_jsonl_candidates() {
        let text = r#"[{"ip":"104.18.1.1","port":8443},{"host":"1.1.1.1"},{"address":"2606:4700::1111","port":"2053"}]"#;
        let rows = parse_candidate_text(text, 443, "ipdb", "");
        assert_eq!(rows.len(), 3, "expected three JSON rows, got {rows:?}");
        assert!(rows.iter().any(|r| r.ip == "104.18.1.1" && r.port == 8443));
        assert!(rows.iter().any(|r| r.ip == "1.1.1.1" && r.port == 443));
        assert!(rows.iter().any(|r| r.ip == "2606:4700::1111" && r.port == 2053));

        let jsonl = "{\"ip\":\"104.18.2.2\",\"port\":2053}\n{\"ip\":\"104.18.3.3\"}\n";
        let rows = parse_candidate_text(jsonl, 443, "ipdb", "");
        assert_eq!(rows.len(), 2, "expected two JSONL rows, got {rows:?}");
        assert!(rows.iter().any(|r| r.ip == "104.18.2.2" && r.port == 2053));
        assert!(rows.iter().any(|r| r.ip == "104.18.3.3" && r.port == 443));
    }

    #[test]
    fn builds_fofa_query_with_placeholders() {
        let query = build_fofa_query(
            r#"country="{country}" && port="{port}" && org="Cloudflare, Inc.""#,
            "JP",
            443,
        );
        assert_eq!(query, r#"country="JP" && port="443" && org="Cloudflare, Inc.""#);
    }
}

pub async fn fetch_cloudflare_cidrs() -> Result<(Vec<IpNet>, Vec<IpNet>), String> {
    let client = http_client(20)?;
    let mut v4 = Vec::new();
    let mut v6 = Vec::new();
    for (url, target) in [
        ("https://www.cloudflare.com/ips-v4", &mut v4),
        ("https://www.cloudflare.com/ips-v6", &mut v6),
    ] {
        match client.get(url).send().await {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(text) = resp.text().await {
                    for line in text.lines() {
                        if let Ok(net) = line.trim().parse::<IpNet>() {
                            target.push(net);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    if v4.is_empty() {
        for cidr in [
            "173.245.48.0/20", "103.21.244.0/22", "103.22.200.0/22", "103.31.4.0/22",
            "141.101.64.0/18", "108.162.192.0/18", "190.93.240.0/20", "188.114.96.0/20",
            "197.234.240.0/22", "198.41.128.0/17", "162.158.0.0/15", "104.16.0.0/13",
            "104.24.0.0/14", "172.64.0.0/13", "131.0.72.0/22",
        ] {
            if let Ok(net) = cidr.parse() {
                v4.push(net);
            }
        }
    }
    if v6.is_empty() {
        for cidr in ["2606:4700::/32", "2803:f800::/32", "2405:b500::/32", "2405:8100::/32", "2a06:98c0::/29", "2c0f:f248::/32"] {
            if let Ok(net) = cidr.parse() {
                v6.push(net);
            }
        }
    }
    Ok((v4, v6))
}

fn sample_v4(net: &IpNet, count: usize) -> Vec<Ipv4Addr> {
    let IpNet::V4(v4) = net else { return Vec::new() };
    let network = u32::from(v4.network());
    let prefix = v4.prefix_len();
    let host_bits = 32 - prefix as u32;
    let host_count = if host_bits >= 32 { u32::MAX as u64 } else { 1u64 << host_bits };
    let usable = host_count.saturating_sub(2).max(1);
    let mut rng = rand::thread_rng();
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    let attempts = (count * 8).max(32);
    for _ in 0..attempts {
        if out.len() >= count { break; }
        let offset = if usable > 1 { rng.gen_range(1..usable) } else { 0 };
        let value = network.wrapping_add(offset as u32);
        if seen.insert(value) {
            out.push(Ipv4Addr::from(value));
        }
    }
    out
}

fn sample_v6(net: &IpNet, count: usize) -> Vec<Ipv6Addr> {
    let IpNet::V6(v6) = net else { return Vec::new() };
    let network = u128::from(v6.network());
    let prefix = v6.prefix_len();
    let host_bits = 128 - prefix as u32;
    let max_offset = if host_bits >= 64 { u64::MAX as u128 } else { (1u128 << host_bits) - 1 };
    let mut rng = rand::thread_rng();
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    let attempts = (count * 8).max(32);
    for _ in 0..attempts {
        if out.len() >= count { break; }
        let offset = rng.gen_range(0..=max_offset.max(1));
        let value = network.wrapping_add(offset);
        if seen.insert(value) {
            out.push(Ipv6Addr::from(value));
        }
    }
    out
}

pub async fn cloudflare_candidates(ipv4: bool, ipv6: bool, sample_per_cidr: usize) -> Result<(Vec<CandidateSeed>, usize), String> {
    let (v4, v6) = fetch_cloudflare_cidrs().await?;
    let mut out = Vec::new();
    if ipv4 {
        for net in &v4 {
            for ip in sample_v4(net, sample_per_cidr.max(1)) {
                out.push(CandidateSeed {
                    ip: ip.to_string(),
                    port: 443,
                    source: "cloudflare".to_string(),
                    region: String::new(),
                });
            }
        }
    }
    if ipv6 {
        for net in &v6 {
            for ip in sample_v6(net, sample_per_cidr.max(1)) {
                out.push(CandidateSeed {
                    ip: ip.to_string(),
                    port: 443,
                    source: "cloudflare".to_string(),
                    region: String::new(),
                });
            }
        }
    }
    let segments = v4.len() + v6.len();
    Ok((out, segments))
}

