// Copyright 2019 TiKV Project Authors. Licensed under Apache-2.0.

// We use `default` method a lot to be support prost and rust-protobuf at the
// same time. And reassignment can be optimized by compiler.
#![allow(clippy::field_reassign_with_default)]

use nix::sys::socket::{setsockopt, sockopt};
use slog::{debug, Drain};
use std::collections::{BTreeMap, VecDeque};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{self, Receiver, Sender, SyncSender, TryRecvError};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use std::thread;


use protobuf::Message as PbMessage;
use raft::storage::MemStorage;
use raft::{prelude::*, StateRole};

use slog::{error, o};

use super::timer::{time_now, RaftTimerStorage, TimerStorage};
use super::is_ready_to_send;
use super::message::{Message as MbMessage, RaftTimestampType};
use super::message::MessageType as MbMessageType;
use super::queue::{MQueue, MQueuePool};

const LEADER_NODE: u64 = 6555;

pub fn start_raft(
    proposals: Arc<Mutex<VecDeque<Proposal>>>,
    mq_pool: Arc<RwLock<MQueuePool>>,
    my_address: String,
    raft_addresses: Vec<String>
) {
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain)
        .chan_size(4096)
        .overflow_strategy(slog_async::OverflowStrategy::Block)
        .build()
        .fuse();
    let logger = slog::Logger::root(drain, o!());

    let num_nodes: u64 = raft_addresses.len() as u64;

    // Channels for getting streams from ather temporary threads
    let (accept_tx, accept_rx) = mpsc::channel();
    let (connect_tx, connect_rx) = mpsc::channel();

    // Add 1000 to the port number for Raft
    let mut parts = my_address.rsplitn(2, ':');
    let my_port = parts.next().unwrap().parse::<u64>().unwrap() + 1000;
    let my_addr = parts.next().unwrap().to_string();
    let my_new_ip = (my_addr, my_port);

    let raft_addresses: Vec<(String, u64)> = raft_addresses
    .iter()
    .map(|ip| {
        let mut parts = ip.rsplitn(2, ':');
        let port = parts.next().unwrap().parse::<u64>().unwrap() + 1000;
        let addr = parts.next().unwrap().to_string();
        (addr, port)
    })
    .collect();

    // Get streams for recv from other nodes
    let my_new_ip_clone = my_new_ip.clone();
    thread::spawn(move || {
        let listener = TcpListener::bind(format!("{}:{}", my_new_ip_clone.0, my_new_ip.1)).unwrap();
        setsockopt(&listener, sockopt::ReuseAddr, &true).unwrap();

        let mut streams: BTreeMap<u64, TcpStream> = BTreeMap::new();
        for _ in 1..num_nodes {
            let (mut stream, _) = listener.accept().unwrap();
            stream.set_nodelay(true).unwrap();

            let mut size_buf = [0; 4];
            stream.read_exact(&mut size_buf).unwrap();
            let size = u32::from_be_bytes(size_buf) as usize;
            let mut buf = vec![0; size];
            stream.read_exact(&mut buf).unwrap();

            let port = u64::from_be_bytes(buf.try_into().unwrap());
            // stream.set_nonblocking(true).unwrap();
            streams.insert(port, stream);
        }
        // return streams to main thread
        accept_tx.send(streams).unwrap();
    });

    // Get streams for send to other nodes
    thread::spawn(move || {
        let mut streams: BTreeMap<u64, TcpStream> = BTreeMap::new();

        for ip in raft_addresses.iter() {
            if *ip == my_new_ip {
                continue; // skip if port is my port
            }
            loop {
                match TcpStream::connect(format!("{}:{}", ip.0, ip.1)) {
                    Ok(mut stream) => {
                        stream.set_nodelay(true).unwrap();

                        let bytes = my_new_ip.1.to_be_bytes();
                        let size = bytes.len();
                        stream.write_all(&(size as u32).to_be_bytes()).unwrap();
                        stream.write_all(&bytes).unwrap();

                        streams.insert(ip.1, stream);
                        break;
                    }
                    Err(_) => {
                        thread::sleep(Duration::from_millis(100));
                        continue;
                    }
                }
            }
        }
        // return streams to main thread
        connect_tx.send(streams).unwrap();
    });

    // Get streams from temporary threads
    // recv_streams is for receiving messages from other nodes.
    // send_streams is for sending messages to other nodes.
    let recv_streams = accept_rx.recv().unwrap();
    let send_streams = connect_rx.recv().unwrap();

    // recv_streams と send_stream から，BtreeMap (port - LEADER_NODE +1, (send_stream, recv_stream)) を作成
    let streams: BTreeMap<u64, (TcpStream, TcpStream)> = recv_streams.iter()
                                                                    .map(
                                                                        |(k, v)| (*k - LEADER_NODE +1, (send_streams.get(k).unwrap().try_clone().unwrap(), v.try_clone().unwrap()))
                                                                    )
                                                                    .collect();

    let mq_pool = Arc::clone(&mq_pool);
    let node = match my_port {
        // Peer 1 is the leader.
        LEADER_NODE => Node::create_raft_leader(1, streams, &logger, mq_pool),
        // Other peers are followers.
        _ => Node::create_raft_follower(streams),
    };

    // A global pending proposals queue. New proposals will be pushed back into the queue, and
    // after it's committed by the raft cluster, it will be poped from the queue.
    let proposals_clone = Arc::clone(&proposals);
    let logger = logger.clone();
    // Here we spawn the node on a new thread and keep a handle so we can join on them later.
    let handle = thread::spawn(move || run_node(node, proposals_clone,  logger));

    // Propose some conf changes so that followers can be initialized.
    let proposals = Arc::clone(&proposals);
    if my_port == LEADER_NODE {
        add_all_followers(proposals.as_ref(), num_nodes);
    }

    // Wait for the thread to finish
    // No return because the broker uses Raft
    handle.join().unwrap();
}

