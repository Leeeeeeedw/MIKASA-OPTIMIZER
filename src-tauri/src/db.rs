use crate::models::*;
use chrono::Local;
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

pub fn now_text() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

pub fn open_database(path: &Path) -> Result<Connection, String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建数据目录失败: {e}"))?;
    }
    let conn = Connection::open(path).map_err(|e| format!("打开数据库失败: {e}"))?;
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA synchronous=NORMAL;
         PRAGMA foreign_keys=ON;
         PRAGMA busy_timeout=5000;",
    )
    .map_err(|e| format!("初始化数据库连接失败: {e}"))?;
    init_schema(&conn)?;
    Ok(conn)
}

fn init_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS candidates (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            ip TEXT NOT NULL,
            port INTEGER NOT NULL,
            sources TEXT NOT NULL DEFAULT '[]',
            initial_region TEXT NOT NULL DEFAULT '',
            first_seen TEXT NOT NULL,
            last_seen TEXT NOT NULL,
            last_colo TEXT NOT NULL DEFAULT '',
            last_latency REAL,
            last_download_mb_s REAL,
            last_loss REAL,
            last_stability REAL,
            last_status TEXT NOT NULL DEFAULT 'untested',
            last_tested TEXT NOT NULL DEFAULT '',
            UNIQUE(ip, port)
        );
        CREATE INDEX IF NOT EXISTS idx_candidates_region ON candidates(initial_region);
        CREATE INDEX IF NOT EXISTS idx_candidates_tested ON candidates(last_tested);
        CREATE TABLE IF NOT EXISTS scans (
            scan_id TEXT PRIMARY KEY,
            started_at TEXT NOT NULL,
            finished_at TEXT NOT NULL DEFAULT '',
            status TEXT NOT NULL,
            params_json TEXT NOT NULL DEFAULT '{}',
            tested INTEGER NOT NULL DEFAULT 0,
            final_count INTEGER NOT NULL DEFAULT 0,
            avg_latency REAL NOT NULL DEFAULT 0,
            avg_speed REAL NOT NULL DEFAULT 0,
            regions_json TEXT NOT NULL DEFAULT '[]'
        );
        CREATE TABLE IF NOT EXISTS results (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            scan_id TEXT NOT NULL,
            rank INTEGER NOT NULL DEFAULT 0,
            ip TEXT NOT NULL,
            port INTEGER NOT NULL,
            region TEXT NOT NULL,
            colo TEXT NOT NULL,
            latency REAL NOT NULL,
            jitter REAL NOT NULL DEFAULT 0,
            speed_mb_s REAL NOT NULL,
            speed_mbps REAL NOT NULL,
            loss REAL NOT NULL DEFAULT 0,
            stability REAL NOT NULL DEFAULT 100,
            source TEXT NOT NULL DEFAULT '',
            status TEXT NOT NULL DEFAULT 'passed',
            tested_at TEXT NOT NULL,
            FOREIGN KEY(scan_id) REFERENCES scans(scan_id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_results_scan ON results(scan_id);
        CREATE INDEX IF NOT EXISTS idx_results_region ON results(region);
        CREATE TABLE IF NOT EXISTS url_sources (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            url TEXT NOT NULL UNIQUE,
            enabled INTEGER NOT NULL DEFAULT 1,
            interval_minutes INTEGER NOT NULL DEFAULT 60,
            last_fetch TEXT NOT NULL DEFAULT '',
            last_count INTEGER NOT NULL DEFAULT 0,
            last_status TEXT NOT NULL DEFAULT '未更新'
        );
        CREATE TABLE IF NOT EXISTS cm_records (
            ip TEXT NOT NULL,
            port INTEGER NOT NULL,
            meta_json TEXT NOT NULL DEFAULT '',
            source_format TEXT NOT NULL DEFAULT '',
            data_updated_at TEXT NOT NULL DEFAULT '',
            fetched_at TEXT NOT NULL DEFAULT '',
            PRIMARY KEY(ip, port)
        );
        CREATE INDEX IF NOT EXISTS idx_cm_records_country ON cm_records(source_format);
        CREATE TABLE IF NOT EXISTS source_stats (
            source TEXT PRIMARY KEY,
            last_fetched INTEGER NOT NULL DEFAULT 0,
            last_added INTEGER NOT NULL DEFAULT 0,
            last_duplicates INTEGER NOT NULL DEFAULT 0,
            last_invalid INTEGER NOT NULL DEFAULT 0,
            last_update TEXT NOT NULL DEFAULT '',
            status TEXT NOT NULL DEFAULT '待获取',
            message TEXT NOT NULL DEFAULT ''
        );
        CREATE TABLE IF NOT EXISTS github_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            repo TEXT NOT NULL,
            branch TEXT NOT NULL,
            files TEXT NOT NULL,
            status TEXT NOT NULL,
            message TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL
        );
        "#,
    )
    .map_err(|e| format!("创建数据表失败: {e}"))?;
    // 兼容旧数据库：候选表现有的 ip_type/ip_version/成功失败计数通过幂等迁移补齐。
    for (name, ddl) in [
        ("ip_type", "TEXT NOT NULL DEFAULT 'community'"),
        ("ip_version", "INTEGER NOT NULL DEFAULT 4"),
        ("success_count", "INTEGER NOT NULL DEFAULT 0"),
        ("fail_count", "INTEGER NOT NULL DEFAULT 0"),
    ] {
        ensure_column(conn, "candidates", name, ddl)?;
    }
    // 旧数据没有版本信息，按字符串形式回填一次；IPv6 地址一定包含冒号。
    let _ = conn.execute(
        "UPDATE candidates SET ip_version=6 WHERE instr(ip, ':') > 0 AND ip_version=4",
        [],
    );
    let _ = conn.execute(
        "UPDATE candidates SET ip_type='official' WHERE ip_type='community' AND sources LIKE '%\"cloudflare\"%'",
        [],
    );
    Ok(())
}

