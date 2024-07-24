pub mod messaging;
pub mod queue;

use actix_web::{web, App, HttpServer, Result};
use messaging::{MsgType, Request, Response};
use queue::{MsgQueuePool, Queue};
use std::process::exit;

const MY_ID: i32 = 5000;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let queue = web::Data::new(MsgQueuePool::new());

    HttpServer::new(move || {
        App::new()
            .app_data(queue.clone())
            .route("/", web::post().to(submit))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}

async fn submit(msg: web::Json<Request>, queue: Queue) -> Result<web::Json<Response>> {
    let msg = msg.into_inner();
    let res = treat_msg(msg, queue);

    Ok(web::Json(res))
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