fn run_node(
    mut node: Node,
    proposals: Arc<Mutex<VecDeque<Proposal>>>,
    logger: slog::Logger,
){
    let raft_timer = RaftTimerStorage::new();
    let raft_timer = Arc::new(Mutex::new(raft_timer));
    // Channels for receiving messages from other nodes.
    let (recv_tx, recv_rx) = mpsc::channel();
    let mut send_txs: BTreeMap<u64, Sender<Vec<Message>>> = BTreeMap::new();
    if ! node.streams.is_empty() {
        let keys: Vec<u64> = node.streams.keys().cloned().collect();
        for key in keys {
            // Channels for sending messages to other nodes.
            let (send_tx, send_rx) = mpsc::channel();
            send_txs.insert(key, send_tx);
            let recv_tx = recv_tx.clone();
            let (mut send_stream, mut recv_stream) = node.streams.remove(&key).unwrap();
            let logger_r = logger.clone();
            let raft_timer_r = Arc::clone(&raft_timer);
            thread::spawn(move || {
                treat_recv_stream( &mut recv_stream, recv_tx, logger_r, raft_timer_r);
            });
            let logger_s = logger.clone();
            let raft_timer_s = Arc::clone(&raft_timer);
            thread::spawn(move || {
                treat_send_stream( &mut send_stream, send_rx, logger_s, raft_timer_s);
            });
        }
    }

    // Tick the raft node per 100ms. So use an `Instant` to trace it.
    let mut t = Instant::now();
    loop {
        loop {
            match recv_rx.try_recv() {
                Ok(msg) => {
                    if let Some(msg_ids) = le_bytes_to_u32_vec(msg.get_context()) {
                        debug!(logger, "MsgAppendResponse: {:?}", msg_ids);
                        for msg_id in msg_ids {
                            // タイムスタンプ 受信部からの受け取り 106
                        raft_timer.lock().unwrap().append(msg_id, RaftTimestampType::BeforeReceivedLogAppend, time_now());
                        }
                    }
                    node.step(msg, &logger)
                },
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return,
            }
        }

        let raft_group = match node.raft_group {
            Some(ref mut r) => r,
            // When Node::raft_group is `None` it means the node is not initialized.
            _ => continue,
        };

        if t.elapsed() >= Duration::from_millis(100) {
            // Tick the raft.
            raft_group.tick();
            t = Instant::now();
        }

        // Let the leader pick pending proposals from the global queue.
        if raft_group.raft.state == StateRole::Leader {
            // Handle new proposals.
            let mut proposals = proposals.lock().unwrap();
            for p in proposals.iter_mut().skip_while(|p| p.proposed > 0) {
                // TODO: タイムスタンプ ログの追加
                // // key に対応するキューに挿入
                // raft_timer.lock().unwrap().entry(msg.index).or_default().push((1, time_now()));
                propose(raft_group, p);
            }
        }

        let raft_timer_c = Arc::clone(&raft_timer);
        // Handle readies from the raft.
        on_ready(
            raft_group,
            Arc::clone(&node.mq_pool),
            &mut send_txs,
            &proposals,
            &logger,
            raft_timer_c,
        );
    }
}

