use crate::export::ExportFile;
use crate::models::GithubUploadOutcome;
use base64::Engine;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::Duration;

const API: &str = "https://api.github.com";

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .use_rustls_tls()
        .user_agent("MikasaOptimizer/1.0")
        .timeout(Duration::from_secs(60))
        .connect_timeout(Duration::from_secs(12))
        .build()
        .map_err(|e| format!("创建 GitHub 客户端失败: {e}"))
}

fn resolve_repo(input: &str) -> Result<(String, String), String> {
    let mut value = input.trim().trim_end_matches('/').to_string();
    if value.is_empty() {
        return Err("请填写 GitHub 仓库地址".to_string());
    }
    value = value
        .trim_start_matches("https://github.com/")
        .trim_start_matches("http://github.com/")
        .trim_start_matches("github.com/")
        .trim_end_matches(".git")
        .to_string();
    if let Some(rest) = value.strip_prefix("git@github.com:") {
        value = rest.trim_end_matches(".git").to_string();
    }
    let parts: Vec<&str> = value.split('/').filter(|s| !s.is_empty()).collect();
    if parts.len() < 2 {
        return Err("仓库地址格式应为 https://github.com/用户名/仓库名".to_string());
    }
    let owner = parts[0].to_string();
    let repo = parts[1].to_string();
    if !owner.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        || !repo.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    {
        return Err("仓库地址包含不支持的字符".to_string());
    }
    Ok((owner, repo))
}

fn auth_header(token: &str) -> Result<reqwest::header::HeaderValue, String> {
    let mut value = reqwest::header::HeaderValue::from_str(&format!("Bearer {}", token.trim()))
        .map_err(|_| "GitHub Token 包含无效字符".to_string())?;
    value.set_sensitive(true);
    Ok(value)
}

fn remote_path(remote_dir: &str, name: &str) -> String {
    if remote_dir.trim().is_empty() {
        name.to_string()
    } else {
        format!("{}/{}", remote_dir.trim().trim_matches('/'), name)
    }
}

fn contents_url(owner: &str, repo: &str, path: &str) -> Result<String, String> {
    let mut url = url::Url::parse(&format!("{API}/repos/{owner}/{repo}/contents/"))
        .map_err(|e| format!("创建 GitHub Contents URL 失败: {e}"))?;
    {
        let mut segments = url
            .path_segments_mut()
            .map_err(|_| "无法构造 GitHub Contents URL".to_string())?;
        segments.pop_if_empty();
        for segment in path.split('/') {
            segments.push(segment);
        }
    }
    Ok(url.to_string())
}

async fn error_text(resp: reqwest::Response, action: &str) -> String {
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    let detail = serde_json::from_str::<Value>(&text)
        .ok()
        .and_then(|v| v.get("message").and_then(|m| m.as_str()).map(|s| s.to_string()))
        .unwrap_or_else(|| text.chars().take(280).collect());
    if status.as_u16() == 401 {
        format!("{action}失败：GitHub Token 无效或已过期（401）")
    } else if status.as_u16() == 403 {
        format!("{action}失败：Token 权限不足或 API 频率受限（403）{detail}")
    } else if status.as_u16() == 404 {
        format!("{action}失败：仓库或分支不存在，或 Token 无权访问（404）")
    } else {
        format!("{action}失败：HTTP {status} {detail}")
    }
}

async fn get_json(client: &reqwest::Client, token: &str, url: &str) -> Result<(u16, Value), String> {
    let resp = client
        .get(url)
        .header(reqwest::header::AUTHORIZATION, auth_header(token)?)
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .await
        .map_err(|e| format!("连接 GitHub 失败: {e}"))?;
    let status = resp.status().as_u16();
    let value: Value = resp.json().await.map_err(|e| format!("解析 GitHub 响应失败: {e}"))?;
    Ok((status, value))
}

