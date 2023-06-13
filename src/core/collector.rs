use tokio::sync::mpsc;

use crate::config::model;
use crate::core::doctor::*;
use crate::core::ent::*;

use std::time::{SystemTime, UNIX_EPOCH};
// use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub struct ServiceChecker {
    db: Vec<Service>,
    dc: Doctor,
    tx: mpsc::Sender<Event>,
}

impl ServiceChecker {
    pub fn new(
        dc: Doctor,
        tx: mpsc::Sender<Event>,
        services: Vec<model::Service>,
    ) -> ServiceChecker {
        let mut db = Vec::new();
        for srv in services {
            db.push(Service {
                name: String::from(srv.name),
                api: String::from(srv.api),
                latency: 0,
                last_updated: 0,
            });
        }
        ServiceChecker { db, dc, tx }
    }
    pub async fn close(&self) {
        self.tx.closed().await;
    }
    pub async fn patrol(&self) {
        //TODO: Graceful Shutdown
        tracing::info!("begin service tranverse check");
        for srv in &self.db {
            tracing::info!("service = {:?}", srv);
            let (status, msg) = self.dc.check_service(&srv.api).await;
            tracing::info!("check result = {:?} {:?}", status, msg);
            self.tx
                .send(Event::Heartbeat(HealthInfo {
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
        tracing::info!("end service tranverse check");
    }
}
