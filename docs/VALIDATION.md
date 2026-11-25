# 三笠优选 1.0.4 本地验收记录（含 1.0.0 / 1.0.2 / 1.0.3 历史）

验收时间：2026-11-23（Asia/Shanghai）；最近复测：2026-11-25（Asia/Shanghai）
验收环境：Windows 10 x64 / 河南联通 / release 构建

## 0. 1.0.4 发布验收（2026-11-25）

| 项目 | 结果 |
|---|---|
| `npm run build` | 通过，5436 modules |
| `cargo test --lib` | 11 passed / 0 failed |
| `npx tauri build --bundles nsis` | 成功，release 编译 2m 16s |
| 安装包 | `release\三笠优选_1.0.4_x64-setup.exe` |
| 布局验证 | 1180×720 / 1280×800 / 1440×900 / 1920×1080 九页共 45 张截图，整页横向溢出为 0 |
| 浅色主题 | 九页逐页检查；关于页 hero 插画增加浅色柔光，来源/排名/历史/GitHub/设置/打赏弹窗均为浅底深字 |
| 任务栏图标 | EXE、NSIS 安装包、标题栏、任务栏图标一致，均使用自由之翼风格徽章 |
| GitHub 空仓库上传 | 私有仓库 `guyue1337/MikasaIP` 首次上传成功；commit `9b947bfbb637dcfddfe075360f42c3dec11ba35f`，远端 SHA 校验通过 |

SHA256 以 `release\发布说明.txt` 为准（每次重新打包后自动重算）。

1.0.4 关键变更：

- 节点命名升级为城市级：香港、台湾、新加坡只显示自身；日本使用东京/大阪/福冈等真实 colo 城市，美国使用洛杉矶/圣何塞/西雅图等真实 colo 城市。
- 默认导出去除延迟字段，格式为 `IP:端口#三笠优选 | 地区城市 | 下载 x.xxMB/s`；备注字段可另行勾选速度、延迟、colo、来源。
- 导出新增最低下载速度、最高 TCP 延迟筛选，并提供真实结果预览与命中数量。
- IP 来源页按九页设计稿第 2 页重做科技化来源矩阵、一键刷新和候选池概览，保留全部来源配置能力。
- 关于页按第 9 页方向重做夜空、樱花与童年三笠立绘，并补齐浅色主题适配。
- 新增左侧底部「打赏作者」入口，支持微信/支付宝收款码、Esc 和点击遮罩关闭。
- GitHub 空仓库首次上传修复：先通过 Contents API 写入初始文件并创建默认分支，再使用 Git Data API 批量提交剩余文件。
- GitHub 仓库、分支、远程目录持久化；已保存 Token 时重启后无需重复输入。
- 窗口拉伸改为最小列宽与内部滚动策略，九页在四种分辨率下无横向溢出。

## 0. 1.0.3 发布验收（2026-11-25）

| 项目 | 结果 |
|---|---|
| `npm run build` | 通过，5430 modules |
| `cargo test --lib` | 8 passed / 0 failed |
| `npx tauri build --bundles nsis` | 成功，release 编译 2m 10s |
| 安装包 | `release\三笠优选_1.0.3_x64-setup.exe`（5,396,197 bytes） |
| 便携版 | `release\三笠优选_1.0.3_x64-portable.zip`（6,110,381 bytes） |
| 源码包 | `release\三笠优选_1.0.3_source.zip`（7,460,935 bytes） |
| 布局验证 | 1180×720 / 1440×900 / 1920×1080 九页共 36 张截图，整页横向溢出为 0 |
| 浅色主题 | 九页逐页检查，表头、GitHub 预览块、扫描日志、标题与按钮均改为浅底深字 |

SHA256 以 `release\发布说明.txt` 为准（每次重新打包后自动重算）。

1.0.3 关键变更：

- 延迟改为多轮 TCP 握手 RTT（默认 4 轮），与 v2rayN 的 tcping 口径一致；HTTPS 探测只负责 SNI / CF-RAY / colo，不再把 TLS + HTTP 耗时当作网络延迟。
- 导出行去除延迟字段：`IP:端口#三笠优选 | 地区 | 下载 x.xxMB/s`。
- 窗口缩放适配：多列表格设置最小列宽与内部滚动；侧栏按窗口高度压缩；首页地区卡在 1180 宽度自动 4×2。
- 浅色主题补全：表格表头、GitHub 上传预览、扫描日志、页面标题、侧栏品牌字、按钮文字全部修正。
- 主题向九页设计稿靠拢：品牌横幅、HUD 卡片高光、页面标题强调条、樱花枝与网格底纹。
- 扫描任务新增暂停 / 继续；暂停只停止派发新任务，已发出的请求自然完成。

> 说明：1.0.3 之前产生的历史结果仍保留旧 HTTPS 延迟值；在地区排名点击「复测」可刷新为 TCP 口径。

