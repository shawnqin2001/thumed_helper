# 清华医学院集群助手 / THU Med cluster helper 

Terminal UI for creating, entering and removing your RStudio pod on the THU Med
computational cluster. Works on macOS, Linux and Windows.

启动后按提示存放管理员提供的 `config` 文件即可。默认路径为 `~/.kube/config`
（Windows 为 `%USERPROFILE%\.kube\config`），也支持 `KUBECONFIG` 路径列表。
程序会自动检查环境，安装缺少的 kubectl、Helm，并配置 Helm 仓库。

## 使用 / Usage

Run `thumed_helper`, then:

| Key | Action / 操作 |
| --- | --- |
| `↑`/`↓` or `j`/`k` | move / 移动 |
| `Enter` | select / 选择 |
| `Tab` | next form field / 下一字段 |
| `F2` | switch 中文 / EN |
| `Esc` | back, cancel, stop forwarding / 返回、取消、停止转发 |
| `q` | quit from the menu / 在主菜单退出 |

Menu items: install pod, login to pod, forward port 8787
(stays inside the UI, `Esc` stops it), uninstall pod, update user information,
check environment (last).

### 首次使用 / First launch

首次启动只提示 `config` 文件的存放位置，不再要求输入账号密码或手动安装工具。
检测到文件后自动继续：复用可用的 kubectl/Helm，安装缺少的工具，校验当前配置
引用的证书和密钥，检测集群访问并添加/更新 Helm 仓库。初始化过程中会逐步显示
正在检查、获取版本、下载安装包/校验文件、校验 SHA-256、解压、验证可执行文件、
安装及更新仓库等操作，并显示已完成项目和待检查项目。

检查结束后，无论成功或失败均保留详细报告，不自动返回菜单。报告列出 config 路径、
工具版本及来源、当前 context/集群地址/命名空间、用户名和密码、Pod 列表和 Helm
仓库信息。前置条件失败的项目会明确标记为跳过。`↑/↓`、`PgUp/PgDn`、`Home/End`
浏览长报告，`r` 重查，`u` 更新账号，`Enter/Esc` 返回菜单。缺失的私人证书/密钥、
网络或访问权限仍需处理，程序不会伪造补齐。

自动下载来自 `dl.k8s.io` 和 `get.helm.sh`，选择当前平台的 kubectl 稳定版和
Helm 3，使用 HTTPS、SHA-256 校验及运行验证后安装到应用配置目录的 `bin/`。
支持 macOS/Linux/Windows 的 x86_64、arm64；不使用 sudo，不修改系统 PATH，
不执行远程安装脚本。下载与解包使用系统 `curl`、`tar`；校验使用 macOS 的
`shasum`、Linux 的 `sha256sum` 或 Windows PowerShell。系统工具或网络不可用时
会保留失败状态，不会把未验证的下载文件当作已安装工具。

全部检查通过后记录 `initialized`；下次复用现有环境。“检查环境”仍在菜单末尾。
旧版初始化标记会触发一次重新检查。

### 授课版账号 / Lecture credentials

默认用户名取当前 kubeconfig context 的 `user` 字段，不取系统登录名或用户列表
第一项；默认密码为 `Test1234`。可通过菜单“更新用户信息”或报告中的 `u` 修改。
表单预填当前值，`Delete` 清空当前字段；保存后用于后续创建 Pod，并在重查报告中显示。
此操作只更改本地应用登录信息，不更改 kubeconfig 身份或集群中的实际密码。


Place the administrator-provided config at the displayed path. Setup automatically
installs missing tools and shows progress before each operation. Completed checks
remain in a scrollable report, including plaintext lecture username/password.
Use Update user information (or `u` in the report) to save a context-bound override.
Defaults remain the active config username and `Test1234`. Private keys and tokens
are never selected for display; local credential edits do not change cluster passwords.


