pub mod cm_source;
pub mod commands;
pub mod db;
pub mod donation;
pub mod export;
pub mod github;
pub mod models;
pub mod network;
pub mod registry;
pub mod scanner;
pub mod sources;
pub mod speed_source;
pub mod speedtest;
pub mod vault;

use crate::models::{AppSettings, ScanProgress};
use chrono::NaiveDateTime;
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{Emitter, Manager};

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Mutex<Connection>>,
    pub settings: Arc<Mutex<AppSettings>>,
    pub cancel: Arc<AtomicBool>,
    pub paused: Arc<AtomicBool>,
    pub running_scan: Arc<Mutex<Option<String>>>,
    pub last_progress: Arc<Mutex<Option<ScanProgress>>>,
    pub cm_fetching: Arc<AtomicBool>,
    pub ui_zoom: Arc<Mutex<f64>>,
    pub db_path: PathBuf,
}

fn monitor_logical_size(window: &tauri::WebviewWindow) -> Option<(f64, f64)> {
    if let Ok(Some(monitor)) = window.current_monitor() {
        let scale_factor = monitor.scale_factor().max(1.0);
        return Some((
            monitor.size().width as f64 / scale_factor,
            monitor.size().height as f64 / scale_factor,
        ));
    }
    let size = window.inner_size().ok()?;
    let scale_factor = window.scale_factor().ok()?.max(1.0);
    Some((
        size.width as f64 / scale_factor,
        size.height as f64 / scale_factor,
    ))
}

fn desired_ui_scale(window: &tauri::WebviewWindow, state: &AppState) -> Result<f64, String> {
    let setting = state
        .settings
        .lock()
        .map(|settings| settings.ui_scale.clone())
        .unwrap_or_else(|_| "auto".to_string());
    let (logical_width, logical_height) = monitor_logical_size(window).unwrap_or((1920.0, 1080.0));
    let auto = (logical_width / 1920.0)
        .min(logical_height / 1080.0)
        .clamp(1.0, 2.25);
    let desired = if setting.trim().is_empty() || setting.eq_ignore_ascii_case("auto") {
        auto
    } else {
        setting
            .trim()
            .parse::<f64>()
            .map_err(|_| "界面缩放设置无效".to_string())?
            .clamp(0.75, 2.5)
    };
    // 窗口必须仍能容纳 1360×860 的设计尺寸；2K/4K 自动放大窗口和 WebView 缩放，
    // 而不是只放大字体导致布局溢出。
    let fit = ((logical_width * 0.92) / 1360.0)
        .min((logical_height * 0.90) / 860.0)
        .max(1.0);
    Ok(desired.min(fit).clamp(1.0, 2.5))
}

pub fn apply_ui_scale(
    window: &tauri::WebviewWindow,
    state: &AppState,
    force: bool,
) -> Result<(), String> {
    let factor = desired_ui_scale(window, state)?;
    let rounded = (factor * 20.0).round() / 20.0;
    let mut last = state.ui_zoom.lock().map_err(|e| e.to_string())?;
    let changed = (*last - rounded).abs() >= 0.001;
    if !force && !changed {
        return Ok(());
    }
    if force && changed {
        if let Some((logical_width, logical_height)) = monitor_logical_size(window) {
            let width = (1360.0 * rounded)
                .min(logical_width * 0.92)
                .max(1280.0);
            let height = (860.0 * rounded)
                .min(logical_height * 0.90)
                .max(800.0);
            let _ = window.set_min_size(Some(tauri::LogicalSize::new(
                1280.0 * rounded,
                800.0 * rounded,
            )));
            let _ = window.set_size(tauri::LogicalSize::new(width, height));
            let _ = window.center();
        }
    }
    window
        .set_zoom(rounded)
        .map_err(|e| format!("设置界面缩放失败: {e}"))?;
    *last = rounded;
    Ok(())
}

