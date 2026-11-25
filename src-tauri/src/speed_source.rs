//! 多测速源系统。
//!
//! 地址与选择逻辑来自 CFData-WEB 主分支 `combined_refactor/speed_url.go`（GPL-3.0，仅参考语义，未复制代码）：
//!   cmSpeedURL              = "cf.090227.xyz/__down?bytes=99999999"
//!   mobileDedicatedSpeedURL = "speed.okl.abrdns.com"
//!   cloudflareSpeedURL      = "speed.cloudflare.com/__down?bytes=99999999"
//!   ispProbeURL             = "https://cf.090227.xyz/cf.json"
//!
//! 说明：CM 指 `cf.090227.xyz` 这套由 CM 线路提供的测速端点，和中国移动专属源是两个不同地址，
//! 不能当成同一概念。测速源只提供下载数据，不决定 colo，地区仍由 Cloudflare trace / CF-RAY 判定。

use std::net::IpAddr;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::speedtest;

pub const CLOUDFLARE_SPEED_URL: &str = "https://speed.cloudflare.com/__down?bytes=99999999";
pub const CM_SPEED_URL: &str = "https://cf.090227.xyz/__down?bytes=99999999";
pub const MOBILE_SPEED_URL: &str = "https://speed.okl.abrdns.com";
pub const ISP_PROBE_URL: &str = "https://cf.090227.xyz/cf.json";

