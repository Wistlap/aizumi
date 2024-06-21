use actix_web::{get, post, web, App, HttpServer, Result};
use serde::{Deserialize, Serialize};
use serde_json;

#[derive(Serialize, Deserialize)]
struct Request {
    msg_type: MsgType,
    saddr: i32,
    daddr: i32,
    id: i32,
    payload: String,
}

#[derive(Serialize, Deserialize)]
struct Response {
    code: i32,
    status: String,
    msg_type: MsgType,
    saddr: i32,
    daddr: i32,
    id: i32,
    payload: String,
}

#[derive(Serialize, Deserialize)]
enum MsgType {
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

#[post("/")]
async fn submit(info: web::Json<Request>) -> Result<web::Json<Response>> {
    let res = Response {
        code: 200,
        status: String::from("OK"),
        msg_type: MsgType::MSG_SEND_REQ,
        saddr: 1,
        daddr: 100,
        id: 10000,
        payload: String::from("hello"),
    };
    Ok(web::Json(res))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().service(submit))
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}
