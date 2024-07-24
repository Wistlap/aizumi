use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Request {
    pub msg_type: MsgType,
    pub saddr: i32,
    pub daddr: i32,
    pub id: i32,
    pub payload: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Response {
    pub code: i32,
    pub status: String,
    pub msg_type: MsgType,
    pub saddr: i32,
    pub daddr: i32,
    pub id: i32,
    pub payload: String,
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