fn ensure_column(conn: &Connection, table: &str, name: &str, ddl: &str) -> Result<(), String> {
    let sql = format!("PRAGMA table_info({table})");
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let exists = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .any(|column| column == name);
    drop(stmt);
    if !exists {
        conn.execute(
            &format!("ALTER TABLE {table} ADD COLUMN {name} {ddl}"),
            [],
        )
        .map_err(|e| format!("迁移字段 {table}.{name} 失败: {e}"))?;
    }
    Ok(())
}

pub fn get_meta(conn: &Connection, key: &str) -> Option<String> {
    conn.query_row("SELECT value FROM meta WHERE key = ?1", params![key], |r| r.get(0))
        .optional()
        .ok()
        .flatten()
}

pub fn set_meta(conn: &Connection, key: &str, value: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO meta(key,value) VALUES(?1,?2)
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        params![key, value],
    )
    .map_err(|e| format!("写入设置失败: {e}"))?;
    Ok(())
}

pub fn load_settings(conn: &Connection) -> AppSettings {
    let mut settings = get_meta(conn, "app_settings")
        .and_then(|s| serde_json::from_str::<AppSettings>(&s).ok())
        .unwrap_or_default();
    // 迁移：旧版本默认延迟上限 300ms 会在真实大陆网络下淘汰全部候选（实测 520-941ms），
    // 导致下载测速阶段没有任何 IP 可测。这里一次性把仍是旧默认值的用户提升到 1500ms。
    if get_meta(conn, "migrate.latency_default_v2").is_none() {
        if settings.latency_limit_ms > 0.0 && settings.latency_limit_ms <= 300.0 {
            settings.latency_limit_ms = 1500.0;
            let _ = save_settings(conn, &settings);
        }
        let _ = set_meta(conn, "migrate.latency_default_v2", "1");
    }
    // 迁移：稳定性复测在新版本中默认关闭。旧版 2 轮复测会把可用 IP 的
    // 稳定性评分拉低到 50 以下，导致正式结果为空。保留字段供旧数据库兼容，
    // 但一次性把旧默认值归零。
    if get_meta(conn, "migrate.stability_disabled_v1").is_none() {
        if settings.stability_rounds > 0 {
            settings.stability_rounds = 0;
            let _ = save_settings(conn, &settings);
        }
        let _ = set_meta(conn, "migrate.stability_disabled_v1", "1");
    }
    settings
}

pub fn save_settings(conn: &Connection, settings: &AppSettings) -> Result<(), String> {
    let json = serde_json::to_string(settings).map_err(|e| e.to_string())?;
    set_meta(conn, "app_settings", &json)
}

#[derive(Debug, Clone)]
pub struct CandidateSeed {
    pub ip: String,
    pub port: u16,
    pub source: String,
    pub region: String,
}

fn classify_ip_type(sources_json: &str) -> &'static str {
    let sources: Vec<String> = serde_json::from_str(sources_json).unwrap_or_default();
    if sources.iter().any(|s| s == "cloudflare") {
        "official"
    } else if sources.iter().any(|s| s == "url" || s == "github") {
        "public"
    } else {
        "community"
    }
}

pub fn insert_candidates(conn: &mut Connection, seeds: Vec<CandidateSeed>) -> Result<ImportResult, String> {
    let now = now_text();
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let mut added = 0usize;
    let mut updated = 0usize;
    let mut unique: HashMap<String, CandidateSeed> = HashMap::new();
    for seed in seeds {
        let key = format!("{}:{}", seed.ip, seed.port);
        unique
            .entry(key)
            .and_modify(|e| {
                if !e.source.split('|').any(|v| v == seed.source) {
                    e.source = format!("{}|{}", e.source, seed.source);
                }
            })
            .or_insert(seed);
    }
    let mut preview = Vec::new();
    let mut source_counts: BTreeMap<String, usize> = BTreeMap::new();
    for seed in unique.into_values() {
        let existing: Option<(i64, String, String)> = tx
            .query_row(
                "SELECT id, sources, initial_region FROM candidates WHERE ip=?1 AND port=?2",
                params![seed.ip, seed.port],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()
            .map_err(|e| e.to_string())?;
        let sources: Vec<String> = seed.source.split('|').filter(|s| !s.is_empty()).map(|s| s.to_string()).collect();
        for s in &sources {
            *source_counts.entry(s.clone()).or_insert(0) += 1;
        }
        if let Some((id, old_sources, old_region)) = existing {
            let mut merged: Vec<String> = serde_json::from_str(&old_sources).unwrap_or_default();
            for s in sources {
                if !merged.contains(&s) {
                    merged.push(s);
                }
            }
            let region = if old_region.is_empty() { seed.region.clone() } else { old_region };
            let merged_text = serde_json::to_string(&merged).unwrap_or_else(|_| "[]".into());
            let ip_type = classify_ip_type(&merged_text);
            tx.execute(
                "UPDATE candidates SET sources=?1, initial_region=?2, last_seen=?3, ip_type=?4 WHERE id=?5",
                params![merged_text, region, now, ip_type, id],
            )
            .map_err(|e| e.to_string())?;
            updated += 1;
        } else {
            let sources_text = serde_json::to_string(&sources).unwrap_or_else(|_| "[]".into());
            let ip_type = classify_ip_type(&sources_text);
            let ip_version = if seed.ip.contains(':') { 6i64 } else { 4i64 };
            tx.execute(
                "INSERT INTO candidates(ip,port,sources,initial_region,first_seen,last_seen,last_status,ip_type,ip_version)
                 VALUES(?1,?2,?3,?4,?5,?5,'untested',?6,?7)",
                params![
                    seed.ip,
                    seed.port as i64,
                    sources_text,
                    seed.region,
                    now,
                    ip_type,
                    ip_version
                ],
            )
            .map_err(|e| e.to_string())?;
            added += 1;
        }
        if preview.len() < 60 {
            preview.push(CandidatePreview {
                ip: seed.ip,
                port: seed.port,
                region: seed.region,
                source: seed.source,
            });
        }
    }
    tx.commit().map_err(|e| e.to_string())?;
    let total = conn
        .query_row("SELECT COUNT(*) FROM candidates", [], |r| r.get::<_, i64>(0))
        .unwrap_or(0) as usize;
    Ok(ImportResult {
        added,
        updated,
        total,
        preview,
        source_counts,
    })
}

fn row_to_candidate(row: &rusqlite::Row<'_>) -> rusqlite::Result<Candidate> {
    let sources_text: String = row.get(3)?;
    Ok(Candidate {
        id: row.get(0)?,
        ip: row.get(1)?,
        port: row.get::<_, i64>(2)? as u16,
        sources: serde_json::from_str(&sources_text).unwrap_or_default(),
        initial_region: row.get(4)?,
        first_seen: row.get(5)?,
        last_seen: row.get(6)?,
        last_colo: row.get(7)?,
        last_latency: row.get(8)?,
        last_download_mb_s: row.get(9)?,
        last_loss: row.get(10)?,
        last_stability: row.get(11)?,
        last_status: row.get(12)?,
        last_tested: row.get(13)?,
        ip_type: row.get(14)?,
        ip_version: row.get::<_, i64>(15)? as u8,
        success_count: row.get::<_, i64>(16)? as usize,
        fail_count: row.get::<_, i64>(17)? as usize,
    })
}

pub fn list_candidates(
    conn: &Connection,
    limit: usize,
    offset: usize,
    search: &str,
    region: &str,
    source: &str,
) -> Result<(Vec<Candidate>, usize), String> {
    let mut clauses = Vec::new();
    let mut values: Vec<String> = Vec::new();
    if !search.is_empty() {
        clauses.push(format!("ip LIKE ?{}", values.len() + 1));
        values.push(format!("%{}%", search));
    }
    if !region.is_empty() && region != "all" {
        clauses.push(format!("initial_region = ?{}", values.len() + 1));
        values.push(region.to_string());
    }
    if !source.is_empty() && source != "all" {
        clauses.push(format!("sources LIKE ?{}", values.len() + 1));
        values.push(format!("%\"{}\"%", source));
    }
    let where_clause = if clauses.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", clauses.join(" AND "))
    };
    let total: i64 = conn
        .query_row(
            &format!("SELECT COUNT(*) FROM candidates{}", where_clause),
            rusqlite::params_from_iter(values.iter()),
            |r| r.get(0),
        )
        .unwrap_or(0);
    let sql = format!(
        "SELECT id,ip,port,sources,initial_region,first_seen,last_seen,last_colo,last_latency,
         last_download_mb_s,last_loss,last_stability,last_status,last_tested,ip_type,ip_version,success_count,fail_count
         FROM candidates{} ORDER BY CASE WHEN last_download_mb_s IS NULL THEN 0 ELSE 1 END DESC,
         last_download_mb_s DESC, last_latency ASC, id DESC LIMIT {} OFFSET {}",
        where_clause, limit, offset
    );
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(rusqlite::params_from_iter(values.iter()), |r| row_to_candidate(r))
        .map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for row in rows {
        if let Ok(item) = row {
            out.push(item);
        }
    }
    Ok((out, total as usize))
}

