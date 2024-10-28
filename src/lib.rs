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

use core::arch::x86_64::_rdtsc;
use std::collections::VecDeque;
use std::fs::{File, OpenOptions};
use std::io::{Cursor, Write};
use std::sync::Arc;
use std::os::fd::AsRawFd;

use nix::sys::socket::{self, setsockopt, sockopt, SockaddrIn};

use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::{timeout, Duration};

use actix_web::middleware;
use actix_web::middleware::Logger;
use actix_web::web::Data;
use actix_web::web::Json;
use actix_web::HttpServer;
use actix_web::Result;

use openraft::Config;

use app::App;
use messaging::Request;
use messaging::Response;
use messaging::MsgType;
use messaging::RPC;
use messaging::RpcType;
use network::management;
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
    let network = Network {
        my_addr: http_addr.clone(),
    };

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

    let mut file = File::create("timestamp.log").unwrap();
    let header = "id,msg_type,tsc\n";
    file.write_all(header.as_bytes()).unwrap();

    // start_broker() と start_axtic_web_server() を並列に呼び出す
    // let runtime = tokio::runtime::Runtime::new().unwrap();
    let axtic_web_server = tokio::spawn(start_axtic_web_server(http_addr.clone(), app_data.clone()));
    let broker = tokio::spawn(start_broker(http_addr.clone(), app_data.clone()));
    let udp_server = tokio::spawn(start_udp_serve(http_addr.clone(), app_data.clone()));
    // let _udp_server = tokio::task::spawn_blocking(move || runtime.block_on(start_udp_serve(http_addr.clone(), app_data.clone())));

    let _ = axtic_web_server.await?;
    let _ = broker.await?;
    let _ = udp_server.await?;

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
            // .service(raft::append)
            // .service(raft::snapshot)
            // .service(raft::vote)
            // admin API
            .service(management::init)
            .service(management::add_learner)
            .service(management::change_membership)
            .service(management::metrics)
            // application API
            // .route("/", web::post().to(submit))
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

pub async fn start_udp_serve(
    addr: String,
    app_data: Data<App>
) -> std::io::Result<()> {

    // Create a UDP soket with libc
    // This is needed to set sockopt (ReuseAddr) to new socket before bind
    // Std library does not provide socket creation without bind
    let socket = socket::socket(
        socket::AddressFamily::Inet,
        socket::SockType::Datagram,
        socket::SockFlag::empty(),
        None,
    )
    .unwrap();

    let mut parts = addr.split(':');
    let ip = parts.next().unwrap();
    let (octet1, octet2, octet3, octet4): (u8, u8, u8, u8) = {
        let mut parts = ip.split('.').map(|x| x.parse::<u8>().unwrap());
        (
            parts.next().unwrap(),
            parts.next().unwrap(),
            parts.next().unwrap(),
            parts.next().unwrap(),
        )
    };
    let port = parts.next().unwrap().parse().unwrap();

    // Set sockopt (ReuseAddr, ReusePort) to newly-created socket
    setsockopt(&socket, sockopt::ReuseAddr, &true).unwrap();
    setsockopt(&socket, sockopt::ReusePort, &true).unwrap();

    // Bind the socket
    socket::bind(socket.as_raw_fd(), &SockaddrIn::new(octet1, octet2, octet3, octet4, port)).unwrap();

    // // Create std's UdpSocket from nix's socket
    let socket = UdpSocket::from_std(std::net::UdpSocket::from(socket)).unwrap();

    // let socket = UdpSocket::bind(addr).await?;
    let mut buf = [0; 1024 * 32];

    tracing::info!("UDP server started at {}", addr);

    loop {
        let (n, peer) = socket.recv_from(&mut buf).await?;
        tracing::debug!("Received {} bytes from {}", n, peer);
        let rpc: RPC = bincode::deserialize(&buf[..n]).unwrap();
        match rpc.rpc_type {
            RpcType::AppendEntries => {
                tracing::debug!("Received AppendEntries RPC from {}", peer);
                let req: openraft::raft::AppendEntriesRequest<TypeConfig> = bincode::deserialize(&rpc.request).unwrap();
                tracing::debug!("Request: {:?}", req);
                let res = app_data.raft.append_entries(req).await;
                let res = bincode::serialize(&res).unwrap();
                socket.send_to(&res, peer).await?;
            }
            RpcType::InstallSnapshot => {
                tracing::debug!("Received InstallSnapshot RPC from {}", peer);
                let req: openraft::raft::InstallSnapshotRequest<TypeConfig> = bincode::deserialize(&rpc.request).unwrap();
                let res = app_data.raft.install_snapshot(req).await;
                let res = bincode::serialize(&res).unwrap();
                socket.send_to(&res, peer).await?;
            }
            RpcType::Vote => {
                tracing::debug!("Received Vote RPC from {}", peer);
                let req: openraft::raft::VoteRequest<NodeId> = bincode::deserialize(&rpc.request).unwrap();
                let res = app_data.raft.vote(req).await;
                let res = bincode::serialize(&res).unwrap();
                socket.send_to(&res, peer).await?;
            }
        }
    }
}