async fn read_remote_file(
    client: &reqwest::Client,
    token: &str,
    owner: &str,
    repo: &str,
    path: &str,
    branch: &str,
) -> Result<Option<String>, String> {
    let mut url = url::Url::parse(&contents_url(owner, repo, path)?)
        .map_err(|e| format!("创建 GitHub Contents URL 失败: {e}"))?;
    url.query_pairs_mut().append_pair("ref", branch);
    let (status, body) = get_json(client, token, url.as_str()).await?;
    if status == 404 {
        return Ok(None);
    }
    if status != 200 {
        return Err(format!("读取远端文件 {path} 失败：HTTP {status}"));
    }
    if body.get("encoding").and_then(|value| value.as_str()) == Some("none") {
        return Err(format!("远端文件 {path} 超过 1 MB，无法使用追加更新"));
    }
    let encoded = body
        .get("content")
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .chars()
        .filter(|ch| *ch != '\n' && *ch != '\r')
        .collect::<String>();
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|e| format!("解析远端文件 {path} 失败: {e}"))?;
    let text = String::from_utf8(bytes)
        .map_err(|e| format!("远端文件 {path} 不是 UTF-8 文本: {e}"))?;
    Ok(Some(text))
}

fn subscription_key(line: &str, csv: bool) -> Option<String> {
    if csv {
        let mut parts = line.splitn(3, ',');
        let ip = parts.next()?.trim().trim_matches('"').trim();
        let port = parts.next()?.trim().trim_matches('"').trim();
        if ip.is_empty() || port.is_empty() {
            return None;
        }
        Some(format!("{ip}:{port}"))
    } else {
        let key = line.split('#').next().unwrap_or("").trim();
        if key.is_empty() {
            None
        } else {
            Some(key.to_string())
        }
    }
}

/// 追加更新：保留远端已有节点，本次同 IP:端口 的记录覆盖为最新测速数据，
/// 新节点追加到末尾。TXT 与 CSV 都按第一列 IP + 端口去重。
fn merge_subscription(existing: &str, incoming: &str) -> String {
    let incoming_clean = incoming.trim_start_matches('\u{feff}');
    let csv_header = incoming_clean
        .lines()
        .next()
        .filter(|line| line.trim_start().starts_with("IP,"));
    let csv = csv_header.is_some();
    let mut order: Vec<String> = Vec::new();
    let mut lines: HashMap<String, String> = HashMap::new();

    let mut collect = |text: &str| {
        for raw in text.lines() {
            let line = raw
                .trim_start_matches('\u{feff}')
                .trim_end_matches('\r');
            if line.trim().is_empty() {
                continue;
            }
            if csv && line.trim_start().starts_with("IP,") {
                continue;
            }
            if let Some(key) = subscription_key(line, csv) {
                if !lines.contains_key(&key) {
                    order.push(key.clone());
                }
                lines.insert(key, line.to_string());
            }
        }
    };
    collect(existing);
    collect(incoming_clean);

    let mut merged = String::new();
    if let Some(header) = csv_header {
        merged.push_str(header.trim_end_matches('\r'));
        merged.push('\n');
    }
    for key in order {
        if let Some(line) = lines.get(&key) {
            merged.push_str(line);
            merged.push('\n');
        }
    }
    merged
}

