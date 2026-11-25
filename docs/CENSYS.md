# Censys 平台 API 接入说明

## 用途

Censys 在本软件中只作为 **候选 IP 发现来源**，不参与最终地区判定。导入的候选仍需本机完成 TCP、HTTPS、CF-RAY colo、多轮延迟和真实下载测试。

## 数据与查询

- API：Censys Platform Global Data Search
- 端点：`https://api.platform.censys.io/v3/global/search/query`
- 查询：`host.services.port: 443 and host.autonomous_system.asn: 13335`
- 认证：`Authorization: Bearer <Personal Access Token>`
- 每次最多导入 25 / 50 / 100 条，避免无边界消耗 credits。

## 凭据

Token 保存在 Windows 凭据管理器，账号名 `censys_api_token`。设置、数据库、日志不会保存明文 Token。

## 真实状态（重要）

当前开发环境**没有 Censys 账号与 Token**，因此只验证了：

1. 命令注册与界面状态「未配置」正常显示；
2. 未配置时不会影响其他来源；
3. 填写 Token 后的真实 HTTP 请求路径与错误分支已实现：401/403 会提示 Token 无效或无权限，402 会提示 credits 不足，429 会提示限流；
4. 真实查询返回结果尚未在本机跑通，不能宣称「Censys 已实测可用」。

获取免费/Starter credits 说明：<https://docs.censys.com/docs/platform-credits-free-starter>

## 免费额度判断

Censys 免费/Starter 账号按 credits 计费，不是无限查询。软件每次只取一页（最多 100 条），且在界面明确提示「仅取前 N 条」。如果账号额度不足或 API 权限不包含 Global Data Search，界面会显示原始错误，不会回退成伪造数据，也不会影响 CM / GitHub / Cloudflare 来源。
