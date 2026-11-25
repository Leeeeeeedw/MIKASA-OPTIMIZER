# 多测速源系统（参考 CFData-WEB）

本文件记录三笠优选 v1.0.0 多测速源系统的源码依据、实现方式与本机真实测试结果。

## 一、参考项目的真实源码依据

参考项目：<https://github.com/PoemMisty/CFData-WEB>
核对文件（当前主分支）：`combined_refactor/speed_url.go`

```go
const autoSpeedURLValue = "auto"
const (
	cmSpeedURL              = "cf.090227.xyz/__down?bytes=99999999"
	mobileDedicatedSpeedURL = "speed.okl.abrdns.com"
	cloudflareSpeedURL      = "speed.cloudflare.com/__down?bytes=99999999"
	ispProbeURL             = "https://cf.090227.xyz/cf.json"
)
```

关键逻辑（同名函数在 CFData-WEB 中可直接对照）：

| 函数 | 作用 |
| --- | --- |
| `resolveStartupSpeedTestURL` | Auto 入口：先探运营商，再选源 |
| `detectSpeedTestISP` | `GET https://cf.090227.xyz/cf.json`，取 `asn` / `asOrganization`，5s 超时 |
| `isChinaMobileISP` | 关键词 `cmi,cmnet,chinamobile,china mobile,cmcc,mobile communications,移动`，或 ASN ∈ {9808, 24400, 56040, 56041, 56044} |
| `checkSpeedURLAvailability` | 把 `bytes=` 数字替换为 `1024` 后 GET，要求 2xx 且至少读到 1 字节 |
| `speedURLLabel` | `Cloudflare` / `CM提供` / `移动专属` / `自定义` |

CFData-WEB 的 Auto 策略：

- 中国移动：在 `cmSpeedURL` 与 `mobileDedicatedSpeedURL` 中挑可用者，都不可用才退 Cloudflare。
- 非移动网络：优先 Cloudflare，Cloudflare 不可用才依次退 CM、移动专属。

**CM 提供 ≠ 中国移动**：`cf.090227.xyz` 是一个第三方反代域名，与移动专属域名 `speed.okl.abrdns.com` 是两个独立站点。

## 二、三笠优选的实现

新增文件 `src-tauri/src/speed_source.rs`：

- 常量与 CFData-WEB 完全对齐（`CLOUDFLARE_SPEED_URL`、`CM_SPEED_URL`、`MOBILE_SPEED_URL`、`ISP_PROBE_URL`）。
- `probe_url(key, url)`：DNS 解析 → HTTPS 连接 → TLS 握手 → HTTP 状态 → 读取首块数据（8s 上限），逐项记录 `dns_ok`/`tcp_ok`/`tls_ok`/`status`/`first_chunk_bytes`/`elapsed_ms`/`redirected`/`final_url`/`resolved_ip`/`connected_ip`/`error`。
- `detect_isp()`：先请求 `cf.json` 取 ASN/组织；失败或未返回 ASN 时回退到本机运营商检测（与顶栏「省份 · 运营商」同源），并记录原因。
- `resolve_auto(custom)`：按 CFData-WEB 的策略选源，返回源键、URL 与全部预检结果。
- `resolve_backup(primary, preferred, custom)`：主源失败时的备用源。
- `compare_for_ip(ip, port, custom, duration, min_bytes)`：对**同一个目标 IP + 同一端口**依次用各源做真实下载，记录 `connected_ip`、状态码、字节数、耗时、MB/s、Mbps、失败原因，并给出推荐源。

下载绑定保证（`src-tauri/src/speedtest.rs`）：

- `pinned_client` 使用 `.resolve(host, SocketAddr::new(ip, port))`，强制把测速域名解析到**待测 IP**；
- TLS SNI 与 HTTP `Host` 均使用测速域名；
- `DownloadProbe` 新增 `connected_ip`，直接取 `resp.remote_addr()`，作为「确实连到了目标 IP」的证据。

设置项（`AppSettings`）：`speedSource`、`backupSpeedSource`、`autoSwitchSpeedSource`、`customSpeedUrl`，向下兼容旧配置文件（`#[serde(default)]`）。

界面：

- 首页「优选参数」新增「测速地址」下拉与当前生效源徽标（不改变原有地区选择、开始优选、实时日志布局）。
- 设置页新增「测速源」卡片：主源、备用源、手动地址、自动切换开关、**测速源预检**、**测速源对比**（同一 IP 逐源实测）。