pub fn get_candidates_for_scan(
    conn: &Connection,
    request: &ScanRequest,
) -> Result<Vec<Candidate>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id,ip,port,sources,initial_region,first_seen,last_seen,last_colo,last_latency,
             last_download_mb_s,last_loss,last_stability,last_status,last_tested,ip_type,ip_version,success_count,fail_count
             FROM candidates",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| row_to_candidate(r))
        .map_err(|e| e.to_string())?;
    let all: Vec<Candidate> = rows.filter_map(|r| r.ok()).collect();
    Ok(select_candidates_for_scan(all, request))
}

/// 从完整候选池中选出本轮深度测试窗口。
///
/// 不能直接按导入顺序截断，否则新来源会挤掉历史验证过的高质量节点。
/// 这里按以下规则排序并分配名额：
/// 1. 已有真实历史成绩优先：下载速度、成功次数、低延迟、低丢包；
/// 2. 从未测试的新节点保留探索名额，但排在已验证节点之后；
/// 3. 每个已选地区有独立基础名额，避免候选数多的地区吃光整个窗口；
/// 4. 剩余名额再按全局历史得分补齐。
///
/// 没有历史数据的新节点无法在测速前预知真实下载速度，因此该窗口是
/// “已知最优 + 新节点探索”的结果，而不是把首次导入的前 N 条直接送去测速。
fn select_candidates_for_scan(mut all: Vec<Candidate>, request: &ScanRequest) -> Vec<Candidate> {
    let selected: Vec<String> = if request.regions.iter().any(|x| x == "all") {
        REGION_ORDER.iter().map(|s| s.to_string()).collect()
    } else {
        request.regions.clone()
    };
    if !request.source_filter.is_empty() && !request.source_filter.iter().any(|s| s == "all") {
        all.retain(|c| c.sources.iter().any(|s| request.source_filter.contains(s)));
    }
    // 七地区优先，但绝不丢弃已知属于其他国家的候选：
    // 每轮固定留出一部分探索名额，保证德国、加拿大、澳洲等非首选地区同样能被识别和测速。

    let reserve = if request.smart_supplement { 3.0 } else { 1.8 };
    let cap = ((request.per_region_top.max(1) * selected.len().max(1)) as f64 * reserve) as usize + 600;
    let limit = request.max_candidates.min(cap.max(500));
    if all.is_empty() || limit == 0 {
        return Vec::new();
    }

    fn history_score(c: &Candidate) -> f64 {
        let speed = c.last_download_mb_s.unwrap_or(0.0).clamp(0.0, 50.0);
        let latency = c
            .last_latency
            .map(|value| (1500.0 - value).clamp(0.0, 1500.0) / 15.0)
            .unwrap_or(0.0);
        let loss = c
            .last_loss
            .map(|value| (100.0 - value).clamp(0.0, 100.0) / 2.0)
            .unwrap_or(0.0);
        let total = c.success_count + c.fail_count;
        let success_ratio = if total == 0 {
            0.0
        } else {
            c.success_count as f64 / total as f64 * 100.0
        };
        let has_speed = speed > 0.0;
        let has_latency = c.last_latency.unwrap_or(0.0) > 0.0;
        let tier = if has_speed {
            1000.0
        } else if has_latency
            && matches!(
                c.last_status.as_str(),
                "latency_ok" | "latency_below" | "download_failed"
            )
        {
            600.0
        } else if c.last_status.is_empty()
            || c.last_tested.is_empty()
            || c.last_status == "never"
        {
            400.0
        } else if matches!(
            c.last_status.as_str(),
            "tcp_failed" | "https_failed" | "latency_failed" | "colo_unmatched"
        ) {
            100.0
        } else {
            300.0
        };
        tier + speed * 30.0 + latency + loss + success_ratio
    }

    fn sort_best_first(items: &mut [Candidate]) {
        items.sort_by(|a, b| {
            history_score(b)
                .partial_cmp(&history_score(a))
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.last_tested.cmp(&a.last_tested))
                .then_with(|| b.id.cmp(&a.id))
        });
    }

    let mut groups: HashMap<String, Vec<Candidate>> = HashMap::new();
    let mut group_order: Vec<String> = selected.clone();
    group_order.push("__unlabeled".to_string());
    group_order.push("__other".to_string());
    for candidate in all {
        let key = if selected.contains(&candidate.initial_region) {
            candidate.initial_region.clone()
        } else if candidate.initial_region.is_empty() {
            "__unlabeled".to_string()
        } else {
            "__other".to_string()
        };
        groups.entry(key).or_default().push(candidate);
    }

    // Each non-empty region gets a fair base window, then remaining slots are
    // filled by global historical score, so a large US pool cannot starve JP/HK.
    let group_count = groups.len().max(1);
    let per_group_share = (limit / group_count).max(1);
    let base_quota = (request.per_region_top.max(1) * 2)
        .min(per_group_share)
        .max(1);
    // 非首选地区固定探索配额：窗口越大保留越多，但不超过一半，避免反过来挤掉首选地区。
    let other_quota = (limit / 6).clamp(60, 240).min(limit / 2).max(1);
    let preferred_budget = limit.saturating_sub(other_quota);
    let mut chosen: Vec<Candidate> = Vec::with_capacity(limit);
    let mut leftovers: Vec<Candidate> = Vec::new();
    for key in &group_order {
        let Some(mut group) = groups.remove(key) else {
            continue;
        };
        sort_best_first(&mut group);
        let quota = if key == "__other" { other_quota } else { base_quota };
        let budget = if key == "__other" {
            limit.saturating_sub(chosen.len())
        } else {
            preferred_budget.saturating_sub(chosen.len())
        };
        let take = quota
            .min(group.len())
            .min(budget);
        chosen.extend(group.drain(..take));
        leftovers.extend(group);
    }
    sort_best_first(&mut leftovers);
    if chosen.len() < limit {
        let remaining = limit - chosen.len();
        chosen.extend(leftovers.into_iter().take(remaining));
    }
    chosen.truncate(limit);
    chosen
}

