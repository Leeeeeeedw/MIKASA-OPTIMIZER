use crate::{db, models::*, speedtest, AppState};
use chrono::Local;
use futures_util::stream::{self, StreamExt};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct LogEvent {
    scan_id: String,
    time: String,
    level: String,
    message: String,
}

#[derive(Clone)]
struct Verified {
    candidate: Candidate,
    region: String,
    /// 统计分组：首选地区使用其 key；其他实际国家归入 global；无法识别归入 unknown
    group: String,
    trace: speedtest::TraceProbe,
    latency: speedtest::LatencyProbe,
    download: Option<speedtest::DownloadProbe>,
    stability: f64,
    score: f64,
    /// 延迟测试成功但超过用户设置的延迟上限：可展示、可下载测速，但不进入正式导出
    below_limit: bool,
}

fn selected_regions(req: &ScanRequest) -> Vec<String> {
    if req.regions.iter().any(|r| r == "all") {
        REGION_ORDER.iter().map(|s| s.to_string()).collect()
    } else {
        req.regions
            .iter()
            .filter(|r| REGION_ORDER.contains(&r.as_str()))
            .cloned()
            .collect()
    }
}

fn log(app: &AppHandle, state: &AppState, scan_id: &str, level: &str, message: &str) {
    let evt = LogEvent {
        scan_id: scan_id.to_string(),
        time: Local::now().format("%H:%M:%S").to_string(),
        level: level.to_string(),
        message: message.to_string(),
    };
    let _ = app.emit("scan-log", evt);
    if let Ok(dir) = app.path().app_data_dir() {
        let log_dir = dir.join("logs");
        let _ = std::fs::create_dir_all(&log_dir);
        let line = format!(
            "[{}] [{}] {}\n",
            Local::now().format("%Y-%m-%d %H:%M:%S"),
            level.to_uppercase(),
            message
        );
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(log_dir.join(format!("scan-{scan_id}.log"))) {
            let _ = f.write_all(line.as_bytes());
        }
    }
    let _ = state;
}

fn emit_progress(app: &AppHandle, progress: &Arc<Mutex<ScanProgress>>, state: &AppState) {
    let snapshot = progress.lock().map(|p| p.clone()).unwrap_or_default();
    if let Ok(mut last) = state.last_progress.lock() {
        *last = Some(snapshot.clone());
    }
    let _ = app.emit("scan-progress", snapshot);
}

fn is_cancelled(cancel: &Arc<AtomicBool>) -> bool {
    cancel.load(Ordering::Relaxed)
}

