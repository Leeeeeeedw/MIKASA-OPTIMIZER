use crate::{
    cm_source, db, export, github, models::*, network, registry, scanner, sources, speedtest, vault,
    AppState,
};
use rusqlite::params;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager, State};

fn db_guard<'a>(state: &'a State<'_, AppState>) -> Result<std::sync::MutexGuard<'a, rusqlite::Connection>, String> {
    state.db.lock().map_err(|e| format!("数据库忙: {e}"))
}

fn nonempty_lines(text: &str) -> usize {
    text.lines().filter(|line| {
        let value = line.trim();
        !value.is_empty() && !value.starts_with('#') && !value.starts_with("//")
    }).count()
}

fn record_import(conn: &rusqlite::Connection, source: &str, fetched: usize, invalid: usize, result: &ImportResult) {
    let duplicates = fetched.saturating_sub(result.added + invalid);
    let _ = db::record_source_import(conn, source, fetched, result.added, duplicates, invalid, "正常", "");
}

fn save_settings_inner(state: &State<'_, AppState>, settings: &AppSettings) -> Result<(), String> {
    let conn = db_guard(state)?;
    db::save_settings(&conn, settings)?;
    drop(conn);
    *state.settings.lock().map_err(|e| e.to_string())? = settings.clone();
    Ok(())
}

