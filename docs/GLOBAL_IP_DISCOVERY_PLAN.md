# 三笠优选 · 全球 Cloudflare IP 最大化发现与聚合升级实施方案

> 本方案基于现有 Rust + Tauri 2 纯客户端架构，不引入服务器、不改变现有界面风格，采集、存储、测速三个模块保持独立。

## 一、现有项目分析结论

1. **已有架构**
   - `src-tauri/src/sources.rs` 已有统一解析器（纯 IP、`IP:PORT`、CSV、CIDR）与 HTTP 抓取函数；
   - `src-tauri/src/db.rs` 已有 SQLite `candidates` 表（`ip,port,sources,initial_region,first_seen,last_seen,last_colo,last_latency,last_download_mb_s,last_loss,last_stability,last_status,last_tested`）；
   - `src-tauri/src/scanner.rs` 已有 TCP → HTTPS/colo → 延迟 → 下载 → 可选稳定性阶段；实际 colo 通过 `CF-Ray` 获取；
   - `src-tauri/src/models.rs` 已扩展 `GLOBAL_REGION_ORDER`、`global_region_for_colo`、`region_for_any_colo`，非七地区达标 IP 采用 `usize::MAX` 配额全部保留；
   - 前端来源页已有七类主来源与五个社区源卡片。

2. **缺口**
   - 社区源只有 5 个，缺少 `free-nodes/bestcf`、`XIU2/CloudflareSpeedTest` 的显式接入；
   - 没有数据源注册表与 README 递归发现；
   - `candidates` 未持久化 IP 类型、IPv4/IPv6、成功/失败次数；
   - 没有来源质量评分与最终验收报告；
   - 没有按来源质量动态调整采集优先级。

## 二、技术方案

### 1. 数据源注册表
新增 `src-tauri/src/registry.rs`：
- 静态登记 7 个重点项目的仓库、许可证、分支、类型（数据源 / 扫描器参考）、已知入口；
- `discover()` 读取 GitHub README，正则提取公开 API、GitHub Raw、Release 数据地址；
- 仅允许 `raw.githubusercontent.com`、`cdn.jsdelivr.net`、`api.github.com`、`github.com`、`bestcf.tools12345.com`、`ips.gaoji.uk`、`ips.de5.net`、`ipdb.api.030101.xyz`、`bestcf.pages.dev`、`cf.junzhen.qzz.io` 等白名单主机；
- 拒绝 localhost、回环、私网、链路本地地址；
- 使用 `visited_repos`、`visited_urls` 双集合防循环与重复；
- 每个候选端点按 `kind` 分类：`github-raw` / `github-api` / `release` / `static-data` / `scanner-reference`。

### 2. 扩展社区源
在 `sources.rs` 增加：
- `free_nodes_bestcf`：读取 `bestcf.tools12345.com` 下 tiancheng/s5gy/wetest/uouin/cfyes/cmliu/cmliu2/vvhan/nirevil/gslege/zhixuanwang/lajiao/moistr 等公开子源；
- `xiu2`：读取 `XIU2/CloudflareSpeedTest` 的 `ip.txt`、`ipv6.txt`（CIDR 抽样）；
- `cf-best-ip` 上游继续读取 joname1/KafeMars/bestcf.pages.dev/s5gy/lzj/nirevil；
- `bestcf`、`best_cf_ips`、`ipdb`、`cloudflare_ip` 保持并补镜像。
- `SamNet-dev/cfray` 登记为**扫描器参考**，不提供公共 IP 列表，不作为候选数据源。

### 3. 候选数据库扩展
`candidates` 新增列（启动时 `ALTER TABLE` 幂等迁移）：
- `ip_type`：`official` / `reverse-proxy` / `community` / `history` / `public`；
- `ip_version`：4 或 6；
- `success_count` / `fail_count`：累计测速成功/失败次数；
- 原有来源、发现/最后时间、地区、colo、历史测速字段保留。

### 4. 来源质量评分
新增 `SourceQuality` 与 `db::source_quality()`：
- 原始数量、去重后数量、独立新增、重复率、有效连接、高速 IP、地区覆盖、更新频率、最近成功时间；
- 评分 = 唯一贡献 30% + 有效连接 30% + 高速 IP 20% + 地区覆盖 10% + 低重复率 10%；
- 仅用于排序和采集优先级，不代表 IP 已优选。

### 5. UI
- 来源页新增「数据源质量」表与「上游发现」按钮；
- 展示注册源数量、已发现端点、校验通过/失败、上次发现时间；
- 全球结果在排名页继续按实际 colo 地区展示，非七地区不丢失。

## 三、修改文件清单

| 文件 | 修改 |
|---|---|
| `src-tauri/src/registry.rs` | 新增注册表、README 递归发现、端点分类与校验 |
| `src-tauri/src/sources.rs` | 扩展 `community_sources/community_urls`，增加 free-nodes、XIU2；解析器过滤私网 IP；并发拉取 |
| `src-tauri/src/db.rs` | candidates 迁移、成功/失败计数、来源质量统计 |
| `src-tauri/src/models.rs` | `Candidate`、`SourceQuality`、`Registry*` 模型 |
| `src-tauri/src/commands.rs` | `registry_sources`、`registry_discover`、`source_quality` |
| `src-tauri/src/lib.rs` | 注册新模块与命令 |
| `src/types.ts` / `src/api.ts` | 新类型与 API 方法 |
| `src/views/SourcesView.vue` | 数据源质量表、上游发现卡片 |
| `README.md` / `docs/VALIDATION.md` | 新来源、验收口径、真实数据 |

## 四、验收口径

所有数字必须由本地实际抓取与测速产生：
1. 接入数据源总数；
2. 递归发现上游来源数；
3. IPv4 候选数、IPv6 候选数、去重后总候选数；
4. 每个来源的独立贡献与质量分；
5. 首选地区有效 IP 数；
6. 其他国家/地区高速 IP 数；
7. 数据库历史 IP 数；
8. 实际测速成功率。
