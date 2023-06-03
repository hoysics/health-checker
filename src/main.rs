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

use axum::{
    error_handling::HandleErrorLayer,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get,delete},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, RwLock},
    time::Duration,
};
use tokio::sync::mpsc;
use tower::{BoxError, ServiceBuilder};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
// use std::time::{SystemTime, UNIX_EPOCH};

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
    //TODO 根据配置生成医生
    let dc=Doctor::new();
    // 节点监听服务用的channel
    let (tx1, mut rx1) = mpsc::channel(32);
    // 定时器用的channel
    // let (tx1, mut rx2) = mpsc::channel(32);
    tokio::spawn(async move {
        // Init Monitor
        let monitor=Monitor::new();
        println!("into watch");
        loop {
            tokio::select! {
                Some(event)=rx1.recv()=> monitor.log(event),
                else => { break }
            };
        }
        println!("break watch");
    });
    // let db = Db::default();
    let app_state = Arc::new(AppState {
        db: RwLock::new(HashMap::new()),
        tx:tx1,
        dc ,
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
                .timeout(Duration::from_secs(10))
                .layer(TraceLayer::new_for_http())
                .into_inner(),
        )
        .with_state(app_state);

    // Init server & wait
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

// The query parameters for nodes index
#[derive(Debug, Deserialize, Default)]
pub struct Pagination {
    pub offset: Option<usize>,
    pub limit: Option<usize>,
}

async fn nodes_index(
    pagination: Option<Query<Pagination>>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let nodes = state.db.read().unwrap();

    let Query(pagination) = pagination.unwrap_or_default();

    let nodes = nodes
        .values()
        .skip(pagination.offset.unwrap_or(0))
        .take(pagination.limit.unwrap_or(usize::MAX))
        .cloned()
        .collect::<Vec<_>>();

    Json(nodes)
}

#[derive(Debug, Deserialize)]
struct UpsertNode {
    system_hostname: String,
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
}

async fn node_upsert(
    State(state): State<Arc<AppState>>,
    Json(input): Json<UpsertNode>,
) -> impl IntoResponse {
    let todo = Node {
        id: input.system_hostname,
        system_ip: input.system_ip,
        time_day: input.time_day,
        load_1: input.load_1,
        load_5: input.load_5,
        load_15: input.load_15,
        mem_status_total: input.mem_status_total,
        mem_status_use: input.mem_status_use,
        mem_status_per: input.mem_status_per,
        mem_status: input.mem_status,
        disk_f: input.disk_f,
        disk_total: input.disk_total,
        disk_free: input.disk_free,
        disk_per: input.disk_per,
        disk_f_60: input.disk_f_60,
        disk_per_60: input.disk_per_60,
        disk_status: input.disk_status,
    };
    state
        .db
        .write()
        .unwrap()
        .insert(String::from(&todo.id), todo.clone());
    //TODO: check health. if unhealth, tell node.
    let health=state.dc.check_node(&todo);
    let tx=state.tx.clone();
    match health {
        Health::Green=> {
            tx.send(NodeEvent::Trace(todo.clone())).await.unwrap();
        },
        Health::Yellow => {
            tx.send(NodeEvent::Warn(todo.clone())).await.unwrap();
        },
        Health::Red=>{
            tx.send(NodeEvent::Error(todo.clone())).await.unwrap();
        },
    };
    (StatusCode::OK, Json(todo))
}


async fn node_delete(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let tx=state.tx.clone();
    tx.send(NodeEvent::Offline(id.clone())).await.unwrap();
    if state.db.write().unwrap().remove(&id).is_some() {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    }
}

struct AppState {
    db: RwLock<HashMap<String, Node>>,
    tx: mpsc::Sender<NodeEvent>,
    dc: Doctor,
}


#[derive(Debug, Serialize, Clone)]
struct Node {
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
}

#[derive(Debug)]
enum Health {
    Red,
    Yellow,
    Green,
}

#[derive(Debug)]
enum NodeEvent {
    Trace(Node),
    Warn(Node),
    Error(Node),
    Offline(String),
}

// 记录数据同时判断是否需要报警
struct Monitor {
    db: HashMap<String, Node>,//存储原始的节点信息
}

impl Monitor {
    fn new() ->Monitor{
        Monitor {db:HashMap::new()}
    }
    fn log(&self,event: NodeEvent){
        match event {
            NodeEvent::Trace(node)=>{
                println!("node work good: {:?}",node);
            },
            NodeEvent::Warn(node)=>{
                println!("node warning: {:?}",node);

            },
            NodeEvent::Error(node)=>{
                println!("node overload: {:?}",node);

            },
            NodeEvent::Offline(id)=>{

                println!("node({:?}) get offline",id);
            },
        };
    }
    fn check_nodes(&self){

    }
    // fn trace(&self,node: &Node){
    //
    // }
    // fn warn(&self,node: &Node){
    //
    // }
    // fn error(&self,node: &Node){
    //
    // }
    // fn offline(&self,node: String){
    //
    // }
}
//TODO: 由配置文件加载的节点健康状态判断
struct Doctor {

}

impl Doctor {
    fn new()->Doctor {
        Doctor{}
    }
    fn check_node(&self,node: &Node) ->Health{
        if node.load_1>80 {
            return Health::Red
        }
        if node.load_1>60 {
            return Health::Yellow
        }
        Health::Green
    }
    fn check_service(&self){ }
}
