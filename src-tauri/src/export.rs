use crate::db;
use crate::models::*;
use rusqlite::Connection;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct ExportFile {
    pub name: String,
    pub content: String,
}

fn number_label(value: f64) -> String {
    if (value - value.round()).abs() < 0.005 {
        format!("{value:.0}")
    } else {
        format!("{value:.1}")
    }
}

fn txt_value(value: &str) -> String {
    value.replace(['\r', '\n'], " ").trim().to_string()
}

fn txt_line(r: &ScanResult, options: &ExportOptions) -> String {
    let mut line = format!("{}:{}#三笠优选", r.ip, r.port);
    if options.include_city && !r.region_name.trim().is_empty() {
        line.push_str(&format!(" | {}", txt_value(&r.region_name)));
    }
    if options.include_speed {
        line.push_str(&format!(" | 下载 {:.2}MB/s", r.speed_mb_s));
    }
    if options.include_latency {
        line.push_str(&format!(" | 延迟 {:.0}ms", r.latency));
    }
    if options.include_colo && !r.colo.trim().is_empty() {
        line.push_str(&format!(" | {}", txt_value(&r.colo)));
    }
    if options.include_source && !r.source.trim().is_empty() {
        line.push_str(&format!(" | {}", txt_value(&r.source)));
    }
    line
}

fn csv_escape(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn csv_line(r: &ScanResult) -> String {
    [
        csv_escape(&r.ip),
        r.port.to_string(),
        csv_escape(&r.region_name),
        csv_escape(&r.colo),
        format!("{:.1}", r.latency),
        format!("{:.2}", r.speed_mb_s),
        format!("{:.1}", r.speed_mbps),
        format!("{:.1}", r.loss),
        format!("{:.1}", r.stability),
        csv_escape(&r.source),
        csv_escape(&r.tested_at),
    ]
    .join(",")
}

fn header(csv: bool) -> &'static str {
    if csv {
        "IP,端口,国家/城市,colo,平均延迟(ms),下载速度(MB/s),下载速度(Mbps),丢包率(%),稳定性(%),来源,测试时间\n"
    } else {
        ""
    }
}

fn append_rows(content: &mut String, rows: &[&ScanResult], csv: bool, options: &ExportOptions) {
    content.push_str(header(csv));
    for row in rows {
        if csv {
            content.push_str(&csv_line(row));
        } else {
            content.push_str(&txt_line(row, options));
        }
        content.push('\n');
    }
}

fn filter_description(min_speed: f64, max_latency: f64) -> String {
    let mut parts = Vec::new();
    if min_speed > 0.0 {
        parts.push(format!("速度 ≥ {} MB/s", number_label(min_speed)));
    }
    if max_latency > 0.0 {
        parts.push(format!("延迟 ≤ {} ms", number_label(max_latency)));
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!("（{}）", parts.join("、"))
    }
}

fn filtered_rows<'a>(all_results: &'a [ScanResult], options: &ExportOptions) -> Vec<&'a ScanResult> {
    let min_speed = if options.min_speed_mb_s.is_finite() {
        options.min_speed_mb_s.max(0.0)
    } else {
        0.0
    };
    let max_latency = if options.max_latency_ms.is_finite() {
        options.max_latency_ms.max(0.0)
    } else {
        0.0
    };
    let region = options.region.trim();
    let export_all = region.is_empty() || region.eq_ignore_ascii_case("all");
    let export_global = region.eq_ignore_ascii_case("global");
    let export_unknown = region.eq_ignore_ascii_case("unknown");
    all_results
        .iter()
        .filter(|r| {
            export_all
                || if export_global {
                    !REGION_ORDER.contains(&r.region.as_str()) && r.region != "unknown"
                } else if export_unknown {
                    r.region == "unknown"
                } else {
                    r.region == region
                }
        })
        .filter(|r| min_speed <= 0.0 || r.speed_mb_s >= min_speed)
        .filter(|r| max_latency <= 0.0 || r.latency <= max_latency)
        .collect()
}

fn filter_file_suffix(options: &ExportOptions) -> String {
    let min_speed = if options.min_speed_mb_s.is_finite() {
        options.min_speed_mb_s.max(0.0)
    } else {
        0.0
    };
    let max_latency = if options.max_latency_ms.is_finite() {
        options.max_latency_ms.max(0.0)
    } else {
        0.0
    };
    let mut parts = Vec::new();
    if min_speed > 0.0 {
        parts.push(format!("{}MBs以上", number_label(min_speed)));
    }
    if max_latency > 0.0 {
        parts.push(format!("{}ms内", number_label(max_latency)));
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!("_{}", parts.join("_"))
    }
}

