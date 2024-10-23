use std::os::fd::AsRawFd;

use nix::sys::socket::{self, setsockopt, sockopt, SockaddrIn};

use openraft::error::InstallSnapshotError;
use openraft::error::NetworkError;
use openraft::error::RemoteError;
use openraft::error::Unreachable;
use openraft::network::RPCOption;
use openraft::network::RaftNetwork;
use openraft::network::RaftNetworkFactory;
use openraft::raft::AppendEntriesRequest;
use openraft::raft::AppendEntriesResponse;
use openraft::raft::InstallSnapshotRequest;
use openraft::raft::InstallSnapshotResponse;
use openraft::raft::VoteRequest;
use openraft::raft::VoteResponse;
use openraft::BasicNode;
use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio::net::UdpSocket;

use crate::messaging::{self, RpcType};
use crate::typ;
use crate::NodeId;
use crate::TypeConfig;

pub struct Network {
    pub my_addr: String,
}

impl Network {
    // FIXME: メソッドが呼び出されるたびにソケットを作成している
    // NetworkConnection にソケットを持たせるべきか？
    pub async fn send_rpc<Req, Resp, Err>(
        &self,
        target: NodeId,
        target_node: &BasicNode,
        prc_type: RpcType,
        req: Req,
    ) -> Result<Resp, openraft::error::RPCError<NodeId, BasicNode, Err>>
    where
        Req: Serialize,
        Err: std::error::Error + DeserializeOwned,
        Resp: DeserializeOwned,
    {
        let src = &self.my_addr;
        let dst = &target_node.addr;

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

        let mut parts = src.split(':');
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

        // Create std's UdpSocket from nix's socket
        let socket = UdpSocket::from_std(std::net::UdpSocket::from(socket)).unwrap();

        socket.connect(dst).await.unwrap();
        tracing::debug!("connection is created for: {}", dst);

        let rpc = messaging::RPC {
            rpc_type: prc_type,
            request: bincode::serialize(&req).unwrap(),
        };
        let rpc = bincode::serialize(&rpc).unwrap();

        socket.send(&rpc).await.map_err(|e| {
            // If the error is a connection error, we return `Unreachable` so that connection isn't retried
            // immediately.
            if e.kind() == std::io::ErrorKind::ConnectionRefused {
                return openraft::error::RPCError::Unreachable(Unreachable::new(&e));
            }
            openraft::error::RPCError::Network(NetworkError::new(&e))
        })?;
        tracing::debug!("sent rpc to: {}", dst);

        let mut buf = [0; 1024];
        let n = socket.recv(&mut buf).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::ConnectionRefused {
                return openraft::error::RPCError::Unreachable(Unreachable::new(&e));
            }
            openraft::error::RPCError::Network(NetworkError::new(&e))
        })?;
        tracing::debug!("received {} bytes, rpc from: {}", n, dst);

        let res: Result<Resp, Err> = bincode::deserialize(&buf[..n]).map_err(|e| openraft::error::RPCError::Network(NetworkError::new(&e)))?;
        res.map_err(|e| openraft::error::RPCError::RemoteError(RemoteError::new(target, e)))
    }
}

// NOTE: This could be implemented also on `Arc<ExampleNetwork>`, but since it's empty, implemented
// directly.
impl RaftNetworkFactory<TypeConfig> for Network {
    type Network = NetworkConnection;

    async fn new_client(&mut self, target: NodeId, node: &BasicNode) -> Self::Network {
        NetworkConnection {
            owner: Network { my_addr: self.my_addr.clone() },
            target,
            target_node: node.clone(),
        }
    }
}

pub struct NetworkConnection {
    owner: Network,
    target: NodeId,
    target_node: BasicNode,
}

impl RaftNetwork<TypeConfig> for NetworkConnection {
    async fn append_entries(
        &mut self,
        req: AppendEntriesRequest<TypeConfig>,
        _option: RPCOption,
    ) -> Result<AppendEntriesResponse<NodeId>, typ::RPCError> {
        self.owner.send_rpc(self.target, &self.target_node, RpcType::AppendEntries, req).await
    }

    async fn install_snapshot(
        &mut self,
        req: InstallSnapshotRequest<TypeConfig>,
        _option: RPCOption,
    ) -> Result<InstallSnapshotResponse<NodeId>, typ::RPCError<InstallSnapshotError>> {
        self.owner.send_rpc(self.target, &self.target_node, RpcType::InstallSnapshot, req).await
    }

    async fn vote(
        &mut self,
        req: VoteRequest<NodeId>,
        _option: RPCOption,
    ) -> Result<VoteResponse<NodeId>, typ::RPCError> {
        self.owner.send_rpc(self.target, &self.target_node, RpcType::Vote, req).await
    }
}
