# 三笠优选 · 全球 Cloudflare IP 多源聚合验收报告

> 验收时间：2026-11-25 00:10–00:15（Asia/Shanghai）
> 数据来源：本机 SQLite `%APPDATA%\com.mikasa.optimizer\mikasa-optimizer.sqlite`、真实 HTTP 抓取、真实 Tauri 扫描；没有使用模拟数据。

## 1. 接入与递归发现

| 项目 | 实测值 |
|---|---:|
| 社区公开源适配器 | 7 个 |
| 数据源注册仓库 | 7 个（其中 `SamNet-dev/cfray` 仅作扫描器参考，不导入候选） |
| README 递归发现端点总数 | 208 个 |
| 白名单校验通过 | 129 个 |
| 拒绝的未知/私网端点 | 79 个 |
| 实际成功拉取的社区源 | 7/7（部分源存在单文件失败，已记录并继续） |

七个社区源：`cf_best_ip`、`bestcf`、`best_cf_ips`、`ipdb`、`cloudflare_ip`、`free_nodes_bestcf`、`xiu2`。

补充：2026-11-25 01:18 在 release 界面复跑「扫描 README 发现上游」得到 176 个端点、白名单通过 123 个、拒绝 53 个；与上表 probe 的 208/129/79 差异来自部分 README 请求的网络级失败，两次均为真实执行，界面按实际结果记录。

## 2. 候选池真实数量

| 指标 | 实测值 |
|---|---:|
| `candidates` 端点总数 | 15,575 |
| 去重独立 IP 总数 | 12,739 |
| IPv4 端点数 | 14,720 |
| IPv6 端点数 | 855 |
| 当前待测/未测候选 | 12,690 pending + 1,549 untested |

## 3. 每个社区源的独立贡献

| 来源 | 获取 | 新增 | 重复 | 状态 |
|---|---:|---:|---:|---|
| cf-best-ip 上游聚合 | 271 | 32 | 239 | 部分成功（5 个文件成功，2 个 raw 请求失败） |
| BestCF（DustinWin） | 89 | 89 | 0 | 正常（5 个文件成功） |
| best-cf-ips（LancelotRar） | 400 | 399 | 1 | 正常 |
| IPDB（ymyuuu） | 49 | 49 | 0 | 正常（3 个文件成功） |
| cloudflare_ip（svip-s） | 397 | 0 | 397 | 正常（数据已在历史池中） |
| free-nodes/bestcf 上游源 | 552 | 370 | 182 | 正常（16 个文件成功） |
| XIU2/CloudflareSpeedTest | 25 | 24 | 1 | 部分成功（IPv4 成功；IPv6 文件本次请求失败，状态已记录） |

## 4. 真实扫描结果

扫描 ID：`scan-878aaac32469`；开始 `2026-11-24 23:59:37`，结束 `2026-11-25 00:02:33`。

| 阶段 | 实测 |
|---|---:|
| 候选数 | 1,020 |
| TCP 通过 | 344 |
| HTTPS 通过 | 330 / 344 |
| colo 命中 | 330 |
| 延迟通过 | 329（超限 73） |
| 下载成功 | 55 / 68 |
| 正式保留 | 27 |
| 参考结果 | 1 |
| 扫描耗时 | 约 2 分 56 秒 |

## 5. 地区保留结果

| 地区 | 正式通过 | 参考 | 平均延迟 | 平均下载 |
|---|---:|---:|---:|---:|
| 香港 HK | 4 | 1（latency_below） | 1065.1 ms | 1.23 MB/s |
| 美国 US | 17 | 0 | 748.6 ms | 7.26 MB/s |
| 德国 DE | 2 | 0 | 1101.4 ms | 2.20 MB/s |
| 荷兰 NL | 4 | 0 | 789.9 ms | 9.43 MB/s |
| 台湾/日本/新加坡/韩国/法国 | 0 | 0 | — | — |
| 全球其他地区合计 | 6（DE 2 + NL 4） | 0 | — | — |
| 未知地区 | 0 | 0 | — | — |

结论：非首选地区的 6 条达标 IP 已全部进入正式结果，没有因地区筛选被删除；`RankingsView` 已提供“全部 / 全球其他 / 未知地区”分组。

## 6. 构建与测试

| 命令 | 结果 |
|---|---|
| `cargo test --manifest-path src-tauri/Cargo.toml --lib --no-default-features` | 8 passed；0 failed |
| `cargo check --manifest-path src-tauri/Cargo.toml --lib --no-default-features` | Finished dev profile |
| `npm run build` | 通过；仅有 chunk size 警告 |

## 7. 已知限制

- Censys 真实查询仍需要调用方提供 Token；当前只能验证未配置状态与界面。
- 部分 `raw.githubusercontent.com` 请求会出现网络级失败；社区源已使用 jsDelivr / 官方镜像回退，并在来源状态中记录失败文件。
- 本报告只记录本次真实执行结果；不同时间、不同本地网络会得到不同的测速结果。

## 8. 1.0.2 Release

- 最终构建：2026-11-25 01:02（Asia/Shanghai），`src-tauri\target\release\mikasa-optimizer.exe` 10,267,136 bytes。
- NSIS 安装包：`release\三笠优选_1.0.2_x64-setup.exe`，5,391,877 bytes，SHA256 `8446A23027AE073BE54FFC7ED9EBDFEE53DFDC2968AC0800FE582889F870FBFE`。
- 便携包：`release\三笠优选_1.0.2_x64-portable.zip`（校验值见 `release\发布说明.txt`）。
- 源码包：`release\三笠优选_1.0.2_source.zip`（校验值见 `release\发布说明.txt`）。
- 便携版内 `三笠优选.exe` SHA256 `F8D1683B2328FD7D622C8465DE0EC610D4B80E18CB1399006D58769C1AB9BD25`。
- UI 复验截图：`artifacts/release-102-final-overview.png`、`release-102-final-sources.png`、`release-102-final-scan.png`、`release-102-final-rankings.png`、`release-102-final-export.png`、`release-102-final-github.png`、`release-102-final-history.png`、`release-102-final-settings.png`、`release-102-final-about.png`、`release-102-final-community-cards.png`、`release-102-final-registry.png`、`release-102-final-rankings-global.png`。
