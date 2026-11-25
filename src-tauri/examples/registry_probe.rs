#[tokio::main]
async fn main() {
    let result = mikasa_optimizer_lib::registry::discover().await;
    println!(
        "sources={} endpoints={} accepted={} rejected={} warnings={}",
        result.total_sources,
        result.total_endpoints,
        result.accepted,
        result.rejected,
        result.warnings.len()
    );
    for source in &result.sources {
        println!("source: {} | {} | {} | {}", source.key, source.repo, source.license, source.kind);
    }
    for endpoint in result.endpoints.iter().take(40) {
        println!(
            "endpoint: {} | {} | allowed={} | {}",
            endpoint.kind, endpoint.url, endpoint.allowed, endpoint.reason
        );
    }
    for warning in &result.warnings {
        println!("warning: {warning}");
    }
}
