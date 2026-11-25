use crate::models::{DiscoveredEndpoint, RegistryDiscovery, RegistrySource};
use crate::sources::fetch_url_text;
use base64::Engine;
use futures_util::stream::{self, StreamExt};
use regex::Regex;
use std::collections::HashSet;
use std::net::IpAddr;
use std::str::FromStr;

const ALLOWED_HOSTS: [&str; 13] = [
    "raw.githubusercontent.com",
    "github.com",
    "api.github.com",
    "cdn.jsdelivr.net",
    "bestcf.tools12345.com",
    "bestcf.pages.dev",
    "bestcf.eu.cc",
    "ips.gaoji.uk",
    "ips.de5.net",
    "ipdb.api.030101.xyz",
    "cf.junzhen.qzz.io",
    "addressesapi.090227.xyz",
    "ip.164746.xyz",
];

pub fn registry_sources() -> Vec<RegistrySource> {
    [
        ("cf_best_ip", "cf-best-ip 上游聚合", "LeilaoMi/cf-best-ip", "MIT", "main", "worker-aggregator", "Cloudflare Worker 聚合项目；不抓取固定示例域名，读取其公开上游文件", "https://raw.githubusercontent.com/LeilaoMi/cf-best-ip/main/README.md"),
        ("ipdb", "IPDB（ymyuuu）", "ymyuuu/IPDB", "MIT", "main", "data-repository", "提供 bestcf API、IPv4/IPv6 CSV 与大量分类目录", "https://raw.githubusercontent.com/ymyuuu/IPDB/main/README.md"),
        ("free_nodes_bestcf", "free-nodes/bestcf", "free-nodes/bestcf", "未标注（仅公开数据）", "main", "data-aggregator", "提供多个运营商/社区子源的公开 TXT 数据", "https://raw.githubusercontent.com/free-nodes/bestcf/main/README.md"),
        ("bestcf", "BestCF（DustinWin）", "DustinWin/BestCF", "未标注（仅公开数据）", "bestcf", "data-repository", "bestcf 分支提供移动/联通/电信/统一/代理五类 TXT", "https://raw.githubusercontent.com/DustinWin/BestCF/bestcf/README.md"),
        ("best_cf_ips", "best-cf-ips（LancelotRar）", "LancelotRar/best-cf-ips", "未标注（仅公开数据）", "main", "data-repository", "每 6 小时更新扫描榜单，读取完整 top400", "https://raw.githubusercontent.com/LancelotRar/best-cf-ips/main/README.md"),
        ("xiu2", "XIU2/CloudflareSpeedTest", "XIU2/CloudflareSpeedTest", "GPL-3.0", "master", "data-repository", "官方示例 CIDR 清单，用于官方网段分层抽样", "https://raw.githubusercontent.com/XIU2/CloudflareSpeedTest/master/README.md"),
        ("cfray", "SamNet-dev/cfray", "SamNet-dev/cfray", "MIT", "main", "scanner-reference", "扫描器参考实现，不提供公共 IP 列表，不导入候选", "https://raw.githubusercontent.com/SamNet-dev/cfray/main/README.md"),
    ]
    .into_iter()
    .map(|(key, name, repo, license, branch, kind, note, readme_url)| RegistrySource {
        key: key.to_string(),
        name: name.to_string(),
        repo: repo.to_string(),
        url: format!("https://github.com/{repo}"),
        license: license.to_string(),
        branch: branch.to_string(),
        kind: kind.to_string(),
        note: note.to_string(),
        readme_url: readme_url.to_string(),
    })
    .collect()
}

fn normalize_url(raw: &str) -> Option<String> {
    let mut value = raw.trim().trim_matches('`').trim_matches('"').trim_matches('\'');
    value = value.trim_end_matches(|c: char| matches!(c, '.' | ',' | ';' | ':' | ')' | ']' | '}' | '>'));
    if value.is_empty() {
        return None;
    }
    let parsed = url::Url::parse(value).ok()?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return None;
    }
    Some(parsed.to_string())
}

