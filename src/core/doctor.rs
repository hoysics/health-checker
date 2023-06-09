use crate::core::ent::*;
use std::time::{SystemTime, UNIX_EPOCH};
//TODO: 由配置文件加载的节点健康状态判断
pub struct Doctor {}

impl Doctor {
    pub fn new() -> Doctor {
        Doctor {}
    }
    pub fn check_node(&self, node: &Node) -> (HealthStatus, String) {
        if node.load_1 > 80 {
            return (HealthStatus::Red, "load too high".to_string());
        }
        if node.load_1 > 60 {
            return (HealthStatus::Yellow, "".to_string());
        }
        let cur_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        if cur_time - node.last_updated > 600 {
            // 暂时不做：先检查一下服务器上端口的健康状态
            // 如果正常 则仅记录本次为节点监控失效
            // 如果不正常 则发出警告: 节点和服务均下线
            return (HealthStatus::Red, "".to_string());
        } else if cur_time - node.last_updated > 300 {
            return (HealthStatus::Yellow, "".to_string());
        }
        (HealthStatus::Green, "".to_string())
    }
    pub fn check_service(&self) {}
}
