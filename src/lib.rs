#![allow(clippy::uninlined_format_args)]
#![deny(unused_qualifications)]

pub mod app;
pub mod client;
pub mod messaging;
pub mod network;
pub mod queue;
pub mod store;
#[cfg(test)]
mod test;

use std::io::Cursor;
use std::sync::Arc;

use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use futures::future::ok;

use actix_web::middleware;
use actix_web::middleware::Logger;
use actix_web::web;
use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::HttpServer;
use actix_web::Result;

use openraft::Config;

use app::App;
use messaging::Request;
use messaging::Response;
use network::management;
use network::raft;
use network::Network;

openraft::declare_raft_types!(
    /// Declare the type configuration for example K/V store.
    pub TypeConfig:
        D = Request,
        R = Response,
);

pub type LogStore = store::LogStore<TypeConfig>;
pub type StateMachineStore = store::StateMachineStore;
pub type Raft = openraft::Raft<TypeConfig>;
pub type NodeId = u64;

pub mod typ {
    use openraft::BasicNode;

    use crate::NodeId;
    use crate::TypeConfig;

    pub type RaftError<E = openraft::error::Infallible> = openraft::error::RaftError<NodeId, E>;
    pub type RPCError<E = openraft::error::Infallible> =
        openraft::error::RPCError<NodeId, BasicNode, RaftError<E>>;

    pub type ClientWriteError = openraft::error::ClientWriteError<NodeId, BasicNode>;
    pub type CheckIsLeaderError = openraft::error::CheckIsLeaderError<NodeId, BasicNode>;
    pub type ForwardToLeader = openraft::error::ForwardToLeader<NodeId, BasicNode>;
    pub type InitializeError = openraft::error::InitializeError<NodeId, BasicNode>;

    pub type ClientWriteResponse = openraft::raft::ClientWriteResponse<TypeConfig>;
}

pub async fn start_raft_node(node_id: NodeId, http_addr: String) -> std::io::Result<()> {
    // Create a configuration for the raft instance.
    let config = Config {
        heartbeat_interval: 500,
        election_timeout_min: 1500,
        election_timeout_max: 3000,
        ..Default::default()
    };

    let config = Arc::new(config.validate().unwrap());

    // Create a instance of where the Raft logs will be stored.
    let log_store = LogStore::default();
    // Create a instance of where the Raft data will be stored.
    let state_machine_store = Arc::new(StateMachineStore::default());

    // Create the network layer that will connect and communicate the raft instances and
    // will be used in conjunction with the store created above.
    let network = Network {};

    // Create a local raft instance.
    let raft = Arc::new(openraft::Raft::new(
        node_id,
        config.clone(),
        network,
        log_store.clone(),
        state_machine_store.clone(),
    )
    .await
    .unwrap());

    // Create an application that will store all the instances created above, this will
    // later be used on the actix-web services.
    let app_data = Data::new(App {
        id: node_id,
        addr: http_addr.clone(),
        raft: Arc::clone(&raft),
        log_store,
        state_machine_store,
        config,
    });


    // start_broker() と start_axtic_web_server() を並列に呼び出す
    let axtic_web_server = tokio::spawn(start_axtic_web_server(http_addr.clone(), app_data.clone()));
    let broker = tokio::spawn(start_broker(http_addr.clone(), app_data.clone()));

    axtic_web_server.await?;
    broker.await?;

    Ok(())
}

pub async fn start_axtic_web_server(
    http_addr: String,
    app_data: Data<App>,
) -> std::io::Result<()> {

    // Start the actix-web server.
    let server = HttpServer::new(move || {
        actix_web::App::new()
            .wrap(Logger::default())
            .wrap(Logger::new("%a %{User-Agent}i"))
            .wrap(middleware::Compress::default())
            .app_data(app_data.clone())
            // raft internal RPC
            .service(raft::append)
            .service(raft::snapshot)
            .service(raft::vote)
            // admin API
            .service(management::init)
            .service(management::add_learner)
            .service(management::change_membership)
            .service(management::metrics)
            // application API
            .route("/", web::post().to(submit))
    });

    let x = server.bind(http_addr)?;

    x.run().await
}

pub async fn submit(app: Data<App>, req: Json<Request>) -> Result<Json<Response>> {
    let req = req.into_inner();
    let res = match app.raft.client_write(req).await {
        Ok(res) => res.data,
        Err(_) => Response::create_error_response(),
    };
    Ok(Json(res))
}

pub async fn start_broker(
    http_addr: String,
    app_data: Data<App>
) -> std::io::Result<()> {

    // NOTE: http 通信 と tcp通信でそれぞれ別のポートを使っている
    let listener = TcpListener::bind("127.0.0.1:21010").await?;
    // setsockopt(&listener, sockopt::ReuseAddr, &true)?;

    loop{
        let (stream, _) = listener.accept().await?;

        let app = app_data.clone();
        tokio::spawn(treat_client(stream, app));
    }
}

pub async fn treat_client(mut stream: TcpStream, app: Data<App> ) {
    let mut buf = [0; 1024];
    loop {
        // クライアントからのデータを受信
        match stream.read(&mut buf).await {
            Ok(0) => {
                println!("Client disconnected.");
                break;
            }
            Ok(n) => {
                let req: Json<Request> = Json(serde_json::from_slice(&buf[..n]).unwrap());
                let req = req.into_inner();
                let res = match app.raft.client_write(req).await {
                    Ok(res) => res.data,
                    Err(_) => Response::create_error_response(),
                };
                // resをJSON形式に変換し，クライアントに送信
                let res = serde_json::to_string(&res).unwrap();
                stream.write_all(res.as_bytes()).await.unwrap();
            }
            Err(_) => {
                println!("Error occurred, terminating connection with {}", stream.peer_addr().unwrap());
            }
        }
    }
}