#[tauri::command]
pub fn get_bootstrap(state: State<'_, AppState>) -> Result<Value, String> {
    let settings = state.settings.lock().map(|s| s.clone()).unwrap_or_default();
    let conn = db_guard(&state)?;
    let dashboard = db::dashboard(&conn)?;
    let mut source_counts: BTreeMap<String, usize> = BTreeMap::new();
    {
        let mut stmt = conn.prepare("SELECT sources FROM candidates").map_err(|e| e.to_string())?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0)).map_err(|e| e.to_string())?;
        for row in rows.flatten() {
            if let Ok(list) = serde_json::from_str::<Vec<String>>(&row) {
                for source in list {
                    *source_counts.entry(source).or_insert(0) += 1;
                }
            }
        }
    }
    let source_stats = db::source_overview(&conn).unwrap_or_default();
    let data_dir = state.db_path.parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
    Ok(json!({
        "settings": settings,
        "regions": region_options(),
        "dashboard": dashboard,
        "sourceCounts": source_counts,
        "sourceStats": source_stats,
        "dataDir": data_dir,
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

#[tauri::command]
pub async fn detect_network(state: State<'_, AppState>, force: Option<bool>) -> Result<NetworkInfo, String> {
    if !force.unwrap_or(false) {
        if let Ok(conn) = state.db.lock() {
            if let Some(text) = db::get_meta(&conn, "network_cache") {
                if let Ok(info) = serde_json::from_str::<NetworkInfo>(&text) {
                    if info.ok {
                        return Ok(info);
                    }
                }
            }
        }
    }
    let info = network::detect_network().await;
    if let Ok(conn) = state.db.lock() {
        let _ = db::set_meta(&conn, "network_cache", &serde_json::to_string(&info).unwrap_or_default());
    }
    Ok(info)
}

#[tauri::command]
pub fn get_dashboard(state: State<'_, AppState>) -> Result<DashboardData, String> {
    let conn = db_guard(&state)?;
    db::dashboard(&conn)
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<AppSettings, String> {
    Ok(state.settings.lock().map(|s| s.clone()).unwrap_or_default())
}

#[tauri::command]
pub fn save_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: AppSettings,
) -> Result<AppSettings, String> {
    if settings.default_port == 0 {
        return Err("默认端口必须在 1-65535 之间".to_string());
    }
    if settings.concurrency == 0 || settings.concurrency > 512 {
        return Err("并发数量必须在 1-512 之间".to_string());
    }
    if settings.per_region_top == 0 || settings.per_region_top > 500 {
        return Err("每地区保留数量必须在 1-500 之间".to_string());
    }
    if settings.download_top_per_region > 2000 {
        return Err("每地区测速数量不能超过 2000（0 表示全部）".to_string());
    }
    let ui_scale = settings.ui_scale.trim().to_lowercase();
    if ui_scale != "auto"
        && !ui_scale
            .parse::<f64>()
            .map(|value| (0.75..=2.5).contains(&value))
            .unwrap_or(false)
    {
        return Err("界面缩放必须为自动或 75%-250%".to_string());
    }
    if settings.download_seconds < 2 || settings.download_seconds > 120 {
        return Err("下载测速时长必须在 2-120 秒之间".to_string());
    }
    if settings.latency_rounds == 0 || settings.latency_rounds > 10 {
        return Err("延迟测试轮数必须在 1-10 之间".to_string());
    }
    if settings.stability_rounds > 5 {
        return Err("稳定性复测轮数必须在 0-5 之间".to_string());
    }
    if settings.latency_limit_ms <= 0.0 || settings.speed_limit_mb_s <= 0.0 || settings.min_download_bytes < 65536 {
        return Err("延迟上限、速度下限与最低有效数据量必须为正数".to_string());
    }
    let mut normalized = settings;
    normalized.ui_scale = ui_scale;
    normalized.download_url = normalized.download_url.trim().to_string();
    normalized.backup_download_url = normalized.backup_download_url.trim().to_string();
    if !normalized.download_url.starts_with("https://") {
        return Err("主测速地址必须使用 HTTPS".to_string());
    }
    if !normalized.backup_download_url.is_empty() && !normalized.backup_download_url.starts_with("https://") {
        return Err("备用测速地址必须使用 HTTPS".to_string());
    }
    save_settings_inner(&state, &normalized)?;
    if let Some(window) = app.get_webview_window("main") {
        let _ = crate::apply_ui_scale(&window, &state, true);
    }
    Ok(normalized)
}

#[tauri::command]
pub fn list_candidates(
    state: State<'_, AppState>,
    page: Option<usize>,
    page_size: Option<usize>,
    search: Option<String>,
    region: Option<String>,
    source: Option<String>,
) -> Result<Value, String> {
    let page = page.unwrap_or(1).max(1);
    let page_size = page_size.unwrap_or(50).clamp(1, 500);
    let conn = db_guard(&state)?;
    let (rows, total) = db::list_candidates(
        &conn,
        page_size,
        (page - 1) * page_size,
        search.as_deref().unwrap_or(""),
        region.as_deref().unwrap_or(""),
        source.as_deref().unwrap_or(""),
    )?;
    Ok(json!({"items": rows, "total": total, "page": page, "pageSize": page_size}))
}

#[tauri::command]
pub fn preview_file(path: String) -> Result<FilePreview, String> {
    let meta = std::fs::metadata(&path).map_err(|e| format!("读取文件失败: {e}"))?;
    if meta.len() > 16 * 1024 * 1024 {
        return Err("文件超过 16MB，请拆分后再导入".to_string());
    }
    let bytes = std::fs::read(&path).map_err(|e| format!("读取文件失败: {e}"))?;
    let content = String::from_utf8_lossy(&bytes).chars().take(200_000).collect::<String>();
    let name = PathBuf::from(&path)
        .file_name()
        .map(|v| v.to_string_lossy().to_string())
        .unwrap_or_else(|| path.clone());
    Ok(FilePreview {
        name,
        content,
        bytes: bytes.len(),
    })
}

#[tauri::command]
pub fn import_text(
    state: State<'_, AppState>,
    text: String,
    default_port: Option<u16>,
    region: Option<String>,
    source: Option<String>,
) -> Result<ImportResult, String> {
    let port = default_port.unwrap_or_else(|| state.settings.lock().map(|s| s.default_port).unwrap_or(443));
    let seeds = sources::parse_candidate_text(
        &text,
        port,
        source.as_deref().unwrap_or("local"),
        region.as_deref().unwrap_or(""),
    );
    if seeds.is_empty() {
        return Err("没有解析到有效 IP，请检查文本格式".to_string());
    }
    let fetched = nonempty_lines(&text);
    let invalid = fetched.saturating_sub(seeds.len());
    let mut conn = db_guard(&state)?;
    let result = db::insert_candidates(&mut conn, seeds)?;
    record_import(&conn, source.as_deref().unwrap_or("local"), fetched, invalid, &result);
    Ok(result)
}

#[tauri::command]
pub fn import_file(
    state: State<'_, AppState>,
    path: String,
    default_port: Option<u16>,
    region: Option<String>,
    source: Option<String>,
) -> Result<ImportResult, String> {
    let meta = std::fs::metadata(&path).map_err(|e| format!("读取文件失败: {e}"))?;
    if meta.len() > 32 * 1024 * 1024 {
        return Err("文件超过 32MB，请拆分后再导入".to_string());
    }
    let bytes = std::fs::read(&path).map_err(|e| format!("读取文件失败: {e}"))?;
    let text = String::from_utf8_lossy(&bytes).to_string();
    let port = default_port.unwrap_or_else(|| state.settings.lock().map(|s| s.default_port).unwrap_or(443));
    let seeds = sources::parse_candidate_text(
        &text,
        port,
        source.as_deref().unwrap_or("local"),
        region.as_deref().unwrap_or(""),
    );
    if seeds.is_empty() {
        return Err("文件里没有解析到有效 IP".to_string());
    }
    let fetched = nonempty_lines(&text);
    let invalid = fetched.saturating_sub(seeds.len());
    let mut conn = db_guard(&state)?;
    let result = db::insert_candidates(&mut conn, seeds)?;
    record_import(&conn, source.as_deref().unwrap_or("local"), fetched, invalid, &result);
    Ok(result)
}

#[tauri::command]
pub async fn import_urls(
    state: State<'_, AppState>,
    urls: Vec<String>,
    default_port: Option<u16>,
    region: Option<String>,
    source: Option<String>,
) -> Result<ImportResult, String> {
    if urls.is_empty() {
        return Err("请填写至少一个链接".to_string());
    }
    let port = default_port.unwrap_or(443);
    let label = source.unwrap_or_else(|| "url".to_string());
    let mut seeds = Vec::new();
    let mut errors = Vec::new();
    let mut fetched = 0usize;
    for url in &urls {
        match sources::fetch_url_text(url, 8 * 1024 * 1024).await {
            Ok(text) => {
                fetched += nonempty_lines(&text);
                let parsed = sources::parse_candidate_text(&text, port, &label, region.as_deref().unwrap_or(""));
                if parsed.is_empty() {
                    errors.push(format!("{url}: 未解析到 IP"));
                } else {
                    seeds.extend(parsed);
                }
            }
            Err(e) => errors.push(format!("{url}: {e}")),
        }
    }
    if seeds.is_empty() {
        return Err(format!("全部链接获取失败：{}", errors.join("；")));
    }
    let invalid = fetched.saturating_sub(seeds.len());
    let mut conn = db_guard(&state)?;
    let result = db::insert_candidates(&mut conn, seeds)?;
    record_import(&conn, &label, fetched, invalid, &result);
    if !errors.is_empty() {
        // The import succeeded for at least one URL; errors are surfaced by the caller's log.
    }
    Ok(result)
}

#[tauri::command]
pub fn community_source_list() -> Vec<sources::CommunitySourceInfo> {
    sources::community_sources()
}

#[tauri::command]
pub fn community_source_status(state: State<'_, AppState>) -> Result<Vec<SourceOverview>, String> {
    let conn = db_guard(&state)?;
    let keys = sources::COMMUNITY_SOURCE_KEYS;
    let mut list = db::source_overview(&conn)?;
    list.retain(|row| keys.contains(&row.key.as_str()));
    Ok(list)
}

#[tauri::command]
pub async fn community_fetch(
    state: State<'_, AppState>,
    source: String,
    max_results: Option<usize>,
) -> Result<ImportResult, String> {
    let key = source.trim().to_string();
    if !sources::COMMUNITY_SOURCE_KEYS.contains(&key.as_str()) {
        return Err(format!("未知社区源: {key}"));
    }
    let (seeds, files_ok, errors) =
        match sources::community_candidates(&key, max_results.unwrap_or(5000)).await {
            Ok(value) => value,
            Err(error) => {
                if let Ok(conn) = state.db.lock() {
                    let _ = db::record_source_import(&conn, &key, 0, 0, 0, 0, "更新失败", &error);
                }
                return Err(error);
            }
        };
    let fetched = seeds.len();
    let mut conn = db_guard(&state)?;
    let result = db::insert_candidates(&mut conn, seeds)?;
    let duplicates = fetched.saturating_sub(result.added);
    let status = if errors.is_empty() { "正常" } else { "部分成功" };
    let message = if errors.is_empty() {
        format!("{files_ok} 个文件获取成功")
    } else {
        format!("{files_ok} 个文件成功；{}", errors.join("；"))
    };
    let _ = db::record_source_import(
        &conn,
        &key,
        fetched,
        result.added,
        duplicates,
        0,
        status,
        &message,
    );
    Ok(result)
}

#[tauri::command]
pub fn source_quality(state: State<'_, AppState>) -> Result<Vec<SourceQuality>, String> {
    let conn = db_guard(&state)?;
    db::source_quality(&conn)
}

#[tauri::command]
pub fn registry_sources() -> Vec<RegistrySource> {
    registry::registry_sources()
}

#[tauri::command]
pub async fn registry_discover() -> Result<RegistryDiscovery, String> {
    Ok(registry::discover().await)
}

#[tauri::command]
pub async fn cloudflare_fetch(
    state: State<'_, AppState>,
    ipv4: Option<bool>,
    ipv6: Option<bool>,
    per_cidr: Option<usize>,
) -> Result<ImportResult, String> {
    let (seeds, _segments) = sources::cloudflare_candidates(
        ipv4.unwrap_or(true),
        ipv6.unwrap_or(false),
        per_cidr.unwrap_or(4).clamp(1, 30),
    )
    .await?;
    if seeds.is_empty() {
        return Err("Cloudflare 官方 IP 段获取失败".to_string());
    }
    let fetched = seeds.len();
    let mut conn = db_guard(&state)?;
    let result = db::insert_candidates(&mut conn, seeds)?;
    record_import(&conn, "cloudflare", fetched, 0, &result);
    Ok(result)
}

#[tauri::command]
pub fn fofa_preview(state: State<'_, AppState>, regions: Vec<String>) -> Result<Vec<FofaQueryPreview>, String> {
    let settings = state.settings.lock().map(|s| s.clone()).unwrap_or_default();
    let preview = sources::fofa_preview(&settings, &regions);
    if preview.is_empty() {
        return Err("请至少选择一个地区".to_string());
    }
    Ok(preview)
}

#[tauri::command]
pub fn fofa_status(state: State<'_, AppState>) -> Result<FofaStatus, String> {
    let settings = state.settings.lock().map(|s| s.clone()).unwrap_or_default();
    let key = vault::get_secret("fofa_api_key")?.unwrap_or_default();
    let masked = if key.len() > 4 {
        format!("••••••••{}", &key[key.len() - 4..])
    } else if key.is_empty() {
        String::new()
    } else {
        "••••".to_string()
    };
    Ok(FofaStatus {
        configured: !key.is_empty() && !settings.fofa_email.is_empty(),
        email: settings.fofa_email,
        key_masked: masked,
    })
}

#[tauri::command]
pub fn save_fofa_config(state: State<'_, AppState>, email: String, api_key: String) -> Result<FofaStatus, String> {
    if email.trim().is_empty() {
        return Err("请填写 FOFA 邮箱".to_string());
    }
    let email = email.trim().to_string();
    if !email.contains('@') {
        return Err("FOFA 邮箱格式不正确".to_string());
    }
    {
        let mut settings = state.settings.lock().map_err(|e| e.to_string())?;
        settings.fofa_email = email.clone();
        let snapshot = settings.clone();
        drop(settings);
        save_settings_inner(&state, &snapshot)?;
    }
    if !api_key.trim().is_empty() {
        vault::save_secret("fofa_api_key", api_key.trim())?;
    }
    fofa_status(state)
}

#[tauri::command]
pub fn clear_fofa_config(state: State<'_, AppState>) -> Result<(), String> {
    vault::delete_secret("fofa_api_key")?;
    let mut settings = state.settings.lock().map_err(|e| e.to_string())?;
    settings.fofa_email.clear();
    let snapshot = settings.clone();
    drop(settings);
    save_settings_inner(&state, &snapshot)
}

#[tauri::command]
pub async fn fofa_fetch(
    state: State<'_, AppState>,
    regions: Vec<String>,
    max_results: Option<usize>,
) -> Result<ImportResult, String> {
    let settings = state.settings.lock().map(|s| s.clone()).unwrap_or_default();
    let key = vault::get_secret("fofa_api_key")?.unwrap_or_default();
    if key.is_empty() {
        return Err("请先配置 FOFA API Key".to_string());
    }
    let (seeds, requests) = sources::fofa_search(
        &settings.fofa_email,
        &key,
        &settings,
        &regions,
        max_results.unwrap_or(400).clamp(20, 5000),
    )
    .await?;
    let fetched = seeds.len();
    let mut conn = db_guard(&state)?;
    let mut result = db::insert_candidates(&mut conn, seeds)?;
    record_import(&conn, "fofa", fetched, 0, &result);
    result.source_counts.insert("FOFA请求数".to_string(), requests);
    Ok(result)
}

#[tauri::command]
pub fn cm_status(state: State<'_, AppState>) -> Result<CmSourceStatus, String> {
    let conn = db_guard(&state)?;
    Ok(cm_source::load_status(&conn))
}

#[tauri::command]
pub async fn cm_fetch(
    app: AppHandle,
    state: State<'_, AppState>,
    format: Option<String>,
) -> Result<CmFetchOutcome, String> {
    let requested = format.unwrap_or_else(|| "auto".into());
    let outcome = cm_source::refresh_into_state(state.inner(), &requested).await?;
    let _ = app.emit("cm-source-updated", &outcome);
    Ok(outcome)
}

#[tauri::command]
pub fn cm_save_config(
    state: State<'_, AppState>,
    auto_update: bool,
    interval_minutes: u64,
) -> Result<CmSourceStatus, String> {
    if !(30..=10080).contains(&interval_minutes) {
        return Err("自动更新间隔需在 30 到 10080 分钟之间".into());
    }
    let conn = db_guard(&state)?;
    let mut status = cm_source::load_status(&conn);
    status.auto_update = auto_update;
    status.interval_minutes = interval_minutes;
    cm_source::save_status(&conn, &status)?;
    Ok(status)
}

#[tauri::command]
pub fn cm_preview(state: State<'_, AppState>, format: Option<String>) -> Result<CmRawPreview, String> {
    cm_source::preview_format(state.inner(), format.as_deref().unwrap_or("txt"))
}

#[tauri::command]
pub fn cm_clear(state: State<'_, AppState>) -> Result<CmSourceStatus, String> {
    let conn = db_guard(&state)?;
    conn.execute("DELETE FROM cm_records", []).map_err(|e| format!("清空 CM 来源记录失败: {e}"))?;
    let status = CmSourceStatus::default();
    cm_source::save_status(&conn, &status)?;
    Ok(status)
}

#[tauri::command]
pub fn list_url_sources(state: State<'_, AppState>) -> Result<Vec<UrlSource>, String> {
    let conn = db_guard(&state)?;
    db::list_url_sources(&conn)
}

#[tauri::command]
pub fn add_url_source(state: State<'_, AppState>, url: String, interval_minutes: Option<u64>) -> Result<Vec<UrlSource>, String> {
    let parsed = url::Url::parse(url.trim()).map_err(|_| "链接格式不正确".to_string())?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err("仅支持 http/https 链接".to_string());
    }
    let conn = db_guard(&state)?;
    db::upsert_url_source(&conn, url.trim(), interval_minutes.unwrap_or(60).max(5))?;
    db::list_url_sources(&conn)
}

#[tauri::command]
pub fn remove_url_source(state: State<'_, AppState>, id: i64) -> Result<Vec<UrlSource>, String> {
    let conn = db_guard(&state)?;
    db::remove_url_source(&conn, id)?;
    db::list_url_sources(&conn)
}

#[tauri::command]
pub async fn fetch_url_source(state: State<'_, AppState>, id: i64) -> Result<ImportResult, String> {
    let (url, interval) = {
        let conn = db_guard(&state)?;
        conn.query_row("SELECT url, interval_minutes FROM url_sources WHERE id=?1", params![id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)? as u64))
        })
        .map_err(|_| "链接记录不存在".to_string())?
    };
    let _ = interval;
    let result = async {
        let text = sources::fetch_url_text(&url, 8 * 1024 * 1024).await?;
        let port = state.settings.lock().map(|s| s.default_port).unwrap_or(443);
        let seeds = sources::parse_candidate_text(&text, port, "url", "");
        if seeds.is_empty() {
            return Err("链接内容中没有有效 IP".to_string());
        }
        let fetched = seeds.len();
        let mut conn = db_guard(&state)?;
        db::insert_candidates(&mut conn, seeds).map(|result| (result, fetched))
    }
    .await;
    if let Ok(conn) = state.db.lock() {
        match &result {
            Ok((r, fetched)) => {
                let duplicates = fetched.saturating_sub(r.added + r.updated);
                let _ = db::record_source_import(&conn, "url", *fetched, r.added, duplicates, 0, "正常", "");
                let _ = db::update_url_source_status(&conn, id, r.added + r.updated, "成功");
            }
            Err(e) => {
                let _ = db::record_source_import(&conn, "url", 0, 0, 0, 0, "更新失败", e);
                let _ = db::update_url_source_status(&conn, id, 0, &format!("失败: {e}"));
            }
        }
    }
    result.map(|(r, _)| r)
}

