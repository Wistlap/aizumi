#![allow(clippy::uninlined_format_args)]
#![deny(unused_qualifications)]

use std::io::Cursor;
use std::sync::Arc;

use actix_web::middleware;
use actix_web::middleware::Logger;
use actix_web::web;
use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::HttpServer;
use actix_web::Responder;
use actix_web::Result;
use openraft::Config;

use messaging::MsgType;
use queue::{MsgQueuePool, Queue};
use std::process::exit;

use crate::app::App;
use crate::network::management;
use crate::network::raft;
use crate::network::Network;
use messaging::Request;
use messaging::Response;

pub mod app;
pub mod client;
pub mod network;
pub mod store;

pub mod messaging;
pub mod queue;
#[cfg(test)]
mod test;

pub type NodeId = u64;

const MY_ID: i32 = 5000;

openraft::declare_raft_types!(
    /// Declare the type configuration for example K/V store.
    pub TypeConfig:
        D = Request,
        R = Response,
);

pub type LogStore = store::LogStore<TypeConfig>;
pub type StateMachineStore = store::StateMachineStore;
pub type Raft = openraft::Raft<TypeConfig>;

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

pub async fn start_example_raft_node(node_id: NodeId, http_addr: String) -> std::io::Result<()> {
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
    let raft = openraft::Raft::new(
        node_id,
        config.clone(),
        network,
        log_store.clone(),
        state_machine_store.clone(),
    )
    .await
    .unwrap();

    // Create an application that will store all the instances created above, this will
    // later be used on the actix-web services.
    let app_data = Data::new(App {
        id: node_id,
        addr: http_addr.clone(),
        raft,
        log_store,
        state_machine_store,
        config,
    });

    let queue = Data::new(MsgQueuePool::new());

    // Start the actix-web server.
    let server = HttpServer::new(move || {
        actix_web::App::new()
            .wrap(Logger::default())
            .wrap(Logger::new("%a %{User-Agent}i"))
            .wrap(middleware::Compress::default())
            .app_data(app_data.clone())
            .app_data(queue.clone())
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
            // .service(api::write)
            // .service(api::read)
            // .service(api::consistent_read)
            .route("/", web::post().to(submit))
    });

    let x = server.bind(http_addr)?;

    x.run().await
}

pub async fn submit(app: Data<App>, req: Json<Request>) -> actix_web::Result<impl Responder> {
    let response = app.raft.client_write(req.0).await;
    Ok(Json(response))
}

fn treat_msg(msg: Request, queue: Queue) -> Response {
    match msg.msg_type {
        MsgType::MSG_SEND_REQ => treat_send_req(msg, queue),
        MsgType::MSG_RECV_REQ => treat_recv_req(msg, queue),
        MsgType::MSG_FREE_REQ => treat_free_req(msg, queue),
        MsgType::MSG_PUSH_REQ => treat_push_req(msg, queue),
        MsgType::MSG_HELO_REQ => treat_helo_req(msg, queue),
        MsgType::MSG_STAT_REQ => treat_stat_req(msg, queue),
        MsgType::MSG_GBYE_REQ => treat_gbye_req(msg, queue),
        _ => {
            eprintln!("Unexpected MsgType: {:?}", msg.msg_type);
            exit(1)
        }
    }
}

/// Add msg to queue and return SEND_ACK
fn treat_send_req(msg: Request, queue: Queue) -> Response {
    let ack = Response::new(
        MsgType::MSG_SEND_ACK,
        MY_ID,
        msg.saddr,
        msg.id,
        String::default(),
    );
    let queue_id = msg.daddr;
    queue.enqueue(msg, queue_id);
    ack
}

/// Pop msg from queue and return RECV_ACK
fn treat_recv_req(msg: Request, queue: Queue) -> Response {
    let msg = queue.dequeue(msg.saddr);
    if let Some(msg) = msg {
        Response::new(
            MsgType::MSG_RECV_ACK,
            msg.saddr,
            msg.daddr,
            msg.id,
            msg.payload,
        )
    } else {
        Response {
            code: 404,
            status: String::from("Not Found"),
            ..Response::default()
        }
    }
}

/// Return FREE_ACK
fn treat_free_req(msg: Request, _queue: Queue) -> Response {
    Response::new(
        MsgType::MSG_FREE_ACK,
        MY_ID,
        msg.saddr,
        msg.id,
        String::default(),
    )
}

/// Not support
fn treat_push_req(_msg: Request, _queue: Queue) -> Response {
    Response {
        code: 400,
        status: String::from("Bad Request"),
        ..Response::default()
    }
}

/// Return HELO_ACK
fn treat_helo_req(msg: Request, _queue: Queue) -> Response {
    Response::new(
        MsgType::MSG_HELO_ACK,
        MY_ID,
        msg.saddr,
        msg.id,
        String::default(),
    )
}

/// Return STAT_RES
fn treat_stat_req(msg: Request, _queue: Queue) -> Response {
    Response::new(
        MsgType::MSG_STAT_RES,
        MY_ID,
        msg.saddr,
        msg.id,
        // serde_json::to_string(&_queue.status()).unwrap(),
        serde_json::to_string("").unwrap(),
    )
}

/// Return GBYE_ACK
fn treat_gbye_req(msg: Request, queue: Queue) -> Response {
    let ack = Response::new(
        MsgType::MSG_GBYE_ACK,
        MY_ID,
        msg.saddr,
        msg.id,
        String::default(),
    );
    let queue_id = msg.saddr;
    queue.remove_queue(queue_id);
    ack
}
