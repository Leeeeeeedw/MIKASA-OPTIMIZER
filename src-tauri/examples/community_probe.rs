use std::time::Instant;

#[tokio::main]
async fn main() {
    for key in mikasa_optimizer_lib::sources::COMMUNITY_SOURCE_KEYS {
        let started = Instant::now();
        match mikasa_optimizer_lib::sources::community_candidates(key, 5000).await {
            Ok((seeds, files_ok, errors)) => {
                let unique_ipv4 = seeds.iter().filter(|s| s.ip.contains('.')).count();
                let unique_ipv6 = seeds.len().saturating_sub(unique_ipv4);
                let sample = seeds
                    .iter()
                    .take(3)
                    .map(|s| format!("{}:{}{}", s.ip, s.port, if s.region.is_empty() { String::new() } else { format!("#{}", s.region) }))
                    .collect::<Vec<_>>()
                    .join(", ");
                println!(
                    "{key}: seeds={} ipv4={} ipv6={} files_ok={} errors={} elapsed={}ms sample=[{}]",
                    seeds.len(),
                    unique_ipv4,
                    unique_ipv6,
                    files_ok,
                    errors.len(),
                    started.elapsed().as_millis(),
                    sample
                );
                for error in errors {
                    println!("  error: {error}");
                }
            }
            Err(error) => println!("{key}: ERROR {error} elapsed={}ms", started.elapsed().as_millis()),
        }
    }
}