#[tauri::command]
pub async fn fetch_all_url_sources(state: State<'_, AppState>) -> Result<Value, String> {
    let list = {
        let conn = db_guard(&state)?;
        db::list_url_sources(&conn)?
    };
    let mut total_added = 0usize;
    let mut total_updated = 0usize;
    let mut failed = 0usize;
    let mut messages = Vec::new();
    for item in list.into_iter().filter(|s| s.enabled) {
        match fetch_url_source(state.clone(), item.id).await {
            Ok(r) => {
                total_added += r.added;
                total_updated += r.updated;
                messages.push(format!("[成功] {}: +{} / ~{}", item.url, r.added, r.updated));
            }
            Err(e) => {
                failed += 1;
                messages.push(format!("[失败] {}: {e}", item.url));
            }
        }
    }
    Ok(json!({"added": total_added, "updated": total_updated, "failed": failed, "messages": messages}))
}

#[tauri::command]
pub fn start_scan(app: AppHandle, state: State<'_, AppState>, request: ScanRequest) -> Result<String, String> {
    {
        let running = state.running_scan.lock().map_err(|e| e.to_string())?;
        if running.is_some() {
            return Err("已有扫描任务在运行".to_string());
        }
    }
    if request.regions.is_empty() {
        return Err("请至少选择一个地区".to_string());
    }
    if request.port == 0 {
        return Err("测试端口无效".to_string());
    }
    let scan_id = format!("scan-{}", &uuid::Uuid::new_v4().simple().to_string()[..12]);
    {
        let conn = db_guard(&state)?;
        db::create_scan(&conn, &scan_id, &request)?;
    }
    state.cancel.store(false, Ordering::SeqCst);
    state.paused.store(false, Ordering::SeqCst);
    *state.running_scan.lock().map_err(|e| e.to_string())? = Some(scan_id.clone());
    if let Ok(mut p) = state.last_progress.lock() {
        *p = Some(ScanProgress {
            scan_id: scan_id.clone(),
            status: "running".to_string(),
            phase: "starting".to_string(),
            message: "正在准备扫描任务".to_string(),
            started_at: db::now_text(),
            ..Default::default()
        });
    }
    let app2 = app.clone();
    let st = state.inner().clone();
    let sid = scan_id.clone();
    tauri::async_runtime::spawn(async move {
        let result = scanner::run_scan(app2.clone(), st.clone(), sid.clone(), request, st.cancel.clone()).await;
        if let Err(err) = result {
            if let Ok(conn) = st.db.lock() {
                let _ = db::finish_scan(&conn, &sid, "failed", 0, 0, 0.0, 0.0);
            }
            if let Ok(mut p) = st.last_progress.lock() {
                if let Some(progress) = p.as_mut() {
                    progress.status = "failed".to_string();
                    progress.phase = "failed".to_string();
                    progress.message = err.clone();
                    progress.finished_at = db::now_text();
                }
            }
            let _ = app2.emit(
                "scan-log",
                json!({
                    "scanId": sid,
                    "time": chrono::Local::now().format("%H:%M:%S").to_string(),
                    "level": "error",
                    "message": err,
                }),
            );
            let _ = app2.emit("scan-finished", json!({"scanId": sid, "status": "failed", "count": 0, "error": err}));
        }
        if let Ok(mut running) = st.running_scan.lock() {
            *running = None;
        }
        st.paused.store(false, Ordering::SeqCst);
    });
    Ok(scan_id)
}