pub const SOURCE_AUTO: &str = "auto";
pub const SOURCE_CLOUDFLARE: &str = "cloudflare";
pub const SOURCE_CM: &str = "cm";
pub const SOURCE_MOBILE: &str = "mobile";
pub const SOURCE_CUSTOM: &str = "custom";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeedSourceProbe {
    pub key: String,
    pub label: String,
    pub url: String,
    pub ok: bool,
    pub dns_ok: bool,
    pub tcp_ok: bool,
    pub tls_ok: bool,
    pub status: u16,
    pub first_chunk_bytes: u64,
    pub elapsed_ms: u64,
    pub redirected: bool,
    pub final_url: String,
    pub resolved_ip: String,
    pub connected_ip: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IspInfo {
    pub asn: i64,
    pub org: String,
    pub isp: String,
    pub is_china_mobile: bool,
    pub probe_ok: bool,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeedSourceCompareRow {
    pub key: String,
    pub label: String,
    pub url: String,
    pub target_ip: String,
    pub connected_ip: String,
    pub status: u16,
    pub bytes: u64,
    pub seconds: f64,
    pub mb_s: f64,
    pub mbps: f64,
    pub ok: bool,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeedSourceCompare {
    pub ip: String,
    pub port: u16,
    pub rows: Vec<SpeedSourceCompareRow>,
    pub recommended: String,
    pub note: String,
}

pub fn label_for(key: &str) -> String {
    match key {
        SOURCE_AUTO => "自动选择".to_string(),
        SOURCE_CLOUDFLARE => "Cloudflare".to_string(),
        SOURCE_CM => "CM 提供".to_string(),
        SOURCE_MOBILE => "移动专属".to_string(),
        SOURCE_CUSTOM => "手动输入".to_string(),
        other => other.to_string(),
    }
}

/// 返回给定测速源键对应的真实下载地址。
pub fn url_for(key: &str, custom: &str) -> Option<String> {
    match key {
        SOURCE_CLOUDFLARE => Some(CLOUDFLARE_SPEED_URL.to_string()),
        SOURCE_CM => Some(CM_SPEED_URL.to_string()),
        SOURCE_MOBILE => Some(MOBILE_SPEED_URL.to_string()),
        SOURCE_CUSTOM => {
            let value = custom.trim();
            if value.is_empty() {
                None
            } else if value.starts_with("http://") || value.starts_with("https://") {
                Some(value.to_string())
            } else {
                Some(format!("https://{value}"))
            }
        }
        _ => None,
    }
}

fn probe_target(url: &str) -> String {
    // 预检只取 1KB，避免为了检查可用性而真的下载上百 MB。
    match url.find("bytes=") {
        Some(idx) => {
            let rest = &url[idx + 6..];
            let end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
            format!("{}bytes=1024{}", &url[..idx], &rest[end..])
        }
        None => url.to_string(),
    }
}

async fn resolve_host(host: &str) -> (bool, String) {
    match tokio::net::lookup_host((host, 443)).await {
        Ok(mut iter) => match iter.next() {
            Some(addr) => (true, addr.ip().to_string()),
            None => (false, String::new()),
        },
        Err(_) => (false, String::new()),
    }
}

/// 对单个测速源做真实预检：DNS → TCP → TLS → HTTP 状态 → 首个数据块 → 是否重定向。
pub async fn probe_url(key: &str, url: &str) -> SpeedSourceProbe {
    let parsed = match reqwest::Url::parse(url) {
        Ok(u) => u,
        Err(e) => {
            return SpeedSourceProbe {
                key: key.to_string(),
                label: label_for(key),
                url: url.to_string(),
                ok: false,
                dns_ok: false,
                tcp_ok: false,
                tls_ok: false,
                status: 0,
                first_chunk_bytes: 0,
                elapsed_ms: 0,
                redirected: false,
                final_url: String::new(),
                resolved_ip: String::new(),
                connected_ip: String::new(),
                error: format!("测速地址无效: {e}"),
            }
        }
    };
    let host = parsed.host_str().unwrap_or_default().to_string();
    let target = probe_target(url);

    let (dns_ok, resolved_ip) = resolve_host(&host).await;
    if !dns_ok {
        return SpeedSourceProbe {
            key: key.to_string(),
            label: label_for(key),
            url: url.to_string(),
            ok: false,
            dns_ok: false,
            tcp_ok: false,
            tls_ok: false,
            status: 0,
            first_chunk_bytes: 0,
            elapsed_ms: 0,
            redirected: false,
            final_url: String::new(),
            resolved_ip: String::new(),
            connected_ip: String::new(),
            error: format!("DNS 解析失败：{host}"),
        };
    }

    let client = match reqwest::Client::builder()
        .use_rustls_tls()
        .user_agent("MikasaOptimizer/1.0")
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return SpeedSourceProbe {
                key: key.to_string(),
                label: label_for(key),
                url: url.to_string(),
                ok: false,
                dns_ok,
                tcp_ok: false,
                tls_ok: false,
                status: 0,
                first_chunk_bytes: 0,
                elapsed_ms: 0,
                redirected: false,
                final_url: String::new(),
                resolved_ip,
                connected_ip: String::new(),
                error: format!("创建预检客户端失败: {e}"),
            }
        }
    };

    let started = Instant::now();
    let request = client.get(&target).header("Cache-Control", "no-store");
    let resp = match request.send().await {
        Ok(r) => r,
        Err(e) => {
            let text = e.to_string();
            let tls_ok = !text.contains("certificate") && !text.contains("handshake") && !text.contains("tls");
            return SpeedSourceProbe {
                key: key.to_string(),
                label: label_for(key),
                url: url.to_string(),
                ok: false,
                dns_ok: true,
                tcp_ok: !text.contains("connect"),
                tls_ok,
                status: 0,
                first_chunk_bytes: 0,
                elapsed_ms: started.elapsed().as_millis() as u64,
                redirected: false,
                final_url: String::new(),
                resolved_ip,
                connected_ip: String::new(),
                error: format!("HTTPS 请求失败: {e}"),
            };
        }
    };

    let status = resp.status().as_u16();
    let final_url = resp.url().to_string();
    let redirected = final_url.trim_end_matches('/') != target.trim_end_matches('/');
    let connected_ip = resp
        .remote_addr()
        .map(|a| a.ip().to_string())
        .unwrap_or_else(|| resolved_ip.clone());
    let mut first_chunk_bytes = 0u64;
    let mut error = String::new();
    if resp.status().is_success() {
        let mut stream = resp.bytes_stream();
        use futures_util::StreamExt;
        match tokio::time::timeout(Duration::from_secs(8), stream.next()).await {
            Ok(Some(Ok(chunk))) => first_chunk_bytes = chunk.len() as u64,
            Ok(Some(Err(e))) => error = format!("读取数据失败: {e}"),
            Ok(None) => error = "响应体为空".to_string(),
            Err(_) => error = "首个数据块读取超时".to_string(),
        }
    } else {
        error = format!("HTTP {status}");
    }

    SpeedSourceProbe {
        key: key.to_string(),
        label: label_for(key),
        url: url.to_string(),
        ok: resp_ok(status) && first_chunk_bytes > 0,
        dns_ok: true,
        tcp_ok: true,
        tls_ok: true,
        status,
        first_chunk_bytes,
        elapsed_ms: started.elapsed().as_millis() as u64,
        redirected,
        final_url,
        resolved_ip,
        connected_ip,
        error,
    }
}

fn resp_ok(status: u16) -> bool {
    (200..300).contains(&status)
}

/// 通过 cf.json 识别当前出口运营商（用于 Auto 选择策略）。
pub async fn detect_isp() -> IspInfo {
    let client = match reqwest::Client::builder()
        .use_rustls_tls()
        .timeout(Duration::from_secs(6))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return IspInfo {
                asn: 0,
                org: String::new(),
                isp: "网络环境未知".to_string(),
                is_china_mobile: false,
                probe_ok: false,
                error: e.to_string(),
            }
        }
    };
    let mut cf_error = String::new();
    let body = match client
        .get(ISP_PROBE_URL)
        .header(reqwest::header::ACCEPT_ENCODING, "identity")
        .send()
        .await
    {
        Ok(r) => match r.bytes().await {
            Ok(bytes) => String::from_utf8_lossy(&bytes).to_string(),
            Err(e) => {
                cf_error = format!("读取 cf.json 失败: {e}");
                String::new()
            }
        },
        Err(e) => {
            cf_error = format!("请求 cf.json 失败: {e}");
            String::new()
        }
    };
    let value: serde_json::Value = serde_json::from_str(&body).unwrap_or(serde_json::Value::Null);
    let asn = value
        .get("asn")
        .and_then(|v| v.as_i64().or_else(|| v.as_str().and_then(|s| s.parse::<i64>().ok())))
        .unwrap_or(0);
    let org = value
        .get("asOrganization")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    if !org.is_empty() || asn != 0 {
        let isp = classify_isp(&org, asn);
        return IspInfo {
            asn,
            is_china_mobile: is_china_mobile(&org, asn),
            org,
            isp,
            probe_ok: true,
            error: String::new(),
        };
    }
    let fallback = crate::network::detect_network().await;
    let carrier = fallback.carrier.clone();
    let usable = !carrier.is_empty() && carrier != "运营商未知";
    IspInfo {
        asn: 0,
        org: carrier.clone(),
        isp: if usable {
            classify_isp(&carrier, 0)
        } else {
            "网络环境未知".to_string()
        },
        is_china_mobile: usable && is_china_mobile(&carrier, 0),
        probe_ok: usable,
        error: if cf_error.is_empty() {
            "cf.json 未返回 ASN，已改用本机运营商检测".to_string()
        } else {
            format!("{cf_error}；已改用本机运营商检测")
        },
    }
}

