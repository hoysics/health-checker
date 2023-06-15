use crate::core::alarm::*;
use crate::core::doctor::*;
use crate::core::ent::*;
use std::collections::HashMap;
// 记录数据同时判断是否需要报警
pub struct Logger {
    nodes: HashMap<String, Node>,       //存储原始的节点信息
    services: HashMap<String, Service>, //存储原始的服务信息
    dc: Doctor,
    alarm: Alarm,
}

impl Logger {
    pub fn new(dc: Doctor, alarm: Alarm) -> Logger {
        Logger {
            nodes: HashMap::new(),
            services: HashMap::new(),
            dc,
            alarm,
        }
    }
    pub fn log(&mut self, event: Event) {
        //TODO: 带颜色的打印控制台日志
        //1. 打印日志 记录节点状态
        //2. 将节点状况计入Map中
        //3. 根据不同情况 决定是否立即邮件抱紧
        match event {
            Event::Heartbeat(health) => {
                match health.status {
                    HealthStatus::Green => tracing::info!("recv heartbeat: need nothing"),
                    HealthStatus::Yellow => tracing::info!("recv heartbeat: need warning"),
                    HealthStatus::Red => {
                        tracing::info!("recv heartbeat: it's error, notify now");
                        self.alarm.notify(vec![health.clone()]);
                    }
                }
                match health.target {
                    Target::Node(id, node) => self.update_node(id, node),
                    Target::Service(name, service) => self.update_service(name, service),
                };
            }
            Event::Offline(target) => self.offline(target),
            Event::CheckAll => self.tranverse_check(),
        };
    }
    fn update_node(&mut self, id: String, node: Option<Node>) {
        tracing::info!("try to update node {:?},{:?}", id, node);
        match node {
            Some(node) => {
                self.nodes.insert(id, node);
            }
            None => tracing::info!("update node fail, no node info"),
        };
    }
    fn update_service(&mut self, name: String, service: Option<Service>) {
        tracing::info!("try to update service {:?},{:?}", name, service);
        match service {
            Some(service) => {
                self.services.insert(name, service);
            }
            None => tracing::info!("update service fail, no node info"),
        };
    }
    fn offline(&mut self, target: Target) {
        match target {
            Target::Node(id, _) => {
                let node = self.nodes.remove(&id).unwrap();
                tracing::info!("node offline {:?}", node);
            }
            Target::Service(name, _) => {
                let service = self.services.remove(&name).unwrap();
                tracing::info!("service offline {:?}", service);
            }
        };
    }
    fn tranverse_check(&self) {
        let mut result: Vec<HealthInfo> = Vec::new();
        //TODO: 检查节点 遍历Map 检查每个节点的健康状态
        for (id, node) in self.nodes.iter() {
            let (status, msg) = self.dc.check_node(node);
            let mut n = node.clone();
            n.status_msg = Some(msg);
            result.push(HealthInfo {
                target: Target::Node(String::from(id), Some(n)),
                status,
            });
        }
        //TODO: 检查服务
        //1. 将状态先输出至单独的本地文件 用以留档
        tracing::info!("finished check all nodes\n{:?}", result);
        //2. 发邮件通知当前的文件健康状态
        self.alarm.notify(result);
    }
}
