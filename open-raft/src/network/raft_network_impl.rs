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
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

use crate::messaging::{self, RpcType};
use crate::typ;
use crate::NodeId;
use crate::TypeConfig;

pub struct Network {}

impl Network {
    pub async fn send_rpc<Req, Resp, Err>(
        &self,
        target: NodeId,
        stream: &mut TcpStream,
        prc_type: RpcType,
        req: Req,
    ) -> Result<Resp, openraft::error::RPCError<NodeId, BasicNode, Err>>
    where
        Req: Serialize,
        Err: std::error::Error + DeserializeOwned,
        Resp: DeserializeOwned,
    {
        tracing::debug!("before sending rpc to: {}", stream.peer_addr().unwrap());
        let rpc = messaging::RPC {
            rpc_type: prc_type,
            request: bincode::serialize(&req).unwrap(),
        };
        let rpc = bincode::serialize(&rpc).unwrap();

        stream.write_all(&rpc).await.map_err(|e| {
            // If the error is a connection error, we return `Unreachable` so that connection isn't retried
            // immediately.
            if e.kind() == std::io::ErrorKind::ConnectionRefused {
                return openraft::error::RPCError::Unreachable(Unreachable::new(&e));
            }
            openraft::error::RPCError::Network(NetworkError::new(&e))
        })?;
        tracing::debug!("sent rpc to: {}", stream.peer_addr().unwrap());

        let mut buf = [0; 1024];
        let n = stream.read(&mut buf).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::ConnectionRefused {
                return openraft::error::RPCError::Unreachable(Unreachable::new(&e));
            }
            openraft::error::RPCError::Network(NetworkError::new(&e))
        })?;
        tracing::debug!("received {} bytes, rpc from: {}", n, stream.peer_addr().unwrap());

        let res: Result<Resp, Err> = bincode::deserialize(&buf[..n]).map_err(|e| openraft::error::RPCError::Network(NetworkError::new(&e)))?;
        res.map_err(|e| openraft::error::RPCError::RemoteError(RemoteError::new(target, e)))
    }
}

// NOTE: This could be implemented also on `Arc<ExampleNetwork>`, but since it's empty, implemented
// directly.
impl RaftNetworkFactory<TypeConfig> for Network {
    type Network = NetworkConnection;

    async fn new_client(&mut self, target: NodeId, node: &BasicNode) -> Self::Network {
        let addr = node.addr.clone();
        let mut parts = addr.split(':');
        let ip = parts.next().unwrap();
        let port = parts.next().unwrap();

        let port: u16 = port.parse().unwrap();
        let port = port + 10;

        let tcp_addr = format!("{}:{}", ip, port);
        let stream = TcpStream::connect(tcp_addr.clone()).await.unwrap();
        stream.set_nodelay(true).unwrap();
        tracing::debug!("new client connection to: {}", stream.peer_addr().unwrap());
        NetworkConnection {
            owner: Network {},
            target,
            stream,
        }
    }
}

pub struct NetworkConnection {
    owner: Network,
    target: NodeId,
    stream: TcpStream,
}

impl RaftNetwork<TypeConfig> for NetworkConnection {
    async fn append_entries(
        &mut self,
        req: AppendEntriesRequest<TypeConfig>,
        _option: RPCOption,
    ) -> Result<AppendEntriesResponse<NodeId>, typ::RPCError> {
        self.owner.send_rpc(self.target, &mut self.stream, RpcType::AppendEntries, req).await
    }

    async fn install_snapshot(
        &mut self,
        req: InstallSnapshotRequest<TypeConfig>,
        _option: RPCOption,
    ) -> Result<InstallSnapshotResponse<NodeId>, typ::RPCError<InstallSnapshotError>> {
        self.owner.send_rpc(self.target, &mut self.stream, RpcType::InstallSnapshot, req).await
    }

    async fn vote(
        &mut self,
        req: VoteRequest<NodeId>,
        _option: RPCOption,
    ) -> Result<VoteResponse<NodeId>, typ::RPCError> {
        self.owner.send_rpc(self.target, &mut self.stream, RpcType::Vote, req).await
    }
}
