use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct Node {
    pub id: String,
    pub time_day: String,
    pub system_ip: String,
    pub load_1: u32,
    pub load_5: u32,
    pub load_15: f32,
    pub mem_status_total: String,
    pub mem_status_use: String,
    pub mem_status_per: u32,
    pub mem_status: String,
    pub disk_f: String,
    pub disk_total: String,
    pub disk_free: String,
    pub disk_per: u32,
    pub disk_f_60: String,
    pub disk_per_60: String,
    pub disk_status: String,
    pub last_updated: u64,
    pub status_msg: Option<String>,
}

#[derive(Debug)]
pub struct Service {
    pub host: String,
    pub api: String,
    pub name: String,
    pub latency: u128,
    pub last_updated: u64,
}

#[derive(Debug)]
pub enum HealthStatus {
    Red,
    Yellow,
    Green,
}

#[derive(Debug)]
pub enum Target {
    Node(String, Option<Node>),
    Service(String, Option<Service>),
}

#[derive(Debug)]
pub enum Event {
    Heartbeat(HealthInfo),
    Offline(Target),
}

#[derive(Debug)]
pub struct HealthInfo {
    pub target: Target,
    pub status: HealthStatus,
}