/// 单个地区只写一个文件；全部地区合并为一个文件。
/// `region` 为空或 "all" 表示全部地区合并。
pub fn build_export_files(conn: &Connection, options: &ExportOptions) -> Result<Vec<ExportFile>, String> {
    let scan_id = if options.scan_id.trim().is_empty() {
        db::latest_scan_id(conn).ok_or_else(|| "还没有完成的扫描记录，请先执行优选".to_string())?
    } else {
        options.scan_id.trim().to_string()
    };
    // 正式导出只包含全部达标的 IP；已完成测速但未达延迟/速度标准的参考结果绝不混入
    let all_results: Vec<ScanResult> = db::list_results(conn, &scan_id, "all")?
        .into_iter()
        .filter(|r| r.status == "passed")
        .collect();
    if all_results.is_empty() {
        return Err("该扫描没有同时满足延迟、下载速度与稳定性要求的 IP，不能生成正式优选文件".to_string());
    }

    let csv = options.format.eq_ignore_ascii_case("csv");
    let ext = if csv { "csv" } else { "txt" };
    let region = options.region.trim();
    let export_all = region.is_empty() || region.eq_ignore_ascii_case("all");
    let rows = filtered_rows(&all_results, options);
    if rows.is_empty() {
        let region_label = if export_all { "全部地区" } else { region_name(region) };
        let min_speed = options.min_speed_mb_s.max(0.0);
        let max_latency = options.max_latency_ms.max(0.0);
        return Err(format!(
            "{}筛选后没有符合条件的 IP{}，请降低筛选条件或重新扫描",
            region_label,
            filter_description(min_speed, max_latency)
        ));
    }

    let base_name = if export_all { "全部地区".to_string() } else { region_name(region).to_string() };
    let mut content = String::new();
    append_rows(&mut content, &rows, csv, options);
    Ok(vec![ExportFile {
        name: format!("{}{}.{ext}", base_name, filter_file_suffix(options)),
        content,
    }])
}

/// GitHub 订阅使用固定的 ASCII 路径，避免中文文件名在脚本、CDN 和客户端里
/// 变成一长串 percent-encoding。合并文件为 all.txt，分地区文件为 JP/all.txt。
pub fn build_github_files(
    conn: &Connection,
    options: &ExportOptions,
    split_regions: bool,
) -> Result<Vec<ExportFile>, String> {
    let ext = if options.format.eq_ignore_ascii_case("csv") { "csv" } else { "txt" };
    let mut files = Vec::new();

    if split_regions {
        for code in REGION_ORDER {
            let mut regional = options.clone();
            regional.region = code.to_string();
            match build_export_files(conn, &regional) {
                Ok(mut built) => {
                    if let Some(mut file) = built.pop() {
                        file.name = format!("{}/all.{ext}", code.to_ascii_uppercase());
                        files.push(file);
                    }
                }
                // 某个地区没有达标节点是正常情况，不应阻止其它地区上传。
                Err(error) if error.contains("没有符合条件") => {}
                Err(error) => return Err(error),
            }
        }
        // 非首选地区也单独生成可订阅文件，避免用户只能下载到合并后的 all.txt。
        for code in ["global", "unknown"] {
            let mut regional = options.clone();
            regional.region = code.to_string();
            match build_export_files(conn, &regional) {
                Ok(mut built) => {
                    if let Some(mut file) = built.pop() {
                        file.name = format!("{}/all.{ext}", code.to_ascii_uppercase());
                        files.push(file);
                    }
                }
                Err(error) if error.contains("没有符合条件") => {}
                Err(error) => return Err(error),
            }
        }

        let mut merged = options.clone();
        merged.region = "all".to_string();
        let mut all = build_export_files(conn, &merged)?;
        if let Some(mut file) = all.pop() {
            file.name = format!("all.{ext}");
            files.push(file);
        }
    } else {
        let region = options.region.trim();
        let mut built = build_export_files(conn, options)?;
        if let Some(mut file) = built.pop() {
            file.name = if region.is_empty() || region.eq_ignore_ascii_case("all") {
                format!("all.{ext}")
            } else {
                format!("{}/all.{ext}", region.to_ascii_uppercase())
            };
            files.push(file);
        }
    }

    if files.is_empty() {
        return Err("没有符合筛选条件的 IP，无法生成 GitHub 订阅文件".to_string());
    }
    Ok(files)
}

