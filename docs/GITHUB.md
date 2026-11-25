# GitHub 上传说明

三笠优选使用 GitHub Git Data API 将导出的优选文件批量提交到指定仓库，不需要本地安装 Git。

## 1. 创建仓库

1. 登录 GitHub，点击右上角「+ → New repository」。
2. 填写仓库名，例如 `mikasa-results`。
3. 如果要把 raw 直链直接放进脚本，仓库必须设为 **Public**；私有仓库的 raw 链接需要登录或 Token。
4. 可以勾选「Add a README file」，也可以创建完全空仓库；空仓库首次上传会自动创建初始提交和默认分支。
5. 记下仓库地址，例如 `https://github.com/YOUR_NAME/mikasa-results`。

## 2. 创建 Token

1. 打开 GitHub「Settings → Developer settings → Personal access tokens → Fine-grained tokens」。
2. 点击「Generate new token」。
3. 选择目标仓库（Repository access → Only select repositories）。
4. 权限设置：
   - **Contents：Read and write**（必需）
   - Metadata：Read-only（自动包含）
5. 生成后复制 Token（`github_pat_...` 或 `ghp_...`），只显示一次。

> 不要使用过期、泄露或来源不明的旧 Token。软件不会把 Token 写入日志、SQLite 或导出文件。

## 3. 在软件中配置

1. 打开「GitHub 上传」。
2. 填写：
   - 仓库：`https://github.com/YOUR_NAME/mikasa-results` 或 `YOUR_NAME/mikasa-results`
   - 分支：默认 `main`（也可填 `master` 或新分支名）
   - 远程目录：默认 `mikasa-results`
   - Token：粘贴上一步生成的 Token
3. 点击「保存配置」。Token 保存到 Windows 凭据管理器。
4. 点击「检查仓库」。软件会读取仓库默认分支、目标分支是否存在、当前 Token 是否具有 push 权限；未通过检查时不会允许上传。
5. 点击「开始上传」，在弹出的窗口中选择：
   - 合并全部地区（生成 `all.txt`）
   - 按地区拆分（生成 `HK/all.txt`、`JP/all.txt`、`US/all.txt` 等，并额外生成 `all.txt`）
   - 最低下载速度、最高延迟
   - TXT 备注字段（城市、速度、延迟、colo、来源）
   - TXT 或 CSV
6. 点击「上传」，软件会：
   - 读取仓库默认分支的最新 commit 与 tree
   - 若仓库为空、默认分支尚不存在，先通过 Contents API 写入第一个文件，创建初始 commit 与默认分支
   - 为每个文件创建 blob
   - 创建新 tree 与新 commit
   - 更新目标分支 ref
   - 回读 ref 校验提交 SHA
7. 上传历史会记录仓库、分支、文件数、commit SHA 与结果。

## 4. 上传失败排查

| 状态 | 原因 | 处理 |
|---|---|---|
| 401 | Token 无效或过期 | 重新生成 Token |
| 403 | 权限不足或分支保护 | 确认 Contents 读写权限，或关闭分支保护 |
| 404 | 仓库不存在/无访问权限/分支不存在 | 检查仓库地址与分支名 |
| 409 | 空仓库尚无默认分支，或分支被并发修改 | 1.0.4 已自动处理空仓库；并发冲突时重试，软件会重新读取最新 ref |
| 网络错误 | 代理、DNS 或 GitHub 不可达 | 检查网络后重试 |

上传失败不会影响本地导出。可先导出到本地目录，再重试上传。

## 5. 文件目录与脚本直链

```text
mikasa-results/
├─ all.txt
├─ HK/
│  └─ all.txt
├─ JP/
│  └─ all.txt
└─ US/
   └─ all.txt
```

公开仓库上传完成后可直接复制 raw 直链：

```text
https://raw.githubusercontent.com/用户名/仓库名/main/mikasa-results/all.txt
https://raw.githubusercontent.com/用户名/仓库名/main/mikasa-results/JP/all.txt
```

jsDelivr 加速直链：

```text
https://cdn.jsdelivr.net/gh/用户名/仓库名@main/mikasa-results/all.txt
```

文件放在 `mikasa-results/` 子目录即可，不需要放到仓库根目录。私有仓库不会提供
匿名 raw 访问；如果脚本不能携带 Token，请把仓库改成 Public。

## 6. 安全建议

- 使用 Fine-grained Token，只授权目标仓库，不勾选无关权限。
- 定期轮换 Token。
- 不要把 Token 粘贴到聊天、Issue、截图或公开文档。
- 如怀疑泄露，立即在 GitHub 撤销并重新生成。
