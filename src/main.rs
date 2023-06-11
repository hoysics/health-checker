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

mod config;
mod core;

use crate::config::model::{load_bootstrap_config, Bootstrap};
use crate::core::*;
// use lazy_static::lazy_static;
use tokio::sync::mpsc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// lazy_static! {
//     pub static ref BOOTSTRAP: Bootstrap = load_bootstrap_config().unwrap();
// }

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
    let config = load_bootstrap_config().unwrap();
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
        let mut logger = Logger::new(dc1);
        println!("into watch");
        loop {
            tokio::select! {
                Some(event)=rx1.recv()=> logger.log(event),
                else => { break }
            };
        }
        println!("break watch");
    });
    //3. 启动用于轮询各服务Health接口的任务
    //   同时此任务负责定时通知Monitor遍历节点以检查有哪些节点超时未更新
    tokio::spawn(async move {
        let srv_caller = ServiceChecker::new(dc2, tx2, config.services);
        srv_caller.turn_on().await;
    });

    //4. 启动监听节点健康状况的服务
    collector::listen(tx1, dc, config.server).await;
}
