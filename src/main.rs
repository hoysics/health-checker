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
use tokio::time;
use tower::{BoxError, ServiceBuilder};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use std::time::{SystemTime, UNIX_EPOCH};

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
    let dc=Doctor::new();
    let dc1=Doctor::new();
    // 节点监听服务用的channel
    let (tx1, mut rx1) = mpsc::channel(32);
    // 定时器用的channel
    // let (tx1, mut rx2) = mpsc::channel(32);
    //2. 启动用于监听节点状态和服务状态的任务 
    tokio::spawn(async move {
        // Init Monitor
        let mut monitor=Monitor::new(dc1);
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
    let mut services=Vec::new();
    services.push("http://127.0.0.1:9000");
    tokio::spawn(async move {
        let mut interval = time::interval(time::Duration::from_secs(5));
        loop {
            interval.tick().await;
            println!("linglingling");
        }
    });

    //4. 启动监听节点健康状况的服务
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
    let mut todo = Node {
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
        last_updated: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        status_msg:Option::None,
    };
    state
        .db
        .write()
        .unwrap()
        .insert(String::from(&todo.id), todo.clone());
    //TODO: check health. if unhealth, tell node.
    let (health,msg)=state.dc.check_node(&todo);
    todo.status_msg=Some(msg);
    let tx=state.tx.clone();
    tx.send(Event::Heartbeat(HealthInfo { target: Target::Node(String::from(&todo.id),Some(todo.clone())), status: health})).await.unwrap();
    (StatusCode::OK, Json(todo))
}


async fn node_delete(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let tx=state.tx.clone();
    tx.send(Event::Offline(Target::Node(String::from(&id), Option::None))).await.unwrap();
    if state.db.write().unwrap().remove(&id).is_some() {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    }
}

struct AppState {
    db: RwLock<HashMap<String, Node>>,
    tx: mpsc::Sender<Event>,
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
    last_updated: u64,
    status_msg:Option<String>,
}

#[derive(Debug)]
struct Service {
    host: String,
    api:String,
    name:String,
    latency: u128,
    last_updated: u64,
}

#[derive(Debug)]
enum HealthStatus {
    Red,
    Yellow,
    Green,
}

#[derive(Debug)]
enum Target {
    Node(String,Option<Node>),
    Service(String,Option<Service>),
}

#[derive(Debug)]
enum Event {
    Heartbeat(HealthInfo),
    Offline(Target),
}

#[derive(Debug)]
struct HealthInfo {
    target: Target,
    status: HealthStatus,
}

// 记录数据同时判断是否需要报警
struct Monitor {
    db: HashMap<String, Node>,//存储原始的节点信息
    dc: Doctor,
}

impl Monitor {
    fn new(dc:Doctor) ->Monitor{
        Monitor {db:HashMap::new(), dc}
    }
    fn log(&self,event: Event){
        //TODO: 带颜色的打印控制台日志
        //1. 打印日志 记录节点状态
        //2. 将节点状况计入Map中
        //3. 根据不同情况 决定是否立即邮件抱紧
        match event {
            Event::Heartbeat(info) => {
                match info.target {
                    Target::Node(id, node) => self.update_node(id,node),
                    Target::Service(name, service) => self.update_service(name,service),
                };
            },
            Event::Offline(target) => self.offline(target),
        };
    }
    fn update_node(&self,id: String,node:Option<Node>) {
        
    }
    fn update_service(&self,name:String,service:Option<Service>) {
        
    }
    fn offline(&self,target:Target) {
        
    }
    fn tranverse_check(&self){
        let mut result: Vec<HealthInfo>=Vec::new();
        //TODO: 遍历Map 检查每个节点的健康状态
        for (id,node) in self.db.iter()   {
            let (status,msg)=self.dc.check_node(node);
            let mut n=node.clone();
            n.status_msg=Some(msg);
            result.push(HealthInfo { target: Target::Node(String::from(id), Some(n)), status });
        };
        //1. 将状态先输出至单独的本地文件 用以留档
        println!("finished check nodes\n{:?}",result);
        //2. 发邮件通知当前的文件健康状态
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
    fn check_node(&self,node: &Node) ->(HealthStatus,String){
        if node.load_1>80 {
            return (HealthStatus::Red,"load too high".to_string())
        }
        if node.load_1>60 {
            return (HealthStatus::Yellow,"".to_string())
        }
        let cur_time= SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        if cur_time-node.last_updated>600 {
            // 暂时不做：先检查一下服务器上端口的健康状态
            // 如果正常 则仅记录本次为节点监控失效
            // 如果不正常 则发出警告: 节点和服务均下线
            return (HealthStatus::Red,"".to_string())
        }else if cur_time-node.last_updated>300 {
            return (HealthStatus::Yellow,"".to_string())
        } 
        (HealthStatus::Green,"".to_string())
    }
    fn check_service(&self){ }
}
