# 第二轮完善修改清单

## 一、CM IP 库（第五来源）

| 文件 | 修改 |
|---|---|
| `src-tauri/src/cm_source.rs` | 新增：TXT/JSON 拉取、解析、缓存、ETag、格式差异、自动更新、手动更新 |
| `src-tauri/src/models.rs` | 新增 `CmSourceStatus`、`CmFetchOutcome`、`CmRawPreview`、`SourceOverview` |
| `src-tauri/src/db.rs` | 新增 `cm_records` 与 `source_stats` 表、`record_source_import`、`source_overview` |
| `src-tauri/src/lib.rs` | 注册 CM 命令、启动自动刷新任务 |
| `src-tauri/src/commands.rs` | 新增 `cm_status`、`cm_fetch`、`cm_save_config`、`cm_preview`、`cm_clear`、`github_check`、`retest_ip` |
| `src/types.ts`、`src/api.ts`、`src/store.ts` | 新增 CM 类型、API、`sourceStats` 状态 |
| `src/views/SourcesView.vue` | 新增第五个「CM IP 库」标签页、原始数据预览、更新状态、五来源真实统计表 |

CM 端点：`https://zip.cm.edu.kg/all.txt`、`https://zip.cm.edu.kg/all.json`。均只作为候选来源；最终地区以实际 `CF-RAY` / trace `colo` 为准。

## 二、扫描任务与地区统计

| 文件 | 修改 |
|---|---|
| `src-tauri/src/scanner.rs` | TCP 通过时同步累加对应地区的 `tcp_ok`；扫描完成时进度置为 100%；保留真实 TCP→HTTPS→colo→延迟→下载→稳定性流程 |
| `src/views/ScanView.vue` | 地区统计表展示候选、TCP、HTTPS、colo、延迟、下载、达标、最终 |
| `src-tauri/src/speedtest.rs` | 延迟轮次记录 `samples`，下载按目标 IP 固定解析并保留正确 SNI/Host |
| `src-tauri/src/commands.rs` | `retest_ip` 单 IP 真实复测 |
| `src/views/RankingsView.vue` | 单 IP 详情、逐轮记录、单 IP/批量复测、综合评分规则 |

## 三、导出与 GitHub

| 文件 | 修改 |
|---|---|
| `src-tauri/src/export.rs` | TXT 去除重复的尾部 `IP:IP:port`；导出改为单选地区或全部地区，每次只生成一个文件；新增导出单元测试 |
| `src/views/ExportView.vue` | 新增地区单选（全部 + 七地区）、单文件预览、单文件导出结果卡 |
| `src-tauri/src/github.rs` | 新增仓库/分支/权限检查 `check_repository` |
| `src/views/GithubView.vue` | 上传前检查仓库、分支、`canPush`；未通过不允许上传 |
| `docs/GITHUB.md` | 补充检查流程 |

## 四、UI

- `src/App.vue`、`src/style.css`、`src/theme.ts`：明暗主题切换，浅色主题补齐文字对比度。
- `src/components/RegionFlag.vue`：真实 SVG 国旗，替换 emoji。
- `src/components/Sidebar.vue`、`TitleBar.vue`、`StatusStrip.vue`：三笠主题视觉和小图标优化。
- `src-tauri/icons/icon-source.png`：由用户提供的图标去除黑色背景并重新生成 Windows 图标。

## 五、验证结论

详见 [VALIDATION.md](VALIDATION.md)：真实扫描 240 个候选，最终 14 个完整通过；包含真实 MB/s 下载日志、CM TXT/JSON 各 12,129 条交叉校验、TXT/CSV 实际导出结果。FOFA 与 GitHub 真实成功调用仍需要用户提供自己的 API Key / Token，软件没有伪造成功状态。

## 六、延迟筛选与分阶段统计修复（2026-11-24）

| 文件 | 修改 |
|---|---|
| `src-tauri/src/models.rs` | `latency_limit_ms` 默认 300→1500；`RegionStats` 增加 `latency_eliminated`；`ScanProgress` 增加 TCP/HTTPS/延迟/下载分阶段字段 |
| `src-tauri/src/scanner.rs` | 延迟上限只在最终筛选生效；超限 IP 仍完成真实下载并作为参考结果；区分正式保留与参考结果；下载阶段不再把进度归零 |
| `src-tauri/src/export.rs` | 正式导出只包含 `status=passed` 的完整达标结果 |
| `src/views/ScanView.vue` | 6 项阶段计数、地区 11 列统计、延迟上限说明 |
| `src/views/RankingsView.vue` | 状态区分正式/未达延迟/速度偏低；正式 X / 参考 Y |
| `src/store.ts` | 默认延迟上限 1500 ms，补齐分阶段字段 |

验证：`scan-836b1001c481` 中 29 个 `latency_below` 全部完成真实下载测速并写入参考结果，证明延迟上限不再在下载前淘汰候选。

## 七、来源扩展（2026-11-24）

| 文件 | 修改 |
|---|---|
| `src-tauri/src/sources.rs` | `parse_candidate_text` 支持 CIDR，每个网段采样 1 个；新增 `censys_search` |
| `src-tauri/src/models.rs` | 新增 `CensysStatus`、`CensysFetchOutcome` |
| `src-tauri/src/db.rs` | 来源统计增加 GitHub Raw、Censys 行 |
| `src-tauri/src/commands.rs`、`lib.rs` | 新增 Censys 状态、保存/清除配置、查询导入命令 |
| `src/views/SourcesView.vue` | 新增 GitHub Raw、Censys 标签页；来源卡支持点击启停；七来源统计 |
| `src/types.ts`、`src/api.ts` | GitHub / Censys 类型与 API |

GitHub Raw 已真实导入 25 条；Censys 因无 Token 未做真实查询，只验证未配置和错误分支。

## 八、九页 UI 与验收

- 新增 `docs/UI_ACCEPTANCE.md`，记录九页截图、首页功能清单、交互测试、真实数据与未完成项。
- 截图：`artifacts/page-final-overview.png` 等 9 张，全景总览 `artifacts/nine-pages-final.png`。
- 侧栏童年三笠人物、樱花枝和全局面板花瓣动画按参考图完善；`young-mikasa-hero.png` 已裁掉参考图自带文字。
