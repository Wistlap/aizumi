use actix_web::{web, App, HttpServer, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::exit;
use std::sync::{Arc, Mutex, RwLock};

const MY_ID: i32 = 5000;

#[derive(Serialize, Deserialize, Debug)]
struct Request {
    msg_type: MsgType,
    saddr: i32,
    daddr: i32,
    id: i32,
    payload: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Response {
    code: i32,
    status: String,
    msg_type: MsgType,
    saddr: i32,
    daddr: i32,
    id: i32,
    payload: String,
}

impl Response {
    fn new(msg_type: MsgType, saddr: i32, daddr: i32, id: i32, payload: String) -> Self {
        Self {
            msg_type,
            saddr,
            daddr,
            id,
            payload,
            ..Default::default()
        }
    }
}

impl Default for Response {
    fn default() -> Self {
        Self {
            code: 200,
            status: String::from("OK"),
            msg_type: MsgType::default(),
            saddr: 0,
            daddr: 0,
            id: 0,
            payload: String::default(),
        }
    }
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

type Queue = web::Data<MsgQueue>;

struct MsgQueue {
    hash: RwLock<HashMap<i32, Arc<Mutex<Vec<Request>>>>>,
}

impl MsgQueue {
    fn new() -> Self {
        Self {
            hash: RwLock::new(HashMap::new()),
        }
    }

    /// If specified queue_id's queue already exists, enqueue data (Request).
    /// Else, create queue with specified queue_id, and enqueue data.
    fn enqueue(&self, data: Request, queue_id: i32) {
        if let Some(queue) = self.hash.read().unwrap().get(&queue_id) {
            queue.lock().unwrap().push(data);
            return;
        }
        let queue = Arc::new(Mutex::new(vec![data]));
        self.hash.write().unwrap().insert(queue_id, queue);
    }

    /// Dequeue specified id's data (Request) from specified queue_id's queue.
    /// If that data does not exist, return None.
    #[allow(clippy::manual_map)]
    fn dequeue_with_id(&self, queue_id: i32, msg_id: i32) -> Option<Request> {
        if let Some(queue) = self.hash.read().unwrap().get(&queue_id) {
            let mut queue = queue.lock().unwrap();
            if let Some(idx) = queue.iter().position(|msg| msg.id == msg_id) {
                Some(queue.remove(idx))
            } else {
                None
            }
        } else {
            None
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let queue = web::Data::new(MsgQueue::new());

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
    let msg = queue.dequeue_with_id(msg.saddr, msg.id);
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
        String::from("stat res"),
    )
}

/// Return GBYE_ACK
fn treat_gbye_req(msg: Request, _queue: Queue) -> Response {
    Response::new(
        MsgType::MSG_GBYE_ACK,
        MY_ID,
        msg.saddr,
        msg.id,
        String::default(),
    )
}