pub fn classify_isp(org: &str, asn: i64) -> String {
    let lower = org.to_lowercase();
    if is_china_mobile(org, asn) {
        return "中国移动".to_string();
    }
    if lower.contains("unicom") || lower.contains("cucc") || org.contains("联通") {
        return "中国联通".to_string();
    }
    if lower.contains("chinanet")
        || lower.contains("telecom")
        || lower.contains("chinatelecom")
        || org.contains("电信")
    {
        return "中国电信".to_string();
    }
    if lower.contains("cernet") || org.contains("教育") {
        return "中国教育网".to_string();
    }
    if org.is_empty() {
        "网络环境未知".to_string()
    } else {
        org.to_string()
    }
}

pub fn is_china_mobile(org: &str, asn: i64) -> bool {
    let lower = org.to_lowercase();
    const KEYWORDS: [&str; 7] = [
        "cmi",
        "cmnet",
        "chinamobile",
        "china mobile",
        "cmcc",
        "mobile communications",
        "移动",
    ];
    if KEYWORDS.iter().any(|k| lower.contains(k)) {
        return true;
    }
    matches!(asn, 9808 | 24400 | 56040 | 56041 | 56044)
}

/// 自动选择：移动网络优先移动专属/CM，其余网络优先 Cloudflare，均以真实预检结果为准。
pub async fn resolve_auto(custom: &str) -> (String, String, Vec<SpeedSourceProbe>) {
    let mut probes = Vec::new();
    let isp = detect_isp().await;
    for key in [SOURCE_CLOUDFLARE, SOURCE_CM, SOURCE_MOBILE] {
        if let Some(url) = url_for(key, custom) {
            probes.push(probe_url(key, &url).await);
        }
    }
    let available = |key: &str| probes.iter().find(|p| p.key == key).map(|p| p.ok).unwrap_or(false);

    let chosen = if isp.is_china_mobile {
        if available(SOURCE_MOBILE) {
            SOURCE_MOBILE
        } else if available(SOURCE_CM) {
            SOURCE_CM
        } else if available(SOURCE_CLOUDFLARE) {
            SOURCE_CLOUDFLARE
        } else {
            SOURCE_CLOUDFLARE
        }
    } else if available(SOURCE_CLOUDFLARE) {
        SOURCE_CLOUDFLARE
    } else if available(SOURCE_CM) {
        SOURCE_CM
    } else if available(SOURCE_MOBILE) {
        SOURCE_MOBILE
    } else {
        SOURCE_CLOUDFLARE
    };

    let url = url_for(chosen, custom).unwrap_or_else(|| CLOUDFLARE_SPEED_URL.to_string());
    (chosen.to_string(), url, probes)
}