fn treat_recv_stream(
    recv_stream: &mut TcpStream,
    recv_tx: Sender<Message>,
    logger: slog::Logger,
    raft_timer: Arc<Mutex<RaftTimerStorage>>
) {
    loop {
        // If there are messages received from other nodes, send it to the Raft node.
        let mut size_buf = [0; 4];
        match recv_stream.read_exact(&mut size_buf) {
            Ok(_) => {
                let size = u32::from_be_bytes(size_buf) as usize;
                let mut buf = vec![0; size];
                recv_stream.read_exact(&mut buf).unwrap();
                let msg = Message::parse_from_bytes(&buf);
                if let Ok(msg) = msg {
                    debug!(logger, "{:?}", msg);

                    if let Some(msg_ids) = le_bytes_to_u32_vec(msg.get_context()) {
                        debug!(logger, "MsgAppendResponse: {:?}", msg_ids);
                        for msg_id in msg_ids {
                            // タイムスタンプ RPC 受信後 105
                            raft_timer.lock().unwrap().append(msg_id, RaftTimestampType::AfterRPCReceived, time_now());
                        }
                    }

                    let _ = recv_tx.send(msg);
                }
            },
            Err(e) => {
                error!(logger, "Error in function treat_recv_stream: {}", e);
                break;
            }
        }
    }
}

fn treat_send_stream(
    send_stream: &mut TcpStream,
    send_rx: Receiver<Vec<Message>>,
    logger: slog::Logger,
    raft_timer: Arc<Mutex<RaftTimerStorage>>
) {
    loop {
        // If there are messages to send to other nodes, send it.
        let msgs = match send_rx.recv(){
            Ok(msgs) => msgs,
            Err(e) => {
                error!(logger, "Error in function treat_send_stream: {}", e);
                break;
            }
        };
        for msg in msgs {
            let to = msg.to;
            let bytes = msg.write_to_bytes().unwrap();
            let size = bytes.len();
            send_stream.write_all(&size.to_be_bytes()).unwrap();
            if send_stream.write_all(&bytes).is_err() {
                error!(
                    logger,
                    "send raft message to {} fail, let Raft retry it", to
                );
            }
            msg.entries
                .iter()
                .for_each(|entry| {
                    if let Some(msg_id) = le_bytes_to_u32(entry.get_context()) {
                        // タイムスタンプ RPC 送信後 104
                        raft_timer.lock().unwrap().append(msg_id, RaftTimestampType::AfterRPCSent, time_now());
                    }
                });
        }
    }
}

#[allow(dead_code)]
enum Signal {
    Terminate,
}

#[allow(dead_code)]
fn check_signals(receiver: &Arc<Mutex<mpsc::Receiver<Signal>>>) -> bool {
    match receiver.lock().unwrap().try_recv() {
        Ok(Signal::Terminate) => true,
        Err(TryRecvError::Empty) => false,
        Err(TryRecvError::Disconnected) => true,
        // _ => false,
    }
}

struct Node {
    // None if the raft is not initialized.
    raft_group: Option<RawNode<MemStorage>>,
    streams: BTreeMap<u64, (TcpStream, TcpStream)>,
    // Key-value pairs after applied. `MemStorage` only contains raft logs,
    // so we need an additional storage engine.
    mq_pool: Arc<RwLock<MQueuePool>>,
}

impl Node {
    // Create a raft leader only with itself in its configuration.
    fn create_raft_leader(
        id: u64,
        streams: BTreeMap<u64, (TcpStream, TcpStream)>,
        logger: &slog::Logger,
        mq_pool: Arc<RwLock<MQueuePool>>,
    ) -> Self {
        let mut cfg = example_config();
        cfg.id = id;
        let logger = logger.new(o!("tag" => format!("peer_{}", id)));
        let mut s = Snapshot::default();
        // Because we don't use the same configuration to initialize every node, so we use
        // a non-zero index to force new followers catch up logs by snapshot first, which will
        // bring all nodes to the same initial state.
        s.mut_metadata().index = 1;
        s.mut_metadata().term = 1;
        s.mut_metadata().mut_conf_state().voters = vec![1];
        let storage = MemStorage::new();
        storage.wl().apply_snapshot(s).unwrap();
        let raft_group = Some(RawNode::new(&cfg, storage, &logger).unwrap());
        Node {
            raft_group,
            streams,
            mq_pool,
        }
    }

