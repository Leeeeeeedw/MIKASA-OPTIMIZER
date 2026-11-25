const SERVICE: &str = "com.mikasa.optimizer";

fn entry(account: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, account).map_err(|e| format!("访问 Windows 凭据存储失败: {e}"))
}

pub fn save_secret(account: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return delete_secret(account);
    }
    entry(account)?
        .set_password(value)
        .map_err(|e| format!("写入 Windows 凭据存储失败: {e}"))
}

pub fn get_secret(account: &str) -> Result<Option<String>, String> {
    match entry(account)?.get_password() {
        Ok(v) => Ok(Some(v)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("读取 Windows 凭据存储失败: {e}")),
    }
}

pub fn delete_secret(account: &str) -> Result<(), String> {
    match entry(account)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("删除 Windows 凭据失败: {e}")),
    }
}
