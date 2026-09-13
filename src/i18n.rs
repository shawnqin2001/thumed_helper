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
    pub no_pods_loaded: &'static str,
    pub and_more: &'static str,
    pub form_hint: &'static str,
    pub picker_number: &'static str,
    pub uninstall_warning: &'static str,
    pub uninstall_pod: &'static str,
    pub uninstall_release: &'static str,
    pub uninstall_continue: &'static str,
    pub status_label: &'static str,
    pub footer_menu: &'static str,
    pub footer_picker: &'static str,
    pub footer_form: &'static str,
    pub footer_confirm: &'static str,
    pub footer_back: &'static str,
    pub footer_forward: &'static str,
    pub footer_initialize: &'static str,
    pub footer_install_wait: &'static str,
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
    pub forward_stop: &'static str,
    pub status_loading: &'static str,
    pub status_env_done: &'static str,
    pub status_env_incomplete: &'static str,
    pub status_pods_loaded: &'static str,
    pub status_no_pods: &'static str,
    pub status_login_ended: &'static str,
    pub status_forward_started: &'static str,
    pub status_forward_stopped: &'static str,
    pub status_forward_failed: &'static str,
    pub status_pod_range: &'static str,
    pub status_installing: &'static str,
    pub status_installed: &'static str,
    pub status_user_saved: &'static str,
    pub status_uninstalled: &'static str,
    pub status_uninstall_cancelled: &'static str,
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
    pub invalid_no_release: &'static str,
    pub invalid_release_changed: &'static str,
}