    // Create a raft follower.
    fn create_raft_follower(
        streams: BTreeMap<u64, (TcpStream, TcpStream)>,
    ) -> Self {
        Node {
            raft_group: None,
            streams,
            mq_pool: Arc::new(RwLock::new(MQueuePool::new())),
        }
    }

    // Initialize raft for followers.
    fn initialize_raft_from_message(&mut self, msg: &Message, logger: &slog::Logger) {
        if !is_initial_msg(msg) {
            return;
        }
        let mut cfg = example_config();
        cfg.id = msg.to;
        let logger = logger.new(o!("tag" => format!("peer_{}", msg.to)));
        let storage = MemStorage::new();
        self.raft_group = Some(RawNode::new(&cfg, storage, &logger).unwrap());
    }

    // Step a raft message, initialize the raft if need.
    fn step(&mut self, msg: Message, logger: &slog::Logger) {
        if self.raft_group.is_none() {
            if is_initial_msg(&msg) {
                self.initialize_raft_from_message(&msg, logger);
            } else {
                return;
            }
        }
        let raft_group = self.raft_group.as_mut().unwrap();
        let _ = raft_group.step(msg);
    }
}

fn on_ready(
    raft_group: &mut RawNode<MemStorage>,
    mq_pool: Arc<RwLock<MQueuePool>>,
    send_txs: &mut BTreeMap<u64, Sender<Vec<Message>>>,
    proposals: &Mutex<VecDeque<Proposal>>,
    logger: &slog::Logger,
    raft_timer: Arc<Mutex<RaftTimerStorage>>,
) {
    if !raft_group.has_ready() {
        return;
    }
    let store = raft_group.raft.raft_log.store.clone();

    // Get the `Ready` with `RawNode::ready` interface.
    let mut ready = raft_group.ready();

    fn handle_messages(msgs: Vec<Message>, send_txs: &mut BTreeMap<u64, Sender<Vec<Message>>>, raft_timer: Arc<Mutex<RaftTimerStorage>>) {
        let mut msgs_group: BTreeMap<u64, Vec<Message>> = BTreeMap::new();
        for msg in msgs {
            let key = msg.to;
            // devide the messages by the destination node.
            msgs_group.entry(key).or_default().push(msg);
        }
        for (key, send_tx) in send_txs {
            if let Some(msgs) = msgs_group.remove(key) {
                for msg in msgs.iter() {
                    msg.entries
                        .iter()
                        .for_each(|entry| {
                            if let Some(msg_id) = le_bytes_to_u32(entry.get_context()) {
                                // タイムスタンプ 送信部にメッセージ受け渡し前 103
                                raft_timer.lock().unwrap().append(msg_id, RaftTimestampType::BeforeMessageSend, time_now());
                            }
                        });
                }
                send_tx.send(msgs).unwrap();
            }
        }
    }

    if !ready.messages().is_empty() {
        // Send out the messages come from the node.
        handle_messages(ready.take_messages(), send_txs, Arc::clone(&raft_timer));
    }

    // Apply the snapshot. It's necessary because in `RawNode::advance` we stabilize the snapshot.
    if *ready.snapshot() != Snapshot::default() {
        let s = ready.snapshot().clone();
        if let Err(e) = store.wl().apply_snapshot(s) {
            error!(
                logger,
                "apply snapshot fail: {:?}, need to retry or panic", e
            );
            return;
        }
    }

    let handle_committed_entries =
        |rn: &mut RawNode<MemStorage>, committed_entries: Vec<Entry>| {
            for entry in committed_entries {
                if entry.data.is_empty() {
                    // From new elected leaders.
                    continue;
                }
                let res = if let EntryType::EntryConfChange = entry.get_entry_type() {
                    // For conf change messages, make them effective.
                    let mut cc = ConfChange::default();
                    cc.merge_from_bytes(&entry.data).unwrap();
                    let cs = rn.apply_conf_change(&cc).unwrap();
                    store.wl().set_conf_state(cs);
                    None
                } else {
                    // For normal proposals, extract the key-value pair and then
                    // insert them into the kv engine.

                    let msg_id = le_bytes_to_u32(entry.get_context()).unwrap();
                    let msg = MbMessage::from_bytes(&entry.data);
                    // コミット済みエントリのステートマシン適用前 107
                    raft_timer.lock().unwrap().append(msg_id, RaftTimestampType::BeforeStateMachineApply, time_now());
                    let res = match msg.header.msg_type() {
                        MbMessageType::SendReq => {
                            let mut mq_pool = mq_pool.write().unwrap();
                            let mqueue = match mq_pool.find_by_id(msg.header.daddr) {
                                Some(mqueue) => Arc::clone(mqueue),
                                None => {
                                    let client_id = msg.header.daddr;
                                    Arc::clone(mq_pool.add(client_id, MQueue::new(client_id)))
                                }
                            };
                            drop(mq_pool);
                            debug!(logger, "peer {}: process SendReq: {:?}", rn.raft.id, msg.header.id);
                            mqueue.write().unwrap().waiting_queue.enqueue(msg);
                            None
                        }
                        MbMessageType::FreeReq => {
                            let msg_id = msg.header.id;
                            let saddr = msg.header.saddr;
                            let mqueue = {
                                let mq_pool = mq_pool.read().unwrap();
                                mq_pool.find_by_id(saddr).unwrap().clone()
                            };
                            mqueue
                                .write()
                                .unwrap()
                                .delivered_queue
                                .dequeue_by(|queued_msg| queued_msg.header.id == msg_id)
                                .unwrap();
                            debug!(logger, "peer {}: process FreeReq: {:?}", rn.raft.id, msg_id);
                            None
                        }
                        MbMessageType::PushReq => {
                            let saddr = msg.header.saddr;
                            let mqueue = {
                                let mq_pool = mq_pool.read().unwrap();
                                mq_pool.find_by_id(saddr).unwrap().clone()
                            };
                            let mut mqueue = mqueue.write().unwrap();
                            if is_ready_to_send(&mqueue) {
                                let mut msg = mqueue.waiting_queue.dequeue().unwrap();
                                msg.header.change_msg_type(MbMessageType::PushReq);
                                // timer.append(msg.header.id, msg.header.msg_type(), time_now());
                                // stream.send_msg(&mut msg).unwrap();
                                mqueue.delivered_queue.enqueue(msg.clone());
                                debug!(logger, "peer {}: process inner PushReq or Timeout: {:?}", rn.raft.id, msg.header.id);
                                Some(msg)
                            } else {
                                None
                            }
                        }
                        MbMessageType::HeloReq => {
                            let client_id = msg.header.saddr;
                            {
                                let mut mq_pool = mq_pool.write().unwrap();
                                if mq_pool.find_by_id(client_id).is_none() {
                                    mq_pool.add(client_id, MQueue::new(client_id));
                                };
                            }
                            debug!(logger, "peer {}: process HeloReq: {:?}", rn.raft.id, msg.header.id);
                            None
                        }
                        _ => {
                            // The other MessageType will never be received
                            None
                        }
                    };
                    // コミット済みエントリのステートマシン適用後 108
                    raft_timer.lock().unwrap().append(msg_id, RaftTimestampType::AfterStateMachineApply, time_now());
                    res
                };
                if rn.raft.state == StateRole::Leader {
                    // The leader should response to the clients, tell them if their proposals
                    // succeeded or not.
                    let proposal = proposals.lock().unwrap().pop_front().unwrap();
                    match le_bytes_to_u32(&entry.context) {
                        Some(msg_id) => proposal.propose_success.send((res, raft_timer.lock().unwrap().take(msg_id))).unwrap(),
                        None => proposal.propose_success.send((res, None)).unwrap()
                    }
                }
            }
        };
    // Apply all committed entries.
    handle_committed_entries(raft_group, ready.take_committed_entries());

    // Persistent raft logs. It's necessary because in `RawNode::advance` we stabilize
    // raft logs to the latest position.
    if let Err(e) = store.wl().append(ready.entries()) {
        error!(
            logger,
            "persist raft log fail: {:?}, need to retry or panic", e
        );
        return;
    }

    if let Some(hs) = ready.hs() {
        // Raft HardState changed, and we need to persist it.
        store.wl().set_hardstate(hs.clone());
    }

    if !ready.persisted_messages().is_empty() {
        // Send out the persisted messages come from the node.
        handle_messages(ready.take_persisted_messages(), send_txs, Arc::clone(&raft_timer));
    }

    // Call `RawNode::advance` interface to update position flags in the raft.
    let mut light_rd = raft_group.advance(ready);
    // Update commit index.
    if let Some(commit) = light_rd.commit_index() {
        store.wl().mut_hard_state().set_commit(commit);
    }
    // Send out the messages.
    handle_messages(light_rd.take_messages(), send_txs, Arc::clone(&raft_timer));
    // Apply all committed entries.
    handle_committed_entries(raft_group, light_rd.take_committed_entries());
    // Advance the apply index.
    raft_group.advance_apply();
}