#[tauri::command]
pub fn cancel_scan(state: State<'_, AppState>) -> Result<bool, String> {
    let running = state.running_scan.lock().map_err(|e| e.to_string())?.clone();
    if running.is_some() {
        state.cancel.store(true, Ordering::SeqCst);
        state.paused.store(false, Ordering::SeqCst);
        Ok(true)
    } else {
        Ok(false)
    }
}

#[tauri::command]
pub fn pause_scan(state: State<'_, AppState>) -> Result<bool, String> {
    let running = state.running_scan.lock().map_err(|e| e.to_string())?.clone();
    if running.is_none() {
        return Err("当前没有运行中的扫描任务".to_string());
    }
    state.paused.store(true, Ordering::SeqCst);
    Ok(true)
}

#[tauri::command]
pub fn resume_scan(state: State<'_, AppState>) -> Result<bool, String> {
    let running = state.running_scan.lock().map_err(|e| e.to_string())?.clone();
    if running.is_none() {
        return Err("当前没有运行中的扫描任务".to_string());
    }
    state.paused.store(false, Ordering::SeqCst);
    Ok(true)
}

#[tauri::command]
pub fn get_current_scan(state: State<'_, AppState>) -> Result<Option<ScanProgress>, String> {
    Ok(state.last_progress.lock().map_err(|e| e.to_string())?.clone())
}

