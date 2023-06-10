//! Provides a RESTful web server managing some nodes.
//!
//! API will be:
//!
//! - `GET /nodes`: return a JSON list of nodes.
//! - `POST /nodes`: create a new Node.
//! - `PATCH /nodes/:id`: update a specific Node.
//! - `DELETE /nodes/:id`: delete a specific Node.
//!
//! Run with
//!
//! ```not_rust
//! cargo run -p example-nodes
//! ```

mod core;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::core::*;
use tokio::sync::mpsc;
use tokio::time;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "example_nodes=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    //TODO: 初始化配置
    //TODO: 根据配置生成医生
    let dc = Doctor::new();
    let dc1 = dc.clone();
    let dc2 = dc.clone();
    // 节点监听服务用的channel
    let (tx1, mut rx1) = mpsc::channel(32);
    let tx2 = tx1.clone();
    // 定时器用的channel
    // let (tx1, mut rx2) = mpsc::channel(32);
    //2. 启动用于监听节点状态和服务状态的任务
    tokio::spawn(async move {
        // Init Monitor
        let mut monitor = Logger::new(dc1);
        println!("into watch");
        loop {
            tokio::select! {
                Some(event)=rx1.recv()=> monitor.log(event),
                else => { break }
            };
        }
        println!("break watch");
    });
    //3. 启动用于轮询各服务Health接口的任务
    //   同时此任务负责定时通知Monitor遍历节点以检查有哪些节点超时未更新
    let mut services = Vec::new();
    services.push(Service {
        name: "rust website".to_string(),
        api: "https://www.rust-lang.org".to_string(),
        latency: 0,
        last_updated: 0,
    });
    tokio::spawn(async move {
        let mut should_export = 0;
        //TODO: Graceful Shutdown
        loop {
            println!("linglingling");
            time::sleep(time::Duration::from_secs(10)).await;
            for srv in &services {
                println!("service = {:?}", srv);
                let (status, msg) = dc2.check_service(&srv.api).await;
                println!("check result = {:?} {:?}", status, msg);
                tx2.send(Event::Heartbeat(HealthInfo {
                    target: Target::Service(
                        String::from(&srv.name),
                        Some(Service {
                            name: String::from(&srv.name),
                            api: String::from(&srv.api),
                            latency: srv.latency,
                            last_updated: SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap()
                                .as_secs(),
                        }),
                    ),
                    status: status,
                }))
                .await
                .unwrap();
            }
            // 每轮询5次 触发一次全局汇报
            should_export += 1;
            if should_export == 5 {
                tx2.send(Event::CheckAll).await.unwrap();
                should_export = 0;
            }
        }
    });

    //4. 启动监听节点健康状况的服务
    collector::listen(tx1, dc).await;
}
