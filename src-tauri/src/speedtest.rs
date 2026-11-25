use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use std::time::{Duration, Instant};
use tokio::net::TcpStream;

pub const TEST_HOST: &str = "speed.cloudflare.com";
pub const TRACE_URL: &str = "https://speed.cloudflare.com/cdn-cgi/trace";
pub const DEFAULT_DOWNLOAD_URL: &str = "https://speed.cloudflare.com/__down?bytes=104857600";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TcpProbe {
    pub ok: bool,
    pub connect_ms: f64,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TraceProbe {
    pub ok: bool,
    pub status: u16,
    pub colo: String,
    pub loc: String,
    pub server_ip: String,
    pub elapsed_ms: f64,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LatencyProbe {
    pub ok: bool,
    pub avg_ms: f64,
    pub min_ms: f64,
    pub max_ms: f64,
    pub jitter_ms: f64,
    pub loss_pct: f64,
    pub success: usize,
    pub attempts: usize,
    pub samples: Vec<f64>,
    pub last_error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProbe {
    pub ok: bool,
    pub bytes: u64,
    pub seconds: f64,
    pub mb_s: f64,
    pub mbps: f64,
    pub status: u16,
    pub connected_ip: String,
    pub error: String,
}

fn pinned_client(ip: IpAddr, port: u16, host: &str, timeout_secs: u64) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .use_rustls_tls()
        .http1_only()
        .user_agent("MikasaOptimizer/1.0")
        .connect_timeout(Duration::from_secs(4))
        .timeout(Duration::from_secs(timeout_secs))
        .resolve(host, SocketAddr::new(ip, port))
        .build()
        .map_err(|e| format!("创建测速客户端失败: {e}"))
}

fn endpoint_with_port(url: &str, port: u16) -> Result<reqwest::Url, String> {
    let mut parsed = reqwest::Url::parse(url).map_err(|e| format!("测速地址无效: {e}"))?;
    if parsed.scheme() != "https" {
        return Err("测速地址必须使用 HTTPS".to_string());
    }
    if port != 443 {
        parsed.set_port(Some(port)).map_err(|_| "设置测速端口失败".to_string())?;
    }
    Ok(parsed)
}

pub async fn tcp_probe(ip: IpAddr, port: u16, timeout_ms: u64) -> TcpProbe {
    let started = Instant::now();
    match tokio::time::timeout(Duration::from_millis(timeout_ms), TcpStream::connect(SocketAddr::new(ip, port))).await {
        Ok(Ok(stream)) => {
            drop(stream);
            TcpProbe {
                ok: true,
                connect_ms: started.elapsed().as_secs_f64() * 1000.0,
                error: String::new(),
            }
        }
        Ok(Err(e)) => TcpProbe { ok: false, connect_ms: 0.0, error: format!("TCP 连接失败: {e}") },
        Err(_) => TcpProbe { ok: false, connect_ms: 0.0, error: "TCP 连接超时".to_string() },
    }
}

/// 多轮 TCP 握手 RTT，语义与 v2rayN / tcping 的「延迟」一致。
/// 只统计 connect 往返，不包含 TLS 握手、HTTP 请求与响应体，因此数值会比
/// `trace_probe` 的完整 HTTPS 请求耗时低 2-4 倍，也更接近代理客户端显示值。
pub async fn tcp_latency_probe(ip: IpAddr, port: u16, rounds: u8, timeout_ms: u64) -> LatencyProbe {
    let attempts = rounds.clamp(1, 10) as usize;
    let mut values = Vec::with_capacity(attempts);
    let mut errors = Vec::new();
    for _ in 0..attempts {
        let started = Instant::now();
        match tokio::time::timeout(
            Duration::from_millis(timeout_ms),
            TcpStream::connect(SocketAddr::new(ip, port)),
        )
        .await
        {
            Ok(Ok(stream)) => {
                drop(stream);
                values.push(started.elapsed().as_secs_f64() * 1000.0);
            }
            Ok(Err(e)) => errors.push(format!("TCP 连接失败: {e}")),
            Err(_) => errors.push("TCP 连接超时".to_string()),
        }
    }
    if values.is_empty() {
        return LatencyProbe {
            ok: false,
            attempts,
            loss_pct: 100.0,
            last_error: errors.last().cloned().unwrap_or_else(|| "TCP 延迟测试全部失败".to_string()),
            ..Default::default()
        };
    }
    let success = values.len();
    let avg = values.iter().sum::<f64>() / success as f64;
    let min = values.iter().cloned().fold(f64::MAX, f64::min);
    let max = values.iter().cloned().fold(f64::MIN, f64::max);
    let variance = values.iter().map(|v| (v - avg).powi(2)).sum::<f64>() / success as f64;
    LatencyProbe {
        ok: success * 2 >= attempts,
        avg_ms: avg,
        min_ms: min,
        max_ms: max,
        jitter_ms: variance.sqrt(),
        loss_pct: (attempts - success) as f64 / attempts as f64 * 100.0,
        success,
        attempts,
        samples: values,
        last_error: errors.last().cloned().unwrap_or_default(),
    }
}

pub async fn trace_probe(ip: IpAddr, port: u16, timeout_ms: u64) -> TraceProbe {
    let client = match pinned_client(ip, port, TEST_HOST, (timeout_ms / 1000).max(4) + 2) {
        Ok(c) => c,
        Err(e) => return TraceProbe { error: e, ..Default::default() },
    };
    let url = match endpoint_with_port(TRACE_URL, port) {
        Ok(u) => u,
        Err(e) => return TraceProbe { error: e, ..Default::default() },
    };
    let started = Instant::now();
    let result = tokio::time::timeout(
        Duration::from_millis(timeout_ms),
        client
            .get(url)
            .header("Cache-Control", "no-cache")
            .header("Connection", "close")
            .send(),
    )
    .await;
    let resp = match result {
        Ok(Ok(r)) => r,
        Ok(Err(e)) => return TraceProbe { error: format!("HTTPS 验证失败: {e}"), ..Default::default() },
        Err(_) => return TraceProbe { error: "HTTPS 验证超时".to_string(), ..Default::default() },
    };
    let status = resp.status().as_u16();
    if !resp.status().is_success() {
        return TraceProbe { status, error: format!("HTTPS 状态码 {status}"), ..Default::default() };
    }
    let body = match resp.text().await {
        Ok(t) => t,
        Err(e) => return TraceProbe { status, error: format!("读取 CF trace 失败: {e}"), ..Default::default() },
    };
    let mut colo = String::new();
    let mut loc = String::new();
    let mut server_ip = String::new();
    for line in body.lines() {
        if let Some(v) = line.strip_prefix("colo=") {
            colo = v.trim().to_ascii_uppercase();
        } else if let Some(v) = line.strip_prefix("loc=") {
            loc = v.trim().to_ascii_uppercase();
        } else if let Some(v) = line.strip_prefix("ip=") {
            server_ip = v.trim().to_string();
        }
    }
    let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
    if colo.is_empty() {
        return TraceProbe {
            status,
            error: "响应中未找到 Cloudflare colo 信息".to_string(),
            elapsed_ms,
            ..Default::default()
        };
    }
    TraceProbe {
        ok: true,
        status,
        colo,
        loc,
        server_ip,
        elapsed_ms,
        error: String::new(),
    }
}

pub async fn latency_probe(ip: IpAddr, port: u16, rounds: u8, timeout_ms: u64) -> LatencyProbe {
    latency_probe_with_initial(ip, port, rounds, timeout_ms, None).await
}

pub async fn latency_probe_with_initial(
    ip: IpAddr,
    port: u16,
    rounds: u8,
    timeout_ms: u64,
    initial_ms: Option<f64>,
) -> LatencyProbe {
    let attempts = rounds.max(1) as usize;
    let mut values = Vec::new();
    let mut errors = Vec::new();
    if let Some(v) = initial_ms {
        if v > 0.0 {
            values.push(v);
        }
    }
    let remaining = attempts.saturating_sub(values.len());
    for _ in 0..remaining {
        let result = trace_probe(ip, port, timeout_ms).await;
        if result.ok {
            values.push(result.elapsed_ms);
        } else {
            errors.push(result.error);
        }
    }
    if values.is_empty() {
        return LatencyProbe {
            ok: false,
            attempts,
            loss_pct: 100.0,
            last_error: errors.last().cloned().unwrap_or_else(|| "延迟测试全部失败".to_string()),
            ..Default::default()
        };
    }
    let success = values.len();
    let avg = values.iter().sum::<f64>() / success as f64;
    let min = values.iter().cloned().fold(f64::MAX, f64::min);
    let max = values.iter().cloned().fold(f64::MIN, f64::max);
    let variance = values.iter().map(|v| (v - avg).powi(2)).sum::<f64>() / success as f64;
    LatencyProbe {
        ok: success * 2 >= attempts,
        avg_ms: avg,
        min_ms: min,
        max_ms: max,
        jitter_ms: variance.sqrt(),
        loss_pct: (attempts - success) as f64 / attempts as f64 * 100.0,
        success,
        attempts,
        samples: values,
        last_error: errors.last().cloned().unwrap_or_default(),
    }
}

pub async fn download_probe(
    ip: IpAddr,
    port: u16,
    url: &str,
    duration_secs: u64,
    min_bytes: u64,
) -> DownloadProbe {
    let parsed = match reqwest::Url::parse(url) {
        Ok(u) => u,
        Err(e) => return DownloadProbe { error: format!("下载地址无效: {e}"), ..Default::default() },
    };
    if parsed.scheme() != "https" {
        return DownloadProbe { error: "下载地址必须使用 HTTPS".to_string(), ..Default::default() };
    }
    let host = match parsed.host_str() {
        Some(h) => h.to_string(),
        None => return DownloadProbe { error: "下载地址缺少主机名".to_string(), ..Default::default() },
    };
    let client = match pinned_client(ip, port, &host, duration_secs + 25) {
        Ok(c) => c,
        Err(e) => return DownloadProbe { error: e, ..Default::default() },
    };
    let target = match endpoint_with_port(url, port) {
        Ok(u) => u,
        Err(e) => return DownloadProbe { error: e, ..Default::default() },
    };
    let request = client
        .get(target)
        .header("Cache-Control", "no-store")
        .header("Accept", "*/*");
    let resp = match tokio::time::timeout(Duration::from_secs(15), request.send()).await {
        Ok(Ok(r)) => r,
        Ok(Err(e)) => return DownloadProbe { error: format!("下载请求失败: {e}"), ..Default::default() },
        Err(_) => return DownloadProbe { error: "下载请求超时".to_string(), ..Default::default() },
    };
    let status = resp.status().as_u16();
    let connected_ip = resp.remote_addr().map(|a| a.ip().to_string()).unwrap_or_default();
    if !resp.status().is_success() {
        return DownloadProbe { status, connected_ip, error: format!("下载接口返回 HTTP {status}"), ..Default::default() };
    }
    let declared = resp.content_length().unwrap_or(0);
    let started = Instant::now();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(duration_secs.max(1));
    let mut stream = resp.bytes_stream();
    let mut received: u64 = 0;
    let mut last_error = String::new();
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        match tokio::time::timeout(remaining, stream.next()).await {
            Ok(Some(Ok(chunk))) => {
                received += chunk.len() as u64;
                if received >= 2 * 1024 * 1024 * 1024 {
                    break;
                }
            }
            Ok(Some(Err(e))) => {
                last_error = format!("下载流中断: {e}");
                break;
            }
            Ok(None) => break,
            Err(_) => break,
        }
    }
    let seconds = started.elapsed().as_secs_f64();
    let mb_s = if seconds > 0.0 { received as f64 / 1_000_000.0 / seconds } else { 0.0 };
    let mbps = mb_s * 8.0;
    if received < min_bytes {
        return DownloadProbe {
            ok: false,
            bytes: received,
            seconds,
            mb_s,
            mbps,
            status,
            connected_ip,
            error: format!(
                "有效数据不足（{} 字节 < {} 字节，声明长度 {}）{}",
                received,
                min_bytes,
                declared,
                if last_error.is_empty() { String::new() } else { format!("；{last_error}") }
            ),
        };
    }
    if received == 0 {
        return DownloadProbe { ok: false, status, error: "下载 0 字节".to_string(), ..Default::default() };
    }
    DownloadProbe {
        ok: true,
        bytes: received,
        seconds,
        mb_s,
        mbps,
        status,
        connected_ip,
        error: last_error,
    }
}

pub fn parse_ip(value: &str) -> Result<IpAddr, String> {
    IpAddr::from_str(value).map_err(|_| format!("IP 地址无效: {value}"))
}
