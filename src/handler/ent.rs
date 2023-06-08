#[derive(Debug, Serialize, Clone)]
pub struct Node {
    id: String,
    time_day: String,
    system_ip: String,
    load_1: u32,
    load_5: u32,
    load_15: f32,
    mem_status_total: String,
    mem_status_use: String,
    mem_status_per: u32,
    mem_status: String,
    disk_f: String,
    disk_total: String,
    disk_free: String,
    disk_per: u32,
    disk_f_60: String,
    disk_per_60: String,
    disk_status: String,
    last_updated: u64,
    status_msg:Option<String>,
}

#[derive(Debug)]
pub struct Service {
    host: String,
    api:String,
    name:String,
    latency: u128,
    last_updated: u64,
}

#[derive(Debug)]
pub enum HealthStatus {
    Red,
    Yellow,
    Green,
}

#[derive(Debug)]
pub enum Target {
    Node(String,Option<Node>),
    Service(String,Option<Service>),
}

#[derive(Debug)]
pub enum Event {
    Heartbeat(HealthInfo),
    Offline(Target),
}

#[derive(Debug)]
pub struct HealthInfo {
    target: Target,
    status: HealthStatus,
}
