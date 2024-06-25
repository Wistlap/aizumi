use actix_web::{post, web, App, HttpServer, Result};
use serde::{Deserialize, Serialize};
use std::process::exit;
use std::sync::Mutex;

type Queue = web::Data<Mutex<Vec<Request>>>;

#[derive(Serialize, Deserialize, Debug)]
struct Request {
    msg_type: MsgType,
    saddr: i32,
    daddr: i32,
    id: i32,
    payload: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct Response {
    code: i32,
    status: String,
    msg_type: MsgType,
    saddr: i32,
    daddr: i32,
    id: i32,
    payload: String,
}

#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default)]
enum MsgType {
    #[default]
    MSG_SEND_REQ = 1,
    MSG_SEND_ACK = 2,
    MSG_RECV_REQ = 3,
    MSG_RECV_ACK = 4,
    MSG_FREE_REQ = 5,
    MSG_FREE_ACK = 6,
    MSG_PUSH_REQ = 7,
    MSG_PUSH_ACK = 8,
    MSG_HELO_REQ = 9,
    MSG_HELO_ACK = 10,
    MSG_STAT_REQ = 11,
    MSG_STAT_RES = 12,
    MSG_GBYE_REQ = 13,
    MSG_GBYE_ACK = 14,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let queue = web::Data::new(Mutex::new(Vec::<Request>::new()));

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

fn treat_send_req(msg: Request, queue: Queue) -> Response {
    queue.lock().unwrap().push(msg);
    Response::default()
}
fn treat_recv_req(msg: Request, queue: Queue) -> Response {
    Response::default()
}
fn treat_free_req(msg: Request, queue: Queue) -> Response {
    Response::default()
}
fn treat_push_req(msg: Request, queue: Queue) -> Response {
    Response::default()
}
fn treat_helo_req(msg: Request, queue: Queue) -> Response {
    Response::default()
}
fn treat_stat_req(msg: Request, queue: Queue) -> Response {
    Response::default()
}
fn treat_gbye_req(msg: Request, queue: Queue) -> Response {
    Response::default()
}
