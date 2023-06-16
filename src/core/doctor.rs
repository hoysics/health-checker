use crate::core::ent::*;
use reqwest::{Client, StatusCode};
use std::{
    cmp,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
//TODO: 由配置文件加载的节点健康状态判断
#[derive(Debug, Clone)]
pub struct Doctor {
    client: Client,
}

impl Doctor {
    pub fn new() -> Doctor {
        Doctor {
            client: Client::builder()
                .timeout(Duration::from_secs(3))
                .build()
                .unwrap(),
        }
    }
    pub fn check_node(&self, node: &Node) -> (HealthStatus, String) {
        let mut level = 0;
        let mut msg = String::from("");
        //条件1 硬盘占用率
        if node.disk_per >= 90 {
            level = 2;
            msg.push_str("Error: disk > 90% .\n");
        } else if node.disk_per >= 70 {
            level = 1;
            msg.push_str("Warn: disk > 70% .\n");
        }
        //条件2 内存占用率
        if node.mem_status_per >= 90 {
            level = cmp::max(level, 2);
            msg.push_str("Error: mem > 90% .\n");
        } else if node.mem_status_per >= 70 {
            level = cmp::max(level, 1);
            msg.push_str("Warn: mem > 70% .\n");
        }
        //条件3 节点上次更新时间
        let cur_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        if cur_time - node.last_updated > 1200 {
            // 暂时不做：先检查一下服务器上端口的健康状态
            // 如果正常 则仅记录本次为节点监控失效
            // 如果不正常 则发出警告: 节点和服务均下线
            level = cmp::max(level, 2);
            msg.push_str("Error: node hasn't update for 20 min.")
        } else if cur_time - node.last_updated > 600 {
            level = cmp::max(level, 1);
            msg.push_str("Warn: node hasn't update for 10 min.")
        }
        match level {
            0 => (HealthStatus::Green, "everything looks fine".to_string()),
            1 => (HealthStatus::Yellow, msg),
            _ => (HealthStatus::Red, msg),
        }
    }
    pub async fn check_service(&self, url: &String) -> (HealthStatus, String) {
        let resp = self.client.get(url).send().await;
        match resp {
            Ok(resp) => {
                if resp.status() == StatusCode::OK {
                    return (HealthStatus::Green, "success".to_string());
                }
                (
                    HealthStatus::Yellow,
                    "Warn: service resp statuscode not 200".to_string(),
                )
            }
            Err(err) => (
                HealthStatus::Red,
                format!("Error: service({:?}) get fail {:?}", url, err),
            ),
        }
    }
}