pub const ZH: Strings = Strings {
    version: "授课版",
    menu_labels: [
        "创建 Pod",
        "登录 Pod",
        "端口转发",
        "卸载 Pod",
        "更新用户信息",
        "检查环境",
    ],
    menu_details: [
        "填写配置并用 Helm 创建 Pod。",
        "选择一个 Pod，打开其命令行。",
        "选择一个 Pod，转发 8787 端口到本机。",
        "选择一个 Pod，确认后删除对应的 Helm release。",
        "更新本地授课版账号和密码，保存后用于当前 context。",
        "检测环境并自动补齐 kubectl、Helm 和仓库配置。",
    ],
    install_labels: [
        "Pod 名称（小写字母和数字）",
        "CPU 核数（留空为默认）",
        "内存 GB（留空为默认）",
    ],
    credential_labels: ["用户名", "密码"],
    title_credentials: "更新用户信息",
    credentials_hint: "授课版明文显示。修改本地登录信息，不修改集群实际密码；Delete 清空当前字段。",
    context_label: "当前 context",
    server_label: "集群地址",
    namespace_label: "命名空间",
    account_customized: "来源：本地保存的账号信息",
    account_default: "来源：config 用户名及授课版默认密码",
    title_login: "登录 Pod",
    title_forward: "端口转发",
    title_uninstall: "卸载 Pod",
    title_install: "创建 Pod",
    panel_actions: "操作",
    panel_details: "详情",
    panel_pods: "Pod 列表",
    panel_confirm: "确认卸载",
    panel_report: "环境检查结果",
    panel_forward: "端口转发中",
    panel_initialize: "环境初始化",
    initialize_hint: "请将 config 文件放到以下位置：",
    no_pods_loaded: "暂无 Pod。",
    and_more: "……还有 {} 个",
    form_hint: "填写后按 Enter 提交。",
    picker_number: "编号",
    uninstall_warning: "此操作会删除 Helm release 及其数据。",
    uninstall_pod: "Pod：{}",
    uninstall_release: "Helm release：{}",
    uninstall_continue: "确认继续？(y/n)",
    status_label: "状态：",
    footer_menu: "↑/↓ 或 j/k 移动   Enter 选择   F2 中文/EN   q/Esc 退出",
    footer_picker: "↑/↓ 或 j/k 选择   1-99 输入编号   Enter 确认   F2 中文/EN   Esc 返回",
    footer_form: "输入内容   ↑/↓/Tab 切换字段   Enter 提交   F2 中文/EN   Esc 取消",
    footer_confirm: "y 确认卸载   n/Esc 取消   F2 中文/EN",
    footer_back: "↑/↓/PgUp/PgDn 浏览   Enter/Esc 返回   r 重查   u 更新账号   F2 中文/EN",
    footer_forward: "Esc 或 q 停止转发   F2 中文/EN",
    footer_initialize: "Esc 返回   F2 中文/EN",
    footer_install_wait: "正在创建，请等待结果（暂不接受按键）。",
    check_credentials: "账号与证书",
    check_kubeconfig: "kubeconfig",
    check_kubectl: "kubectl",
    check_helm: "Helm",
    check_cluster: "集群连接",
    check_repo: "Helm 仓库",
    check_pending: "等待检查",
    check_running: "正在自动配置环境……",
    forward_running: "Pod {} 的 {} 端口已转发到本机。",
    forward_url: "浏览器打开 http://localhost:{} 使用 RStudio。",
    forward_stop: "按 Esc 或 q 停止转发并返回菜单。",
    status_loading: "正在加载 Pod……",
    status_env_done: "初始化完成。",
    status_env_incomplete: "环境检查未全部通过，请补齐配置后按 r 重试。",
    status_pods_loaded: "已加载 {} 个 Pod。",
    status_no_pods: "没有可用 Pod，请先创建。",
    status_login_ended: "Pod 登录会话已结束。",
    status_forward_started: "端口转发已启动。",
    status_forward_stopped: "端口转发已停止。",
    status_forward_failed: "端口转发中断：{}",
    status_pod_range: "Pod 编号需在 1 到 {} 之间。",
    status_installing: "正在创建 Pod，请稍候……",
    status_installed: "Pod 创建成功。",
    status_user_saved: "用户信息已保存，可重新检查环境查看详情。",
    status_uninstalled: "Pod {} 的 release {} 已卸载。",
    status_uninstall_cancelled: "已取消卸载。",
    err_io: "系统错误：{}",
    err_missing_tool: "未找到命令 {}，请先安装并确保它在 PATH 中。",
    err_missing_kubeconfig: "请将 config 文件放到：{}",
    err_command_failed: "命令 {} 执行失败：{}",
    err_pod_not_found: "找不到 Pod {}。",
    invalid_kube_user: "config 当前 context 中没有有效用户名，请检查配置。",
    invalid_saved_credentials:
        "账号或密码格式无效，请通过“更新用户信息”重新保存；密码不能为空或含控制字符。",
    setup_prerequisite: "前置检查未通过，本项已跳过。",
    invalid_platform: "自动安装支持 macOS、Linux、Windows 的 x86_64/arm64 平台。",
    invalid_download: "下载文件或版本信息校验失败，未安装，请重试。",
    invalid_pod_name: "Pod 名称只能包含小写字母和数字。",
    invalid_cpu: "CPU 核数需为 1 到 255 的整数。",
    invalid_memory: "内存 GB 需为 1 到 255 的整数。",
    invalid_no_release: "该 Pod 没有 Helm release 标签，无法卸载。",
    invalid_release_changed: "该 Pod 的 Helm release 已变化，请重新选择。",
};