pub async fn check_repository(repo_input: &str, branch_input: &str, token: &str) -> Result<Value, String> {
    if token.trim().is_empty() {
        return Err("请先保存 GitHub Token".to_string());
    }
    let (owner, repo) = resolve_repo(repo_input)?;
    let client = client()?;
    let (status, body) = get_json(&client, token, &format!("{API}/repos/{owner}/{repo}")).await?;
    if status == 404 {
        return Err("仓库不存在，或当前 Token 无权访问（404）".to_string());
    }
    if status == 401 || status == 403 {
        return Err(format!("GitHub Token 无效或权限不足（HTTP {status}）"));
    }
    if status != 200 {
        return Err(format!("检查仓库失败：HTTP {status}"));
    }
    let default_branch = body.get("default_branch").and_then(|v| v.as_str()).unwrap_or("main").to_string();
    let can_push = body
        .get("permissions")
        .and_then(|p| p.get("push"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let branch = if branch_input.trim().is_empty() { default_branch.clone() } else { branch_input.trim().to_string() };
    let (branch_status, _) = get_json(&client, token, &format!("{API}/repos/{owner}/{repo}/branches/{branch}")).await?;
    Ok(json!({
        "repo": format!("{owner}/{repo}"),
        "defaultBranch": default_branch,
        "branch": branch,
        "branchExists": branch_status == 200,
        "canPush": can_push,
        "private": body.get("private").and_then(|v| v.as_bool()).unwrap_or(false),
        "permissions": body.get("permissions").cloned().unwrap_or(Value::Null),
    }))
}

pub async fn upload_results(
    repo_input: &str,
    branch_input: &str,
    remote_dir: &str,
    token: &str,
    mut files: Vec<ExportFile>,
    scan_id: &str,
    merge_existing: bool,
) -> Result<GithubUploadOutcome, String> {
    if token.trim().is_empty() {
        return Err("请先保存 GitHub Token".to_string());
    }
    if files.is_empty() {
        return Err("没有可上传的导出文件".to_string());
    }
    let (owner, repo) = resolve_repo(repo_input)?;
    let client = client()?;
    let auth = auth_header(token)?;
    let repo_url = format!("{API}/repos/{owner}/{repo}");
    let (status, repo_json) = get_json(&client, token, &repo_url).await?;
    if status != 200 {
        return Err(match status {
            401 => "GitHub Token 无效或已过期".to_string(),
            404 => "仓库不存在，或 Token 无权访问该仓库".to_string(),
            _ => format!("读取仓库失败：HTTP {status} {}", repo_json.get("message").and_then(|v| v.as_str()).unwrap_or("")),
        });
    }
    let default_branch = repo_json
        .get("default_branch")
        .and_then(|v| v.as_str())
        .unwrap_or("main")
        .to_string();
    let branch = if branch_input.trim().is_empty() {
        default_branch.clone()
    } else {
        branch_input.trim().to_string()
    };
    let ref_url = format!("{API}/repos/{owner}/{repo}/git/ref/heads/{branch}");
    let (mut ref_status, mut ref_json) = get_json(&client, token, &ref_url).await?;
    let mut created_branch = false;
    // GitHub 对空仓库返回 409 Git Repository is empty；此时没有父提交，
    // 需要创建首个 commit 后再建立 refs/heads/{branch}。
    let mut empty_repo = ref_status == 409;
    if ref_status == 404 {
        let default_ref_url = format!("{API}/repos/{owner}/{repo}/git/ref/heads/{default_branch}");
        let (ds, dj) = get_json(&client, token, &default_ref_url).await?;
        if ds == 409 {
            empty_repo = true;
        } else if ds != 200 {
            return Err(format!("读取仓库默认分支 {default_branch} 失败：HTTP {ds}"));
        } else {
            let base_sha = dj
                .pointer("/object/sha")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "默认分支缺少 commit SHA".to_string())?;
            let create = client
                .post(format!("{API}/repos/{owner}/{repo}/git/refs"))
                .header(reqwest::header::AUTHORIZATION, auth.clone())
                .header(reqwest::header::ACCEPT, "application/vnd.github+json")
                .header("X-GitHub-Api-Version", "2022-11-28")
                .json(&json!({"ref": format!("refs/heads/{branch}"), "sha": base_sha}))
                .send()
                .await
                .map_err(|e| format!("创建分支请求失败: {e}"))?;
            if !create.status().is_success() {
                return Err(error_text(create, "创建分支").await);
            }
            ref_status = 201;
            let (new_status, new_ref) = get_json(&client, token, &ref_url).await?;
            if new_status != 200 && new_status != 201 {
                return Err(format!("读取新建分支 {branch} 失败：HTTP {new_status}"));
            }
            ref_json = new_ref;
            created_branch = true;
        }
    }
    let mut head_sha = if empty_repo {
        None
    } else {
        if ref_status != 200 && ref_status != 201 {
            return Err(format!("读取分支 {branch} 失败：HTTP {ref_status}"));
        }
        Some(
            ref_json
                .pointer("/object/sha")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "分支缺少 commit SHA".to_string())?
                .to_string(),
        )
    };

    if merge_existing {
        for file in files.iter_mut() {
            let path = remote_path(remote_dir, &file.name);
            if let Some(existing) = read_remote_file(&client, token, &owner, &repo, &path, &branch).await? {
                file.content = merge_subscription(&existing, &file.content);
            }
        }
    }

    let mut uploaded_names = Vec::new();

    // GitHub 的 Git Data API 在完全空的仓库上会统一返回 409。
    // 因此空仓库先用 Contents API 写入第一个文件，创建初始 commit 和默认分支，
    // 再对其余文件使用 Git Data API 批量提交。
    if empty_repo {
        let first = &files[0];
        let path = remote_path(remote_dir, &first.name);
        let init_url = contents_url(&owner, &repo, &path)?;
        let init_resp = client
            .put(init_url)
            .header(reqwest::header::AUTHORIZATION, auth.clone())
            .header(reqwest::header::ACCEPT, "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .json(&json!({
                "message": format!("三笠优选测速结果 {} · 初始化仓库", scan_id),
                "content": base64::engine::general_purpose::STANDARD.encode(first.content.as_bytes())
            }))
            .send()
            .await
            .map_err(|e| format!("初始化空仓库失败: {e}"))?;
        if !init_resp.status().is_success() {
            return Err(error_text(init_resp, "初始化空仓库").await);
        }
        let init_json: Value = init_resp
            .json()
            .await
            .map_err(|e| format!("解析初始化 commit 响应失败: {e}"))?;
        let first_commit_sha = init_json
            .pointer("/commit/sha")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "GitHub 未返回初始化 commit SHA".to_string())?
            .to_string();
        head_sha = Some(first_commit_sha.clone());
        uploaded_names.push(first.name.clone());
        created_branch = true;

        if branch != default_branch {
            let create = client
                .post(format!("{API}/repos/{owner}/{repo}/git/refs"))
                .header(reqwest::header::AUTHORIZATION, auth.clone())
                .header(reqwest::header::ACCEPT, "application/vnd.github+json")
                .header("X-GitHub-Api-Version", "2022-11-28")
                .json(&json!({"ref": format!("refs/heads/{branch}"), "sha": first_commit_sha}))
                .send()
                .await
                .map_err(|e| format!("创建分支请求失败: {e}"))?;
            if !create.status().is_success() {
                return Err(error_text(create, "创建分支").await);
            }
        }

        if files.len() == 1 {
            let (verify_status, verify_json) = get_json(&client, token, &ref_url).await?;
            let verified = verify_status == 200
                && verify_json
                    .pointer("/object/sha")
                    .and_then(|v| v.as_str())
                    .map(|s| s == first_commit_sha)
                    .unwrap_or(false);
            if !verified {
                return Err("上传后校验失败：远端分支 SHA 与本次提交不一致".to_string());
            }
            let _ = auth;
            return Ok(GithubUploadOutcome {
                repo: format!("{owner}/{repo}"),
                branch: format!("{branch}（新建）"),
                files: uploaded_names,
                commit_sha: first_commit_sha,
                verified,
            });
        }
    }

    let base_tree = if let Some(head) = &head_sha {
        let (commit_status, commit_json) = get_json(
            &client,
            token,
            &format!("{API}/repos/{owner}/{repo}/git/commits/{head}"),
        )
        .await?;
        if commit_status != 200 {
            return Err(format!("读取父提交失败：HTTP {commit_status}"));
        }
        Some(
            commit_json
                .pointer("/tree/sha")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "父提交缺少 tree SHA".to_string())?
                .to_string(),
        )
    } else {
        None
    };

    let mut tree_entries = Vec::new();
    let start_index = if empty_repo { 1 } else { 0 };
    for file in files.iter().skip(start_index) {
        let blob_resp = client
            .post(format!("{API}/repos/{owner}/{repo}/git/blobs"))
            .header(reqwest::header::AUTHORIZATION, auth.clone())
            .header(reqwest::header::ACCEPT, "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .json(&json!({
                "content": base64::engine::general_purpose::STANDARD.encode(file.content.as_bytes()),
                "encoding": "base64"
            }))
            .send()
            .await
            .map_err(|e| format!("上传 {} 失败: {e}", file.name))?;
        if !blob_resp.status().is_success() {
            return Err(error_text(blob_resp, &format!("创建 blob {}", file.name)).await);
        }
        let blob_json: Value = blob_resp.json().await.map_err(|e| format!("解析 blob 响应失败: {e}"))?;
        let sha = blob_json
            .get("sha")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "GitHub 未返回 blob SHA".to_string())?;
        let path = remote_path(remote_dir, &file.name);
        tree_entries.push(json!({
            "path": path,
            "mode": "100644",
            "type": "blob",
            "sha": sha
        }));
        uploaded_names.push(file.name.clone());
    }

    let tree_payload = match &base_tree {
        Some(base) => json!({"base_tree": base, "tree": tree_entries}),
        None => json!({"tree": tree_entries}),
    };
    let tree_resp = client
        .post(format!("{API}/repos/{owner}/{repo}/git/trees"))
        .header(reqwest::header::AUTHORIZATION, auth.clone())
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .json(&tree_payload)
        .send()
        .await
        .map_err(|e| format!("创建 tree 失败: {e}"))?;
    if !tree_resp.status().is_success() {
        return Err(error_text(tree_resp, "创建 tree").await);
    }
    let tree_json: Value = tree_resp.json().await.map_err(|e| format!("解析 tree 响应失败: {e}"))?;
    let tree_sha = tree_json
        .get("sha")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "GitHub 未返回 tree SHA".to_string())?;

    let commit_message = format!("三笠优选测速结果 {} · {} 个文件", scan_id, uploaded_names.len());
    let parent = head_sha
        .as_ref()
        .ok_or_else(|| "缺少父提交，无法创建 commit".to_string())?;
    let commit_payload = json!({"message": commit_message, "tree": tree_sha, "parents": [parent]});
    let commit_resp = client
        .post(format!("{API}/repos/{owner}/{repo}/git/commits"))
        .header(reqwest::header::AUTHORIZATION, auth.clone())
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .json(&commit_payload)
        .send()
        .await
        .map_err(|e| format!("创建 commit 失败: {e}"))?;
    if !commit_resp.status().is_success() {
        return Err(error_text(commit_resp, "创建 commit").await);
    }
    let commit_created: Value = commit_resp.json().await.map_err(|e| format!("解析 commit 响应失败: {e}"))?;
    let new_commit_sha = commit_created
        .get("sha")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "GitHub 未返回 commit SHA".to_string())?
        .to_string();

    let update_resp = client
        .patch(format!("{API}/repos/{owner}/{repo}/git/refs/heads/{branch}"))
        .header(reqwest::header::AUTHORIZATION, auth.clone())
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .json(&json!({"sha": new_commit_sha, "force": false}))
        .send()
        .await
        .map_err(|e| format!("更新分支失败: {e}"))?;
    if !update_resp.status().is_success() {
        return Err(error_text(update_resp, "更新分支").await);
    }
    let (verify_status, verify_json) = get_json(&client, token, &ref_url).await?;
    let verified = verify_status == 200
        && verify_json
            .pointer("/object/sha")
            .and_then(|v| v.as_str())
            .map(|s| s == new_commit_sha)
            .unwrap_or(false);
    if !verified {
        return Err("上传后校验失败：远端分支 SHA 与本次提交不一致".to_string());
    }
    let _ = auth;
    Ok(GithubUploadOutcome {
        repo: format!("{owner}/{repo}"),
        branch: if created_branch { format!("{branch}（新建）") } else { branch },
        files: uploaded_names,
        commit_sha: new_commit_sha,
        verified,
    })
}

