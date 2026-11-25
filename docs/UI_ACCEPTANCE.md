# 九页 UI 验收记录

验收时间：2026-11-24–2026-11-25（Asia/Shanghai）
运行环境：Windows 10 x64 / 1442×942 / Tauri 2 + Vue 3 + WebView2
截图目录：`artifacts/`

## 1.0.4 九页与浅色主题复验

- 版本：`v1.0.4`；release 实机运行，使用现有真实候选池与历史测速结果，不注入演示数据。
- 截图矩阵：1180×720、1280×800、1440×900、1920×1080 四种尺寸各九页；另含深浅打赏弹窗。共 45 张布局截图，横向溢出 0 处。
- 截图目录：`artifacts/release-104/`；布局报告：`artifacts/release-104/layout-report.json`。
- 浅色重点：关于页插画柔光与文字、IP 来源页卡片与状态、地区排名表头与操作、历史失败状态、GitHub 表单与上传历史、设置说明文字、打赏弹窗均已检查为浅底深字。
- 窗口拉伸：表格保留最小列宽并内部滚动；首页地区卡随宽度换行；侧栏在 720 高度压缩插画和导航，不遮挡操作区。
- 默认主题：每次启动固定夜间主题；顶部主题按钮只切换当前会话，浅色主题保持完整可用。
- 图标：EXE、安装包、标题栏、任务栏、侧栏品牌标、关于页徽章统一为自由之翼风格徽章；任务栏实机截图见 `artifacts/desktop-taskbar-104.png`。
- 新增入口：左侧底部「打赏作者」，微信/支付宝收款码弹窗符合主题，不参与业务数据。

## 1.0.3 主题与窗口适配复验

- 版本：`v1.0.3`；release 实机运行，真实数据（候选池 15,575 / 当前结果 119）。
- 尺寸矩阵：1440×900、1180×720、1920×1080 各九页，共 27 张暗色截图 + 9 张浅色截图，整页横向溢出 0 处。
- 截图目录：`artifacts/release-103/`；九页拼图：`release/九页预览.png`。
- 浅色主题：侧栏标题、页面标题、表格表头、GitHub 上传预览、扫描日志、图表卡片全部为浅底深字；无透明文字、无深底深字。
- 缩放适配：首页地区卡在 1180 宽度自动 4×2；排名/历史/扫描表格保留内部横向滚动，不压缩列宽；侧栏在 720 高度压缩插画与导航。
- 主题还原：首页品牌横幅（三笠优选 / MIKASA OPTIMIZER / 更优的连接，更好的体验）、HUD 卡片顶部高光、页面标题强调条、樱花枝、网格底纹。
- 小屏 1180×720 下九页均可完整操作，底部内容通过页面纵向滚动查看，无控件重叠。

## 1.0.2 多源聚合复验

- 版本：`v1.0.2`；七个社区源卡片真实渲染，单源状态/获取/新增/重复/最后更新均来自 SQLite `source_stats`。
- 来源质量表与「数据源注册与上游发现」面板已上线；实测登记 7 个仓库，README 发现 208 个端点，白名单通过 129 个、拒绝 79 个。
- release 截图：`artifacts/release-102-overview.png`、`artifacts/release-102-sources.png`、`artifacts/release-102-community-cards.png`、`artifacts/release-102-registry.png`、`artifacts/release-102-rankings.png`、`artifacts/release-102-rankings-global.png`。
- 侧栏改为单张 `young-mikasa-rail.png` + 真正的 `sakura-branch.svg` 樱花枝，已确认无重影、无浅色矩形、窗口缩放不复发。

## 一、九页实际截图

| 页面 | 截图 |
|---|---|
| 总览首页 | `artifacts/page-release-overview.png` |
| IP 来源管理 | `artifacts/page-release-sources.png` |
| 扫描任务 | `artifacts/page-release-scan.png` |
| 地区排名 | `artifacts/page-release-rankings.png` |
| 结果导出 | `artifacts/page-release-export.png` |
| GitHub 上传 | `artifacts/page-release-github.png` |
| 历史记录 | `artifacts/page-release-history.png` |
| 设置 | `artifacts/page-release-settings.png` |
| 关于软件 | `artifacts/page-release-about.png` |

