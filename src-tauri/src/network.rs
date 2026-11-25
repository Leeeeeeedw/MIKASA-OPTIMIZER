use crate::db::now_text;
use crate::models::NetworkInfo;
use serde_json::Value;
use std::process::Command;
use std::time::Duration;

const PROVINCES: &[(&str, &str)] = &[
    ("北京", "北京市"), ("上海", "上海市"), ("天津", "天津市"), ("重庆", "重庆市"),
    ("河北", "河北省"), ("山西", "山西省"), ("辽宁", "辽宁省"), ("吉林", "吉林省"),
    ("黑龙江", "黑龙江省"), ("江苏", "江苏省"), ("浙江", "浙江省"), ("安徽", "安徽省"),
    ("福建", "福建省"), ("江西", "江西省"), ("山东", "山东省"), ("河南", "河南省"),
    ("湖北", "湖北省"), ("湖南", "湖南省"), ("广东", "广东省"), ("海南", "海南省"),
    ("四川", "四川省"), ("贵州", "贵州省"), ("云南", "云南省"), ("陕西", "陕西省"),
    ("甘肃", "甘肃省"), ("青海", "青海省"), ("台湾", "台湾省"), ("内蒙古", "内蒙古自治区"),
    ("广西", "广西壮族自治区"), ("西藏", "西藏自治区"), ("宁夏", "宁夏回族自治区"),
    ("新疆", "新疆维吾尔自治区"), ("香港", "香港特别行政区"), ("澳门", "澳门特别行政区"),
];

const CARRIERS: &[(&str, &str)] = &[
    ("电信", "中国电信"), ("联通", "中国联通"), ("移动", "中国移动"),
    ("广电", "中国广电"), ("铁通", "中国铁通"), ("教育网", "中国教育网"),
    ("China Telecom", "中国电信"), ("China Unicom", "中国联通"),
    ("China Mobile", "中国移动"), ("Unicom", "中国联通"), ("Telecom", "中国电信"),
    ("Mobile", "中国移动"),
];

pub fn normalize_province(text: &str) -> String {
    for (needle, full) in PROVINCES {
        if text.contains(needle) {
            return (*full).to_string();
        }
    }
    String::new()
}

pub fn normalize_carrier(text: &str) -> String {
    for (needle, full) in CARRIERS {
        if text.contains(needle) {
            return (*full).to_string();
        }
    }
    String::new()
}

fn proxy_hint() -> String {
    let env_proxy = ["HTTPS_PROXY", "HTTP_PROXY", "ALL_PROXY"]
        .iter()
        .any(|k| std::env::var(k).map(|v| !v.trim().is_empty()).unwrap_or(false));
    if env_proxy {
        return "检测到代理环境变量".to_string();
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        if let Ok(out) = Command::new("reg")
            .args(["query", r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings", "/v", "ProxyEnable"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
        {
            let text = String::from_utf8_lossy(&out.stdout);
            if text.contains("0x1") {
                return "检测到 Windows 系统代理".to_string();
            }
        }
        if let Ok(out) = Command::new("ipconfig")
            .args(["/all"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
        {
            let text = String::from_utf8_lossy(&out.stdout).to_lowercase();
            if ["tap-windows", "wintun", "wireguard", "clash", "v2ray", "tun", "vethernet"]
                .iter()
                .any(|n| text.contains(n))
            {
                return "检测到 TUN/虚拟网卡（可能影响测速）".to_string();
            }
        }
    }
    "未检测到代理".to_string()
}

async fn from_ipip(client: &reqwest::Client) -> Option<NetworkInfo> {
    let resp = client.get("https://myip.ipip.net").send().await.ok()?;
    let text = resp.text().await.ok()?;
    if !text.contains("来自于") {
        return None;
    }
    let tail = text.split("来自于").nth(1).unwrap_or("").trim();
    let province = normalize_province(tail);
    let carrier = normalize_carrier(tail);
    if province.is_empty() && carrier.is_empty() {
        return None;
    }
    let province = if province.is_empty() { "网络环境未知".to_string() } else { province };
    let carrier = if carrier.is_empty() { "运营商未知".to_string() } else { carrier };
    Some(NetworkInfo {
        display: format!("{} · {}", province, carrier),
        province,
        carrier,
        proxy_hint: proxy_hint(),
        detected_at: now_text(),
        source: "ipip.net".to_string(),
        ok: true,
    })
}

async fn from_ip_sb(client: &reqwest::Client) -> Option<NetworkInfo> {
    let resp = client.get("https://api.ip.sb/geoip").send().await.ok()?;
    let value: Value = resp.json().await.ok()?;
    let region = value.get("region").and_then(|v| v.as_str()).unwrap_or("");
    let isp = value.get("isp").and_then(|v| v.as_str()).unwrap_or("");
    let province = normalize_province(region);
    let carrier = normalize_carrier(isp);
    if province.is_empty() && carrier.is_empty() {
        return None;
    }
    let province = if province.is_empty() { "网络环境未知".to_string() } else { province };
    let carrier = if carrier.is_empty() { "运营商未知".to_string() } else { carrier };
    Some(NetworkInfo {
        display: format!("{} · {}", province, carrier),
        province,
        carrier,
        proxy_hint: proxy_hint(),
        detected_at: now_text(),
        source: "ip.sb".to_string(),
        ok: true,
    })
}

async fn from_ipinfo(client: &reqwest::Client) -> Option<NetworkInfo> {
    let resp = client.get("https://ipinfo.io/json").send().await.ok()?;
    let value: Value = resp.json().await.ok()?;
    let region = value.get("region").and_then(|v| v.as_str()).unwrap_or("");
    let org = value.get("org").and_then(|v| v.as_str()).unwrap_or("");
    let province = normalize_province(region);
    let carrier = normalize_carrier(org);
    if province.is_empty() && carrier.is_empty() {
        return None;
    }
    let province = if province.is_empty() { "网络环境未知".to_string() } else { province };
    let carrier = if carrier.is_empty() { "运营商未知".to_string() } else { carrier };
    Some(NetworkInfo {
        display: format!("{} · {}", province, carrier),
        province,
        carrier,
        proxy_hint: proxy_hint(),
        detected_at: now_text(),
        source: "ipinfo.io".to_string(),
        ok: true,
    })
}

pub async fn detect_network() -> NetworkInfo {
    let client = match reqwest::Client::builder()
        .use_rustls_tls()
        .timeout(Duration::from_secs(12))
        .connect_timeout(Duration::from_secs(6))
        .build()
    {
        Ok(c) => c,
        Err(_) => {
            return NetworkInfo {
                display: "网络环境未知".to_string(),
                province: "网络环境未知".to_string(),
                carrier: "运营商未知".to_string(),
                proxy_hint: proxy_hint(),
                detected_at: now_text(),
                source: "none".to_string(),
                ok: false,
            }
        }
    };
    if let Some(info) = from_ipip(&client).await {
        return info;
    }
    if let Some(info) = from_ip_sb(&client).await {
        return info;
    }
    if let Some(info) = from_ipinfo(&client).await {
        return info;
    }
    NetworkInfo {
        display: "网络环境未知".to_string(),
        province: "网络环境未知".to_string(),
        carrier: "运营商未知".to_string(),
        proxy_hint: proxy_hint(),
        detected_at: now_text(),
        source: "none".to_string(),
        ok: false,
    }
}