fn example_config() -> Config {
    Config {
        election_tick: 30,
        heartbeat_tick: 3,
        ..Default::default()
    }
}

// The message can be used to initialize a raft node or not.
fn is_initial_msg(msg: &Message) -> bool {
    let msg_type = msg.get_msg_type();
    msg_type == MessageType::MsgRequestVote
        || msg_type == MessageType::MsgRequestPreVote
        || (msg_type == MessageType::MsgHeartbeat && msg.commit == 0)
}

#[derive(Clone)]
pub struct Proposal {
    normal: Option<MbMessage>, //log entry ?
    conf_change: Option<ConfChange>, // conf change.
    transfer_leader: Option<u64>,
    // If it's proposed, it will be set to the index of the entry.
    proposed: u64,
    propose_success: SyncSender<ProposalData>,
}

type ProposalData = (Option<MbMessage>, Option<TimerStorage>);

impl Proposal {
    fn conf_change(cc: &ConfChange) -> (Self, Receiver<ProposalData>) {
        let (tx, rx) = mpsc::sync_channel(1);
        let proposal = Proposal {
            normal: None,
            conf_change: Some(cc.clone()),
            transfer_leader: None,
            proposed: 0,
            propose_success: tx,
        };
        (proposal, rx)
    }

    pub fn normal(msg:MbMessage) -> (Self, Receiver<ProposalData>) {
        let (tx, rx) = mpsc::sync_channel(1);
        let proposal = Proposal {
            normal: Some(msg),
            conf_change: None,
            transfer_leader: None,
            proposed: 0,
            propose_success: tx,
        };
        (proposal, rx)
    }
}

