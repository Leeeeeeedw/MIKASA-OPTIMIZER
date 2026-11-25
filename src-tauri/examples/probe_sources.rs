use mikasa_optimizer_lib::speed_source;
use std::time::Instant;

#[tokio::main]
async fn main() {
    println!("=== 1. 运营商探测 (cf.json) ===");
    let isp = speed_source::detect_isp().await;
    println!(
        "probe_ok={} asn={:?} org={} isp={} is_china_mobile={} error={}",
        isp.probe_ok, isp.asn, isp.org, isp.isp, isp.is_china_mobile, isp.error
    );

    println!("\n=== 2. 各测速源预检 ===");
    for key in [
        speed_source::SOURCE_CLOUDFLARE,
        speed_source::SOURCE_CM,
        speed_source::SOURCE_MOBILE,
    ] {
        let Some(url) = speed_source::url_for(key, "") else {
            continue;
        };
        let started = Instant::now();
        let probe = speed_source::probe_url(key, &url).await;
        println!(
            "{} | url={} | ok={} status={} bytes={} elapsed={}ms dns={} tls={} redirected={} conn={} err={}",
            probe.label,
            probe.url,
            probe.ok,
            probe.status,
            probe.first_chunk_bytes,
            probe.elapsed_ms,
            probe.dns_ok,
            probe.tls_ok,
            probe.redirected,
            probe.connected_ip,
            probe.error
        );
        let _ = started;
    }

    println!("\n=== 3. Auto 解析结果 ===");
    let (key, url, probes) = speed_source::resolve_auto("").await;
    println!("auto -> {} | {}", speed_source::label_for(&key), url);
    for p in &probes {
        println!("  probe {} ok={} err={}", p.label, p.ok, p.error);
    }

    println!("\n=== 4. 同一候选 IP 五源对比 ===");
    let target: std::net::IpAddr = "104.16.123.45".parse().expect("valid ip");
    let compare = speed_source::compare_for_ip(target, 443, "", 5, 262_144).await;
    println!("ip={} port={} recommended={}", compare.ip, compare.port, compare.recommended);
    println!("note={}", compare.note);
    for row in &compare.rows {
        println!(
            "{} | {} | status={} bytes={} {:.1}s {:.2}MB/s {:.1}Mbps ok={} conn={} err={}",
            row.label,
            row.url,
            row.status,
            row.bytes,
            row.seconds,
            row.mb_s,
            row.mbps,
            row.ok,
            row.connected_ip,
            row.error
        );
    }

    println!("\n=== 5. 备用源解析 ===");
    if let Some((bk, bu)) = speed_source::resolve_backup("cloudflare", "cm", "").await {
        println!("backup for cloudflare -> {} | {}", speed_source::label_for(&bk), bu);
    } else {
        println!("backup unavailable");
    }
}
