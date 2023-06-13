use crate::core::ent::*;
use reqwest::{Client, StatusCode};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
//TODO: 由配置文件加载的节点健康状态判断
#[derive(Debug, Clone)]
pub struct Doctor {
    client: Client,
}

impl Doctor {
    pub fn new() -> Doctor {
        Doctor {
            client: Client::builder().timeout(Duration::from_secs(2)).build().unwrap(),
        }
    }
    pub fn check_node(&self, node: &Node) -> (HealthStatus, String) {
        let mut is_healthy = 0;
        let mut msg = String::from("");
        //条件1 硬盘占用率
        if node.disk_status.eq_ignore_ascii_case("不正常") {
            is_healthy += 1;
            msg.push_str("Warn: disk use too much.\n");
        }
        //条件2 内存占用率
        if node.mem_status.eq_ignore_ascii_case("不正常") {
            is_healthy += 1;
            msg.push_str("Warn: mem too high.\n");
        }
        //条件3 节点上次更新时间
        let cur_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        if cur_time - node.last_updated > 600 {
            // 大于10分钟时视为已经下线
            // 暂时不做：先检查一下服务器上端口的健康状态
            // 如果正常 则仅记录本次为节点监控失效
            // 如果不正常 则发出警告: 节点和服务均下线
            is_healthy += 2;
            msg.push_str("Error: node hasn't update too long.")
        } else if cur_time - node.last_updated > 300 {
            // 大于5分钟未更新需要警告
            is_healthy += 1;
            msg.push_str("Warn: node hasn't update for a while.")
        }
        match is_healthy {
            0 => (HealthStatus::Green, "it looks good".to_string()),
            1 => (HealthStatus::Yellow, msg),
            _ => (HealthStatus::Red, msg),
        }
    }
    pub async fn check_service(&self, url: &String) -> (HealthStatus, String) {
        let resp = self.client.get(url).send().await;
        // let resp = match res {
        //     Ok(resp) => resp,
        //     Err(err) => return (HealthStatus::Red, format!("get fail {:?}", err)),
        // };
        match resp {
            Ok(resp) => {
                if resp.status() == StatusCode::OK {
                    return (HealthStatus::Green, "success".to_string());
                }
                (HealthStatus::Red, "status code not 200".to_string())
            }
            Err(err) => (HealthStatus::Red, format!("get fail {:?}", err)),
        }
    }
}