#[cfg(test)]
mod tests {
    use super::merge_subscription;

    #[test]
    fn merge_txt_keeps_existing_updates_same_endpoint_and_appends_new() {
        let existing = "\
1.1.1.1:443#三笠优选 | 香港 | 下载 5.00MB/s
2606:4700::1:443#三笠优选 | 日本东京 | 下载 6.00MB/s
";
        let incoming = "\
2606:4700::1:443#三笠优选 | 日本东京 | 下载 9.00MB/s
2.2.2.2:2053#三笠优选 | 美国洛杉矶 | 下载 8.00MB/s
";
        let merged = merge_subscription(existing, incoming);
        assert_eq!(
            merged,
            "\
1.1.1.1:443#三笠优选 | 香港 | 下载 5.00MB/s
2606:4700::1:443#三笠优选 | 日本东京 | 下载 9.00MB/s
2.2.2.2:2053#三笠优选 | 美国洛杉矶 | 下载 8.00MB/s
"
        );
    }

    #[test]
    fn merge_csv_keeps_one_header_and_updates_duplicate_port() {
        let existing = "\
IP,端口,国家/城市,colo,平均延迟(ms),下载速度(MB/s),下载速度(Mbps),丢包率(%),稳定性(%),来源,测试时间
1.1.1.1,443,香港,HKG,30.0,5.00,41.9,0.0,100.0,CM,2026-01-01 00:00:00
";
        let incoming = "\
IP,端口,国家/城市,colo,平均延迟(ms),下载速度(MB/s),下载速度(Mbps),丢包率(%),稳定性(%),来源,测试时间
1.1.1.1,443,香港,HKG,18.0,9.00,75.5,0.0,100.0,CM,2026-02-01 00:00:00
2.2.2.2,2053,日本东京,NRT,45.0,7.00,58.7,0.0,100.0,CM,2026-02-01 00:00:00
";
        let merged = merge_subscription(existing, incoming);
        let lines: Vec<&str> = merged.lines().collect();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].starts_with("IP,端口"));
        assert!(lines[1].contains(",18.0,9.00,"));
        assert!(lines[2].starts_with("2.2.2.2,2053,"));
    }
}
