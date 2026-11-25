# 三笠优选 · 全球 Cloudflare IP 多源聚合 — 实施计划

> 版本目标：1.0.2（在 1.0.1 主题修复版上叠加）
> 原则：采集与测速分离；采集阶段不按延迟淘汰；所有统计均来自本地真实运行数据。

## 一、数据源核验结论（2026-11-24 实测）

| 项目 | 许可证 | 实际可抓取数据 | 端点 |
|---|---|---|---|
| LeilaoMi/cf-best-ip | MIT | Cloudflare Worker 聚合程序，无固定公开实例；复用其公开上游清单 | `bestcf.pages.dev/uouin/all.txt`、`s5gy/all.txt`、`lzj/all.txt`、`nirevil/ipv4.txt`、`joname1/BestCFip/ipv4.txt`、`ipv6.txt` |
| DustinWin/BestCF | 未标注（仅链接数据） | `bestcf` 分支 4 个运营商文件 + 统一文件 | `bestcf-ip.txt`、`cmcc-ip.txt`、`cucc-ip.txt`、`ctcc-ip.txt`、`proxy-ip.txt` |
| LancelotRar/best-cf-ips | 未标注（仅链接数据） | 每 6 小时更新的 top20/50/100/200/400 | `best-cf-ip-scanned-top400.txt` |
| ymyuuu/IPDB | MIT | 公开 API + BestCF CSV | `https://ipdb.api.030101.xyz/?type=cfv4;cfv6;proxy`、`BestCF/ipv4.csv` |
| svip-s/cloudflare_ip | 未标注（仅链接数据） | 每日 02:00 更新的 full/best 列表 | `full_ips.txt`、`best_ips.txt`；镜像 `ips.gaoji.uk`、`ips.de5.net` |

补充：无许可证元数据的项目只读取其公开原始数据链接，不复制其代码；所有来源保留项目名与源文件 URL 作为来源标签。

## 二、技术方案

1. **统一社区源适配器**（`src-tauri/src/sources.rs`）
   - 新增 `CommunitySource` 元数据（key、显示名、项目、许可证、更新频率、说明）。
   - 新增 `community_candidates(source, max_per_source)`：按 key 拉取一个或多个公开文件，统一交给 `parse_candidate_text` 解析。
   - 每个文件独立容错；单文件失败只记 warning，不中断整个来源。
   - `cf-best-ip` 的公开上游清单作为 `cf_best_ip` 来源；页面明确标注“Worker 上游聚合”。

2. **数据入库与状态**（`src-tauri/src/db.rs`、`commands.rs`）
   - 继续使用 `candidates(ip,port)` 唯一键与 `sources` JSON 数组，多个来源自然合并去重。
   - `source_stats` 记录每个社区源的 `last_fetched / last_added / last_duplicates / last_invalid / last_update / status / message`。
   - 新增命令 `community_source_status`、`community_fetch`；前端可单独获取，也可一键获取全部。
   - 新增后台刷新任务：按 `source_refresh_minutes` 检查五个社区源，失败不影响其他源。

3. **全球地区识别**（`src-tauri/src/models.rs`）
   - 保留七个首选地区；新增 `GLOBAL_REGION_ORDER` 与 `global_region_for_colo`，采用 Cloudflare 实际 colo 映射（HKG/TPE/KHH/NRT/KIX/.../FRA/DUS/LHR/MAN/YYZ/SYD/...）。
   - 无法识别的 colo → `unknown`，仍允许进入排名；绝不使用 GeoIP 替代 colo。
   - `region_name` / `region_flag` 覆盖全球国家/地区。
   - 扫描结果新增 `global` 汇总行；全球其他地区按国家单独保留结果，不受七地区配额限制。

4. **优质 IP 保留**（`scanner.rs`）
   - 七地区按 `per_region_top` 配额保留正式结果。
   - 所有非首选地区的达标 IP 全部保留为 `passed`，按实际国家分组，单独在“全球其他地区”展示。
   - 仍保留 `latency_below/speed_below` 参考结果，不混入正式导出。
   - 导出支持 `all`（全部正式结果单文件）、七地区、全球各地区。

5. **采集与测速分离**
   - 采集只写候选池，不测延迟、不淘汰。
   - 测速阶段继续使用现有本地 TCP/TLS/HTTP/下载引擎；保留 `source_filter`，可按来源筛选。
   - 现状已支持取消、并发可配、`max_candidates` 上限和分层筛选；本轮补充分批调度与断点续测命令。

6. **UI**
   - `SourcesView.vue` 增加“社区聚合”tab：五个来源卡片、许可证/更新频率、获取/全部获取、状态与数量。
   - `RankingsView.vue` 增加“全球其他地区”分组及按国家标签；保持七地区优先展示。
   - `ScanView.vue` 显示全球结果汇总行。
   - 所有数字绑定真实 API，不写示例数据。

## 三、需修改文件

- 后端：`src-tauri/src/models.rs`、`sources.rs`、`db.rs`、`scanner.rs`、`commands.rs`、`lib.rs`
- 前端：`src/types.ts`、`src/api.ts`、`src/views/SourcesView.vue`、`src/views/RankingsView.vue`、`src/views/ScanView.vue`、`src/views/ExportView.vue`、`src/views/SettingsView.vue`
- 文档：`README.md`、`docs/USAGE.md`、`docs/VALIDATION.md`、`docs/SECOND_ROUND_CHANGELOG.md`、本文件
- 配置：`package.json`、`src-tauri/tauri.conf.json`、`src-tauri/Cargo.toml`、`src/store.ts`（版本 1.0.2）

## 四、验收口径

1. 五个项目逐一显示实际接入方式、许可证与更新频率。
2. 每个社区源显示本次获取数、新增、重复、无效、最近成功更新时间。
3. 汇总候选池显示总候选、IPv4、IPv6、未知地区数量。
4. 真实扫描报告显示首选地区有效 IP 数、其他国家有效 IP 数、未知 colo 数。
5. 所有失败源保留错误原因，不阻断其他源。