#[tauri::command]
pub async fn retest_ip(state: State<'_, AppState>, ip: String, port: u16) -> Result<RetestOutcome, String> {
    let settings = state.settings.lock().map(|s| s.clone()).unwrap_or_default();
    let parsed = crate::speedtest::parse_ip(ip.trim()).map_err(|e| format!("IP 无效: {e}"))?;
    let mut rounds = Vec::new();
    // 与 v2rayN 的 tcping 一致：多轮 TCP 握手 RTT，不含 TLS/HTTP 开销。
    let latency = crate::speedtest::tcp_latency_probe(
        parsed,
        port,
        settings.latency_rounds.clamp(1, 10),
        settings.tcp_timeout_ms,
    )
    .await;
    for (index, sample) in latency.samples.iter().enumerate() {
        rounds.push(TestRound {
            phase: "tcp".to_string(),
            round: index + 1,
            latency_ms: Some(*sample),
            speed_mb_s: None,
            ok: true,
            message: format!("第 {} 轮 TCP 握手 {:.1} ms", index + 1, sample),
        });
    }
    if !latency.ok {
        rounds.push(TestRound {
            phase: "tcp".to_string(),
            round: latency.samples.len() + 1,
            latency_ms: None,
            speed_mb_s: None,
            ok: false,
            message: latency.last_error.clone(),
        });
        return Ok(RetestOutcome {
            ok: false,
            message: format!("TCP 延迟测试失败：{}", latency.last_error),
            tcp_ok: false,
            colo: String::new(),
            result: None,
            rounds,
        });
    }
    let trace = crate::speedtest::trace_probe(parsed, port, settings.https_timeout_ms).await;
    if !trace.ok {
        rounds.push(TestRound {
            phase: "https".to_string(),
            round: 1,
            latency_ms: Some(trace.elapsed_ms),
            speed_mb_s: None,
            ok: false,
            message: trace.error.clone(),
        });
        return Ok(RetestOutcome {
            ok: false,
            message: format!("HTTPS / Cloudflare trace 验证失败：{}", trace.error),
            tcp_ok: true,
            colo: trace.colo,
            result: None,
            rounds,
        });
    }
    rounds.push(TestRound {
        phase: "https".to_string(),
        round: 1,
        latency_ms: Some(trace.elapsed_ms),
        speed_mb_s: None,
        ok: true,
        message: format!(
            "HTTPS 验证通过 · colo {} · {} · 完整请求 {:.0} ms",
            trace.colo, trace.loc, trace.elapsed_ms
        ),
    });
    let download = crate::speedtest::download_probe(
        parsed,
        port,
        &settings.download_url,
        settings.download_seconds.clamp(2, 60),
        settings.min_download_bytes.max(64 * 1024),
    )
    .await;
    rounds.push(TestRound {
        phase: "download".to_string(),
        round: 1,
        latency_ms: None,
        speed_mb_s: Some(download.mb_s),
        ok: download.ok && download.mb_s > 0.0,
        message: if download.ok {
            format!("实际下载 {:.2} MB/s ({:.1} Mbps) · {:.1} MB / {:.1}s", download.mb_s, download.mbps, download.bytes as f64 / 1_000_000.0, download.seconds)
        } else {
            download.error.clone()
        },
    });
    let region = region_for_colo(&trace.colo).unwrap_or("unknown").to_string();
    let source = {
        let conn = db_guard(&state)?;
        conn.query_row(
            "SELECT sources FROM candidates WHERE ip = ?1 AND port = ?2",
            params![ip.trim(), port],
            |row| row.get::<_, String>(0),
        )
        .ok()
        .and_then(|raw| serde_json::from_str::<Vec<String>>(&raw).ok())
        .map(|list| list.join("/"))
        .unwrap_or_else(|| "手动复测".to_string())
    };
    let passed = download.ok && download.mb_s >= settings.speed_limit_mb_s && latency.ok;
    let result = if passed {
        Some(ScanResult {
            id: 0,
            scan_id: "retest".to_string(),
            rank: 0,
            ip: ip.trim().to_string(),
            port,
            region: region.clone(),
            region_name: node_display_name(&region, &trace.colo),
            colo: trace.colo.clone(),
            latency: latency.avg_ms,
            jitter: latency.jitter_ms,
            speed_mb_s: download.mb_s,
            speed_mbps: download.mbps,
            loss: latency.loss_pct,
            stability: 100.0,
            source,
            status: "retested".to_string(),
            tested_at: db::now_text(),
        })
    } else {
        None
    };
    {
        let conn = db_guard(&state)?;
        let _ = db::update_candidate_test(
            &conn,
            ip.trim(),
            port,
            if passed { "passed" } else if download.ok { "speed_below" } else { "download_failed" },
            Some(latency.avg_ms),
            Some(download.mb_s),
            Some(latency.loss_pct),
            Some(100.0),
            &trace.colo,
        );
    }
    Ok(RetestOutcome {
        ok: passed,
        message: if passed {
            format!("复测通过：{} · TCP {:.1} ms · {:.2} MB/s", trace.colo, latency.avg_ms, download.mb_s)
        } else if download.ok {
            format!("下载速度 {:.2} MB/s 低于阈值 {:.2} MB/s", download.mb_s, settings.speed_limit_mb_s)
        } else {
            format!("下载测速失败：{}", download.error)
        },
        tcp_ok: true,
        colo: trace.colo,
        result,
        rounds,
    })
}