/// 备用源：按给定顺序取第一个与主源不同且预检通过的源。
pub async fn resolve_backup(primary: &str, preferred: &str, custom: &str) -> Option<(String, String)> {
    let mut order: Vec<&str> = Vec::new();
    if !preferred.is_empty() && preferred != SOURCE_AUTO {
        order.push(match preferred {
            SOURCE_CM => SOURCE_CM,
            SOURCE_MOBILE => SOURCE_MOBILE,
            SOURCE_CUSTOM => SOURCE_CUSTOM,
            _ => SOURCE_CLOUDFLARE,
        });
    }
    for key in [SOURCE_CLOUDFLARE, SOURCE_CM, SOURCE_MOBILE, SOURCE_CUSTOM] {
        if !order.contains(&key) {
            order.push(key);
        }
    }
    for key in order {
        if key == primary {
            continue;
        }
        if let Some(url) = url_for(key, custom) {
            let probe = probe_url(key, &url).await;
            if probe.ok {
                return Some((key.to_string(), url));
            }
        }
    }
    None
}

/// 测速源诊断：对同一个目标 IP + 端口，逐个测速源做真实下载对比。
pub async fn compare_for_ip(
    ip: IpAddr,
    port: u16,
    custom: &str,
    duration_secs: u64,
    min_bytes: u64,
) -> SpeedSourceCompare {
    let mut rows = Vec::new();
    for key in [SOURCE_CLOUDFLARE, SOURCE_CM, SOURCE_MOBILE, SOURCE_CUSTOM] {
        let Some(url) = url_for(key, custom) else { continue };
        let probe = speedtest::download_probe(ip, port, &url, duration_secs, min_bytes).await;
        rows.push(SpeedSourceCompareRow {
            key: key.to_string(),
            label: label_for(key),
            url,
            target_ip: ip.to_string(),
            connected_ip: probe.connected_ip.clone(),
            status: probe.status,
            bytes: probe.bytes,
            seconds: probe.seconds,
            mb_s: probe.mb_s,
            mbps: probe.mbps,
            ok: probe.ok,
            error: probe.error,
        });
    }
    let best = rows
        .iter()
        .filter(|r| r.ok && r.connected_ip == ip.to_string())
        .max_by(|a, b| a.mb_s.partial_cmp(&b.mb_s).unwrap_or(std::cmp::Ordering::Equal));
    let recommended = best.map(|r| r.key.clone()).unwrap_or_default();
    SpeedSourceCompare {
        ip: ip.to_string(),
        port,
        rows,
        recommended,
        note: "所有测速源都绑定同一个目标 IP 与端口，SNI/Host 使用测速域名；地区仍以 Cloudflare trace 的 colo 为准。".to_string(),
    }
}