## 三、本机真实测试结果（河南省 · 中国联通）

测试时间：2026-11-24；测试方式：`cargo run --example probe_sources`；原始日志见 `docs/speed-source-validation.log`。

### 3.1 运营商识别

```
probe_ok=true asn=0 org=中国联通 isp=中国联通 is_china_mobile=false
error=cf.json 未返回 ASN，已改用本机运营商检测
```

`cf.090227.xyz/cf.json` 在本机网络下未返回可解析的 ASN，已自动回退到本机运营商检测，结果「中国联通」，判定非移动网络。

### 3.2 各测速源预检

| 源 | 真实 URL | 结果 | 首块 | 耗时 |
| --- | --- | --- | --- | --- |
| Cloudflare 官方 | `https://speed.cloudflare.com/__down?bytes=99999999` | 200 可用 | 1024 B | 617 ms |
| CM 提供 | `https://cf.090227.xyz/__down?bytes=99999999` | 403 不可用 | 0 B | 4217 ms |
| 移动专属 | `https://speed.okl.abrdns.com` | 200 可用（未绑定 IP） | 8192 B | 762 ms |

### 3.3 Auto 结果

```
auto -> Cloudflare | https://speed.cloudflare.com/__down?bytes=99999999
  probe Cloudflare ok=true
  probe CM 提供 ok=false err=HTTP 403
  probe 移动专属 ok=true
```

联通网络下 Auto 选择 Cloudflare，与「非移动优先 Cloudflare」策略一致。

### 3.4 同一候选 IP 五源对比（目标 `104.16.123.45:443`）

| 源 | HTTP | 下载字节 | 耗时 | 速度 | 实际连接 IP | 结论 |
| --- | --- | --- | --- | --- | --- | --- |
| Cloudflare 官方 | 200 | 99,999,999 | 3.2 s | **31.03 MB/s（248.3 Mbps）** | `104.16.123.45` | 成功 |
| CM 提供 | 403 | 0 | 0.0 s | — | `104.16.123.45` | 失败：`下载接口返回 HTTP 403` |
| 移动专属 | 200 | 46,669 | 5.0 s | 0.01 MB/s | `104.16.123.45` | 失败：有效数据不足（46,669 B < 262,144 B，声明长度 100,000,000） |

推荐源：`cloudflare`。

### 3.5 结论

在**中国联通**网络下：

1. **Cloudflare 官方源是唯一稳定可用的下载测速源**，能跑满带宽（两次实测 31.03 MB/s、23.97 MB/s），且 `connected_ip` 严格等于被测候选 IP。
2. **CM 提供源（`cf.090227.xyz`）不可用于绑定任意候选 IP**：把该域名解析到非其自身回源的 Cloudflare IP 时，边缘直接返回 403；它只在本机 DNS 解析到的自有回源 IP 上可用。
3. **移动专属源（`speed.okl.abrdns.com`）在联通网络下带宽极低且握手不稳定**：未绑定 IP 时可读到 8 KB 首块，绑定候选 IP 后 5 秒仅 46 KB～236 KB，达不到最低数据量；单次还出现连接直接失败。
4. 因此**不能把移动网络的默认策略套用到联通网络**。三笠优选在联通下应默认 Cloudflare 官方源，备用源使用同域较小 `bytes=` 的地址（`backup_download_url`），而不是切到 CM 或移动专属。

Auto 与备用源逻辑已按此修正：当备用源与主源指向同一地址时，自动退回 `backup_download_url`（较小分片），避免「备用等于主源」导致的重试空转。

## 四、失败切换测试

`resolve_backup("cloudflare", "cm", "")` 在本机返回：

```
backup for cloudflare -> 移动专属 | https://speed.okl.abrdns.com
```

即：CM 预检 403 → 自动跳到移动专属（预检通过）。实际下载阶段若备用源仍失败，日志会写出每个候选的具体失败原因（HTTP 状态、有效数据不足的字节数、连接错误），不会静默丢弃。

## 五、已知限制

- 移动专属源在联通网络下不可用属于**网络路径问题**，不是代码缺陷；软件如实显示失败原因，不伪造速度。
- `cf.json` 在本机网络下未返回 ASN，运营商判定依赖本机运营商检测结果，可能与真实 ASN 存在差异（当前一致：中国联通）。
- 测速源只负责提供下载数据，**不参与 colo 判定**；地区仍然只用 Cloudflare `trace` / `CF-RAY` 的实际 colo。
