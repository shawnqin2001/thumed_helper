// Constants module for THU Med Login Helper

// Default values for pod configuration
pub const DEFAULT_CPU_CORES: u8 = 16;
pub const DEFAULT_MEMORY_GB: u8 = 50;

// Server address and URLs
// pub const SERVER_IP: &str = "166.111.153.65";
pub const HELM_REPO_URL: &str = "http://166.111.153.65:7001";
// pub const WEBSITE_DOMAIN: &str = "apps.med.thu";

// Helm repositories and chart
pub const HELM_REPO_NAME: &str = "med-helm";
pub const HELM_CHART: &str = "med-helm/med";

// Default application name
pub const APP_NAME: &str = "THU-Med Cluster Helper";
pub const APP_VERSION: &str = "Lecture version";

pub const HELM_VALUES_TEMPLATE: &str = r#"replicaCount: 1
mode: deployment

image:
  repository: base.med.thu/public/r-4.6
  pullPolicy: Always
  tag: "v1"

containerName: "{container_name}"

service:
  type: ClusterIP
  port: 8787

resources:
  cpu: "{cpu}"
  memory: "{memory}"

imageCredentials:
  registry: base.med.thu
  username: {username}
  password: {password}

loadDataPath:
  public:
    - "input"
    - "lessonPublic"
  personal:
    - {username}

type: centos

nfs: "Aries"

transfer: false
"#;
