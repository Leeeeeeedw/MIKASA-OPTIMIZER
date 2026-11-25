use aes_gcm::{
    aead::{Aead, Payload},
    Aes256Gcm, KeyInit,
};
use base64::Engine;

const WECHAT: &[u8] = include_bytes!("../resources/donation/wechat.bin");
const ALIPAY: &[u8] = include_bytes!("../resources/donation/alipay.bin");
const KEY_B64: Option<&str> = option_env!("MIKASA_DONATE_KEY");

fn decrypt_png(kind: &str, payload: &[u8]) -> Result<String, String> {
    let key_text = KEY_B64.ok_or_else(|| {
        "源码公开版未附带收款码，请使用作者发布的正式安装包".to_string()
    })?;
    let key = base64::engine::general_purpose::STANDARD
        .decode(key_text.trim())
        .map_err(|_| "打赏码密钥格式无效".to_string())?;
    if key.len() != 32 || payload.len() < 5 + 12 + 16 || &payload[..5] != b"MKSQ1" {
        return Err("打赏码数据损坏或版本不兼容".to_string());
    }
    let cipher =
        Aes256Gcm::new_from_slice(&key).map_err(|_| "打赏码密钥长度无效".to_string())?;
    let nonce = &payload[5..17];
    let plain = cipher
        .decrypt(
            nonce.into(),
            Payload {
                msg: &payload[17..],
                aad: format!("mikasa-donation:{kind}").as_bytes(),
            },
        )
        .map_err(|_| "打赏码完整性校验失败，文件可能已被篡改".to_string())?;
    if !plain.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err("打赏码解密结果不是有效图片".to_string());
    }
    Ok(format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(plain)
    ))
}

pub fn qr_data_url(kind: &str) -> Result<String, String> {
    match kind {
        "wechat" => decrypt_png("wechat", WECHAT),
        "alipay" => decrypt_png("alipay", ALIPAY),
        _ => Err("未知的打赏码类型".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::qr_data_url;

    #[test]
    fn rejects_unknown_donation_kind() {
        assert!(qr_data_url("other").is_err());
    }
}