## 0.1 1.0.2 多源聚合真实验收（2026-11-25，历史）

- 接入社区公开源 7 个：cf-best-ip 上游、BestCF、best-cf-ips、IPDB、cloudflare_ip、free-nodes/bestcf、XIU2；README 上游发现实测 208 个端点，白名单通过 129 个、拒绝 79 个。
- 候选池：15,575 个 `IP:端口` 端点，去重独立 IP 12,739；IPv4 端点 14,720，IPv6 端点 855。
- 真实扫描 `scan-878aaac32469`：1,020 候选 → TCP 344 → HTTPS 330 → colo 330 → 延迟 329 → 下载 55/68 → 正式保留 27，参考 1；耗时约 2 分 56 秒。
- 地区结果：香港 4 正式 + 1 参考（1065.1 ms / 1.23 MB/s）、美国 17 正式（748.6 ms / 7.26 MB/s）、德国 2 正式（1101.4 ms / 2.20 MB/s）、荷兰 4 正式（789.9 ms / 9.43 MB/s）；全球其他地区 6 条全部保留，未知 0。
- 详细数据、每源贡献和限制见 [GLOBAL_IP_DISCOVERY_RESULTS.md](GLOBAL_IP_DISCOVERY_RESULTS.md)。

## 1. 编译与测试

| 项目 | 结果 |
|---|---|
| `npm run build` | 通过，5430 modules |
| `cargo check` | 通过 |
| `cargo test --lib` | 8 passed / 0 failed（候选解析、JSON/JSONL、CIDR 采样、FOFA 语法、私网过滤、导出单地区/全部地区单文件/空地区拒绝） |
| NSIS 安装包 | 构建成功：`三笠优选_1.0.2_x64-setup.exe` |
| 绿色便携版 | `三笠优选_1.0.2_x64-portable.zip`（含 `三笠优选.exe` + README + docs） |
| 最终安装包 | `release\三笠优选_1.0.2_x64-setup.exe` |
| 最终便携版 | `release\三笠优选_1.0.2_x64-portable.zip` |
| 完整源码包 | `release\三笠优选_1.0.2_source.zip` |

## 2. CM IP 库真实获取

- TXT：`https://zip.cm.edu.kg/all.txt`，HTTP 200，解析 12,129 条 `IP:端口#国家码`。
- JSON：`https://zip.cm.edu.kg/all.json`，完整缓存 8,343,946 字节，解析 12,129 条。
- 交叉校验：仅 TXT 0 条、仅 JSON 0 条、交集 12,129 条。
- 当前缓存：`%APPDATA%\com.mikasa.optimizer\cm-cache.json`；状态「已连接」，数据更新时间 `2026-11-23T06:03:14.097057`（源站时间）。
- TXT/JSON 中的国家字段只写为候选标签，实际地区仍由下方真实 trace 的 colo 判定。

## 3. 真实扫描记录

扫描 ID：`scan-df4371a3f219`，完成时间 2026-11-23 21:23:26。

| 阶段 | 结果 |
|---|---|
| 候选 | 240 |
| TCP 成功 | 240 |
| HTTPS 成功 | 40 |
| colo 匹配 | 40 |
| 下载速度达标 | 21 |
| 最终保留 | 14 |

七地区最终结果：香港 1、台湾 0、日本 2、新加坡 2、韩国 0、法国 4、美国 5。没有用其他地区 IP 补足台湾/韩国。

真实下载日志摘录（完整日志见随包 [`docs/scan-sample.log`](scan-sample.log)，原始路径 `%APPDATA%\com.mikasa.optimizer\logs\scan-scan-df4371a3f219.log`）：

```text
[SUCCESS] 150.136.105.229:443 下载完成 · 1.94 MB/s (15.5 Mbps) · 9.7 MB / 5.0s
[SUCCESS] 144.202.123.69:443 下载完成 · 2.81 MB/s (22.5 Mbps) · 14.1 MB / 5.0s
[SUCCESS] 82.110.234.77:8443 下载完成 · 10.86 MB/s (86.9 Mbps)
[WARN] 178.104.164.45:443 下载未通过：下载请求失败
[WARN] 188.166.250.155:443 下载未通过：有效数据不足（156446 字节 < 524288 字节）
```

说明：本次测试机当前经 VPN/代理链路，延迟普遍偏高，因此只用于证明真实链路可跑通；正式使用时关闭 VPN/代理/TUN 后重新扫描，结果会按当前网络重新产生。

### 2026-11-24 延迟阈值修复后的 release 复测

使用最终 release 构建，测试参数：候选上限 60、每地区保留 2、延迟上限 1500 ms、下载 4 秒、最低数据量 1 MB。结果：