pub fn write_exports(
    conn: &Connection,
    directory: &str,
    options: &ExportOptions,
) -> Result<ExportOutcome, String> {
    let files = build_export_files(conn, options)?;
    let node_count = files
        .iter()
        .map(|file| {
            file.content
                .lines()
                .filter(|line| !line.trim().is_empty() && !line.starts_with("IP,"))
                .count()
        })
        .sum();
    let dir = if directory.trim().is_empty() {
        let home = std::env::var("USERPROFILE").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join("Documents").join("三笠优选导出")
    } else {
        PathBuf::from(directory)
    };
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建导出目录失败: {e}"))?;
    let mut written = Vec::new();
    for file in &files {
        let target = dir.join(&file.name);
        std::fs::write(&target, file.content.as_bytes())
            .map_err(|e| format!("写入 {} 失败: {e}", target.display()))?;
        written.push(target.to_string_lossy().to_string());
    }
    Ok(ExportOutcome {
        directory: dir.to_string_lossy().to_string(),
        count: node_count,
        files: written,
        format: if options.format.eq_ignore_ascii_case("csv") { "csv" } else { "txt" }.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_result(region: &str) -> ScanResult {
        let colo = if region == "hk" { "HKG" } else { "LAX" };
        ScanResult {
            id: 0,
            scan_id: "scan-test".into(),
            rank: 1,
            ip: "198.51.100.10".into(),
            port: 443,
            region: region.into(),
            region_name: node_display_name(region, colo),
            colo: colo.into(),
            latency: 42.6,
            jitter: 2.0,
            speed_mb_s: 18.35,
            speed_mbps: 146.8,
            loss: 0.0,
            stability: 100.0,
            source: "CM IP 库".into(),
            status: "passed".into(),
            tested_at: "2026-11-23 21:00:00".into(),
        }
    }

    fn temp_db() -> (Connection, PathBuf) {
        let path = std::env::temp_dir().join(format!(
            "mikasa-export-test-{}-{}.sqlite",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        let _ = std::fs::remove_file(&path);
        let conn = db::open_database(&path).expect("open temp database");
        (conn, path)
    }

    fn seed(conn: &Connection) {
        let req = ScanRequest {
            regions: vec!["hk".into(), "us".into()],
            per_region_top: 20,
            port: 443,
            concurrency: 8,
            latency_limit_ms: 1500.0,
            speed_limit_mb_s: 1.0,
            download_seconds: 5,
            min_download_bytes: 1,
            latency_rounds: 2,
            stability_rounds: 1,
            download_threads: 1,
            smart_supplement: false,
            max_candidates: 10,
            source_filter: Vec::new(),
            download_url: None,
        };
        db::create_scan(conn, "scan-test", &req).unwrap();
        db::insert_result(conn, &sample_result("hk")).unwrap();
    }

    fn seed_other_regions(conn: &Connection) {
        let mut de = sample_result("de");
        de.ip = "198.51.100.11".into();
        de.colo = "FRA".into();
        db::insert_result(conn, &de).unwrap();

        let mut unknown = sample_result("unknown");
        unknown.ip = "198.51.100.12".into();
        unknown.colo = "XXX".into();
        db::insert_result(conn, &unknown).unwrap();
    }

    fn export_options() -> ExportOptions {
        ExportOptions {
            scan_id: "scan-test".into(),
            ..Default::default()
        }
    }

    #[test]
    fn exports_all_regions_as_one_file_without_duplicate_ip_suffix() {
        let (conn, path) = temp_db();
        seed(&conn);
        let files = build_export_files(&conn, &export_options()).unwrap();
        assert_eq!(files.len(), 1, "all regions must merge into one file");
        assert_eq!(files[0].name, "全部地区.txt");
        assert!(files[0].content.contains("香港"));
        assert!(files[0].content.contains("下载 18.35MB/s"));
        assert!(!files[0].content.contains("延迟"));
        assert!(!files[0].content.contains("| IP:"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn exports_single_region_as_one_file() {
        let (conn, path) = temp_db();
        seed(&conn);
        let options = ExportOptions {
            scan_id: "scan-test".into(),
            region: "hk".into(),
            ..Default::default()
        };
        let files = build_export_files(&conn, &options).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].name, "香港.txt");
        assert!(files[0].content.contains("198.51.100.10:443"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn rejects_empty_single_region_export() {
        let (conn, path) = temp_db();
        seed(&conn);
        let options = ExportOptions {
            scan_id: "scan-test".into(),
            region: "jp".into(),
            ..Default::default()
        };
        let err = build_export_files(&conn, &options).unwrap_err();
        assert!(err.contains("日本筛选后没有符合条件的 IP"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn export_supports_optional_suffix_fields() {
        let (conn, path) = temp_db();
        seed(&conn);
        let options = ExportOptions {
            scan_id: "scan-test".into(),
            include_latency: true,
            include_colo: true,
            include_source: true,
            ..Default::default()
        };
        let files = build_export_files(&conn, &options).unwrap();
        assert_eq!(
            files[0].content.trim(),
            "198.51.100.10:443#三笠优选 | 香港 | 下载 18.35MB/s | 延迟 43ms | HKG | CM IP 库"
        );
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn export_filters_by_speed_and_latency() {
        let (conn, path) = temp_db();
        seed(&conn);
        let options = ExportOptions {
            scan_id: "scan-test".into(),
            min_speed_mb_s: 5.0,
            max_latency_ms: 100.0,
            ..Default::default()
        };
        let files = build_export_files(&conn, &options).unwrap();
        assert_eq!(files[0].name, "全部地区_5MBs以上_100ms内.txt");
        assert!(files[0].content.contains("198.51.100.10:443"));

        let too_strict = ExportOptions {
            scan_id: "scan-test".into(),
            min_speed_mb_s: 20.0,
            ..Default::default()
        };
        let err = build_export_files(&conn, &too_strict).unwrap_err();
        assert!(err.contains("速度 ≥ 20 MB/s"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn export_supports_global_and_unknown_regions() {
        let (conn, path) = temp_db();
        seed(&conn);
        seed_other_regions(&conn);

        let global = build_export_files(
            &conn,
            &ExportOptions {
                scan_id: "scan-test".into(),
                region: "global".into(),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(global.len(), 1);
        assert_eq!(global[0].name, "全球其他地区.txt");
        assert!(global[0].content.contains("198.51.100.11:443"));
        assert!(!global[0].content.contains("198.51.100.12:443"));

        let unknown = build_export_files(
            &conn,
            &ExportOptions {
                scan_id: "scan-test".into(),
                region: "unknown".into(),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(unknown.len(), 1);
        assert_eq!(unknown[0].name, "未知地区.txt");
        assert!(unknown[0].content.contains("198.51.100.12:443"));
        assert!(!unknown[0].content.contains("198.51.100.11:443"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn github_files_use_ascii_paths_and_split_regions() {
        let (conn, path) = temp_db();
        seed(&conn);
        let options = ExportOptions {
            scan_id: "scan-test".into(),
            ..Default::default()
        };

        let merged = build_github_files(&conn, &options, false).unwrap();
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].name, "all.txt");
        assert!(merged[0].content.contains("198.51.100.10:443"));

        let split = build_github_files(&conn, &options, true).unwrap();
        assert!(split.iter().any(|file| file.name == "HK/all.txt"));
        assert!(split.iter().any(|file| file.name == "all.txt"));
        assert!(!split.iter().any(|file| file.name.contains("全部地区")));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn github_files_include_global_and_unknown_partitions() {
        let (conn, path) = temp_db();
        seed(&conn);
        seed_other_regions(&conn);
        let options = ExportOptions {
            scan_id: "scan-test".into(),
            ..Default::default()
        };

        let split = build_github_files(&conn, &options, true).unwrap();
        let global = split.iter().find(|file| file.name == "GLOBAL/all.txt").unwrap();
        let unknown = split.iter().find(|file| file.name == "UNKNOWN/all.txt").unwrap();
        assert!(global.content.contains("198.51.100.11:443"));
        assert!(!global.content.contains("198.51.100.12:443"));
        assert!(unknown.content.contains("198.51.100.12:443"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn github_files_honour_speed_filter_and_extension() {
        let (conn, path) = temp_db();
        seed(&conn);
        let options = ExportOptions {
            scan_id: "scan-test".into(),
            format: "csv".into(),
            min_speed_mb_s: 5.0,
            ..Default::default()
        };
        let files = build_github_files(&conn, &options, false).unwrap();
        assert_eq!(files[0].name, "all.csv");
        assert!(files[0].content.starts_with("IP,端口"));

        let strict = ExportOptions {
            scan_id: "scan-test".into(),
            min_speed_mb_s: 50.0,
            ..Default::default()
        };
        let error = build_github_files(&conn, &strict, false).unwrap_err();
        assert!(error.contains("没有符合条件"));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn node_names_include_city_for_supported_countries() {
        assert_eq!(node_display_name("jp", "NRT"), "日本东京");
        assert_eq!(node_display_name("jp", "KIX"), "日本大阪");
        assert_eq!(node_display_name("us", "LAX"), "美国洛杉矶");
        assert_eq!(node_display_name("hk", "HKG"), "香港");
        assert_eq!(node_display_name("tw", "TPE"), "台湾");
        assert_eq!(node_display_name("sg", "SIN"), "新加坡");
    }
}

pub fn ensure_directory(path: &str) -> Result<(), String> {
    if path.trim().is_empty() {
        return Ok(());
    }
    let p = Path::new(path);
    if p.exists() && !p.is_dir() {
        return Err("导出路径不是文件夹".to_string());
    }
    std::fs::create_dir_all(p).map_err(|e| format!("创建目录失败: {e}"))
}
