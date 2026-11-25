use crate::db::{self, CandidateSeed};
use crate::models::{CmFetchOutcome, CmRawPreview, CmSourceStatus};
use crate::AppState;
use chrono::Local;
use reqwest::header::{ACCEPT, ETAG, USER_AGENT};
use serde_json::Value;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::Duration;

pub const CM_TXT_URL: &str = "https://zip.cm.edu.kg/all.txt";
pub const CM_JSON_URL: &str = "https://zip.cm.edu.kg/all.json";
pub const CM_SOURCE: &str = "CM IP 库";
const CM_STATUS_KEY: &str = "cm_source_status";

fn now_text() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

fn map_region(code: &str) -> &'static str {
    match code.trim().to_ascii_uppercase().as_str() {
        "HK" => "hk",
        "TW" => "tw",
        "JP" => "jp",
        "SG" => "sg",
        "KR" => "kr",
        "FR" => "fr",
        "US" => "us",
        _ => "",
    }
}

fn region_name_cn(code: &str) -> &'static str {
    match code {
        "HK" => "香港",
        "TW" => "台湾",
        "JP" => "日本",
        "SG" => "新加坡",
        "KR" => "韩国",
        "FR" => "法国",
        "US" => "美国",
        _ => "",
    }
}

pub fn load_status(conn: &rusqlite::Connection) -> CmSourceStatus {
    let mut status = db::get_meta(conn, CM_STATUS_KEY)
        .and_then(|raw| serde_json::from_str::<CmSourceStatus>(&raw).ok())
        .unwrap_or_default();
    status.count = cm_record_count(conn).unwrap_or(status.count);
    status
}

pub fn save_status(conn: &rusqlite::Connection, status: &CmSourceStatus) -> Result<(), String> {
    let raw = serde_json::to_string(status).map_err(|e| format!("序列化 CM 状态失败: {e}"))?;
    db::set_meta(conn, CM_STATUS_KEY, &raw)
}

pub fn cm_record_count(conn: &rusqlite::Connection) -> Result<usize, String> {
    conn.query_row("SELECT COUNT(*) FROM cm_records", [], |row| row.get::<_, i64>(0))
        .map(|n| n.max(0) as usize)
        .map_err(|e| format!("读取 CM 记录数失败: {e}"))
}

#[derive(Debug)]
struct ParsedSource {
    seeds: Vec<CandidateSeed>,
    records: Vec<(String, u16, String)>,
    duplicates: usize,
    invalid: usize,
    data_updated_at: String,
    etag: String,
}

fn unique_key(ip: &str, port: u16) -> String {
    format!("{ip}|{port}")
}

fn parse_txt(body: &str) -> ParsedSource {
    let mut seeds = Vec::new();
    let mut records = Vec::new();
    let mut seen = HashSet::new();
    let mut duplicates = 0usize;
    let mut invalid = 0usize;
    for raw in body.lines() {
        let line = raw.trim().trim_start_matches('\u{feff}');
        if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
            continue;
        }
        let (target, country) = match line.split_once('#') {
            Some((t, c)) => (t.trim(), c.trim()),
            None => (line, ""),
        };
        let seeds_here = crate::sources::parse_candidate_text(target, 443, CM_SOURCE, map_region(country));
        if seeds_here.is_empty() {
            invalid += 1;
            continue;
        }
        for mut seed in seeds_here {
            let key = unique_key(&seed.ip, seed.port);
            if !seen.insert(key) {
                duplicates += 1;
                continue;
            }
            let meta = serde_json::json!({
                "country": country,
                "countryName": region_name_cn(&country.to_ascii_uppercase()),
                "line": line
            });
            records.push((seed.ip.clone(), seed.port, meta.to_string()));
            seed.source = CM_SOURCE.into();
            seeds.push(seed);
        }
    }
    ParsedSource {
        seeds,
        records,
        duplicates,
        invalid,
        data_updated_at: String::new(),
        etag: String::new(),
    }
}

fn json_ports(item: &Value) -> Vec<u16> {
    let mut ports = Vec::new();
    match item.get("port") {
        Some(Value::Array(list)) => {
            for p in list {
                if let Some(n) = p.as_u64() {
                    if (1..=65535).contains(&n) {
                        ports.push(n as u16);
                    }
                } else if let Some(s) = p.as_str() {
                    if let Ok(n) = s.parse::<u16>() {
                        ports.push(n);
                    }
                }
            }
        }
        Some(Value::Number(n)) => {
            if let Some(v) = n.as_u64() {
                if (1..=65535).contains(&v) {
                    ports.push(v as u16);
                }
            }
        }
        _ => {}
    }
    if ports.is_empty() {
        if let Some(v) = item.get("meta").and_then(|m| m.get("_port")).and_then(|v| v.as_u64()) {
            if (1..=65535).contains(&v) {
                ports.push(v as u16);
            }
        }
    }
    ports.sort_unstable();
    ports.dedup();
    ports
}