/// 在阶段循环之间响应暂停：暂停期间不拉取新的测试任务，已经发出的请求会自然完成。
/// 返回 true 表示等待期间收到了取消请求。
async fn wait_while_paused(
    app: &AppHandle,
    state: &AppState,
    progress: &Arc<Mutex<ScanProgress>>,
    cancel: &Arc<AtomicBool>,
    scan_id: &str,
) -> bool {
    if !state.paused.load(Ordering::Relaxed) {
        return false;
    }
    {
        if let Ok(mut p) = progress.lock() {
            p.status = "paused".to_string();
            p.message = "任务已暂停，点击「继续」恢复扫描".to_string();
        }
        emit_progress(app, progress, state);
        log(app, state, scan_id, "warn", "扫描已暂停：等待当前测试项完成后不再派发新任务");
    }
    loop {
        if is_cancelled(cancel) {
            return true;
        }
        if !state.paused.load(Ordering::Relaxed) {
            if let Ok(mut p) = progress.lock() {
                if p.status == "paused" {
                    p.status = "running".to_string();
                    p.message = "任务已继续".to_string();
                }
            }
            emit_progress(app, progress, state);
            log(app, state, scan_id, "info", "扫描已继续");
            return false;
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

fn save_candidate_status(
    state: &AppState,
    candidate: &Candidate,
    status: &str,
    latency: Option<f64>,
    speed: Option<f64>,
    loss: Option<f64>,
    stability: Option<f64>,
    colo: &str,
) {
    if let Ok(conn) = state.db.lock() {
        let _ = db::update_candidate_test(
            &conn,
            &candidate.ip,
            candidate.port,
            status,
            latency,
            speed,
            loss,
            stability,
            colo,
        );
    }
}

fn score_for(v: &Verified, speed_limit: f64) -> f64 {
    let speed = v.download.as_ref().map(|d| d.mb_s).unwrap_or(0.0);
    let speed_norm = ((speed / speed_limit.max(0.1)).min(3.0) / 3.0 * 100.0).max(0.0);
    let latency_norm = (100.0 - v.latency.avg_ms / 4.0).clamp(0.0, 100.0);
    let loss_norm = (100.0 - v.latency.loss_pct).clamp(0.0, 100.0);
    (speed_norm * 0.60 + latency_norm * 0.30 + loss_norm * 0.10).clamp(0.0, 100.0)
}

pub async fn run_scan(
    app: AppHandle,
    state: AppState,
    scan_id: String,
    req: ScanRequest,
    cancel: Arc<AtomicBool>,
) -> Result<(), String> {
    let settings = state.settings.lock().map(|s| s.clone()).unwrap_or_default();
    let regions = selected_regions(&req);
    if regions.is_empty() {
        return Err("请选择至少一个目标地区".to_string());
    }
    let candidates = {
        let conn = state.db.lock().map_err(|e| format!("读取候选池失败: {e}"))?;
        db::get_candidates_for_scan(&conn, &req)?
    };
    if candidates.is_empty() {
        return Err("候选池为空，请先导入 IP 来源".to_string());
    }
    let mut stat_keys = regions.clone();
    for key in ["global", "unknown"] {
        if !stat_keys.iter().any(|value| value == key) {
            stat_keys.push(key.to_string());
        }
    }
    let mut initial_stats = Vec::new();
    for key in &stat_keys {
        let mut stat = RegionStats {
            key: key.clone(),
            name: region_name(key).to_string(),
            flag: region_flag(key).to_string(),
            ..Default::default()
        };
        if key != "global" && key != "unknown" {
            stat.candidates = candidates.iter().filter(|c| c.initial_region == *key).count();
        }
        initial_stats.push(stat);
    }
    let progress = Arc::new(Mutex::new(ScanProgress {
        scan_id: scan_id.clone(),
        status: "running".to_string(),
        phase: "tcp".to_string(),
        message: "正在测试 TCP 连通性".to_string(),
        total: candidates.len(),
        region_stats: initial_stats,
        started_at: db::now_text(),
        ..Default::default()
    }));
    emit_progress(&app, &progress, &state);
    log(
        &app,
        &state,
        &scan_id,
        "info",
        &format!(
            "开始扫描：{} 个深度测试候选（按历史真实成绩、地区配额与新节点探索综合选取，不按导入顺序截断）；目标地区 {}，并发 {}，端口 {}",
            candidates.len(),
            regions.iter().map(|k| region_name(k)).collect::<Vec<_>>().join("、"),
            req.concurrency,
            req.port
        ),
    );

    // Phase 1: TCP reachability
    let tcp_conc = req.concurrency.clamp(1, 256);
    let mut tcp_ok: Vec<Candidate> = Vec::new();
    let mut tcp_stream = stream::iter(candidates.clone().into_iter().map(|c| {
        let timeout = settings.tcp_timeout_ms;
        async move {
            let ip = speedtest::parse_ip(&c.ip);
            let probe = match ip {
                Ok(ip) => speedtest::tcp_probe(ip, c.port, timeout).await,
                Err(e) => speedtest::TcpProbe { ok: false, error: e, ..Default::default() },
            };
            (c, probe)
        }
    }))
    .buffer_unordered(tcp_conc);
    let mut done = 0usize;
    while let Some((candidate, probe)) = tcp_stream.next().await {
        wait_while_paused(&app, &state, &progress, &cancel, &scan_id).await;
        if is_cancelled(&cancel) {
            break;
        }
        done += 1;
        if probe.ok {
            tcp_ok.push(candidate.clone());
            if let Ok(mut p) = progress.lock() {
                p.tcp_ok += 1;
                p.tcp_tested = done;
                p.tcp_passed = p.tcp_ok;
                p.tested = done;
                if let Some(stat) = p.region_stats.iter_mut().find(|s| s.key == candidate.initial_region) {
                    stat.tcp_ok += 1;
                }
            }
            save_candidate_status(&state, &candidate, "tcp_ok", None, None, None, None, "");
        } else {
            if let Ok(mut p) = progress.lock() {
                p.tcp_tested = done;
            }
            save_candidate_status(&state, &candidate, "tcp_failed", None, None, None, None, "");
        }
        if done % 25 == 0 || done == candidates.len() {
            if let Ok(mut p) = progress.lock() {
                p.tested = done;
                p.message = format!("TCP 连通性：{done}/{}，成功 {}", candidates.len(), p.tcp_ok);
            }
            emit_progress(&app, &progress, &state);
            log(&app, &state, &scan_id, "info", &format!("TCP 进度 {done}/{}，通过 {}", candidates.len(), tcp_ok.len()));
        }
    }
    if is_cancelled(&cancel) {
        finish_cancelled(&app, &state, &progress, &scan_id, candidates.len());
        return Ok(());
    }
    if tcp_ok.is_empty() {
        return Err("TCP 连通性测试无可用目标".to_string());
    }
    log(&app, &state, &scan_id, "success", &format!("TCP 测试完成：{}/{} 可连接", tcp_ok.len(), candidates.len()));

    // Phase 2: HTTPS validation + real multi-round latency
    if let Ok(mut p) = progress.lock() {
        p.phase = "https".to_string();
        p.message = "正在验证 HTTPS 与 Cloudflare colo".to_string();
        p.tested = 0;
    }
    emit_progress(&app, &progress, &state);
    // HTTPS/trace 阶段也会建立 TLS 连接；并发过高会让家用路由排队并污染延迟，
    // 因此上限压到 32，保证后续 TCP RTT 接近 v2rayN 的顺序测速结果。
    let https_conc = req.concurrency.clamp(4, 32);
    // TCP 握手很轻，但并发过高仍会让本机 NAT / 路由器排队，测出的 RTT 会虚高。
    // 用独立的小并发闸门测延迟，才能对齐 v2rayN 的 tcping 结果。
    let latency_conc = (req.concurrency / 8).clamp(4, 10);
    let latency_gate = Arc::new(tokio::sync::Semaphore::new(latency_conc));
    // 所有通过 colo + 多轮延迟的候选都进入下载候选池；延迟上限只在最终筛选时应用
    let mut verified_all: Vec<Verified> = Vec::new();
    let mut https_stream = stream::iter(tcp_ok.clone().into_iter().map(|c| {
        let timeout = settings.https_timeout_ms;
        let tcp_timeout = settings.tcp_timeout_ms;
        let rounds = req.latency_rounds.clamp(1, 10);
        let gate = latency_gate.clone();
        async move {
            let ip = match speedtest::parse_ip(&c.ip) {
                Ok(ip) => ip,
                Err(e) => {
                    return (
                        c,
                        speedtest::TraceProbe { error: e, ..Default::default() },
                        speedtest::LatencyProbe::default(),
                    )
                }
            };
            let trace = speedtest::trace_probe(ip, c.port, timeout).await;
            if !trace.ok {
                return (c, trace, speedtest::LatencyProbe::default());
            }
            let latency = match gate.acquire_owned().await {
                Ok(_permit) => {
                    speedtest::tcp_latency_probe(ip, c.port, rounds, tcp_timeout).await
                }
                Err(_) => speedtest::LatencyProbe::default(),
            };
            (c, trace, latency)
        }
    }))
    .buffer_unordered(https_conc);
    let total_https = tcp_ok.len();
    done = 0;
    while let Some((candidate, trace, latency)) = https_stream.next().await {
        wait_while_paused(&app, &state, &progress, &cancel, &scan_id).await;
        if is_cancelled(&cancel) {
            break;
        }
        done += 1;
        if trace.ok {
            let region = region_for_any_colo(&trace.colo);
            if let Some(key) = region {
                let group_key = if key == "unknown" {
                    "unknown"
                } else if regions.iter().any(|value| value == key) {
                    key
                } else {
                    "global"
                };
                if let Ok(mut p) = progress.lock() {
                    p.https_ok += 1;
                    p.colo_ok += 1;
                    p.https_tested = done;
                    p.https_passed = p.https_ok;
                    if let Some(stat) = p.region_stats.iter_mut().find(|s| s.key == group_key) {
                        // 无初始地区标签的候选（如 CM 库）只有拿到真实 colo 后才知道归属；
                        // 把它补记到实际地区池，避免地区表出现「候选 0 / TCP 0 但 HTTPS 47」。
                        if candidate.initial_region != *key && candidate.initial_region != group_key {
                            stat.candidates += 1;
                            stat.tcp_ok += 1;
                        }
                        stat.https_ok += 1;
                        stat.colo_ok += 1;
                    }
                }
                if latency.ok {
                    // 延迟上限只做标记，不在这一步淘汰：超限候选仍进入下载测速，最终筛选时排除
                    let within_limit = req.latency_limit_ms <= 0.0 || latency.avg_ms <= req.latency_limit_ms;
                    if let Ok(mut p) = progress.lock() {
                        p.latency_ok += 1;
                        p.latency_tested += 1;
                        p.latency_passed += 1;
                        if !within_limit {
                            p.latency_eliminated += 1;
                        }
                        if let Some(stat) = p.region_stats.iter_mut().find(|s| s.key == group_key) {
                            stat.latency_ok += 1;
                            if !within_limit {
                                stat.latency_eliminated += 1;
                            }
                        }
                    }
                    verified_all.push(Verified {
                        candidate: candidate.clone(),
                        region: key.to_string(),
                        group: group_key.to_string(),
                        trace: trace.clone(),
                        latency: latency.clone(),
                        download: None,
                        stability: 100.0,
                        score: 0.0,
                        below_limit: !within_limit,
                    });
                    save_candidate_status(
                        &state,
                        &candidate,
                        if within_limit { "latency_ok" } else { "latency_below" },
                        Some(latency.avg_ms),
                        None,
                        Some(latency.loss_pct),
                        None,
                        &trace.colo,
                    );
                    log(
                        &app,
                        &state,
                        &scan_id,
                        if within_limit { "success" } else { "info" },
                        &format!(
                            "{}:{} → {} {} · TCP {:.1} ms · HTTPS 请求 {:.0} ms · 抖动 {:.1} ms · 丢包 {:.0}%{}",
                            candidate.ip,
                            candidate.port,
                            trace.colo,
                            node_display_name(key, &trace.colo),
                            latency.avg_ms,
                            trace.elapsed_ms,
                            latency.jitter_ms,
                            latency.loss_pct,
                            if within_limit { String::new() } else { format!(" · 超出延迟上限 {:.0} ms，仅展示不导出", req.latency_limit_ms) }
                        ),
                    );
                } else {
                    if let Ok(mut p) = progress.lock() {
                        p.latency_tested += 1;
                    }
                    save_candidate_status(&state, &candidate, "latency_failed", None, None, None, None, &trace.colo);
                    log(
                        &app,
                        &state,
                        &scan_id,
                        "warn",
                        &format!(
                            "{}:{} → {} 延迟测试失败：{}/{} 轮成功{}",
                            candidate.ip,
                            candidate.port,
                            trace.colo,
                            latency.success,
                            latency.attempts,
                            if latency.last_error.is_empty() { String::new() } else { format!(" · {}", latency.last_error) }
                        ),
                    );
                }
            } else {
                save_candidate_status(&state, &candidate, "colo_unmatched", Some(latency.avg_ms), None, None, None, &trace.colo);
                log(
                    &app,
                    &state,
                    &scan_id,
                    "warn",
                    &format!("{}:{} colo {} 无法识别国家/地区，已跳过", candidate.ip, candidate.port, trace.colo),
                );
            }
        } else {
            save_candidate_status(&state, &candidate, "https_failed", None, None, None, None, "");
            log(
                &app,
                &state,
                &scan_id,
                "warn",
                &format!("{}:{} HTTPS/trace 失败：{}", candidate.ip, candidate.port, trace.error),
            );
        }
        if done % 20 == 0 || done == total_https {
            let (h, c, lt, lp, le) = progress
                .lock()
                .map(|p| (p.https_ok, p.colo_ok, p.latency_tested, p.latency_passed, p.latency_eliminated))
                .unwrap_or((0, 0, 0, 0, 0));
            if let Ok(mut p) = progress.lock() {
                p.tested = done;
                p.https_tested = done;
                p.https_passed = h;
                p.message = format!(
                    "HTTPS/colo/延迟：{done}/{total_https}，HTTPS {h}，colo {c}，延迟通过 {lp}/{lt}（超限 {le}）"
                );
            }
            emit_progress(&app, &progress, &state);
            if done % 100 == 0 {
                log(
                    &app,
                    &state,
                    &scan_id,
                    "info",
                    &format!(
                        "阶段进度 HTTPS/colo：{done}/{total_https} · HTTPS 成功 {h} · colo 命中 {c} · 延迟通过 {lp} · 超出上限 {le} · 剩余 {}",
                        total_https.saturating_sub(done)
                    ),
                );
            }
        }
    }
    if is_cancelled(&cancel) {
        finish_cancelled(&app, &state, &progress, &scan_id, candidates.len());
        return Ok(());
    }
    if wait_while_paused(&app, &state, &progress, &cancel, &scan_id).await {
        finish_cancelled(&app, &state, &progress, &scan_id, candidates.len());
        return Ok(());
    }
    // 延迟上限不再用于此阶段淘汰：所有拿到 colo 且延迟测试成功的候选都进入下载测速
    let effective_latency_limit = req.latency_limit_ms;
    let below_now = verified_all.iter().filter(|v| v.below_limit).count();
    let colo_ok_now = progress.lock().map(|p| p.colo_ok).unwrap_or(0);
    log(
        &app,
        &state,
        &scan_id,
        if verified_all.is_empty() { "warn" } else { "success" },
        &format!(
            "HTTPS / colo / 延迟完成：{} 个候选进入下载测速（colo 命中 {} 个，其中 {} 个超出延迟上限 {:.0} ms，仅展示不导出）",
            verified_all.len(),
            colo_ok_now,
            below_now,
            effective_latency_limit
        ),
    );
    if verified_all.is_empty() {
        log(
            &app,
            &state,
            &scan_id,
            "warn",
            if colo_ok_now == 0 {
                "没有任何候选通过 Cloudflare HTTPS/colo 识别：请确认网络未被代理或 VPN 劫持，并优先使用 443 端口候选。"
            } else {
                "有 IP 识别到 colo，但延迟测试全部失败或对应地区未勾选：请检查目标地区选择与网络代理设置。"
            },
        );
    }

    // Phase 3: actual download tests for a surplus shortlist in every region
    // 注意：不要在这里把 p.tested 归零，否则任务数为 0 或下载很慢时界面会一直停在 0% / 上一阶段总数。
    if let Ok(mut p) = progress.lock() {
        p.phase = "download".to_string();
        p.message = format!("正在执行真实下载测速（{} 个候选）", verified_all.len());
        p.tested = p.total;
    }
    emit_progress(&app, &progress, &state);
    let mut by_region: HashMap<String, Vec<usize>> = HashMap::new();
    for (idx, item) in verified_all.iter().enumerate() {
        by_region.entry(item.region.clone()).or_default().push(idx);
    }
    let mut download_jobs: Vec<usize> = Vec::new();
    for key in &regions {
        if let Some(indices) = by_region.get_mut(key) {
            indices.sort_by(|a, b| verified_all[*a].latency.avg_ms.partial_cmp(&verified_all[*b].latency.avg_ms).unwrap_or(std::cmp::Ordering::Equal));
            // 只对延迟最靠前的少量余量做下载复测：既能补齐配额，又不会把
            // 几百个明显没有希望的低速节点全部跑满 10 秒，整体耗时显著下降。
            let top = req.per_region_top.max(1);
            let extra = if req.smart_supplement {
                ((top + 1) / 2).max(8)
            } else {
                4
            };
            let slots = (top + extra).max(10);
            for idx in indices.iter().take(slots) {
                download_jobs.push(*idx);
            }
        }
    }
    // 全球其他地区不属于用户首选配额：按延迟优先安排一批候选完成真实下载，
    // 只要最终达标就全部保留（不再受 per_region_top 限制）。
    let mut global_indices: Vec<usize> = verified_all
        .iter()
        .enumerate()
        .filter(|(_, item)| item.group == "global")
        .map(|(idx, _)| idx)
        .collect();
    global_indices.sort_by(|a, b| {
        verified_all[*a]
            .latency
            .avg_ms
            .partial_cmp(&verified_all[*b].latency.avg_ms)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let global_slots = (req.per_region_top.max(1) * 2 + 20).clamp(30, 120);
    download_jobs.extend(global_indices.into_iter().take(global_slots));
    let mut unknown_indices: Vec<usize> = verified_all
        .iter()
        .enumerate()
        .filter(|(_, item)| item.group == "unknown")
        .map(|(idx, _)| idx)
        .collect();
    unknown_indices.sort_by(|a, b| {
        verified_all[*a]
            .latency
            .avg_ms
            .partial_cmp(&verified_all[*b].latency.avg_ms)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    download_jobs.extend(unknown_indices.into_iter().take(20));
    let total_downloads = download_jobs.len();
    // 下载测速会共享本机出口带宽：1 线程最接近客户端单连接速度，
    // 多线程只用于快速筛选，数值会因互抢带宽而偏低。
    let download_conc = req.download_threads.clamp(1, 32);
    log(
        &app,
        &state,
        &scan_id,
        "info",
        &format!(
            "下载测速线程 {}；{}",
            download_conc,
            if download_conc == 1 {
                "单线程精准模式，结果最接近 v2rayN / CFData 的单连接速度"
            } else {
                "多线程加速模式，单 IP 速度会受并发分摊影响"
            }
        ),
    );
    let primary_url = if let Some(url) = req.download_url.clone() {
        url
    } else if settings.speed_source == crate::speed_source::SOURCE_AUTO {
        let (_key, url, _probes) = crate::speed_source::resolve_auto(&settings.custom_speed_url).await;
        url
    } else {
        crate::speed_source::url_for(&settings.speed_source, &settings.custom_speed_url)
            .unwrap_or_else(|| settings.download_url.clone())
    };
    let backup_url = if settings.auto_switch_speed_source {
        match crate::speed_source::resolve_backup(
            &settings.speed_source,
            &settings.backup_speed_source,
            &settings.custom_speed_url,
        )
        .await
        {
            Some((_key, url)) => url,
            None => settings.backup_download_url.clone(),
        }
    } else {
        settings.backup_download_url.clone()
    };
    let backup_url = if backup_url.trim().is_empty() || backup_url == primary_url {
        settings.backup_download_url.clone()
    } else {
        backup_url
    };
    let duration = req.download_seconds.clamp(2, 60);
    let min_bytes = req.min_download_bytes.max(64 * 1024);
    let download_snapshot = verified_all.clone();
    let mut download_stream = stream::iter(download_jobs.into_iter().map(|idx| {
        let item = download_snapshot[idx].clone();
        let primary = primary_url.clone();
        let backup = backup_url.clone();
        async move {
            let ip = match speedtest::parse_ip(&item.candidate.ip) {
                Ok(ip) => ip,
                Err(e) => {
                    return (idx, item, speedtest::DownloadProbe { error: e, ..Default::default() }, String::new());
                }
            };
            let first = speedtest::download_probe(ip, item.candidate.port, &primary, duration, min_bytes).await;
            if first.ok || backup.trim().is_empty() || backup == primary {
                (idx, item, first, String::new())
            } else {
                let second = speedtest::download_probe(ip, item.candidate.port, &backup, duration, min_bytes).await;
                (idx, item, second, "备用地址".to_string())
            }
        }
    }))
    .buffer_unordered(download_conc);
    done = 0;
    let mut download_ok = 0usize;
    let mut speed_ok = 0usize;
    while let Some((idx, item, probe, via)) = download_stream.next().await {
        wait_while_paused(&app, &state, &progress, &cancel, &scan_id).await;
        if is_cancelled(&cancel) {
            break;
        }
        done += 1;
        let status;
        if probe.ok && probe.mb_s >= req.speed_limit_mb_s {
            speed_ok += 1;
            download_ok += 1;
            status = "download_ok";
            verified_all[idx].download = Some(probe.clone());
            if let Ok(mut p) = progress.lock() {
                p.download_ok += 1;
                p.speed_ok += 1;
                p.download_tested = done;
                p.download_passed = p.download_ok;
                if let Some(stat) = p.region_stats.iter_mut().find(|s| s.key == item.region) {
                    stat.download_ok += 1;
                    stat.speed_ok += 1;
                }
            }
            save_candidate_status(
                &state,
                &item.candidate,
                status,
                Some(item.latency.avg_ms),
                Some(probe.mb_s),
                Some(item.latency.loss_pct),
                None,
                &item.trace.colo,
            );
            log(
                &app,
                &state,
                &scan_id,
                "success",
                &format!(
                    "{}:{} 下载完成 · {:.2} MB/s ({:.1} Mbps) · {:.1} MB / {:.1}s{}",
                    item.candidate.ip,
                    item.candidate.port,
                    probe.mb_s,
                    probe.mbps,
                    probe.bytes as f64 / 1_000_000.0,
                    probe.seconds,
                    if via.is_empty() { String::new() } else { format!(" · {via}") }
                ),
            );
        } else {
            if probe.ok {
                download_ok += 1;
                status = "speed_below";
            } else {
                status = "download_failed";
            }
            save_candidate_status(
                &state,
                &item.candidate,
                status,
                Some(item.latency.avg_ms),
                Some(probe.mb_s),
                Some(item.latency.loss_pct),
                None,
                &item.trace.colo,
            );
            log(
                &app,
                &state,
                &scan_id,
                "warn",
                &format!(
                    "{}:{} 下载未通过：{} ({:.2} MB/s)",
                    item.candidate.ip,
                    item.candidate.port,
                    if probe.error.is_empty() { "速度低于阈值" } else { &probe.error },
                    probe.mb_s
                ),
            );
        }
        if let Ok(mut p) = progress.lock() {
            p.tested = done;
            p.message = format!("真实下载测速：{done}/{total_downloads}，通过 {speed_ok}");
        }
        if done % 2 == 0 || done == total_downloads {
            emit_progress(&app, &progress, &state);
        }
    }
    if is_cancelled(&cancel) {
        finish_cancelled(&app, &state, &progress, &scan_id, candidates.len());
        return Ok(());
    }
    if wait_while_paused(&app, &state, &progress, &cancel, &scan_id).await {
        finish_cancelled(&app, &state, &progress, &scan_id, candidates.len());
        return Ok(());
    }
    // 下载阶段没有任何候选可测，属于流程前置条件不满足，直接给出可行动的失败原因
    if total_downloads == 0 {
        let colo = progress.lock().map(|p| p.colo_ok).unwrap_or(0);
        return Err(if colo == 0 {
            "没有任何候选通过 HTTPS Cloudflare 握手并识别到 colo。请确认网络未被代理/VPN 劫持，并优先使用 443 端口的候选 IP。".to_string()
        } else {
            format!("有 {colo} 个候选识别到 colo，但没有候选完成多轮延迟测试或对应地区未勾选，因此无可下载测速对象。请勾选目标地区后重试。")
        });
    }
    // 有候选可测但全部未达标：不中断扫描，继续完成稳定性与统计，最终以 0 结果结束并保留全过程证据
    if download_ok == 0 {
        log(
            &app,
            &state,
            &scan_id,
            "warn",
            &format!(
                "{} 个候选进入下载测速，但全部未达标（最低速度 {:.2} MB/s、最低数据量 {} KB、单次时长 {}s）。本轮不产生正式优选结果，过程统计与失败原因已保留。",
                total_downloads,
                req.speed_limit_mb_s,
                min_bytes / 1024,
                duration
            ),
        );
    }

    // Phase 4: optional stability re-test. Disabled by default because a single
    // failed stability sample must never hide otherwise usable IPs.
    if req.stability_rounds > 0 {
        if wait_while_paused(&app, &state, &progress, &cancel, &scan_id).await {
            finish_cancelled(&app, &state, &progress, &scan_id, candidates.len());
            return Ok(());
        }
        if let Ok(mut p) = progress.lock() {
            p.phase = "stability".to_string();
            p.message = "正在复测稳定性".to_string();
            p.tested = p.total;
        }
        emit_progress(&app, &progress, &state);
        let mut stability_jobs: Vec<usize> = Vec::new();
        by_region.clear();
        for (idx, item) in verified_all.iter().enumerate() {
            if item.download.as_ref().map(|d| d.ok).unwrap_or(false) {
                by_region.entry(item.region.clone()).or_default().push(idx);
            }
        }
        for key in &regions {
            if let Some(indices) = by_region.get_mut(key) {
                indices.sort_by(|a, b| {
                    let da = verified_all[*a].download.as_ref().map(|d| d.mb_s).unwrap_or(0.0);
                    let db = verified_all[*b].download.as_ref().map(|d| d.mb_s).unwrap_or(0.0);
                    db.partial_cmp(&da).unwrap_or(std::cmp::Ordering::Equal)
                });
                let take = (req.per_region_top.max(1) + 4).min(indices.len());
                stability_jobs.extend(indices.iter().take(take).cloned());
            }
        }
        let total_stability = stability_jobs.len();
        let stability_conc = (req.concurrency / 10).clamp(2, 6);
        let extra_rounds = req.stability_rounds.min(4);
        let short_duration = (duration / 3).clamp(3, 6);
        let short_min = (min_bytes / 2).max(256 * 1024);
        let stability_snapshot = verified_all.clone();
        let mut stability_stream = stream::iter(stability_jobs.into_iter().map(|idx| {
            let item = stability_snapshot[idx].clone();
            let primary = primary_url.clone();
            let timeout = settings.tcp_timeout_ms;
            async move {
                let ip = match speedtest::parse_ip(&item.candidate.ip) {
                    Ok(ip) => ip,
                    Err(_) => return (idx, 0.0),
                };
                let mut success = 0usize;
                let mut attempts = 0usize;
                if extra_rounds == 0 {
                    return (idx, 100.0);
                }
                let lat = speedtest::tcp_latency_probe(ip, item.candidate.port, 2, timeout).await;
                attempts += 1;
                if lat.ok {
                    success += 1;
                }
                for _ in 0..extra_rounds {
                    let probe = speedtest::download_probe(ip, item.candidate.port, &primary, short_duration, short_min).await;
                    attempts += 1;
                    if probe.ok {
                        success += 1;
                    }
                }
                (idx, success as f64 / attempts.max(1) as f64 * 100.0)
            }
        }))
        .buffer_unordered(stability_conc);
        done = 0;
        while let Some((idx, stability)) = stability_stream.next().await {
            wait_while_paused(&app, &state, &progress, &cancel, &scan_id).await;
            if is_cancelled(&cancel) {
                break;
            }
            done += 1;
            verified_all[idx].stability = stability;
            if let Ok(mut p) = progress.lock() {
                p.tested = p.total;
                p.message = format!("稳定性复测：{done}/{total_stability}");
            }
            if done % 5 == 0 || done == total_stability {
                emit_progress(&app, &progress, &state);
            }
        }
        if is_cancelled(&cancel) {
            finish_cancelled(&app, &state, &progress, &scan_id, candidates.len());
            return Ok(());
        }
    }

    // Rank, persist, and report
    if wait_while_paused(&app, &state, &progress, &cancel, &scan_id).await {
        finish_cancelled(&app, &state, &progress, &scan_id, candidates.len());
        return Ok(());
    }
    for item in &mut verified_all {
        item.score = score_for(item, req.speed_limit_mb_s.max(0.1));
    }
    // 两种状态都保留：passed（全部达标，可正式导出）与 latency_below / speed_below（仅展示，不导出）
    let mut final_results: Vec<ScanResult> = Vec::new();
    let mut passed_count = 0usize;
    let mut below_count = 0usize;
    let mut result_regions = regions.clone();
    for item in &verified_all {
        if (item.group == "global" || item.group == "unknown")
            && !result_regions.iter().any(|key| key == &item.region)
        {
            result_regions.push(item.region.clone());
        }
    }
    for key in &result_regions {
        let mut group: Vec<&Verified> = verified_all
            .iter()
            .filter(|v| {
                v.region == *key
                    && v.download.as_ref().map(|d| d.ok).unwrap_or(false)
            })
            .collect();
        group.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        // 首选地区按 per_region_top 保留；全球其他地区和未知地区只要达标全部保留。
        let preferred = REGION_ORDER.contains(&key.as_str());
        let quota = if preferred { req.per_region_top.max(1) } else { usize::MAX };
        let mut taken = 0usize;
        for item in group.iter() {
            let d = item.download.as_ref().unwrap();
            let speed_ok = d.mb_s >= req.speed_limit_mb_s;
            let compliant = !item.below_limit && speed_ok;
            if compliant && taken >= quota {
                continue;
            }
            let status = if compliant {
                taken += 1;
                passed_count += 1;
                "passed"
            } else if item.below_limit {
                below_count += 1;
                "latency_below"
            } else {
                below_count += 1;
                "speed_below"
            };
            final_results.push(ScanResult {
                id: 0,
                scan_id: scan_id.clone(),
                rank: if compliant { taken } else { 0 },
                ip: item.candidate.ip.clone(),
                port: item.candidate.port,
                region: key.clone(),
                region_name: node_display_name(key, &item.trace.colo),
                colo: item.trace.colo.clone(),
                latency: item.latency.avg_ms,
                jitter: item.latency.jitter_ms,
                speed_mb_s: d.mb_s,
                speed_mbps: d.mbps,
                loss: item.latency.loss_pct,
                stability: item.stability,
                source: item.candidate.sources.join("/"),
                status: status.to_string(),
                tested_at: db::now_text(),
            });
            save_candidate_status(
                &state,
                &item.candidate,
                status,
                Some(item.latency.avg_ms),
                Some(d.mb_s),
                Some(item.latency.loss_pct),
                Some(item.stability),
                &item.trace.colo,
            );
        }
    }
    let avg_latency = if final_results.is_empty() { 0.0 } else { final_results.iter().map(|r| r.latency).sum::<f64>() / final_results.len() as f64 };
    let avg_speed = if final_results.is_empty() { 0.0 } else { final_results.iter().map(|r| r.speed_mb_s).sum::<f64>() / final_results.len() as f64 };
    {
        let conn = state.db.lock().map_err(|e| format!("保存结果失败: {e}"))?;
        for r in &final_results {
            db::insert_result(&conn, r)?;
        }
        db::finish_scan(&conn, &scan_id, "completed", candidates.len(), passed_count, avg_latency, avg_speed)?;
    }
    {
        let mut p = progress.lock().map_err(|e| e.to_string())?;
        p.status = "completed".to_string();
        p.phase = "completed".to_string();
        p.message = if passed_count > 0 {
            format!(
                "优选完成：{} 个 IP 通过全部测试，另有 {} 个已完成测速但未达延迟/速度标准（仅展示，不导出）",
                passed_count, below_count
            )
        } else if below_count > 0 {
            format!(
                "本轮没有 IP 达到全部标准；{} 个 IP 已完成测速但未达标（仅展示，不导出）。可放宽延迟上限或降低最低速度后重试。",
                below_count
            )
        } else {
            "本轮没有 IP 完成下载测速，未产生优选结果".to_string()
        };
        p.download_tested = total_downloads;
        p.download_passed = download_ok;
        p.final_count = passed_count;
        p.final_selected = passed_count;
        p.tested = p.total;
        p.finished_at = db::now_text();
        for stat in &mut p.region_stats {
            let group: Vec<&ScanResult> = final_results
                .iter()
                .filter(|r| {
                    (if stat.key == "global" {
                        !REGION_ORDER.contains(&r.region.as_str()) && r.region != "unknown"
                    } else {
                        r.region == stat.key
                    }) && r.status == "passed"
                })
                .collect();
            stat.final_count = group.len();
            stat.avg_latency = if group.is_empty() { 0.0 } else { group.iter().map(|r| r.latency).sum::<f64>() / group.len() as f64 };
            stat.avg_speed = if group.is_empty() { 0.0 } else { group.iter().map(|r| r.speed_mb_s).sum::<f64>() / group.len() as f64 };
        }
    }
    emit_progress(&app, &progress, &state);
    for key in &regions {
        let passed = final_results.iter().filter(|r| r.region == *key && r.status == "passed").count();
        let below = final_results.iter().filter(|r| r.region == *key && r.status != "passed").count();
        log(
            &app,
            &state,
            &scan_id,
            if passed > 0 { "success" } else { "warn" },
            &format!(
                "{}：正式保留 {} 个，另有 {} 个已完成测速但未达标{}",
                region_name(key),
                passed,
                below,
                if passed == 0 && below == 0 { "（未发现该地区实际 colo）" } else { "" }
            ),
        );
    }
    let global_passed = final_results
        .iter()
        .filter(|r| r.status == "passed" && !REGION_ORDER.contains(&r.region.as_str()) && r.region != "unknown")
        .count();
    let global_below = final_results
        .iter()
        .filter(|r| r.status != "passed" && !REGION_ORDER.contains(&r.region.as_str()) && r.region != "unknown")
        .count();
    let unknown_passed = final_results
        .iter()
        .filter(|r| r.status == "passed" && r.region == "unknown")
        .count();
    log(
        &app,
        &state,
        &scan_id,
        if global_passed > 0 { "success" } else { "info" },
        &format!(
            "全球其他地区：正式保留 {} 个，另有 {} 个已完成测速但未达标（所有达标 IP 均保留）",
            global_passed, global_below
        ),
    );
    if unknown_passed > 0 {
        log(
            &app,
            &state,
            &scan_id,
            "success",
            &format!("未知地区：正式保留 {} 个（已保留并标记为未知，不猜测地区）", unknown_passed),
        );
    }
    log(
        &app,
        &state,
        &scan_id,
        "success",
        &format!(
            "扫描完成：正式结果 {} 个，参考结果 {} 个，结果已写入本地数据库",
            passed_count, below_count
        ),
    );
    let _ = app.emit("scan-finished", serde_json::json!({"scanId": scan_id, "status": "completed", "count": passed_count, "referenceCount": below_count}));
    Ok(())
}

fn finish_cancelled(app: &AppHandle, state: &AppState, progress: &Arc<Mutex<ScanProgress>>, scan_id: &str, tested: usize) {
    if let Ok(conn) = state.db.lock() {
        let _ = db::finish_scan(&conn, scan_id, "cancelled", tested, 0, 0.0, 0.0);
    }
    if let Ok(mut p) = progress.lock() {
        p.status = "cancelled".to_string();
        p.phase = "cancelled".to_string();
        p.message = "扫描已取消".to_string();
        p.finished_at = db::now_text();
    }
    emit_progress(app, progress, state);
    log(app, state, scan_id, "warn", "扫描已由用户取消，未产生新的优选结果");
    let _ = app.emit("scan-finished", serde_json::json!({"scanId": scan_id, "status": "cancelled", "count": 0}));
}
