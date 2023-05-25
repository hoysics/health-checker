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
    routing::{get, patch},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, RwLock},
    time::Duration,
};
use tower::{BoxError, ServiceBuilder};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "example_nodes=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();


    // Init Monitor
    let (tx, mut rx )=mpsc::channel(32);
    tokio::spawn(async move {
        println!("event listening");
        while let Some(event) = rx.recv().await {
            match event {
                Event::NodeUpdate(node)=>{
                    println!("recv node update{:?}",node);
                }
            }
        }
    });

    // let db = Db::default();
    let app_state=Arc::new(AppState{ db: RwLock::new(HashMap::new()), tx });
    // Compose the routes
    let app = Router::new()
        .route("/nodes", get(nodes_index).post(nodes_create))
        .route("/nodes/:id", patch(nodes_update).delete(nodes_delete))
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
struct CreateNode {
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

async fn nodes_create(State(state): State<Arc<AppState>>, Json(input): Json<CreateNode>) -> impl IntoResponse {
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
    state.db.write()
        .unwrap()
        .insert(String::from(&todo.id), todo.clone());

    (StatusCode::CREATED, Json(todo))
}

#[derive(Debug, Deserialize)]
struct UpdateNode {
    time_day: Option<String>,
    system_ip: Option<String>,
    load_1: Option<u32>,
    load_5: Option<u32>,
    load_15: Option<f32>,
    mem_status_total: Option<String>,
    mem_status_use: Option<String>,
    mem_status_per: Option<u32>,
    mem_status: Option<String>,
    disk_f: Option<String>,
    disk_total: Option<String>,
    disk_free: Option<String>,
    disk_per: Option<u32>,
    disk_f_60: Option<String>,
    disk_per_60: Option<String>,
    disk_status: Option<String>,
}

async fn nodes_update(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(input): Json<UpdateNode>,
) -> Result<impl IntoResponse, StatusCode> {
    let mut todo = state.db
        .read()
        .unwrap()
        .get(&id)
        .cloned()
        .ok_or(StatusCode::NOT_FOUND)?;

    // 示例中的代码
    if let Some(time_day) = input.time_day {
        todo.time_day = time_day;
    }
    if let Some(system_ip) = input.system_ip {
        todo.system_ip = system_ip;
    }
    if let Some(load_1) = input.load_1 {
        todo.load_1 = load_1;
    }
    if let Some(load_5) = input.load_5 {
        todo.load_5 = load_5;
    }
    if let Some(load_15) = input.load_15 {
        todo.load_15 = load_15;
    }
    if let Some(mem_status_total) = input.mem_status_total {
        todo.mem_status_total = mem_status_total;
    }
    if let Some(mem_status_use) = input.mem_status_use {
        todo.mem_status_use = mem_status_use;
    }
    if let Some(mem_status_per) = input.mem_status_per {
        todo.mem_status_per = mem_status_per;
    }
    if let Some(mem_status) = input.mem_status {
        todo.mem_status = mem_status;
    }
    if let Some(disk_f) = input.disk_f {
        todo.disk_f = disk_f;
    }
    if let Some(disk_total) = input.disk_total {
        todo.disk_total = disk_total;
    }
    if let Some(disk_free) = input.disk_free {
        todo.disk_free = disk_free;
    }
    if let Some(disk_per) = input.disk_per {
        todo.disk_per = disk_per;
    }
    if let Some(disk_f_60) = input.disk_f_60 {
        todo.disk_f_60 = disk_f_60;
    }
    if let Some(disk_per_60) = input.disk_per_60 {
        todo.disk_per_60 = disk_per_60;
    }
    if let Some(disk_status) = input.disk_status {
        todo.disk_status = disk_status;
    }

    state.db.write()
        .unwrap()
        .insert(String::from(&todo.id), todo.clone());

    if is_node_unhealth(&todo) {
        let tx=state.tx.clone();
        tx.send(Event::NodeUpdate(todo.clone())).await.unwrap();

    }


    Ok(Json(todo))
}

fn is_node_unhealth(node: &Node)  -> bool{
    println!("check node {:?}",node);
    node.load_1>80
}

async fn nodes_delete(Path(id): Path<String>, State(state): State<Arc<AppState>>) -> impl IntoResponse {
    if state.db.write().unwrap().remove(&id).is_some() {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    }
}
struct AppState {
    db:RwLock<HashMap<String, Node>>,
    tx:mpsc::Sender<Event>,
}

type Db = Arc<RwLock<HashMap<String, Node>>>;

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

// Monitor
#[derive(Debug)]
enum Event {
    NodeUpdate(Node),
}