fn spawn_cm_refresher(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(40)).await;
        let mut ticker = tokio::time::interval(Duration::from_secs(60));
        loop {
            ticker.tick().await;
            let state = app.state::<AppState>().inner().clone();
            let (auto, due, format) = {
                let Ok(conn) = state.db.lock() else { continue };
                let status = cm_source::load_status(&conn);
                let stamp = if status.last_attempt.trim().is_empty() {
                    status.last_fetch.clone()
                } else {
                    status.last_attempt.clone()
                };
                let due = if stamp.trim().is_empty() {
                    true
                } else {
                    NaiveDateTime::parse_from_str(&stamp, "%Y-%m-%d %H:%M:%S")
                        .map(|t| {
                            chrono::Local::now()
                                .naive_local()
                                .signed_duration_since(t)
                                .num_minutes() as u64
                                >= status.interval_minutes.max(30)
                        })
                        .unwrap_or(true)
                };
                (status.auto_update, due, status.format.clone())
            };
            if auto && due {
                if let Ok(outcome) = cm_source::refresh_into_state(&state, &format).await {
                    let _ = app.emit("cm-source-updated", outcome);
                }
            }
        }
    });
}

fn spawn_url_refresher(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(25)).await;
        let mut ticker = tokio::time::interval(Duration::from_secs(60));
        loop {
            ticker.tick().await;
            let state = app.state::<AppState>().inner().clone();
            let due: Vec<(i64, String, u64)> = {
                let Ok(conn) = state.db.lock() else { continue };
                let Ok(list) = db::list_url_sources(&conn) else { continue };
                list.into_iter()
                    .filter(|s| s.enabled)
                    .filter(|s| {
                        if s.last_fetch.trim().is_empty() {
                            return true;
                        }
                        let last = NaiveDateTime::parse_from_str(&s.last_fetch, "%Y-%m-%d %H:%M:%S").ok();
                        last.map(|t| {
                            let elapsed = chrono::Local::now().naive_local().signed_duration_since(t);
                            elapsed.num_minutes() as u64 >= s.interval_minutes.max(5)
                        })
                        .unwrap_or(true)
                    })
                    .map(|s| (s.id, s.url, s.interval_minutes))
                    .collect()
            };
            if due.is_empty() {
                continue;
            }
            let mut total = 0usize;
            for (id, url, _interval) in due {
                match sources::fetch_url_text(&url, 8 * 1024 * 1024).await {
                    Ok(text) => {
                        let port = state.settings.lock().map(|s| s.default_port).unwrap_or(443);
                        let seeds = sources::parse_candidate_text(&text, port, "url", "");
                        if seeds.is_empty() {
                            if let Ok(conn) = state.db.lock() {
                                let _ = db::update_url_source_status(&conn, id, 0, "未解析到 IP");
                            }
                            continue;
                        }
                        let inserted = if let Ok(mut conn) = state.db.lock() {
                            db::insert_candidates(&mut conn, seeds)
                        } else {
                            Err("数据库忙".to_string())
                        };
                        match inserted {
                            Ok(r) => {
                                total += r.added + r.updated;
                                if let Ok(conn) = state.db.lock() {
                                    let _ = db::update_url_source_status(&conn, id, r.added + r.updated, "自动更新成功");
                                }
                            }
                            Err(e) => {
                                if let Ok(conn) = state.db.lock() {
                                    let _ = db::update_url_source_status(&conn, id, 0, &format!("失败: {e}"));
                                }
                            }
                        }
                    }
                    Err(e) => {
                        if let Ok(conn) = state.db.lock() {
                            let _ = db::update_url_source_status(&conn, id, 0, &format!("失败: {e}"));
                        }
                    }
                }
            }
            if total > 0 {
                let _ = app.emit("url-sources-updated", serde_json::json!({"imported": total}));
            }
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .on_window_event(|window, event| {
            let force = matches!(event, tauri::WindowEvent::ScaleFactorChanged { .. });
            if matches!(
                event,
                tauri::WindowEvent::Resized(_) | tauri::WindowEvent::ScaleFactorChanged { .. }
            ) {
                if let (Some(state), Some(webview)) = (
                    window.try_state::<AppState>(),
                    window.app_handle().get_webview_window(window.label()),
                ) {
                    let _ = apply_ui_scale(&webview, &state, force);
                }
            }
        })
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("无法获取应用数据目录: {e}"))?;
            std::fs::create_dir_all(&data_dir)?;
            let db_path = data_dir.join("mikasa-optimizer.sqlite");
            let conn = db::open_database(&db_path).map_err(|e| Box::<dyn std::error::Error>::from(e))?;
            let settings = db::load_settings(&conn);
            let _ = conn.execute(
                "UPDATE scans SET status='failed', finished_at=?1 WHERE status='running'",
                rusqlite::params![db::now_text()],
            );
            let state = AppState {
                db: Arc::new(Mutex::new(conn)),
                settings: Arc::new(Mutex::new(settings)),
                cancel: Arc::new(AtomicBool::new(false)),
                paused: Arc::new(AtomicBool::new(false)),
                running_scan: Arc::new(Mutex::new(None)),
                last_progress: Arc::new(Mutex::new(None)),
                cm_fetching: Arc::new(AtomicBool::new(false)),
                ui_zoom: Arc::new(Mutex::new(1.0)),
                db_path,
            };
            app.manage(state);
            if let Some(window) = app.get_webview_window("main") {
                if let Some(icon) = app.default_window_icon() {
                    let _ = window.set_icon(icon.clone());
                }
                if let Some(state) = app.try_state::<AppState>() {
                    let _ = apply_ui_scale(&window, &state, true);
                }
            }
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(Duration::from_millis(220)).await;
                if let Some(window) = handle.get_webview_window("main") {
                    if let Some(state) = handle.try_state::<AppState>() {
                        let _ = apply_ui_scale(&window, &state, false);
                    }
                }
            });
            spawn_url_refresher(app.handle().clone());
            spawn_cm_refresher(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_bootstrap,
            commands::detect_network,
            commands::get_dashboard,
            commands::get_settings,
            commands::save_settings,
            commands::list_candidates,
            commands::preview_file,
            commands::import_text,
            commands::import_file,
            commands::import_urls,
            commands::community_source_list,
            commands::community_source_status,
            commands::community_fetch,
            commands::source_quality,
            commands::registry_sources,
            commands::registry_discover,
            commands::cloudflare_fetch,
            commands::fofa_preview,
            commands::fofa_status,
            commands::save_fofa_config,
            commands::clear_fofa_config,
            commands::fofa_fetch,
            commands::cm_status,
            commands::cm_fetch,
            commands::cm_save_config,
            commands::cm_preview,
            commands::cm_clear,
            commands::list_url_sources,
            commands::add_url_source,
            commands::remove_url_source,
            commands::fetch_url_source,
            commands::fetch_all_url_sources,
            commands::start_scan,
            commands::cancel_scan,
            commands::pause_scan,
            commands::resume_scan,
            commands::get_current_scan,
            commands::get_results,
            commands::retest_ip,
            commands::get_history,
            commands::delete_scan,
            commands::clear_history,
            commands::backup_database,
            commands::export_results,
            commands::github_status,
            commands::save_github_config,
            commands::clear_github_token,
            commands::github_check,
            commands::github_upload,
            commands::get_github_history,
            commands::vault_status,
            commands::censys_status,
            commands::save_censys_config,
            commands::clear_censys_config,
            commands::censys_fetch,
            commands::speed_source_probe,
            commands::speed_source_compare,
            commands::speed_source_info,
            commands::app_info,
            commands::get_donation_qr,
            commands::open_external_url,
            commands::open_data_dir
        ])
        .run(tauri::generate_context!())
        .expect("error while running Mikasa Optimizer");
}