pub const EN: Strings = Strings {
    version: "Lecture version",
    menu_labels: [
        "Install pod",
        "Login to pod",
        "Forward port",
        "Uninstall pod",
        "Update user information",
        "Check environment",
    ],
    menu_details: [
        "Create a pod with Helm.",
        "Choose a pod and open its shell.",
        "Choose a pod and forward port 8787 to this machine.",
        "Choose a pod and remove its Helm release after confirmation.",
        "Save a lecture username and password for the active context.",
        "Check the environment and set up missing kubectl, Helm and repo configuration.",
    ],
    install_labels: [
        "Pod name (lowercase letters and digits)",
        "CPU cores (blank = default)",
        "Memory GB (blank = default)",
    ],
    credential_labels: ["Username", "Password"],
    title_credentials: "Update user information",
    credentials_hint: "Lecture credentials are shown in plain text. This changes local login details, not the cluster password. Delete clears a field.",
    context_label: "Current context",
    server_label: "Cluster server",
    namespace_label: "Namespace",
    account_customized: "Source: locally saved credentials",
    account_default: "Source: config username and lecture default password",
    title_login: "Login to pod",
    title_forward: "Forward port",
    title_uninstall: "Uninstall pod",
    title_install: "Install pod",
    panel_actions: "Actions",
    panel_details: "Details",
    panel_pods: "Pods",
    panel_confirm: "Confirm uninstall",
    panel_report: "Environment check",
    panel_forward: "Port forwarding",
    panel_initialize: "Environment setup",
    initialize_hint: "Place your config file here:",
    no_pods_loaded: "No pods loaded.",
    and_more: "... and {} more",
    form_hint: "Fill in the fields, then press Enter.",
    picker_number: "number",
    uninstall_warning: "This deletes the Helm release and its data.",
    uninstall_pod: "Pod: {}",
    uninstall_release: "Helm release: {}",
    uninstall_continue: "Continue? (y/n)",
    status_label: "Status: ",
    footer_menu: "↑/↓ or j/k move   Enter select   F2 中文/EN   q/Esc quit",
    footer_picker: "↑/↓ or j/k select   1-99 type number   Enter confirm   F2 中文/EN   Esc back",
    footer_form: "Type values   ↑/↓/Tab switch field   Enter submit   F2 中文/EN   Esc cancel",
    footer_confirm: "y confirm uninstall   n/Esc cancel   F2 中文/EN",
    footer_back: "↑/↓/PgUp/PgDn scroll   Enter/Esc back   r recheck   u credentials   F2 中文/EN",
    footer_forward: "Esc or q stops forwarding   F2 中文/EN",
    footer_initialize: "Esc back   F2 中文/EN",
    footer_install_wait: "Creating pod. Please wait for the result (keys are temporarily disabled).",
    check_credentials: "User and certificates",
    check_kubeconfig: "kubeconfig",
    check_kubectl: "kubectl",
    check_helm: "Helm",
    check_cluster: "Cluster access",
    check_repo: "Helm repo",
    check_pending: "Pending",
    check_running: "Setting up the environment...",
    forward_running: "Pod {} port {} is forwarded to this machine.",
    forward_url: "Open http://localhost:{} in a browser to use RStudio.",
    forward_stop: "Press Esc or q to stop forwarding and go back.",
    status_loading: "Loading pods...",
    status_env_done: "Initialization complete.",
    status_env_incomplete: "Some checks failed. Complete the configuration and press r to retry.",
    status_pods_loaded: "Loaded {} pod(s).",
    status_no_pods: "No pods available. Create one first.",
    status_login_ended: "Pod login session ended.",
    status_forward_started: "Port forwarding started.",
    status_forward_stopped: "Port forwarding stopped.",
    status_forward_failed: "Port forwarding stopped: {}",
    status_pod_range: "Pod number must be between 1 and {}.",
    status_installing: "Installing pod, please wait...",
    status_installed: "Pod installed successfully.",
    status_user_saved: "User information saved. Recheck the environment to view details.",
    status_uninstalled: "Pod {} - release {} uninstalled.",
    status_uninstall_cancelled: "Uninstall cancelled.",
    err_io: "System error: {}",
    err_missing_tool: "Command {} was not found. Install it and add it to PATH.",
    err_missing_kubeconfig: "Place your config file at: {}",
    err_command_failed: "Command {} failed: {}",
    err_pod_not_found: "Pod {} was not found.",
    invalid_kube_user: "The active config context has no valid username. Check the configuration.",
    invalid_saved_credentials: "Invalid account or password. Use Update user information to save it again; passwords cannot be empty or contain control characters.",
    setup_prerequisite: "Skipped because a prerequisite check failed.",
    invalid_platform: "Automatic installation supports macOS, Linux and Windows on x86_64/arm64.",
    invalid_download: "Download or version verification failed; nothing was installed. Retry setup.",
    invalid_pod_name: "Pod name may only contain lowercase letters and digits.",
    invalid_cpu: "CPU cores must be a whole number between 1 and 255.",
    invalid_memory: "Memory GB must be a whole number between 1 and 255.",
    invalid_no_release: "This pod has no Helm release label and cannot be uninstalled.",
    invalid_release_changed: "This pod's Helm release changed. Select the pod again.",
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
        Invalid::NoReleaseLabel => t.invalid_no_release,
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
        assert!(error_text(&error, Lang::En).starts_with("Command helm"));
        assert_ne!(
            error_text(&ThumedError::Invalid(Invalid::PodName), Lang::Zh),
            error_text(&ThumedError::Invalid(Invalid::PodName), Lang::En)
        );
    }
}