#[tauri::command]
pub fn get_results(state: State<'_, AppState>, scan_id: Option<String>, region: Option<String>) -> Result<Vec<ScanResult>, String> {
    let conn = db_guard(&state)?;
    let sid = scan_id.filter(|s| !s.trim().is_empty()).or_else(|| db::latest_scan_id(&conn)).unwrap_or_default();
    if sid.is_empty() {
        return Ok(Vec::new());
    }
    db::list_results(&conn, &sid, region.as_deref().unwrap_or("all"))
}

#[tauri::command]
pub fn get_history(state: State<'_, AppState>, limit: Option<usize>) -> Result<Vec<ScanHistoryRow>, String> {
    let conn = db_guard(&state)?;
    db::list_history(&conn, limit.unwrap_or(100).min(500))
}

#[tauri::command]
pub fn delete_scan(state: State<'_, AppState>, scan_id: String) -> Result<(), String> {
    let conn = db_guard(&state)?;
    db::delete_scan(&conn, &scan_id)
}

#[tauri::command]
pub fn clear_history(state: State<'_, AppState>, scope: String) -> Result<(), String> {
    let conn = db_guard(&state)?;
    db::clear_history(&conn, &scope)
}

#[tauri::command]
pub fn backup_database(state: State<'_, AppState>, directory: Option<String>) -> Result<String, String> {
    let dir = directory.filter(|s| !s.trim().is_empty()).unwrap_or_else(|| {
        let home = std::env::var("USERPROFILE").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join("Documents").join("三笠优选备份").to_string_lossy().to_string()
    });
    let conn = db_guard(&state)?;
    db::backup_database(&conn, &dir)
}

#[tauri::command]
pub fn export_results(
    state: State<'_, AppState>,
    options: ExportOptions,
) -> Result<ExportOutcome, String> {
    let settings = state.settings.lock().map(|s| s.clone()).unwrap_or_default();
    let dir = if options.directory.trim().is_empty() {
        settings.export_dir
    } else {
        options.directory.trim().to_string()
    };
    let conn = db_guard(&state)?;
    export::write_exports(&conn, &dir, &options)
}

fn mask_secret(value: &str) -> String {
    if value.len() > 4 {
        format!("••••••••{}", &value[value.len() - 4..])
    } else if value.is_empty() {
        String::new()
    } else {
        "••••".to_string()
    }
}

#[tauri::command]
pub fn get_donation_qr(kind: String) -> Result<String, String> {
    crate::donation::qr_data_url(&kind)
}

#[tauri::command]
pub fn open_external_url(url: String) -> Result<(), String> {
    let parsed = url::Url::parse(url.trim()).map_err(|_| "外部链接格式无效".to_string())?;
    let host = parsed.host_str().unwrap_or("").to_ascii_lowercase();
    if !matches!(
        host.as_str(),
        "github.com" | "raw.githubusercontent.com" | "cdn.jsdelivr.net" | "fofa.info" | "www.fofa.info"
    ) {
        return Err(format!("不允许打开该外部链接: {host}"));
    }
    tauri_plugin_opener::open_url(parsed.as_str(), None::<&str>)
        .map_err(|e| format!("打开外部链接失败: {e}"))
}

