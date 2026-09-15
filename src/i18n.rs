// Two-language string table. F2 toggles at runtime, Chinese is the default.
// ponytail: plain structs of &'static str; swap in a real i18n crate only if a
// third language or plural rules appear.

use crate::environment::CheckItem;
use crate::error::{Invalid, ThumedError};
use crate::tools::SetupStep;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    Zh,
    En,
}

impl Lang {
    pub fn toggle(self) -> Self {
        match self {
            Self::Zh => Self::En,
            Self::En => Self::Zh,
        }
    }

    pub fn t(self) -> &'static Strings {
        match self {
            Self::Zh => &ZH,
            Self::En => &EN,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Zh => "中文",
            Self::En => "EN",
        }
    }
}

pub struct Strings {
    pub version: &'static str,
    pub menu_labels: [&'static str; 6],
    pub menu_details: [&'static str; 6],
    pub install_labels: [&'static str; 3],
    pub credential_labels: [&'static str; 2],
    pub title_credentials: &'static str,
    pub credentials_hint: &'static str,
    pub context_label: &'static str,
    pub server_label: &'static str,
    pub namespace_label: &'static str,
    pub account_customized: &'static str,
    pub account_default: &'static str,
    pub title_login: &'static str,
    pub title_forward: &'static str,
    pub title_uninstall: &'static str,
    pub title_install: &'static str,
    pub panel_actions: &'static str,
    pub panel_details: &'static str,
    pub panel_pods: &'static str,
    pub panel_confirm: &'static str,
    pub panel_report: &'static str,
    pub panel_forward: &'static str,
    pub panel_initialize: &'static str,
    pub initialize_hint: &'static str,
    pub initialize_password: &'static str,
    pub panel_install_failed: &'static str,
    pub pod_waiting: &'static str,
    pub no_pods_loaded: &'static str,
    pub and_more: &'static str,
    pub picker_number: &'static str,
    pub uninstall_warning: &'static str,
    pub uninstall_pod: &'static str,
    pub uninstall_release: &'static str,
    pub status_label: &'static str,
    pub footer_menu: &'static str,
    pub footer_picker: &'static str,
    pub footer_form: &'static str,
    pub footer_confirm: &'static str,
    pub footer_back: &'static str,
    pub footer_forward: &'static str,
    pub footer_initialize: &'static str,
    pub footer_install_wait: &'static str,
    pub footer_install_result: &'static str,
    pub footer_uninstall_wait: &'static str,
    pub check_credentials: &'static str,
    pub check_kubeconfig: &'static str,
    pub check_kubectl: &'static str,
    pub check_helm: &'static str,
    pub check_cluster: &'static str,
    pub check_repo: &'static str,
    pub check_pending: &'static str,
    pub check_running: &'static str,
    pub forward_running: &'static str,
    pub forward_url: &'static str,
    pub status_loading: &'static str,
    pub status_env_done: &'static str,
    pub status_env_incomplete: &'static str,
    pub status_pods_loaded: &'static str,
    pub status_no_pods: &'static str,
    pub status_login_ended: &'static str,
    pub status_forward_stopped: &'static str,
    pub status_forward_failed: &'static str,
    pub status_pod_range: &'static str,
    pub status_installing: &'static str,
    pub status_installed: &'static str,
    pub status_user_saved: &'static str,
    pub status_uninstalling: &'static str,
    pub status_uninstalled: &'static str,
    pub err_io: &'static str,
    pub err_missing_tool: &'static str,
    pub err_missing_kubeconfig: &'static str,
    pub err_command_failed: &'static str,
    pub err_pod_not_found: &'static str,
    pub invalid_kube_user: &'static str,
    pub invalid_saved_credentials: &'static str,
    pub setup_prerequisite: &'static str,
    pub invalid_platform: &'static str,
    pub invalid_download: &'static str,
    pub invalid_pod_name: &'static str,
    pub invalid_cpu: &'static str,
    pub invalid_memory: &'static str,
    pub invalid_release_name: &'static str,
    pub invalid_release_changed: &'static str,
}

pub const ZH: Strings = Strings {
    version: "授课版",
    menu_labels: [
        "创建 Pod",
        "登录 Pod",
        "访问 RStudio",
        "卸载 Pod",
        "更新用户信息",
        "检查环境",
    ],
    menu_details: [
        "填写资源并创建 Pod。",
        "进入所选 Pod 命令行终端。",
        "访问集群的 RStudio。",
        "卸载所选 Pod 的 Helm release。",
        "修改本地用户名和密码。",
        "重新检查并配置环境。",
    ],
    install_labels: [
        "Pod 名称（小写字母和数字）",
        "CPU 核数（留空默认为 8）",
        "内存（留空默认为 50Gi）",
    ],
    credential_labels: ["用户名", "密码"],
    title_credentials: "更新用户信息",
    credentials_hint: "仅修改本地登录信息；Delete 清空。",
    context_label: "context",
    server_label: "集群",
    namespace_label: "命名空间",
    account_customized: "来源：本地",
    account_default: "来源：config/默认密码",
    title_login: "登录 Pod",
    title_forward: "端口转发",
    title_uninstall: "卸载 Pod",
    title_install: "创建 Pod",
    panel_actions: "操作",
    panel_details: "详情",
    panel_pods: "Pod 列表",
    panel_confirm: "确认卸载",
    panel_report: "环境检查",
    panel_forward: "端口转发",
    panel_initialize: "初始化",
    initialize_hint: "config 路径：",
    initialize_password: "密码（默认 Test1234）",
    panel_install_failed: "创建失败",
    pod_waiting: "等待 Pod Running（最长 30 秒）……",
    no_pods_loaded: "无 Pod。",
    and_more: "另有 {} 个",
    picker_number: "编号",
    uninstall_warning: "将删除 release 及其数据。",
    uninstall_pod: "Pod：{}",
    uninstall_release: "Helm release：{}",
    status_label: "状态：",
    footer_menu: "↑/↓ 选择   Enter 确认   q/Esc 退出",
    footer_picker: "↑/↓ 选择   数字跳转   Enter 确认   Esc 返回",
    footer_form: "↑/↓/Tab 切换   Enter 提交   Esc 取消",
    footer_confirm: "y 确认   n/Esc 取消",
    footer_uninstall_wait: "卸载中，请等待。",
    footer_back: "↑/↓/PgUp/PgDn 浏览   Enter/Esc 返回   r 重查   u 账号",
    footer_forward: "Esc/q 停止",
    footer_initialize: "输入密码   Enter 继续   Delete 清空   Esc 返回",
    footer_install_wait: "创建中，请等待。",
    footer_install_result: "Enter/Esc 返回",
    check_credentials: "账号与证书",
    check_kubeconfig: "kubeconfig",
    check_kubectl: "kubectl",
    check_helm: "Helm",
    check_cluster: "集群",
    check_repo: "Helm 仓库",
    check_pending: "待检查",
    check_running: "配置环境……",
    forward_running: "Pod {}：端口 {} 已转发。",
    forward_url: "http://localhost:{}",
    status_loading: "加载 Pod……",
    status_env_done: "初始化完成。",
    status_env_incomplete: "检查未通过；修复后按 r。",
    status_pods_loaded: "{} 个 Pod。",
    status_no_pods: "无 Pod。",
    status_login_ended: "登录结束。",
    status_forward_stopped: "转发已停止。",
    status_forward_failed: "转发中断：{}",
    status_pod_range: "编号范围：1–{}。",
    status_installing: "创建中……",
    status_installed: "Pod 已 Running。",
    status_user_saved: "已保存。",
    status_uninstalling: "卸载中……",
    status_uninstalled: "已卸载 {}（release {}）；列表已刷新。",
    err_io: "错误：{}",
    err_missing_tool: "缺少命令 {}。",
    err_missing_kubeconfig: "config 路径：{}",
    err_command_failed: "{} 失败：{}",
    err_pod_not_found: "Pod {} 不存在。",
    invalid_kube_user: "当前 context 无有效用户名。",
    invalid_saved_credentials: "账号无效，或密码为空/含控制字符。",
    setup_prerequisite: "已跳过：前置检查失败。",
    invalid_platform: "仅支持 macOS/Linux/Windows x86_64/arm64。",
    invalid_download: "校验失败，未安装。",
    invalid_pod_name: "Pod 名称仅限小写字母和数字。",
    invalid_cpu: "CPU 必须为 1–255 的整数。",
    invalid_memory: "内存必须为 1–255 GB 的整数。",
    invalid_release_name: "无法从 Pod 名称确定 release。",
    invalid_release_changed: "release 已变化，请重新选择。",
};

pub const EN: Strings = Strings {
    version: "Lecture version",
    menu_labels: [
        "Install pod",
        "Login to pod",
        "Access RStudio",
        "Uninstall pod",
        "Update user information",
        "Check environment",
    ],
    menu_details: [
        "Set resources and create a pod.",
        "Open a shell in the selected pod.",
        "Access RStudio on the cluster.",
        "Uninstall the selected pod's Helm release.",
        "Change the local username and password.",
        "Check and configure the environment again.",
    ],
    install_labels: [
        "Pod name (lowercase letters and digits)",
        "CPU (blank = 8)",
        "Memory (blank = 50Gi)",
    ],
    credential_labels: ["Username", "Password"],
    title_credentials: "Update credentials",
    credentials_hint: "Changes local credentials only. Delete clears a field.",
    context_label: "Context",
    server_label: "Cluster",
    namespace_label: "Namespace",
    account_customized: "Source: local",
    account_default: "Source: config/default password",
    title_login: "Login to pod",
    title_forward: "Forward port",
    title_uninstall: "Uninstall pod",
    title_install: "Install pod",
    panel_actions: "Actions",
    panel_details: "Details",
    panel_pods: "Pods",
    panel_confirm: "Confirm uninstall",
    panel_report: "Environment",
    panel_forward: "Port forwarding",
    panel_initialize: "Setup",
    initialize_hint: "Config path:",
    initialize_password: "Password (default Test1234)",
    panel_install_failed: "Creation failed",
    pod_waiting: "Waiting for Pod Running (up to 30 seconds)...",
    no_pods_loaded: "No pods.",
    and_more: "{} more",
    picker_number: "number",
    uninstall_warning: "Deletes the release and its data.",
    uninstall_pod: "Pod: {}",
    uninstall_release: "Helm release: {}",
    status_label: "Status: ",
    footer_menu: "↑/↓ select   Enter confirm   q/Esc quit",
    footer_picker: "↑/↓ select   number jump   Enter confirm   Esc back",
    footer_form: "↑/↓/Tab switch   Enter submit   Esc cancel",
    footer_confirm: "y confirm   n/Esc cancel",
    footer_uninstall_wait: "Uninstalling. Wait.",
    footer_back: "↑/↓/PgUp/PgDn scroll   Enter/Esc back   r recheck   u credentials",
    footer_forward: "Esc/q stop",
    footer_initialize: "Type password   Enter continue   Delete clear   Esc back",
    footer_install_wait: "Creating. Wait.",
    footer_install_result: "Enter/Esc back",
    check_credentials: "Credentials and certificates",
    check_kubeconfig: "kubeconfig",
    check_kubectl: "kubectl",
    check_helm: "Helm",
    check_cluster: "Cluster",
    check_repo: "Helm repo",
    check_pending: "Pending",
    check_running: "Configuring environment...",
    forward_running: "Pod {}: port {} forwarded.",
    forward_url: "http://localhost:{}",
    status_loading: "Loading pods...",
    status_env_done: "Setup complete.",
    status_env_incomplete: "Checks failed. Fix and press r.",
    status_pods_loaded: "{} pod(s).",
    status_no_pods: "No pods.",
    status_login_ended: "Login ended.",
    status_forward_stopped: "Forwarding stopped.",
    status_forward_failed: "Forwarding failed: {}",
    status_pod_range: "Number range: 1–{}.",
    status_installing: "Creating...",
    status_installed: "Pod is Running.",
    status_user_saved: "Saved.",
    status_uninstalling: "Uninstalling...",
    status_uninstalled: "Uninstalled {} (release {}). List refreshed.",
    err_io: "Error: {}",
    err_missing_tool: "Missing command {}.",
    err_missing_kubeconfig: "Config path: {}",
    err_command_failed: "{} failed: {}",
    err_pod_not_found: "Pod {} not found.",
    invalid_kube_user: "Current context has no valid username.",
    invalid_saved_credentials: "Invalid account, empty password, or control character.",
    setup_prerequisite: "Skipped: prerequisite failed.",
    invalid_platform: "Supports macOS/Linux/Windows x86_64/arm64 only.",
    invalid_download: "Verification failed; not installed.",
    invalid_pod_name: "Pod name: lowercase letters and digits only.",
    invalid_cpu: "CPU: integer 1–255.",
    invalid_memory: "Memory: integer 1–255 GB.",
    invalid_release_name: "Cannot derive release from pod name.",
    invalid_release_changed: "Release changed. Select again.",
};

/// Replace each `{}` in order with the given arguments.
pub fn fill(template: &str, args: &[&str]) -> String {
    let mut out = template.to_string();
    for arg in args {
        out = out.replacen("{}", arg, 1);
    }
    out
}

pub fn setup_step_name(step: SetupStep, lang: Lang) -> &'static str {
    let (zh, en) = match step {
        SetupStep::Check => ("正在检查", "Checking"),
        SetupStep::FetchVersion => ("获取可安装版本", "Fetching release version"),
        SetupStep::DownloadBinary => ("下载安装包", "Downloading package"),
        SetupStep::DownloadChecksum => ("下载校验文件", "Downloading checksum"),
        SetupStep::VerifyChecksum => ("校验 SHA-256", "Verifying SHA-256"),
        SetupStep::Extract => ("解压可执行文件", "Extracting executable"),
        SetupStep::VerifyBinary => ("验证可执行文件", "Validating executable"),
        SetupStep::InstallBinary => ("安装到本地目录", "Installing locally"),
        SetupStep::Configure => ("配置 Helm 仓库", "Configuring Helm repository"),
        SetupStep::Update => ("更新 Helm 仓库索引", "Updating Helm repository index"),
    };
    match lang {
        Lang::Zh => zh,
        Lang::En => en,
    }
}

pub fn check_name(item: CheckItem, lang: Lang) -> &'static str {
    let t = lang.t();
    match item {
        CheckItem::Credentials => t.check_credentials,
        CheckItem::Kubeconfig => t.check_kubeconfig,
        CheckItem::Kubectl => t.check_kubectl,
        CheckItem::Helm => t.check_helm,
        CheckItem::Cluster => t.check_cluster,
        CheckItem::Repo => t.check_repo,
    }
}

pub fn error_text(error: &ThumedError, lang: Lang) -> String {
    let t = lang.t();
    match error {
        ThumedError::Io(e) => fill(t.err_io, &[&e.to_string()]),
        ThumedError::MissingTool(tool) => fill(t.err_missing_tool, &[tool]),
        ThumedError::MissingKubeconfig(path) => fill(t.err_missing_kubeconfig, &[path]),
        ThumedError::CommandFailed { cmd, stderr } => {
            fill(t.err_command_failed, &[cmd, stderr.trim()])
        }
        ThumedError::PodNotFound(name) => fill(t.err_pod_not_found, &[name]),
        ThumedError::Invalid(kind) => invalid_text(*kind, lang).to_string(),
    }
}

fn invalid_text(kind: Invalid, lang: Lang) -> &'static str {
    let t = lang.t();
    match kind {
        Invalid::KubeUser => t.invalid_kube_user,
        Invalid::SavedCredentials => t.invalid_saved_credentials,
        Invalid::SetupPrerequisite => t.setup_prerequisite,
        Invalid::UnsupportedPlatform => t.invalid_platform,
        Invalid::DownloadVerification => t.invalid_download,
        Invalid::PodName => t.invalid_pod_name,
        Invalid::CpuValue => t.invalid_cpu,
        Invalid::MemoryValue => t.invalid_memory,
        Invalid::NoReleaseName => t.invalid_release_name,
        Invalid::ReleaseChanged => t.invalid_release_changed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_replaces_in_order() {
        assert_eq!(fill("a {} b {}", &["1", "2"]), "a 1 b 2");
        assert_eq!(fill("no args", &[]), "no args");
    }

    #[test]
    fn toggle_round_trips() {
        assert_eq!(Lang::Zh.toggle(), Lang::En);
        assert_eq!(Lang::Zh.toggle().toggle(), Lang::Zh);
    }

    #[test]
    fn errors_localize_in_both_languages() {
        let error = ThumedError::MissingTool("helm".to_string());
        assert!(error_text(&error, Lang::Zh).contains("helm"));
        assert!(error_text(&error, Lang::En).contains("helm"));
        assert_ne!(
            error_text(&ThumedError::Invalid(Invalid::PodName), Lang::Zh),
            error_text(&ThumedError::Invalid(Invalid::PodName), Lang::En)
        );
    }
}