fn propose(raft_group: &mut RawNode<MemStorage>, proposal: &mut Proposal) {
    let last_index1 = raft_group.raft.raft_log.last_index() + 1;
    {
        let proposal = proposal.clone();
        if let Some(mut msg) = proposal.normal {
            let context = u32_to_le_bytes(msg.header.id);
            let data = msg.to_bytes();
            let _ = raft_group.propose(context.to_vec(), data.to_vec());
        } else if let Some(ref cc) = proposal.conf_change {
            let _ = raft_group.propose_conf_change(vec![], cc.clone());
        } else if let Some(_transferee) = proposal.transfer_leader {
            // TODO: implement transfer leader.
            unimplemented!();
        }
    }

    let last_index2 = raft_group.raft.raft_log.last_index() + 1;
    if last_index2 == last_index1 {
        // Propose failed, don't forget to respond to the client.
        proposal.propose_success.send((None, None)).unwrap();
        // proposal.propose_success.send(false).unwrap();
    } else {
        proposal.proposed = last_index1;
    }
}

// Proposes some conf change for peers [2, 5].
fn add_all_followers(proposals: &Mutex<VecDeque<Proposal>>, num_nodes: u64) {
    for i in 2..num_nodes+1 {
        let mut conf_change = ConfChange::default();
        conf_change.node_id = i;
        conf_change.set_change_type(ConfChangeType::AddNode);
        loop {
            let (proposal, rx) = Proposal::conf_change(&conf_change);
            proposals.lock().unwrap().push_back(proposal);
            if rx.recv().unwrap().0.is_none() {
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
    }
}

/// u32 をリトルエンディアンのバイト列に変換
fn u32_to_le_bytes(value: u32) -> [u8; 4] {
    value.to_le_bytes()
}

/// リトルエンディアンのバイト列から u32 を復元
fn le_bytes_to_u32(bytes: &[u8]) -> Option<u32> {
    if bytes.len() >= 4 {
        Some(u32::from_le_bytes(bytes[0..4].try_into().unwrap()))
    } else {
        None
    }
}

/// バイト列を u32 のベクタに変換
fn le_bytes_to_u32_vec(bytes: &[u8]) -> Option<Vec<u32>> {
    if bytes.is_empty() {
        return None;
    }
    // if bytes.len() % 4 != 0 {
    //     return None; // バイト列の長さが4の倍数でない場合はエラー
    // }
    Some(
        bytes
            .chunks(4) // 4バイトずつ分割
            .filter_map(|chunk| {
                if chunk.len() == 4 {
                    let arr: [u8; 4] = chunk.try_into().ok()?;
                    Some(u32::from_le_bytes(arr))
                } else {
                    None // 不完全な 4バイトは無視
                }
            })
            .collect(),
    )
}