| 阶段 | 结果 |
|---|---|
| TCP 通过 | 50 / 60 |
| HTTPS 通过 | 50 / 50 |
| colo 匹配 | 50 |
| 延迟通过 | 50（超限 4） |
| 下载通过 | 12 / 13 |
| 正式保留 | 2（美国） |

截图：`artifacts/release-final-scan-result.png`。关键日志：`[SUCCESS] 104.17.255.110:443 下载完成 · 5.99 MB/s (47.9 Mbps) · 24.0 MB / 4.0s · 备用地址`。扫描完成后界面显示「首选完成：2 个 IP 通过全部测试」，并且超限 IP 已完成下载、仅作为参考不导出；证明旧版「延迟 300 ms 在下载前淘汰全部候选」的问题已修复。

## 4. 正式导出

早期版本（v1.0.0 初版，按地区拆分）实际生成：

- `香港.txt`：1 条
- `台湾.txt`：0 条（保留空文件，不填充其他地区）
- `日本.txt`：2 条
- `新加坡.txt`：2 条
- `韩国.txt`：0 条（保留空文件，不填充其他地区）
- `法国.txt`：4 条
- `美国.txt`：5 条
- `全部地区.txt`：14 条（合并）
- CSV：8 个文件全部生成，字段包括 IP、端口、地区、colo、平均延迟、MB/s、Mbps、丢包率、稳定性、来源、测试时间。

> 2026-11-24 起导出逻辑已调整：每次只生成一个文件。选择单个地区时只写该地区；选择「全部地区」时七地区全部合格节点合并写入 `全部地区.txt` / `全部地区.csv`，不再生成多个地区文件。
>
> 实际 release 验收（2026-11-24）：选择「全部地区」生成 `全部地区.txt`（938 字节，11 个节点）；选择「美国」生成 `美国.txt`（938 字节，11 个节点），文件内容均为 `IP:port#三笠优选 | 美国 | 延迟：…ms | 下载：…MB/s`，目录 `%USERPROFILE%\Documents\三笠优选导出\`。

实际文件内容示例：

```text
45.39.199.98:443#三笠优选 | 香港 | 延迟：693.7ms | 下载：7.54MB/s
82.110.234.77:8443#三笠优选 | 日本 | 延迟：515.4ms | 下载：10.86MB/s
```

TXT 不再包含重复的尾部 `IP:IP:port` 字段；每条结果都有真实延迟与真实下载速度。

## 5. 仍待用户提供凭据后实测

- FOFA 真实账号 API Key（界面、查询语法、分页和错误处理已完成，未编造 Key）。
- GitHub 仓库地址、分支和 Token（检查仓库/分支/权限、上传、回读 commit SHA 已完成，未使用任何旧账号/旧 Token）。

这两项在没有用户凭据时只能验证到配置校验与错误分支，不能伪造成功结果。

## 多测速源（2026-11-24）

- 新增 `src-tauri/src/speed_source.rs`，五种测速源：自动选择 / Cloudflare 官方 / CM 提供 / 移动专属 / 手动输入。
- 地址依据 CFData-WEB 主分支 `combined_refactor/speed_url.go`，非 README 转述。
- 本机（河南省 · 中国联通）真实结果：

| 源 | 状态 | 下载 | 速度 | 实际连接 IP |
|---|---|---|---|---|
| Cloudflare 官方 | 200 | 99,999,999 B | 31.03 MB/s | 104.16.123.45 |
| CM 提供 | 403 | 0 B | — | 104.16.123.45 |
| 移动专属 | 200 | 46,669 B | 0.01 MB/s | 104.16.123.45 |

- Auto 在联通网络下选择 Cloudflare，符合「非移动优先 Cloudflare」策略。
- `cf.json` 未返回 ASN，已回退本机运营商检测（结果中国联通）。
- 结论与完整日志：`docs/SPEED_SOURCES.md`、`docs/speed-source-validation.log`。

## 七类来源补充（2026-11-24）

- **GitHub Raw**：真实导入 XIU2/CloudflareSpeedTest `ip.txt`，首次获取 25 条 CIDR 样本；候选池从 13073 增至 13098（其中 CM 12129、Cloudflare 944、GitHub 25）。CIDR 解析每个网段取 1 个样本，记录 source=`github`。
- **Censys 平台**：后端查询 `https://api.platform.censys.io/v3/global/search/query`，查询 Cloudflare ASN 13335 + 443；Token 存 Windows 凭据管理器。当前环境无 Censys Token，**真实查询未跑通**，只验证未配置状态与错误分支，详见 `docs/CENSYS.md`。
- **来源统计**：来源页统计表已扩展为本地/URL/FOFA/Cloudflare/CM/GitHub/Censys 行，数值均来自 `source_stats` 与 `candidates` 实际数据。
- `cargo test --no-default-features`：6 passed / 0 failed（新增 CIDR 采样测试）。
- 九页 UI 验收见 `docs/UI_ACCEPTANCE.md`。
