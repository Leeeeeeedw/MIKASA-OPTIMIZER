# FOFA 配置说明

三笠优选通过 FOFA 官方 API 获取候选 IP，再把候选导入本地真实测速流程。FOFA 的国家字段只是候选来源标签，**不代表 Cloudflare 实际机房地区**。

## 1. 准备 FOFA 账号

1. 登录 [FOFA](https://fofa.info/)。
2. 进入「个人中心 → API 管理」，确认账号具备 API 查询权限。
3. 复制注册邮箱和 API Key。

不同账号等级的 API 配额、字段权限和频率限制不同，请以 FOFA 控制台显示为准。

## 2. 在软件中配置

1. 打开「IP 来源 → FOFA」。
2. 填写 FOFA 邮箱与 API Key。
3. 点击「保存配置」。密钥写入 Windows 凭据管理器，不会出现在日志、SQLite 或导出的配置文件中。
4. 选择地区（支持多选）。软件会自动生成查询语句并显示预览。

## 3. 自动生成的查询语法

默认模板：

```text
country="{country}" && port="{port}" && org="Cloudflare, Inc."
```

地区代码：

| 地区 | country |
|---|---|
| 香港 | HK |
| 台湾 | TW |
| 日本 | JP |
| 新加坡 | SG |
| 韩国 | KR |
| 法国 | FR |
| 美国 | US |

示例：

```text
country="HK" && port="443" && org="Cloudflare, Inc."
country="JP" && port="443" && org="Cloudflare, Inc."
country="KR" && port="443" && org="Cloudflare, Inc."
```

高级用户可在「设置 → 高级设置 → FOFA 查询模板」中修改模板。模板支持 `{country}`、`{port}` 占位符。修改前请先在 FOFA 控制台验证字段与语法；字段名或权限变化时，软件会保留错误状态并提供重试入口，不会伪造数据。

## 4. 获取候选

1. 确认地区与查询预览。
2. 设置单次获取上限（建议先用 100–300 条验证链路）。
3. 点击「获取 IP」。
4. 软件自动分页调用 FOFA API，解析 `ip`、`port` 字段并写入统一候选池。
5. 到「总览」开始优选。FOFA 结果必须再次经过本地 TCP / HTTPS / 延迟 / 下载测试。

## 5. API 调用参数

软件调用：

```text
GET https://fofa.info/api/v1/search/all
```

参数：

- `email`：FOFA 注册邮箱
- `key`：API Key
- `qbase64`：查询语句的 Base64 编码
- `fields`：`ip,port,country,country_name`
- `page`：页码，从 1 开始
- `size`：每页数量

软件会遵守返回的配额与错误码，并在界面上显示失败原因。若返回 401/403/429，请检查账号权限、Key 是否有效或是否触发频率限制。

## 6. 常见问题

**查询有结果但测速全部失败？** FOFA 的 `country` 只表示 IP 注册或观测位置，Cloudflare Anycast 的实际入口由本地网络决定。请以 trace 的 `colo` 为准，并检查候选端口是否为 443。

**提示无权限？** 部分 FOFA 套餐不开放 `org` 字段或 API 全量字段。可在高级设置中简化模板，例如只保留 `country` 与 `port`，或升级账号权限。

**Key 会不会泄露？** 不会写入日志或项目文件。保存后仅通过 Tauri 命令读取到内存，并用于 HTTPS 请求。

**如何清除？** 点击「清除配置」，软件会删除 Windows 凭据管理器中的记录。