fn parse_json(body: &str) -> Result<ParsedSource, String> {
    let root: Value = serde_json::from_str(body).map_err(|e| format!("JSON 解析失败: {e}"))?;
    let data = root
        .get("data")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "JSON 缺少 data 数组".to_string())?;
    let generated = root
        .get("generated_at")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let mut seeds = Vec::new();
    let mut records = Vec::new();
    let mut seen = HashSet::new();
    let mut duplicates = 0usize;
    let mut invalid = 0usize;
    for item in data {
        let ip = item.get("ip").and_then(|v| v.as_str()).unwrap_or("").trim();
        if ip.parse::<std::net::IpAddr>().is_err() {
            invalid += 1;
            continue;
        }
        let meta = item.get("meta").cloned().unwrap_or(Value::Null);
        let country = meta
            .get("country")
            .and_then(|v| v.as_str())
            .or_else(|| meta.get("colo").and_then(|c| c.get("cca2")).and_then(|v| v.as_str()))
            .unwrap_or("")
            .to_string();
        let region = map_region(&country);
        let ports = json_ports(item);
        if ports.is_empty() {
            invalid += 1;
            continue;
        }
        for port in ports {
            let key = unique_key(ip, port);
            if !seen.insert(key) {
                duplicates += 1;
                continue;
            }
            let meta_json = serde_json::json!({
                "country": country,
                "countryName": region_name_cn(&country.to_ascii_uppercase()),
                "asn": meta.get("asn").cloned().unwrap_or(Value::Null),
                "asOrganization": meta.get("asOrganization").cloned().unwrap_or(Value::Null),
                "city": meta.get("city").cloned().unwrap_or(Value::Null),
                "colo": meta.get("colo").cloned().unwrap_or(Value::Null),
                "hostname": meta.get("hostname").cloned().unwrap_or(Value::Null),
                "httpProtocol": meta.get("httpProtocol").cloned().unwrap_or(Value::Null),
            });
            records.push((ip.to_string(), port, meta_json.to_string()));
            seeds.push(CandidateSeed {
                ip: ip.to_string(),
                port,
                source: CM_SOURCE.into(),
                region: region.into(),
            });
        }
    }
    Ok(ParsedSource {
        seeds,
        records,
        duplicates,
        invalid,
        data_updated_at: generated,
        etag: String::new(),
    })
}

fn cache_path(state: &AppState, format: &str) -> PathBuf {
    let parent = state.db_path.parent().unwrap_or_else(|| Path::new("."));
    parent.join(format!("cm-cache.{format}"))
}

fn read_cache(path: &Path) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

fn cached_keys(path: &Path, format: &str) -> Option<HashSet<String>> {
    let body = read_cache(path)?;
    let parsed = match format {
        "json" => parse_json(&body).ok()?,
        _ => parse_txt(&body),
    };
    Some(parsed.seeds.iter().map(|s| unique_key(&s.ip, s.port)).collect())
}

fn compare_formats(state: &AppState, primary: &str) -> String {
    let txt = cached_keys(&cache_path(state, "txt"), "txt");
    let json = cached_keys(&cache_path(state, "json"), "json");
    match (txt, json) {
        (Some(t), Some(j)) => {
            let only_txt = t.difference(&j).count();
            let only_json = j.difference(&t).count();
            let common = t.intersection(&j).count();
            format!(
                "TXT {} 条 / JSON {} 条，交集 {} 条，仅 TXT {} 条，仅 JSON {} 条（当前使用 {}）",
                t.len(), j.len(), common, only_txt, only_json,
                primary.to_ascii_uppercase()
            )
        }
        _ => String::new(),
    }
}

async fn download(url: &str, accept: &str) -> Result<(String, String), String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(180))
        .connect_timeout(Duration::from_secs(20))
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {e}"))?;
    let resp = client
        .get(url)
        .header(USER_AGENT, "MikasaOptimizer/1.0")
        .header(ACCEPT, accept)
        .send()
        .await
        .map_err(|e| format!("请求 CM IP 库失败: {e}"))?;
    let status = resp.status();
    if !status.is_success() {
        return Err(format!("CM IP 库返回 HTTP {}", status.as_u16()));
    }
    let etag = resp
        .headers()
        .get(ETAG)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let bytes = resp.bytes().await.map_err(|e| format!("读取 CM IP 库响应失败: {e}"))?;
    let text = String::from_utf8_lossy(&bytes).to_string();
    Ok((text, etag))
}

