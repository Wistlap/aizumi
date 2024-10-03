use crate::queue::{MsgQueue, MsgQueuePool};
use serde::{Deserialize, Serialize};
use std::process::exit;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Request {
    pub msg_type: MsgType,
    pub saddr: i32,
    pub daddr: i32,
    pub id: i32,
    pub payload: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Response {
    pub code: i32,
    pub status: String,
    pub msg_type: MsgType,
    pub saddr: i32,
    pub daddr: i32,
    pub id: i32,
    pub payload: String,
}

impl Request {
    pub fn new(msg_type: MsgType, saddr: i32, daddr: i32, id: i32, payload: String) -> Self {
        Self {
            msg_type,
            saddr,
            daddr,
            id,
            payload,
        }
    }
}

impl Response {
    pub fn new(msg_type: MsgType, saddr: i32, daddr: i32, id: i32, payload: String) -> Self {
        Self {
            msg_type,
            saddr,
            daddr,
            id,
            payload,
            ..Default::default()
        }
    }

    pub fn create_error_response() -> Self {
        Self {
            code: 500,
            status: String::from("Internal Server Error"),
            msg_type: MsgType::default(),
            saddr: 0,
            daddr: 0,
            id: 0,
            payload: String::default(),
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
pub enum MsgType {
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

pub fn treat_msg(msg: Request, queue: &mut MsgQueuePool, id: i32) -> Response {
    match msg.msg_type {
        MsgType::MSG_SEND_REQ => treat_send_req(msg, queue, id),
        MsgType::MSG_RECV_REQ => treat_recv_req(msg, queue, id),
        MsgType::MSG_FREE_REQ => treat_free_req(msg, queue, id),
        MsgType::MSG_PUSH_REQ => treat_push_req(msg, queue, id),
        MsgType::MSG_HELO_REQ => treat_helo_req(msg, queue, id),
        MsgType::MSG_STAT_REQ => treat_stat_req(msg, queue, id),
        MsgType::MSG_GBYE_REQ => treat_gbye_req(msg, queue, id),
        _ => {
            eprintln!("Unexpected MsgType: {:?}", msg.msg_type);
            exit(1)
        }
    }
}

/// Add msg to queue and return SEND_ACK
fn treat_send_req(msg: Request, queue: &mut MsgQueuePool, id: i32) -> Response {
    let ack = Response::new(
        MsgType::MSG_SEND_ACK,
        id,
        msg.saddr,
        msg.id,
        String::default(),
    );
    let queue_id = msg.daddr;
    queue.enqueue(msg, queue_id);
    ack
}

/// Pop msg from queue and return RECV_ACK
fn treat_recv_req(msg: Request, queue: &mut MsgQueuePool, _id: i32) -> Response {
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
fn treat_free_req(msg: Request, _queue: &mut MsgQueuePool, id: i32) -> Response {
    Response::new(
        MsgType::MSG_FREE_ACK,
        id,
        msg.saddr,
        msg.id,
        String::default(),
    )
}

/// Not support
fn treat_push_req(msg: Request, queue: &mut MsgQueuePool, id: i32) -> Response {
    let msg = queue.dequeue(msg.daddr);
    if let Some(msg) = msg {
        Response::new(
            MsgType::MSG_PUSH_REQ,
            id,
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
    // Response {
    //     code: 400,
    //     status: String::from("Bad Request"),
    //     ..Response::default()
    // }
}

/// Return HELO_ACK
fn treat_helo_req(msg: Request, queue: &mut MsgQueuePool, id: i32) -> Response {
    if queue.is_exist(&msg.saddr) {
        queue.add_queue(msg.saddr, MsgQueue::new());
    }
    Response::new(
        MsgType::MSG_HELO_ACK,
        id,
        msg.saddr,
        msg.id,
        String::default(),
    )
}

/// Return STAT_RES
fn treat_stat_req(msg: Request, _queue: &mut MsgQueuePool, id: i32) -> Response {
    Response::new(
        MsgType::MSG_STAT_RES,
        id,
        msg.saddr,
        msg.id,
        // serde_json::to_string(&_queue.status()).unwrap(),
        serde_json::to_string("").unwrap(),
    )
}

/// Return GBYE_ACK
fn treat_gbye_req(msg: Request, queue: &mut MsgQueuePool, id: i32) -> Response {
    let ack = Response::new(
        MsgType::MSG_GBYE_ACK,
        id,
        msg.saddr,
        msg.id,
        String::default(),
    );
    let queue_id = msg.saddr;
    queue.remove_queue(queue_id);
    ack
}