#[tauri::command]
pub fn github_status(state: State<'_, AppState>) -> Result<Value, String> {
    let settings = state.settings.lock().map(|s| s.clone()).unwrap_or_default();
    let token = vault::get_secret("github_token")?.unwrap_or_default();
    Ok(json!({
        "configured": !token.is_empty(),
        "masked": mask_secret(&token),
        "repo": settings.github_repo,
        "branch": settings.github_default_branch,
        "remoteDir": settings.github_remote_dir,
    }))
}

#[tauri::command]
pub fn save_github_config(
    state: State<'_, AppState>,
    token: Option<String>,
    repo: Option<String>,
    branch: Option<String>,
    remote_dir: Option<String>,
) -> Result<Value, String> {
    if let Some(token) = token {
        if !token.trim().is_empty() {
            vault::save_secret("github_token", token.trim())?;
        }
    }
    let mut settings = state.settings.lock().map(|s| s.clone()).unwrap_or_default();
    if let Some(repo) = repo {
        settings.github_repo = repo.trim().to_string();
    }
    if let Some(branch) = branch {
        if !branch.trim().is_empty() {
            settings.github_default_branch = branch.trim().to_string();
        }
    }
    if let Some(dir) = remote_dir {
        settings.github_remote_dir = dir.trim().trim_matches('/').to_string();
    }
    save_settings_inner(&state, &settings)?;
    github_status(state)
}

#[tauri::command]
pub fn clear_github_token() -> Result<(), String> {
    vault::delete_secret("github_token")
}

#[tauri::command]
pub async fn github_check(state: State<'_, AppState>, repo: String, branch: Option<String>) -> Result<Value, String> {
    if repo.trim().is_empty() {
        return Err("请填写 GitHub 仓库地址".to_string());
    }
    let token = vault::get_secret("github_token")?.unwrap_or_default();
    if token.is_empty() {
        return Err("请先保存 GitHub Token".to_string());
    }
    let settings = state.settings.lock().map(|s| s.clone()).unwrap_or_default();
    github::check_repository(&repo, branch.as_deref().unwrap_or(&settings.github_default_branch), &token).await
}

#[tauri::command]
pub async fn github_upload(
    state: State<'_, AppState>,
    repo: String,
    branch: Option<String>,
    remote_dir: Option<String>,
    scan_id: Option<String>,
    options: Option<ExportOptions>,
    split_regions: Option<bool>,
    write_mode: Option<String>,
) -> Result<GithubUploadOutcome, String> {
    if repo.trim().is_empty() {
        return Err("请填写 GitHub 仓库地址".to_string());
    }
    let token = vault::get_secret("github_token")?.unwrap_or_default();
    if token.is_empty() {
        return Err("请先保存 GitHub Token".to_string());
    }
    let settings = state.settings.lock().map(|s| s.clone()).unwrap_or_default();
    let branch = branch.filter(|s| !s.trim().is_empty()).unwrap_or(settings.github_default_branch);
    let remote_dir = remote_dir.unwrap_or(settings.github_remote_dir);
    let files = {
        let conn = db_guard(&state)?;
        let sid = scan_id
            .clone()
            .filter(|s| !s.trim().is_empty())
            .or_else(|| db::latest_scan_id(&conn))
            .unwrap_or_default();
        let mut export_options = options.unwrap_or_default();
        export_options.scan_id = sid;
        export_options.directory.clear();
        export::build_github_files(&conn, &export_options, split_regions.unwrap_or(false))?
    };
    let scan_label = scan_id.unwrap_or_else(|| "latest".to_string());
    let merge_existing = write_mode
        .as_deref()
        .map(|mode| mode.eq_ignore_ascii_case("merge"))
        .unwrap_or(false);
    let outcome = github::upload_results(
        &repo,
        &branch,
        &remote_dir,
        &token,
        files,
        &scan_label,
        merge_existing,
    )
    .await;
    if let Ok(conn) = state.db.lock() {
        match &outcome {
            Ok(o) => {
                let _ = db::record_github_history(&conn, &o.repo, &o.branch, &o.files, "成功", &format!("commit {}", o.commit_sha));
            }
            Err(e) => {
                let _ = db::record_github_history(&conn, &repo, &branch, &[], "失败", e);
            }
        }
    }
    outcome
}

#[tauri::command]
pub fn get_github_history(state: State<'_, AppState>) -> Result<Vec<Value>, String> {
    let conn = db_guard(&state)?;
    db::github_history(&conn, 50)
}

#[tauri::command]
pub fn censys_status(state: State<'_, AppState>) -> Result<CensysStatus, String> {
    let token = vault::get_secret("censys_api_token")?.unwrap_or_default();
    let masked = if token.len() > 4 {
        format!("••••••••{}", &token[token.len() - 4..])
    } else if token.is_empty() {
        String::new()
    } else {
        "••••".to_string()
    };
    let conn = db_guard(&state)?;
    let saved = db::get_meta(&conn, "censys_status")
        .and_then(|v| serde_json::from_str::<Value>(&v).ok())
        .unwrap_or_else(|| json!({}));
    Ok(CensysStatus {
        configured: !token.is_empty(),
        token_masked: masked,
        last_fetch: saved.get("lastFetch").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        last_count: saved.get("lastCount").and_then(|v| v.as_u64()).unwrap_or(0) as usize,
        last_message: saved.get("lastMessage").and_then(|v| v.as_str()).unwrap_or("").to_string(),
    })
}