pub async fn refresh_into_state(state: &AppState, format: &str) -> Result<CmFetchOutcome, String> {
    if state
        .cm_fetching
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err("CM IP 库正在更新，请稍候".into());
    }
    let requested = match format {
        "txt" | "json" => format,
        _ => "auto",
    };
    let effective = if requested == "json" { "json" } else { "txt" };
    {
        let conn = state.db.lock().map_err(|_| "数据库忙".to_string())?;
        let mut status = load_status(&conn);
        status.status = "获取中".into();
        status.last_attempt = now_text();
        status.format = requested.into();
        status.error.clear();
        save_status(&conn, &status)?;
    }

    let result = async {
        let (body, etag) = if effective == "json" {
            download(CM_JSON_URL, "application/json").await?
        } else {
            download(CM_TXT_URL, "text/plain").await?
        };
        let mut parsed = if effective == "json" {
            parse_json(&body)?
        } else {
            parse_txt(&body)
        };
        parsed.etag = etag;
        if parsed.data_updated_at.is_empty() {
            parsed.data_updated_at = format!("源站未提供（ETag: {}）", if parsed.etag.is_empty() { "无" } else { &parsed.etag });
        }
        let cache = cache_path(state, effective);
        if let Some(parent) = cache.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(&cache, body.as_bytes()).map_err(|e| format!("写入 CM 缓存失败: {e}"))?;

        let fetched_at = now_text();
        let (added, updated) = {
            let mut conn = state.db.lock().map_err(|_| "数据库忙".to_string())?;
            let import = db::insert_candidates(&mut conn, parsed.seeds.clone())?;
            let tx = conn.transaction().map_err(|e| format!("开始 CM 数据事务失败: {e}"))?;
            for (ip, port, meta) in &parsed.records {
                tx.execute(
                    "INSERT INTO cm_records(ip,port,meta_json,source_format,data_updated_at,fetched_at)
                     VALUES(?1,?2,?3,?4,?5,?6)
                     ON CONFLICT(ip,port) DO UPDATE SET meta_json=excluded.meta_json,
                       source_format=excluded.source_format, data_updated_at=excluded.data_updated_at,
                       fetched_at=excluded.fetched_at",
                    rusqlite::params![ip, port, meta, effective, &parsed.data_updated_at, &fetched_at],
                )
                .map_err(|e| format!("写入 CM 来源记录失败: {e}"))?;
            }
            tx.commit().map_err(|e| format!("提交 CM 数据失败: {e}"))?;
            (import.added, import.updated)
        };

        let status_snapshot = {
            let conn = state.db.lock().map_err(|_| "数据库忙".to_string())?;
            let mut status = load_status(&conn);
            status.status = "已连接".into();
            status.count = cm_record_count(&conn).unwrap_or(parsed.seeds.len());
            status.last_fetch = fetched_at.clone();
            status.last_attempt = fetched_at.clone();
            status.last_added = added;
            status.last_updated = updated;
            status.last_duplicates = parsed.duplicates;
            status.last_invalid = parsed.invalid;
            status.format = requested.into();
            status.data_updated_at = parsed.data_updated_at.clone();
            status.cache_used = false;
            status.error.clear();
            status.etag = parsed.etag.clone();
            status.format_diff = compare_formats(state, effective);
            save_status(&conn, &status)?;
            status
        };

        let message = format!(
            "CM {} 获取完成：源数据 {} 条，新增 {}，更新 {}，重复 {}，无效 {}；数据时间 {}",
            effective.to_ascii_uppercase(),
            parsed.seeds.len(),
            added,
            updated,
            parsed.duplicates,
            parsed.invalid,
            parsed.data_updated_at
        );
        Ok::<CmFetchOutcome, String>(CmFetchOutcome {
            added,
            updated,
            total: parsed.seeds.len(),
            duplicates: parsed.duplicates,
            invalid: parsed.invalid,
            count: status_snapshot.count,
            format: effective.to_string(),
            data_updated_at: parsed.data_updated_at,
            fetched_at,
            cache_used: false,
            message,
            format_diff: status_snapshot.format_diff,
        })
    }
    .await;

    state.cm_fetching.store(false, Ordering::SeqCst);
    if let Err(err) = &result {
        if let Ok(conn) = state.db.lock() {
            let mut status = load_status(&conn);
            status.status = "获取失败".into();
            status.last_attempt = now_text();
            status.error = err.clone();
            status.cache_used = status.count > 0;
            let _ = save_status(&conn, &status);
        }
    }
    result
}

pub fn preview_format(state: &AppState, format: &str) -> Result<CmRawPreview, String> {
    let effective = if format == "json" { "json" } else { "txt" };
    let path = cache_path(state, effective);
    let body = std::fs::read_to_string(&path)
        .map_err(|_| format!("尚未缓存 {} 数据，请先点击获取", effective.to_ascii_uppercase()))?;
    let bytes = body.len();
    let lines = body.lines().count();
    let limit = 240_000usize;
    let truncated = body.len() > limit;
    let content = if truncated {
        let mut end = limit;
        while !body.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}…\n\n（已截断，仅显示前 {} KB）", &body[..end], limit / 1024)
    } else {
        body
    };
    Ok(CmRawPreview {
        format: effective.into(),
        bytes,
        lines,
        content,
        truncated,
    })
}