pub fn update_candidate_test(
    conn: &Connection,
    ip: &str,
    port: u16,
    status: &str,
    latency: Option<f64>,
    speed: Option<f64>,
    loss: Option<f64>,
    stability: Option<f64>,
    colo: &str,
) -> Result<(), String> {
    conn.execute(
        "UPDATE candidates SET last_status=?1,last_latency=?2,last_download_mb_s=?3,last_loss=?4,
         last_stability=?5,last_colo=?6,last_tested=?7,
         success_count=success_count + CASE WHEN ?1 IN ('passed','latency_below','speed_below') THEN 1 ELSE 0 END,
         fail_count=fail_count + CASE WHEN ?1 IN ('tcp_failed','https_failed','latency_failed','colo_unmatched') THEN 1 ELSE 0 END
         WHERE ip=?8 AND port=?9",
        params![status, latency, speed, loss, stability, colo, now_text(), ip, port as i64],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn create_scan(conn: &Connection, scan_id: &str, req: &ScanRequest) -> Result<(), String> {
    conn.execute(
        "INSERT INTO scans(scan_id,started_at,status,params_json,regions_json) VALUES(?1,?2,'running',?3,?4)",
        params![
            scan_id,
            now_text(),
            serde_json::to_string(req).unwrap_or_else(|_| "{}".into()),
            serde_json::to_string(&req.regions).unwrap_or_else(|_| "[]".into())
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn finish_scan(
    conn: &Connection,
    scan_id: &str,
    status: &str,
    tested: usize,
    final_count: usize,
    avg_latency: f64,
    avg_speed: f64,
) -> Result<(), String> {
    conn.execute(
        "UPDATE scans SET finished_at=?1,status=?2,tested=?3,final_count=?4,avg_latency=?5,avg_speed=?6 WHERE scan_id=?7",
        params![now_text(), status, tested as i64, final_count as i64, avg_latency, avg_speed, scan_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn insert_result(conn: &Connection, result: &ScanResult) -> Result<(), String> {
    conn.execute(
        "INSERT INTO results(scan_id,rank,ip,port,region,colo,latency,jitter,speed_mb_s,speed_mbps,loss,stability,source,status,tested_at)
         VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
        params![
            result.scan_id,
            result.rank as i64,
            result.ip,
            result.port as i64,
            result.region,
            result.colo,
            result.latency,
            result.jitter,
            result.speed_mb_s,
            result.speed_mbps,
            result.loss,
            result.stability,
            result.source,
            result.status,
            result.tested_at
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn row_to_result(row: &rusqlite::Row<'_>) -> rusqlite::Result<ScanResult> {
    let region: String = row.get(5)?;
    let colo: String = row.get(6)?;
    Ok(ScanResult {
        id: row.get(0)?,
        scan_id: row.get(1)?,
        rank: row.get::<_, i64>(2)? as usize,
        ip: row.get(3)?,
        port: row.get::<_, i64>(4)? as u16,
        region_name: node_display_name(&region, &colo),
        region,
        colo,
        latency: row.get(7)?,
        jitter: row.get(8)?,
        speed_mb_s: row.get(9)?,
        speed_mbps: row.get(10)?,
        loss: row.get(11)?,
        stability: row.get(12)?,
        source: row.get(13)?,
        status: row.get(14)?,
        tested_at: row.get(15)?,
    })
}

pub fn list_results(conn: &Connection, scan_id: &str, region: &str) -> Result<Vec<ScanResult>, String> {
    let mut sql = "SELECT id,scan_id,rank,ip,port,region,colo,latency,jitter,speed_mb_s,speed_mbps,
                   loss,stability,source,status,tested_at FROM results WHERE scan_id=?1".to_string();
    if !region.is_empty() && region != "all" {
        sql.push_str(&format!(" AND region='{}'", region.replace('\'', "")));
    }
    sql.push_str(" ORDER BY region, rank ASC");
    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![scan_id], |r| row_to_result(r))
        .map_err(|e| e.to_string())?;
    let mut out: Vec<ScanResult> = rows.filter_map(|r| r.ok()).collect();
    out.sort_by(|a, b| {
        let ai = REGION_ORDER.iter().position(|x| *x == a.region).unwrap_or(99);
        let bi = REGION_ORDER.iter().position(|x| *x == b.region).unwrap_or(99);
        ai.cmp(&bi).then_with(|| a.rank.cmp(&b.rank))
    });
    Ok(out)
}

pub fn latest_scan_id(conn: &Connection) -> Option<String> {
    conn.query_row(
        "SELECT scan_id FROM scans WHERE status='completed' ORDER BY finished_at DESC LIMIT 1",
        [],
        |r| r.get(0),
    )
    .optional()
    .ok()
    .flatten()
}

pub fn get_scan(conn: &Connection, scan_id: &str) -> Result<Option<ScanHistoryRow>, String> {
    conn.query_row(
        "SELECT scan_id,started_at,finished_at,status,tested,final_count,avg_latency,avg_speed,regions_json
         FROM scans WHERE scan_id=?1",
        params![scan_id],
        |r| {
            let regions_text: String = r.get(8)?;
            Ok(ScanHistoryRow {
                scan_id: r.get(0)?,
                started_at: r.get(1)?,
                finished_at: r.get(2)?,
                status: r.get(3)?,
                tested: r.get::<_, i64>(4)? as usize,
                final_count: r.get::<_, i64>(5)? as usize,
                avg_latency: r.get(6)?,
                avg_speed: r.get(7)?,
                regions: serde_json::from_str(&regions_text).unwrap_or_default(),
            })
        },
    )
    .optional()
    .map_err(|e| e.to_string())
}

pub fn list_history(conn: &Connection, limit: usize) -> Result<Vec<ScanHistoryRow>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT scan_id,started_at,finished_at,status,tested,final_count,avg_latency,avg_speed,regions_json
             FROM scans ORDER BY started_at DESC LIMIT ?1",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![limit as i64], |r| {
            let regions_text: String = r.get(8)?;
            Ok(ScanHistoryRow {
                scan_id: r.get(0)?,
                started_at: r.get(1)?,
                finished_at: r.get(2)?,
                status: r.get(3)?,
                tested: r.get::<_, i64>(4)? as usize,
                final_count: r.get::<_, i64>(5)? as usize,
                avg_latency: r.get(6)?,
                avg_speed: r.get(7)?,
                regions: serde_json::from_str(&regions_text).unwrap_or_default(),
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn dashboard(conn: &Connection) -> Result<DashboardData, String> {
    let total_candidates: i64 = conn
        .query_row("SELECT COUNT(*) FROM candidates", [], |r| r.get(0))
        .unwrap_or(0);
    let latest = latest_scan_id(conn);
    let mut data = DashboardData {
        total_candidates: total_candidates as usize,
        latest_scan_id: latest.clone().unwrap_or_default(),
        ..Default::default()
    };
    if let Some(ref scan_id) = latest {
        if let Some(scan) = get_scan(conn, scan_id)? {
            data.total_tested = scan.tested;
            data.avg_latency = scan.avg_latency;
            data.avg_speed = scan.avg_speed;
            data.latest_scan_at = scan.finished_at;
        }
        let results = list_results(conn, scan_id, "all")?;
        // dashboard 的“可用 / 地区覆盖”只统计正式通过的节点。
        // 结果表同时保留 latency_below / speed_below 参考记录，不能把这些
        // 仅展示的记录误算成可导出节点。
        let passed_results: Vec<&ScanResult> = results
            .iter()
            .filter(|row| row.status == "passed")
            .collect();
        data.usable = passed_results.len();
        data.recent_results = passed_results.iter().take(12).map(|row| (*row).clone()).collect();
        for key in REGION_ORDER {
            let rr: Vec<&ScanResult> = passed_results
                .iter()
                .copied()
                .filter(|r| r.region == key)
                .collect();
            let avg_latency = if rr.is_empty() { 0.0 } else { rr.iter().map(|r| r.latency).sum::<f64>() / rr.len() as f64 };
            let avg_speed = if rr.is_empty() { 0.0 } else { rr.iter().map(|r| r.speed_mb_s).sum::<f64>() / rr.len() as f64 };
            data.region_stats.push(RegionStats {
                key: key.to_string(),
                name: region_name(key).to_string(),
                flag: region_flag(key).to_string(),
                candidates: 0,
                tcp_ok: 0,
                https_ok: 0,
                colo_ok: 0,
                latency_ok: 0,
                download_ok: rr.len(),
                speed_ok: rr.len(),
                final_count: rr.len(),
                avg_latency,
                avg_speed,
                latency_eliminated: 0,
            });
        }
        // 其他国家的达标结果同样进入总览与图表，而不是被折叠成看不见的“0 / 7”。
        let mut extra_groups: HashMap<String, Vec<&ScanResult>> = HashMap::new();
        for row in passed_results.iter().copied() {
            if !REGION_ORDER.contains(&row.region.as_str()) {
                extra_groups.entry(row.region.clone()).or_default().push(row);
            }
        }
        let mut extra_rows: Vec<(String, Vec<&ScanResult>)> = extra_groups.into_iter().collect();
        extra_rows.sort_by(|a, b| b.1.len().cmp(&a.1.len()));
        for (key, rr) in extra_rows {
            if key.is_empty() {
                continue;
            }
            let avg_latency = rr.iter().map(|r| r.latency).sum::<f64>() / rr.len().max(1) as f64;
            let avg_speed = rr.iter().map(|r| r.speed_mb_s).sum::<f64>() / rr.len().max(1) as f64;
            data.region_stats.push(RegionStats {
                key: key.clone(),
                name: region_name(&key).to_string(),
                flag: region_flag(&key).to_string(),
                candidates: 0,
                tcp_ok: 0,
                https_ok: 0,
                colo_ok: 0,
                latency_ok: 0,
                download_ok: rr.len(),
                speed_ok: rr.len(),
                final_count: rr.len(),
                avg_latency,
                avg_speed,
                latency_eliminated: 0,
            });
        }
    } else {
        for key in REGION_ORDER {
            data.region_stats.push(RegionStats {
                key: key.to_string(),
                name: region_name(key).to_string(),
                flag: region_flag(key).to_string(),
                ..Default::default()
            });
        }
    }
    // fill candidate counts by initial region for context
    for stat in &mut data.region_stats {
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM candidates WHERE initial_region=?1",
                params![stat.key],
                |r| r.get(0),
            )
            .unwrap_or(0);
        stat.candidates = count as usize;
    }
    data.network = get_meta(conn, "network_cache").and_then(|s| serde_json::from_str(&s).ok());
    Ok(data)
}

pub fn clear_history(conn: &Connection, scope: &str) -> Result<(), String> {
    let result = match scope {
        "results" => conn.execute("DELETE FROM results", []).map(|_| ()),
        "scans" => conn.execute("DELETE FROM scans", []).map(|_| ()),
        "candidates" => conn.execute("DELETE FROM candidates", []).map(|_| ()),
        _ => conn.execute_batch("DELETE FROM results; DELETE FROM scans; DELETE FROM candidates;"),
    };
    result.map_err(|e| e.to_string())?;
    Ok(())
}

pub fn delete_scan(conn: &Connection, scan_id: &str) -> Result<(), String> {
    conn.execute("DELETE FROM scans WHERE scan_id=?1", params![scan_id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn backup_database(conn: &Connection, directory: &str) -> Result<String, String> {
    let dir = Path::new(directory);
    std::fs::create_dir_all(dir).map_err(|e| format!("创建备份目录失败: {e}"))?;
    let target = dir.join(format!("mikasa-backup-{}.sqlite", Local::now().format("%Y%m%d-%H%M%S")));
    let target_sql = target.to_string_lossy().replace('\'', "''");
    conn.execute_batch(&format!("VACUUM INTO '{}';", target_sql))
        .map_err(|e| format!("数据库备份失败: {e}"))?;
    Ok(target.to_string_lossy().to_string())
}

pub fn list_url_sources(conn: &Connection) -> Result<Vec<UrlSource>, String> {
    let mut stmt = conn
        .prepare("SELECT id,url,enabled,interval_minutes,last_fetch,last_count,last_status FROM url_sources ORDER BY id")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(UrlSource {
                id: r.get(0)?,
                url: r.get(1)?,
                enabled: r.get::<_, i64>(2)? != 0,
                interval_minutes: r.get::<_, i64>(3)? as u64,
                last_fetch: r.get(4)?,
                last_count: r.get::<_, i64>(5)? as usize,
                last_status: r.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

pub fn upsert_url_source(conn: &Connection, url: &str, interval_minutes: u64) -> Result<(), String> {
    conn.execute(
        "INSERT INTO url_sources(url,enabled,interval_minutes) VALUES(?1,1,?2)
         ON CONFLICT(url) DO UPDATE SET enabled=1,interval_minutes=excluded.interval_minutes",
        params![url, interval_minutes as i64],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn remove_url_source(conn: &Connection, id: i64) -> Result<(), String> {
    conn.execute("DELETE FROM url_sources WHERE id=?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn update_url_source_status(conn: &Connection, id: i64, count: usize, status: &str) -> Result<(), String> {
    conn.execute(
        "UPDATE url_sources SET last_fetch=?1,last_count=?2,last_status=?3 WHERE id=?4",
        params![now_text(), count as i64, status, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn record_source_import(
    conn: &Connection,
    source: &str,
    fetched: usize,
    added: usize,
    duplicates: usize,
    invalid: usize,
    status: &str,
    message: &str,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO source_stats(source,last_fetched,last_added,last_duplicates,last_invalid,last_update,status,message)
         VALUES(?1,?2,?3,?4,?5,?6,?7,?8)
         ON CONFLICT(source) DO UPDATE SET last_fetched=excluded.last_fetched,last_added=excluded.last_added,
         last_duplicates=excluded.last_duplicates,last_invalid=excluded.last_invalid,last_update=excluded.last_update,
         status=excluded.status,message=excluded.message",
        params![source, fetched as i64, added as i64, duplicates as i64, invalid as i64, now_text(), status, message],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn source_overview(conn: &Connection) -> Result<Vec<SourceOverview>, String> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    let mut stmt = conn.prepare("SELECT sources FROM candidates").map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |r| r.get::<_, String>(0)).map_err(|e| e.to_string())?;
    for row in rows.flatten() {
        if let Ok(list) = serde_json::from_str::<Vec<String>>(&row) {
            for source in list {
                *counts.entry(source).or_insert(0) += 1;
            }
        }
    }
    let cm = get_meta(conn, "cm_source_status")
        .and_then(|value| serde_json::from_str::<serde_json::Value>(&value).ok());
    let mut result = Vec::new();
    for (key, name) in [
        ("local", "本地文件"),
        ("url", "网络链接"),
        ("fofa", "FOFA 接入"),
        ("cloudflare", "Cloudflare 抓取"),
        ("cm", "CM IP 库"),
        ("github", "GitHub Raw"),
        ("censys", "Censys 平台"),
        ("cf_best_ip", "cf-best-ip 上游聚合"),
        ("bestcf", "BestCF（DustinWin）"),
        ("best_cf_ips", "best-cf-ips（LancelotRar）"),
        ("ipdb", "IPDB（ymyuuu）"),
        ("cloudflare_ip", "cloudflare_ip（svip-s）"),
        ("free_nodes_bestcf", "free-nodes/bestcf 上游源"),
        ("xiu2", "XIU2/CloudflareSpeedTest"),
    ] {
        let mut row = SourceOverview {
            key: key.to_string(),
            name: name.to_string(),
            count: *counts.get(key).unwrap_or(&0),
            ..Default::default()
        };
        if key == "cm" {
            if let Some(status) = &cm {
                row.count = status
                    .get("count")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(row.count as u64) as usize;
                row.fetched = status.get("count").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                row.added = status.get("lastAdded").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                row.duplicates = status.get("lastDuplicates").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                row.invalid = status.get("lastInvalid").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                row.last_update = status.get("lastFetch").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let st = status.get("status").and_then(|v| v.as_str()).unwrap_or("未连接");
                row.status = if st == "已连接" { "正常".to_string() } else { st.to_string() };
            } else {
                row.status = "未连接".to_string();
            }
        } else {
            let stats: Option<(i64, i64, i64, i64, String, String, String)> = conn
                .query_row(
                    "SELECT last_fetched,last_added,last_duplicates,last_invalid,last_update,status,message FROM source_stats WHERE source=?1",
                    params![key],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?)),
                )
                .optional()
                .map_err(|e| e.to_string())?;
            if let Some((fetched, added, duplicates, invalid, update, status, message)) = stats {
                row.fetched = fetched as usize;
                row.added = added as usize;
                row.duplicates = duplicates as usize;
                row.invalid = invalid as usize;
                row.last_update = update;
                row.status = status;
                row.message = message;
            } else {
                row.status = if row.count > 0 { "已导入".to_string() } else { "待获取".to_string() };
            }
        }
        result.push(row);
    }
    Ok(result)
}

pub fn source_quality(conn: &Connection) -> Result<Vec<SourceQuality>, String> {
    #[derive(Default, Clone)]
    struct Row {
        raw: usize,
        added: usize,
        duplicates: usize,
        last_update: String,
        status: String,
    }
    let mut stats: HashMap<String, Row> = HashMap::new();
    let mut stmt = conn
        .prepare("SELECT source,last_fetched,last_added,last_duplicates,last_update,status FROM source_stats")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)? as usize,
                r.get::<_, i64>(2)? as usize,
                r.get::<_, i64>(3)? as usize,
                r.get::<_, String>(4)?,
                r.get::<_, String>(5)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    for row in rows.flatten() {
        stats.insert(
            row.0,
            Row { raw: row.1, added: row.2, duplicates: row.3, last_update: row.4, status: row.5 },
        );
    }
    let mut candidates: Vec<(Vec<String>, String, Option<f64>, String)> = Vec::new();
    let mut stmt = conn
        .prepare("SELECT sources,last_status,last_download_mb_s,last_colo FROM candidates")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            let sources: String = r.get(0)?;
            Ok((
                serde_json::from_str::<Vec<String>>(&sources).unwrap_or_default(),
                r.get::<_, String>(1)?,
                r.get::<_, Option<f64>>(2)?,
                r.get::<_, String>(3)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    candidates.extend(rows.flatten());
    let mut result = Vec::new();
    for info in crate::sources::community_sources() {
        let key = info.key.clone();
        let stat = stats.get(&key).cloned().unwrap_or_default();
        let mut count = 0usize;
        let mut valid = 0usize;
        let mut fast = 0usize;
        let mut regions = std::collections::HashSet::new();
        for (sources, status, speed, colo) in &candidates {
            if !sources.iter().any(|s| s == &key) {
                continue;
            }
            count += 1;
            if matches!(status.as_str(), "passed" | "latency_below" | "speed_below" | "latency_ok") {
                valid += 1;
            }
            if speed.unwrap_or(0.0) >= 1.0 {
                fast += 1;
            }
            if !colo.is_empty() {
                if let Some(region) = region_for_any_colo(colo) {
                    regions.insert(region);
                }
            }
        }
        let total = stat.raw.max(stat.added + stat.duplicates);
        let duplicate_rate = if total == 0 {
            0.0
        } else {
            stat.duplicates as f64 / total as f64 * 100.0
        };
        let unique_ratio = if stat.added == 0 && count > 0 { 1.0 } else if stat.added + stat.duplicates == 0 { 0.0 } else { stat.added as f64 / (stat.added + stat.duplicates) as f64 };
        let valid_ratio = if count == 0 { 0.0 } else { valid as f64 / count as f64 };
        let fast_ratio = if count == 0 { 0.0 } else { fast as f64 / count as f64 };
        let region_ratio = (regions.len() as f64 / REGION_ORDER.len() as f64).min(1.0);
        let score = (unique_ratio * 30.0 + valid_ratio * 30.0 + fast_ratio * 20.0 + region_ratio * 10.0 + (1.0 - duplicate_rate / 100.0) * 10.0).clamp(0.0, 100.0);
        result.push(SourceQuality {
            key: key.clone(),
            name: info.name,
            raw_count: stat.raw.max(stat.added + stat.duplicates),
            unique_count: stat.added,
            duplicate_rate,
            valid_connections: valid,
            high_speed_count: fast,
            region_coverage: regions.len(),
            update_frequency: info.frequency,
            last_success: if stat.status == "正常" || stat.status == "部分成功" { stat.last_update } else { String::new() },
            score,
        });
    }
    result.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    Ok(result)
}

pub fn record_github_history(conn: &Connection, repo: &str, branch: &str, files: &[String], status: &str, message: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO github_history(repo,branch,files,status,message,created_at) VALUES(?1,?2,?3,?4,?5,?6)",
        params![repo, branch, serde_json::to_string(files).unwrap_or_default(), status, message, now_text()],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn github_history(conn: &Connection, limit: usize) -> Result<Vec<serde_json::Value>, String> {
    let mut stmt = conn
        .prepare("SELECT repo,branch,files,status,message,created_at FROM github_history ORDER BY id DESC LIMIT ?1")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![limit as i64], |r| {
            let files: String = r.get(2)?;
            let message: String = r.get(4)?;
            let commit_sha = message
                .strip_prefix("commit ")
                .unwrap_or("")
                .trim()
                .to_string();
            Ok(serde_json::json!({
                "repo": r.get::<_, String>(0)?,
                "branch": r.get::<_, String>(1)?,
                "files": serde_json::from_str::<Vec<String>>(&files).unwrap_or_default(),
                "status": r.get::<_, String>(3)?,
                "message": message,
                "commitSha": commit_sha,
                "createdAt": r.get::<_, String>(5)?,
            }))
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn candidate(
        id: i64,
        region: &str,
        status: &str,
        speed: Option<f64>,
        latency: Option<f64>,
    ) -> Candidate {
        Candidate {
            id,
            ip: format!("10.0.0.{id}"),
            port: 443,
            sources: vec!["test".to_string()],
            initial_region: region.to_string(),
            first_seen: "2026-11-25 00:00:00".to_string(),
            last_seen: "2026-11-25 00:00:00".to_string(),
            last_colo: String::new(),
            last_latency: latency,
            last_download_mb_s: speed,
            last_loss: Some(0.0),
            last_stability: None,
            last_status: status.to_string(),
            last_tested: if status.is_empty() { String::new() } else { "2026-11-25 00:00:00".to_string() },
            ip_type: "ipv4".to_string(),
            ip_version: 4,
            success_count: if speed.is_some() { 3 } else { 0 },
            fail_count: 0,
        }
    }

    fn request(regions: &[&str], per_region_top: usize, max_candidates: usize) -> ScanRequest {
        ScanRequest {
            regions: regions.iter().map(|value| value.to_string()).collect(),
            per_region_top,
            port: 443,
            concurrency: 64,
            latency_limit_ms: 1500.0,
            speed_limit_mb_s: 1.0,
            download_seconds: 10,
            min_download_bytes: 3 * 1024 * 1024,
            latency_rounds: 4,
            stability_rounds: 0,
            download_threads: 1,
            smart_supplement: false,
            max_candidates,
            source_filter: vec!["all".to_string()],
            download_url: None,
        }
    }

    fn temp_db() -> (Connection, PathBuf) {
        let path = std::env::temp_dir().join(format!(
            "mikasa-db-test-{}-{}.sqlite",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        let _ = std::fs::remove_file(&path);
        let conn = open_database(&path).expect("open temp database");
        (conn, path)
    }

    fn scan_result(
        scan_id: &str,
        ip: &str,
        region: &str,
        status: &str,
        latency: f64,
        speed_mb_s: f64,
    ) -> ScanResult {
        ScanResult {
            id: 0,
            scan_id: scan_id.to_string(),
            rank: 1,
            ip: ip.to_string(),
            port: 443,
            region: region.to_string(),
            region_name: String::new(),
            colo: if region == "hk" { "HKG" } else { "LAX" }.to_string(),
            latency,
            jitter: 0.0,
            speed_mb_s,
            speed_mbps: speed_mb_s * 8.0,
            loss: 0.0,
            stability: 100.0,
            source: "test".to_string(),
            status: status.to_string(),
            tested_at: "2026-11-26 00:00:00".to_string(),
        }
    }

    #[test]
    fn scan_window_uses_historical_performance_not_import_order() {
        let pool = vec![
            candidate(1, "hk", "passed", Some(12.0), Some(120.0)),
            candidate(2, "hk", "", None, None),
            candidate(3, "hk", "", None, None),
            candidate(4, "hk", "tcp_failed", None, None),
            candidate(5, "hk", "passed", Some(8.0), Some(90.0)),
        ];
        let selected = select_candidates_for_scan(pool, &request(&["hk"], 1, 3));
        let ids: Vec<i64> = selected.iter().map(|item| item.id).collect();
        assert!(ids.contains(&1));
        assert!(ids.contains(&5));
        assert!(!ids.contains(&4));
        assert_eq!(ids.len(), 3);
    }

    #[test]
    fn scan_window_reserves_slots_for_each_selected_region() {
        let mut pool: Vec<Candidate> = (100..120)
            .map(|id| candidate(id, "hk", "", None, None))
            .collect();
        pool.push(candidate(1, "jp", "passed", Some(10.0), Some(80.0)));
        pool.push(candidate(2, "jp", "passed", Some(8.0), Some(90.0)));
        let selected = select_candidates_for_scan(pool, &request(&["hk", "jp"], 2, 4));
        assert_eq!(selected.len(), 4);
        assert!(selected.iter().any(|item| item.initial_region == "jp"));
        assert!(selected.iter().any(|item| item.initial_region == "hk"));
    }

    #[test]
    fn dashboard_counts_only_passed_results_and_keeps_other_regions() {
        let (conn, path) = temp_db();
        let scan_id = "scan-dashboard-test";
        create_scan(&conn, scan_id, &request(&["hk", "jp", "us"], 2, 8)).unwrap();
        insert_result(
            &conn,
            &scan_result(scan_id, "198.51.100.1", "hk", "passed", 40.0, 20.0),
        )
        .unwrap();
        insert_result(
            &conn,
            &scan_result(scan_id, "198.51.100.2", "jp", "passed", 60.0, 15.0),
        )
        .unwrap();
        insert_result(
            &conn,
            &scan_result(scan_id, "198.51.100.3", "us", "speed_below", 80.0, 0.5),
        )
        .unwrap();
        insert_result(
            &conn,
            &scan_result(scan_id, "198.51.100.4", "de", "passed", 90.0, 12.0),
        )
        .unwrap();
        finish_scan(&conn, scan_id, "completed", 4, 3, 67.5, 15.67).unwrap();

        let dashboard = dashboard(&conn).unwrap();
        assert_eq!(dashboard.usable, 3);
        assert_eq!(dashboard.recent_results.len(), 3);
        assert!(dashboard.recent_results.iter().all(|row| row.status == "passed"));

        let hk = dashboard.region_stats.iter().find(|row| row.key == "hk").unwrap();
        let us = dashboard.region_stats.iter().find(|row| row.key == "us").unwrap();
        let de = dashboard.region_stats.iter().find(|row| row.key == "de").unwrap();
        assert_eq!(hk.final_count, 1);
        assert_eq!(us.final_count, 0, "speed_below must not be counted as usable");
        assert_eq!(de.final_count, 1, "non-preferred regions must remain visible");
        drop(conn);
        let _ = std::fs::remove_file(path);
    }
}