九页全景总览：`artifacts/nine-pages-release.png`。

## 二、首页功能保留清单

- [x] 省份 + 运营商（当前「河南省 · 中国联通」），网络检测状态与重新检测
- [x] 品牌横幅、七地区 + 全部地区选择
- [x] 每地区保留数量、测试端口、并发数量、延迟上限、智能补充、下载时长、最低数据量
- [x] 「开始优选」按钮；运行中任务进度、取消（暂停/继续暂未实现，见未完成项）
- [x] 测试 IP 数量、平均延迟、平均下载速度、可用 IP 数量
- [x] 地区平均延迟、地区平均下载、地区 IP 数量三个图表
- [x] 实时任务日志
- [x] GitHub 上传快捷入口、结果导出快捷入口
- [x] 七地区候选覆盖摘要，无数据显示「待发现」而非伪造数字

## 三、九个页面交互检查

| 页面 | 检查结果 |
|---|---|
| 总览 | 地区多选、全选、参数下拉、开始扫描、日志展示均可交互 |
| IP 来源 | 7 个来源 tab 可切换：本地文件 / 网络链接 / FOFA / Cloudflare / CM IP 库 / GitHub Raw / Censys 平台 |
| 扫描任务 | 阶段进度、6 项阶段统计、七地区 11 列统计、日志、开始/暂停/继续/取消按钮可交互 |
| 地区排名 | 地区 tab、搜索、排序、详情、单 IP 复测、批量复测按钮可交互 |
| 结果导出 | 单选地区/全部地区，单文件预览；实测生成 `美国.txt` 与 `全部地区.txt` |
| GitHub 上传 | 仓库/分支/目录/Token 配置、检查仓库、上传按钮；未配置 Token 时不允许上传 |
| 历史记录 | 列表、行选中、详情卡、查看结果、删除、备份数据库可交互 |
| 设置 | 基础参数、测速参数、测速源、FOFA/colo、导出/GitHub 设置表单可编辑，保存调用 `save_settings` |
| 关于 | 品牌、版本、定位、隐私、测速原则、第三方致谢完整展示 |

## 四、真实数据验证

- GitHub Raw 实际导入：候选池 13,098；GitHub Raw 获取 25 / 新增 25 / 重复 0 / 无效 0，最后更新 2026-11-24 15:04:51。
- CM IP 库：TXT/JSON 各 12,129 条，交集 12,129，仅 TXT 0、仅 JSON 0。
- 真实扫描（历史）：240 候选 → TCP 240 → HTTPS 40 → colo 40 → 下载达标 21 → 最终 14。
- 真实扫描（2026-11-24 release 复测）：60 候选 → TCP 22/60 → HTTPS 22/22 → colo 22 → 延迟通过 22（超限 1）→ 下载 13/13 → 正式保留 2（美国 LAX）；截图 `artifacts/release-final-scan-result.png`。
- 延迟超限验证：`scan-836b1001c481` 中 29 个 `latency_below` 结果全部完成真实下载测速并写入参考结果，证明延迟上限不再在下载前淘汰候选（日志 `docs/scan-latency-proof.log`）。
- 导出实测：`%USERPROFILE%\Documents\三笠优选导出\全部地区.txt` 与 `美国.txt` 各 938 字节、11 个节点。

## 五、编译与测试

- `npm run build`：通过，5,427 modules。
- `cargo test --no-default-features`：6 passed / 0 failed（候选解析、CIDR 采样、FOFA 语法、导出单地区/全部单文件/空地区拒绝）。
- `npx tauri build --bundles nsis`：见构建日志与 `src-tauri/target/release/bundle/nsis/`。
- MSI：因 WiX 下载超时未生成；NSIS 安装版可用。

## 六、未完成项 / 外部依赖

1. Censys 真实查询需要用户提供 Censys Platform Personal Access Token；当前只验证了未配置状态和错误分支，不能宣称真查询跑通。
2. 网络档案持久化、IPv4/IPv6 独立排名、预算管理等路线图进阶项属于后续迭代；本版 UI 不展示伪造数据，也不把未实现功能伪装成可用。