#[tauri::command]
pub fn save_censys_config(token: String) -> Result<CensysStatus, String> {
    if token.trim().is_empty() {
        return Err("请填写 Censys Personal Access Token".to_string());
    }
    vault::save_secret("censys_api_token", token.trim())?;
    Ok(CensysStatus {
        configured: true,
        token_masked: if token.len() > 4 {
            format!("••••••••{}", &token[token.len() - 4..])
        } else {
            "••••".to_string()
        },
        last_fetch: String::new(),
        last_count: 0,
        last_message: String::new(),
    })
}

#[tauri::command]
pub fn clear_censys_config() -> Result<(), String> {
    vault::delete_secret("censys_api_token")
}

#[tauri::command]
pub async fn censys_fetch(
    state: State<'_, AppState>,
    max_results: Option<usize>,
) -> Result<CensysFetchOutcome, String> {
    let token = vault::get_secret("censys_api_token")?.unwrap_or_default();
    if token.is_empty() {
        return Err("请先配置 Censys Personal Access Token（平台 API）".to_string());
    }
    let wanted = max_results.unwrap_or(100).clamp(1, 100);
    let (seeds, total) = sources::censys_search(&token, wanted).await?;
    let fetched = seeds.len();
    let mut conn = db_guard(&state)?;
    let result = db::insert_candidates(&mut conn, seeds)?;
    record_import(&conn, "censys", fetched, 0, &result);
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let _ = db::set_meta(
        &conn,
        "censys_status",
        &json!({"lastFetch": now, "lastCount": fetched, "lastMessage": format!("匹配总数 {}，本页导入 {}", total, fetched)}).to_string(),
    );
    Ok(CensysFetchOutcome {
        fetched,
        total,
        message: format!(
            "Censys 匹配总数 {}，本次导入 {} 条（仅取前 {} 条，避免消耗额外 credits）",
            total, fetched, wanted
        ),
        result,
    })
}

#[tauri::command]
pub fn vault_status() -> Result<VaultStatus, String> {
    let fofa = vault::get_secret("fofa_api_key")?.map(|s| !s.is_empty()).unwrap_or(false);
    let github = vault::get_secret("github_token")?.map(|s| !s.is_empty()).unwrap_or(false);
    Ok(VaultStatus {
        fofa_configured: fofa,
        github_configured: github,
    })
}

#[tauri::command]
pub fn app_info(state: State<'_, AppState>) -> Result<Value, String> {
    Ok(json!({
        "version": env!("CARGO_PKG_VERSION"),
        "dataDir": state.db_path.parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
        "dbPath": state.db_path.to_string_lossy().to_string(),
    }))
}

#[tauri::command]
pub async fn speed_source_info(state: State<'_, AppState>) -> Result<Value, String> {
    let settings = state.settings.lock().map(|s| s.clone()).unwrap_or_default();
    let isp = crate::speed_source::detect_isp().await;
    if settings.speed_source == crate::speed_source::SOURCE_AUTO {
        let (key, _url, probes) = crate::speed_source::resolve_auto(&settings.custom_speed_url).await;
        return Ok(json!({
            "source": settings.speed_source,
            "effectiveSource": key,
            "effectiveLabel": crate::speed_source::label_for(&key),
            "sourceUrl": crate::speed_source::url_for(&key, &settings.custom_speed_url).unwrap_or_default(),
            "backupSource": settings.backup_speed_source,
            "autoSwitch": settings.auto_switch_speed_source,
            "customUrl": settings.custom_speed_url,
            "isp": isp,
            "probes": probes,
        }));
    }
    let url = crate::speed_source::url_for(&settings.speed_source, &settings.custom_speed_url)
        .unwrap_or_else(|| settings.download_url.clone());
    Ok(json!({
        "source": settings.speed_source,
        "effectiveSource": settings.speed_source,
        "effectiveLabel": crate::speed_source::label_for(&settings.speed_source),
        "sourceUrl": url,
        "backupSource": settings.backup_speed_source,
        "autoSwitch": settings.auto_switch_speed_source,
        "customUrl": settings.custom_speed_url,
        "isp": isp,
        "probes": Value::Null,
    }))
}

#[tauri::command]
pub async fn speed_source_probe(state: State<'_, AppState>) -> Result<Vec<crate::speed_source::SpeedSourceProbe>, String> {
    let settings = state.settings.lock().map(|s| s.clone()).unwrap_or_default();
    let mut probes = Vec::new();
    for key in [
        crate::speed_source::SOURCE_CLOUDFLARE,
        crate::speed_source::SOURCE_CM,
        crate::speed_source::SOURCE_MOBILE,
        crate::speed_source::SOURCE_CUSTOM,
    ] {
        if let Some(url) = crate::speed_source::url_for(key, &settings.custom_speed_url) {
            probes.push(crate::speed_source::probe_url(key, &url).await);
        }
    }
    Ok(probes)
}

#[tauri::command]
pub async fn speed_source_compare(
    state: State<'_, AppState>,
    ip: String,
    port: u16,
) -> Result<crate::speed_source::SpeedSourceCompare, String> {
    let settings = state.settings.lock().map(|s| s.clone()).unwrap_or_default();
    let target = speedtest::parse_ip(&ip)?;
    let duration = settings.download_seconds.clamp(2, 8);
    let min_bytes = settings.min_download_bytes.min(2 * 1024 * 1024).max(64 * 1024);
    Ok(crate::speed_source::compare_for_ip(
        target,
        port,
        &settings.custom_speed_url,
        duration,
        min_bytes,
    )
    .await)
}

#[tauri::command]
pub fn open_data_dir(app: AppHandle) -> Result<String, String> {
    let dir = app.path().app_data_dir().map_err(|e| format!("获取数据目录失败: {e}"))?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建数据目录失败: {e}"))?;
    #[cfg(windows)]
    {
        std::process::Command::new("explorer")
            .arg(&dir)
            .spawn()
            .map_err(|e| format!("打开目录失败: {e}"))?;
    }
    Ok(dir.to_string_lossy().to_string())
}
