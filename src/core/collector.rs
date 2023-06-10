use crate::core::doctor::*;
use crate::core::ent::*;

// The query parameters for nodes index
use axum::{
    error_handling::HandleErrorLayer,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, RwLock},
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::mpsc;
use tokio::time;
use tower::{BoxError, ServiceBuilder};
use tower_http::trace::TraceLayer;
// use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub async fn listen(tx: mpsc::Sender<Event>, dc: Doctor) {
    let app_state = Arc::new(AppState {
        db: RwLock::new(HashMap::new()),
        tx: tx,
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
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

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
        status_msg: Option::None,
    };
    state
        .db
        .write()
        .unwrap()
        .insert(String::from(&todo.id), todo.clone());
    //TODO: check health. if unhealth, tell node.
    let (health, msg) = state.dc.check_node(&todo);
    todo.status_msg = Some(msg);
    let tx = state.tx.clone();
    tx.send(Event::Heartbeat(HealthInfo {
        target: Target::Node(String::from(&todo.id), Some(todo.clone())),
        status: health,
    }))
    .await
    .unwrap();
    (StatusCode::OK, Json(todo))
}

async fn node_delete(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let tx = state.tx.clone();
    tx.send(Event::Offline(Target::Node(
        String::from(&id),
        Option::None,
    )))
    .await
    .unwrap();
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