fn endpoint_allowed(url: &str) -> (bool, String) {
    let parsed = match url::Url::parse(url) {
        Ok(value) => value,
        Err(_) => return (false, "URL 无法解析".to_string()),
    };
    let host = parsed.host_str().unwrap_or("").to_ascii_lowercase();
    if host.is_empty() {
        return (false, "缺少主机名".to_string());
    }
    if let Ok(ip) = IpAddr::from_str(&host) {
        let private = match ip {
            IpAddr::V4(v) => v.is_private() || v.is_loopback() || v.is_link_local() || v.is_broadcast(),
            IpAddr::V6(v) => v.is_loopback() || v.is_unspecified() || (v.segments()[0] & 0xfe00) == 0xfc00 || (v.segments()[0] & 0xffc0) == 0xfe80,
        };
        if private {
            return (false, "禁止私网/回环地址".to_string());
        }
    }
    if host == "localhost" || host.ends_with(".localhost") {
        return (false, "禁止 localhost".to_string());
    }
    if ALLOWED_HOSTS.iter().any(|allowed| host == *allowed) {
        (true, "白名单主机".to_string())
    } else {
        (false, format!("主机不在白名单: {host}"))
    }
}

fn classify_endpoint(url: &str) -> &'static str {
    let lower = url.to_ascii_lowercase();
    if lower.contains("api.github.com") {
        "github-api"
    } else if lower.contains("github.com") && lower.contains("/releases/") {
        "release"
    } else if lower.contains("raw.githubusercontent.com") || lower.contains("cdn.jsdelivr.net") {
        "github-raw"
    } else if lower.ends_with(".txt") || lower.ends_with(".csv") || lower.ends_with(".json") || lower.contains(".jsonl") {
        "static-data"
    } else {
        "reference"
    }
}

fn extract_urls(text: &str) -> Vec<String> {
    let re = Regex::new(r#"https?://[^\s<>"'`\)\]\}]+"#).expect("valid url regex");
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    for capture in re.find_iter(text) {
        if let Some(url) = normalize_url(capture.as_str()) {
            if seen.insert(url.clone()) {
                result.push(url);
            }
            if result.len() >= 200 {
                break;
            }
        }
    }
    result
}

async fn discover_one(source: RegistrySource) -> (Vec<DiscoveredEndpoint>, Vec<String>) {
    let mut warnings = Vec::new();
    let mut endpoints = Vec::new();
    let mut readme = None;
    for url in [
        source.readme_url.clone(),
        format!("https://raw.githubusercontent.com/{}/{}/readme.md", source.repo, source.branch),
    ] {
        match fetch_url_text(&url, 2 * 1024 * 1024).await {
            Ok(text) => {
                readme = Some(text);
                break;
            }
            Err(error) => warnings.push(format!("{} README 获取失败: {error}", source.repo)),
        }
    }
    if let Some(text) = readme {
        let decoded = if text.trim_start().starts_with('{') {
            serde_json::from_str::<serde_json::Value>(&text)
                .ok()
                .and_then(|value| value.get("content").and_then(|v| v.as_str()).map(str::to_string))
                .and_then(|encoded| base64::engine::general_purpose::STANDARD.decode(encoded.replace('\n', "")).ok())
                .and_then(|bytes| String::from_utf8(bytes).ok())
                .unwrap_or(text)
        } else {
            text
        };
        for url in extract_urls(&decoded) {
            let (allowed, reason) = endpoint_allowed(&url);
            endpoints.push(DiscoveredEndpoint {
                url: url.clone(),
                kind: classify_endpoint(&url).to_string(),
                source: source.key.clone(),
                allowed,
                reason,
            });
        }
    }
    warnings.truncate(6);
    (endpoints, warnings)
}

pub async fn discover() -> RegistryDiscovery {
    let sources = registry_sources();
    let results = stream::iter(sources.clone().into_iter().map(|source| async move { discover_one(source).await }))
        .buffer_unordered(4)
        .collect::<Vec<_>>()
        .await;
    let mut endpoints = Vec::new();
    let mut warnings = Vec::new();
    for (mut items, mut notes) in results {
        endpoints.append(&mut items);
        warnings.append(&mut notes);
    }
    let mut seen = HashSet::new();
    endpoints.retain(|item| seen.insert(item.url.clone()));
    let accepted = endpoints.iter().filter(|item| item.allowed).count();
    let rejected = endpoints.len().saturating_sub(accepted);
    RegistryDiscovery {
        generated_at: crate::db::now_text(),
        total_sources: sources.len(),
        total_endpoints: endpoints.len(),
        accepted,
        rejected,
        sources,
        endpoints,
        warnings,
    }
}