pub async fn start_broker(
    http_addr: String,
    app_data: Data<App>
) -> std::io::Result<()> {

    let mut parts = http_addr.split(':');
    let ip = parts.next().unwrap();
    let port = parts.next().unwrap();

    let port: u16 = port.parse().unwrap();
    let port = port + 100;

    let tcp_addr = format!("{}:{}", ip, port);

    // NOTE: http 通信 と tcp通信でそれぞれ別のポートを使っている
    let listener = TcpListener::bind(tcp_addr).await?;
    // setsockopt(&listener, sockopt::ReuseAddr, &true)?;

    loop{
        let (stream, _) = listener.accept().await?;
        // stream.set_nodelay(true)?;

        let app = app_data.clone();
        tokio::task::spawn_blocking(move || tokio::runtime::Runtime::new().unwrap().block_on(treat_client(stream, app)));
    }
}

pub async fn treat_client(mut stream: TcpStream, app: Data<App> ) {

    tracing::info!("Client {} connected.", stream.peer_addr().unwrap());

    let mut client_id = 0;
    let mut buf = [0; 1024];
    let timeout_dur = 1;
    let mut tsc_log = VecDeque::new();

    loop {
        // 1ms以内にメッセージを受信する
        let res = timeout(Duration::from_millis(timeout_dur), stream.read(&mut buf)).await;
        match res {
            Ok(Ok(0)) => {
                println!("Client disconnected.");
                break;
            }
            Ok(Ok(n)) => {
                // 受信したメッセージを Request 構造体にデシリアライズ
                let req: Request = bincode::deserialize(&buf[..n]).unwrap();

                match req.msg_type {
                    MsgType::MSG_SEND_REQ |
                    MsgType::MSG_RECV_REQ |
                    MsgType::MSG_HELO_REQ => {
                        let msg_id = req.id;
                        let msg_type = req.msg_type;
                        let tsc = unsafe { _rdtsc() };
                        tsc_log.push_back((msg_id, msg_type, tsc));

                        if client_id == 0 {
                            // クライアントIDを設定
                            client_id = req.saddr;
                        }
                        let res = match app.raft.client_write(req).await {
                            Ok(res) => res.data,
                            Err(_) => Response::create_error_response(),
                        };
                        // resをシリアライズし，クライアントに送信
                        // TODO: Request 構造体のメソッドとして to_bytes() を実装すべきか．
                        // req を &[u8] に変換
                        let raw_res = bincode::serialize(&res).unwrap();
                        let mut formatted_res:[u8; 1024] = [0; 1024];
                        formatted_res[..raw_res.len()].copy_from_slice(&raw_res);
                        // let res = serde_json::to_string(&res).unwrap();
                        stream.write_all(&formatted_res).await.unwrap();
                    }
                    MsgType::MSG_FREE_REQ => {
                        //
                    }
                    MsgType::MSG_PUSH_ACK => {
                        if !app.state_machine_store.state_machine.read().await.data.is_empty(client_id) {
                            let req = Request::new(MsgType::MSG_PUSH_REQ, app.id.try_into().unwrap(), client_id, 0, String::default());
                            let res = match app.raft.client_write(req).await {
                                // TODO: ブローカからの送信であるのに，Response を返している
                                Ok(res) => res.data,
                                Err(_) => Response::create_error_response(),
                            };
                            let msg_id = res.id;
                            let msg_type = res.msg_type;
                            let tsc = unsafe { _rdtsc() };
                            tsc_log.push_back((msg_id, msg_type, tsc));
                            // resをシリアライズし，クライアントに送信
                            // TODO: Request 構造体のメソッドとして to_bytes() を実装すべきか．
                            // req を &[u8] に変換
                            let raw_res = bincode::serialize(&res).unwrap();
                            let mut formatted_res:[u8; 1024] = [0; 1024];
                            formatted_res[..raw_res.len()].copy_from_slice(&raw_res);
                            // let res = serde_json::to_string(&res).unwrap();
                            stream.write_all(&formatted_res).await.unwrap();
                        }
                    }
                    MsgType::MSG_STAT_REQ => {
                        //
                    }
                    MsgType::MSG_GBYE_REQ => {
                        //
                    }
                    _ => {
                        //
                    }
                }
            }
            Ok(Err(_)) => {
                break;
            }
            Err(_) => {
                println!("Timeout!!!");
                // タイムアウト時の処理
                // client_id に対応する MsgQueue が空でない場合，MSG_PUSH_REQ を送信
                if !app.state_machine_store.state_machine.read().await.data.is_empty(client_id) {
                    let req = Request::new(MsgType::MSG_PUSH_REQ, app.id.try_into().unwrap(), client_id, 0, String::default());
                    let res = match app.raft.client_write(req).await {
                        // TODO: ブローカからの送信であるのに，Response を返している
                        Ok(res) => res.data,
                        Err(_) => Response::create_error_response(),
                    };
                    let msg_id = res.id;
                    let msg_type = res.msg_type;
                    let tsc = unsafe { _rdtsc() };
                    tsc_log.push_back((msg_id, msg_type, tsc));
                    // resをシリアライズし，クライアントに送信
                    // TODO: Request 構造体のメソッドとして to_bytes() を実装すべきか．
                    // req を &[u8] に変換
                    let raw_res = bincode::serialize(&res).unwrap();
                    let mut formatted_res:[u8; 1024] = [0; 1024];
                    formatted_res[..raw_res.len()].copy_from_slice(&raw_res);
                    // let res = serde_json::to_string(&res).unwrap();
                    stream.write_all(&formatted_res).await.unwrap();
                }
            }
        }
    }
    let mut file = OpenOptions::new().append(true).create(true).open("timestamp.log").unwrap();
    tsc_log.iter().for_each(|(msg_id, msg_type, tsc)| {
        let content = format!("{:?},{:?},{:?}\n", msg_id, msg_type, tsc);
        file.write_all(content.as_bytes()).unwrap();
    });
}