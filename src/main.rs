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

use crate::config::model::load_bootstrap_config;
use crate::core::*;
// use lazy_static::lazy_static;
use tokio::sync::{broadcast, mpsc};
use tokio::{signal, time};
// use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// lazy_static! {
//     pub static ref BOOTSTRAP: Bootstrap = load_bootstrap_config().unwrap();
// }

// The query parameters for nodes index
use axum::{
    error_handling::HandleErrorLayer,
    http::StatusCode,
    routing::{delete, get},
    Router,
};
use tower_http::trace::TraceLayer;

use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, RwLock},
};
use tower::{BoxError, ServiceBuilder};

#[tokio::main]
async fn main() {
    // tracing_subscriber::registry()
    //     .with(
    //         tracing_subscriber::EnvFilter::try_from_default_env()
    //             .unwrap_or_else(|_| "example_nodes=debug,tower_http=debug".into()),
    //     )
    //     .with(tracing_subscriber::fmt::layer())
    //     .init();
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    //1. 初始化配置
    let config = load_bootstrap_config().unwrap();
    //2. 生成医生
    let dc = Doctor::new();
    let dc1 = dc.clone();
    let dc2 = dc.clone();
    // 节点监听服务用的channel
    let (in_pipe, mut out_pipe) = mpsc::channel(32);
    let in_1 = in_pipe.clone();
    let in_2 = in_pipe.clone();
    // 退出通知
    let (shuntdown_tx, mut shutdown_rx) = broadcast::channel(16);
    let mut shutdown_rx2 = shuntdown_tx.subscribe();
    // 定时器用的channel
    // let (tx1, mut rx2) = mpsc::channel(32);
    //3. 启动用于监听节点状态和服务状态的任务
    tokio::spawn(async move {
        // Init Monitor
        let mut logger = Logger::new(dc1, alarm::Alarm::new());
        tracing::info!("begin nodes watch");
        loop {
            tokio::select! {
                Some(event)=out_pipe.recv()=> logger.log(event),
                _= shutdown_rx.recv()=>{break},
                else => { break }
            };
        }
        tracing::info!("break nodes watch");
    });
    //4. 启动用于轮询各服务Health接口的任务
    //   同时此任务负责定时通知Monitor遍历节点以检查有哪些节点超时未更新
    tokio::spawn(async move {
        let mut check_count = 0;
        let srv_caller = ServiceChecker::new(dc2, in_1, config.services);
        tracing::info!("begin service watch");
        loop {
            time::sleep(time::Duration::from_secs(300)).await;
            if shutdown_rx2.try_recv().is_ok() {
                break;
            }
            srv_caller.patrol().await;
            // 每轮询10次 触发一次全局走查
            check_count += 1;
            if check_count == 10 {
                in_2.send(Event::CheckAll).await.unwrap();
                tracing::info!("call node tranverse check");
                check_count = 0;
            };
        }
        // 清理
        drop(in_2);
        srv_caller.close().await;
        tracing::info!("break service watch");
    });
    //5. 启动监听节点健康状况的服务
    let app_state = Arc::new(AppState {
        db: RwLock::new(HashMap::new()),
        tx: in_pipe,
        dc,
    });
    // Compose the routes
    let app = Router::new()
        .route("/nodes", get(nodes_index).post(node_upsert))
        .route("/nodes/:id", delete(node_delete))
        // Add middleware to all routes
        .layer(
            ServiceBuilder::new()
                .layer(HandleErrorLayer::new(|error: BoxError| async move {
                    if error.is::<tower::timeout::error::Elapsed>() {
                        Ok(StatusCode::REQUEST_TIMEOUT)
                    } else {
                        Err((
                            StatusCode::INTERNAL_SERVER_ERROR,
                            format!("Unhandled internal error: {}", error),
                        ))
                    }
                }))
                .timeout(time::Duration::from_secs(10))
                .layer(TraceLayer::new_for_http())
                .into_inner(),
        )
        .with_state(app_state);

    // Init server & wait
    let addr: SocketAddr = config
        .server
        .addr
        .parse()
        .expect("Unable to parse socket address");
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
    //6. 资源清理
    shuntdown_tx.send(()).unwrap();
    tracing::info!("quit all. bye.");
    time::sleep(time::Duration::from_secs(12)).await;
}

pub async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    println!("signal received, starting graceful shutdown");
}
