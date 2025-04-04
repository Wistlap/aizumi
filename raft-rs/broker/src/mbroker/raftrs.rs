// Copyright 2019 TiKV Project Authors. Licensed under Apache-2.0.

// We use `default` method a lot to be support prost and rust-protobuf at the
// same time. And reassignment can be optimized by compiler.
#![allow(clippy::field_reassign_with_default)]

use slog::{debug, Drain};
use std::collections::{BTreeMap, VecDeque};
// use std::ffi::NulError;
use std::sync::mpsc::{self, Receiver, Sender, SyncSender, TryRecvError};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use std::thread;

use protobuf::Message as PbMessage;
use raft::storage::MemStorage;
use raft::{prelude::*, StateRole};
// use regex::Regex;

use slog::{error, o};

use super::is_ready_to_send;
use super::message::Message as MbMessage;
use super::message::MessageType as MbMessageType;
use super::queue::{MQueue, MQueuePool};

pub fn start_raft(proposals: Arc<Mutex<VecDeque<Proposal>>>, mq_pool: Arc<RwLock<MQueuePool>>, raft_nodes: u32) {
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain)
        .chan_size(4096)
        .overflow_strategy(slog_async::OverflowStrategy::Block)
        .build()
        .fuse();
    let logger = slog::Logger::root(drain, o!());

    let num_nodes: u64 = raft_nodes as u64;
    // Create 5 mailboxes to send/receive messages. Every node holds a `Receiver` to receive
    // messages from others, and uses the respective `Sender` to send messages to others.
    let mut tx_matrix: VecDeque<VecDeque<Sender<Message>>> = VecDeque::with_capacity(num_nodes as usize);
    let mut rx_matrix: VecDeque<VecDeque<Receiver<Message>>> = VecDeque::with_capacity(num_nodes as usize);
    for _ in 0..num_nodes {
        tx_matrix.push_back(VecDeque::new());
    }
    for _ in 0..num_nodes {
        rx_matrix.push_back(VecDeque::new());
    }
    for i in 0..num_nodes as usize {
        for j in 0..num_nodes as usize {
            let (tx, rx) = mpsc::channel();
            tx_matrix[j].push_back(tx);   // j番目のスレッドに向けた送信用
            rx_matrix[i].push_back(rx);   // i番目のスレッドが受信する用
        }
    }
    // tx_matrix[i]とrx_matrix[i]を用いてBtreeMap<u64, (Sender<Message>, Receiver<Message>)>を作成
    let mut mailboxes_v: VecDeque<BTreeMap<u64, (Sender<Message>, Receiver<Message>)>> = VecDeque::new();
    for _ in 0..num_nodes {
        mailboxes_v.push_back(BTreeMap::new());
    }
    for i in 0..num_nodes as usize {
        for j in 0..num_nodes as usize {
            mailboxes_v[i].insert(j as u64 + 1, (tx_matrix[i].pop_front().unwrap(), rx_matrix[i].pop_front().unwrap()));
        }
    }
    println!("{:?}", mailboxes_v);

    let (_tx_stop, rx_stop) = mpsc::channel();
    let rx_stop = Arc::new(Mutex::new(rx_stop));

    // A global pending proposals queue. New proposals will be pushed back into the queue, and
    // after it's committed by the raft cluster, it will be poped from the queue.
    // let proposals = Arc::new(Mutex::new(VecDeque::<Proposal>::new()));

    let mut handles = Vec::new();
    for (i, mailboxes) in mailboxes_v.into_iter().enumerate() {
        // A map[peer_id -> sender]. In the example we create 5 nodes, with ids in [1, 5].
        let mq_pool = Arc::clone(&mq_pool);
        let node = match i {
            // Peer 1 is the leader.
            0 => Node::create_raft_leader(1, mailboxes, &logger, mq_pool),
            // Other peers are followers.
            _ => Node::create_raft_follower(mailboxes),
        };
        let proposals = Arc::clone(&proposals);
        // Clone the stop receiver
        let rx_stop_clone = Arc::clone(&rx_stop);
        let logger = logger.clone();
        // Here we spawn the node on a new thread and keep a handle so we can join on them later.
        let handle = thread::spawn(move || run_node(node, proposals, rx_stop_clone, logger));
        handles.push(handle);
    }

    // Propose some conf changes so that followers can be initialized.
    add_all_followers(proposals.as_ref(), num_nodes);

    // info!(logger, "Propose conf changes success!\n\n");

    // Send terminate signals
    // for _ in 0..num_nodes {
    //     tx_stop.send(Signal::Terminate).unwrap();
    // }

    println!("after add followers");

    // Wait for the thread to finish
    // No return because the broker uses Raft
    for th in handles {
        th.join().unwrap();
    }
}

fn run_node(
    mut node: Node,
    proposals: Arc<Mutex<VecDeque<Proposal>>>,
    rx_stop_clone: Arc<Mutex<mpsc::Receiver<Signal>>>,
    logger: slog::Logger,
){
    // Channels for receiving messages from other nodes.
    let (recv_tx, recv_rx) = mpsc::channel();
    let mut send_txs: BTreeMap<u64, Sender<Vec<Message>>> = BTreeMap::new();
    if ! node.mailboxes.is_empty() {
        let keys: Vec<u64> = node.mailboxes.keys().cloned().collect();
        for key in keys {
            // Channels for sending messages to other nodes.
            let (send_tx, send_rx) = mpsc::channel();
            send_txs.insert(key, send_tx);
            let recv_tx = recv_tx.clone();
            let (mut send_stream, mut recv_stream) = node.mailboxes.remove(&key).unwrap();
            let logger_r = logger.clone();
            thread::spawn(move || {
                treat_recv_stream( &mut recv_stream, recv_tx, logger_r);
            });
            let logger_s = logger.clone();
            thread::spawn(move || {
                treat_send_stream( &mut send_stream, send_rx, logger_s);
            });
        }
    }

    println!("after send&recv threads");

    // Tick the raft node per 100ms. So use an `Instant` to trace it.
    let mut t = Instant::now();
    loop {
        // loop {
        //     // Step raft messages.
        //     match node.my_mailbox.try_recv() {
        //         Ok(msg) => node.step(msg, &logger),
        //         Err(TryRecvError::Empty) => break,
        //         Err(TryRecvError::Disconnected) => return,
        //     }
        // }
        loop {
            match recv_rx.try_recv() {
                Ok(msg) => node.step(msg, &logger),
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
                propose(raft_group, p);
            }
        }

        // Handle readies from the raft.
        on_ready(
            raft_group,
            Arc::clone(&node.mq_pool),
            &mut send_txs,
            &proposals,
            &logger,
        );

        // Check control signals from the main thread.
        if check_signals(&rx_stop_clone) {
            return;
        };
    }
}

fn treat_recv_stream(
    recv_stream: &mut Receiver<Message>,
    recv_tx: Sender<Message>,
    logger: slog::Logger,
) {
    loop {
        // If there are messages received from other nodes, send it to the Raft node.
        let msg  =recv_stream.recv();
        if let Ok(msg) = msg {
            debug!(logger, "{:?}", msg);
            // println!("{:?}", msg);
            let _ = recv_tx.send(msg);
        }
        // let mut size_buf = [0; 4];
        // match recv_stream.read_exact(&mut size_buf) {
        //     Ok(_) => {
        //         let size = u32::from_be_bytes(size_buf) as usize;
        //         let mut buf = vec![0; size];
        //         recv_stream.read_exact(&mut buf).unwrap();
        //         let msg = Message::parse_from_bytes(&buf);
        //         if let Ok(msg) = msg {
        //             debug!(logger, "{:?}", msg);
        //             let _ = recv_tx.send(msg);
        //         }
        //     },
        //     Err(e) => {
        //         error!(logger, "Error in function treat_recv_stream: {}", e);
        //         break;
        //     }
        // }
    }
}

fn treat_send_stream(
    send_stream: &mut Sender<Message>,
    send_rx: Receiver<Vec<Message>>,
    logger: slog::Logger,
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
            // let bytes = msg.write_to_bytes().unwrap();
            // let size = bytes.len();
            // send_stream.write_all(&size.to_be_bytes()).unwrap();
            if send_stream.send(msg).is_err() {
                error!(
                    logger,
                    "send raft message to {} fail, let Raft retry it", to
                );
            }
        }
    }
}


#[allow(dead_code)]
enum Signal {
    Terminate,
}

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
    mailboxes: BTreeMap<u64, (Sender<Message>, Receiver<Message>)>,
    // Key-value pairs after applied. `MemStorage` only contains raft logs,
    // so we need an additional storage engine.
    mq_pool: Arc<RwLock<MQueuePool>>,
}

impl Node {
    // Create a raft leader only with itself in its configuration.
    fn create_raft_leader(
        id: u64,
        mailboxes: BTreeMap<u64, (Sender<Message>, Receiver<Message>)>,
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
            mailboxes,
            mq_pool,
        }
    }

    // Create a raft follower.
    fn create_raft_follower(
        mailboxes: BTreeMap<u64, (Sender<Message>, Receiver<Message>)>,
    ) -> Self {
        Node {
            raft_group: None,
            mailboxes,
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
) {
    if !raft_group.has_ready() {
        return;
    }

    // println!("start consensus")

    let store = raft_group.raft.raft_log.store.clone();

    // Get the `Ready` with `RawNode::ready` interface.
    let mut ready = raft_group.ready();

    // let handle_messages = |msgs: Vec<Message>| {
    //     for msg in msgs {
    //         let to = msg.to;
    //         if mailboxes[&to].send(msg).is_err() {
    //             error!(
    //                 logger,
    //                 "send raft message to {} fail, let Raft retry it", to
    //             );
    //         }
    //     }
    // };

    fn handle_messages(msgs: Vec<Message>, send_txs: &mut BTreeMap<u64, Sender<Vec<Message>>>) {
        let mut msgs_group: BTreeMap<u64, Vec<Message>> = BTreeMap::new();
        for msg in msgs {
            let key = msg.to;
            // devide the messages by the destination node.
            msgs_group.entry(key).or_default().push(msg);
        }
        for (key, send_tx) in send_txs {
            if let Some(value) = msgs_group.remove(key) {
                send_tx.send(value).unwrap();
            }
        }
    }

    if !ready.messages().is_empty() {
        // Send out the messages come from the node.
        handle_messages(ready.take_messages(), send_txs);
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
                    let msg = MbMessage::from_bytes(&entry.data);

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
                            // info!(logger, "peer {}: process SendReq: {:?}", rn.raft.id, msg.header.id);
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
                            // info!(logger, "peer {}: process FreeReq: {:?}", rn.raft.id, msg_id);
                            None
                        }
                        MbMessageType::PushReq => {
                            let saddr = msg.header.saddr;
                            let mqueue = {
                                let mq_pool = mq_pool.read().unwrap();
                                mq_pool.find_by_id(saddr).unwrap().clone()
                            };
                            let mut mqueue = mqueue.write().unwrap();
                            let res = if is_ready_to_send(&mqueue) {
                                let mut msg = mqueue.waiting_queue.dequeue().unwrap();
                                msg.header.change_msg_type(MbMessageType::PushReq);
                                // timer.append(msg.header.id, msg.header.msg_type(), time_now());
                                // stream.send_msg(&mut msg).unwrap();
                                mqueue.delivered_queue.enqueue(msg.clone());
                                //info!(logger, "peer {}: process inner PushReq or Timeout: {:?}", rn.raft.id, msg.header.id);
                                Some(msg)
                            } else {
                                None
                            };
                            res
                        }
                        MbMessageType::HeloReq => {
                            let client_id = msg.header.saddr;
                            {
                                let mut mq_pool = mq_pool.write().unwrap();
                                if mq_pool.find_by_id(client_id).is_none() {
                                    mq_pool.add(client_id, MQueue::new(client_id));
                                };
                            }
                            //info!(logger, "peer {}: process HeloReq: {:?}", rn.raft.id, msg.header.id);
                            None
                        }
                        _ => {
                            // The other MessageType will never be received
                            None
                        }
                    };
                    res
                };
                if rn.raft.state == StateRole::Leader {
                    // The leader should response to the clients, tell them if their proposals
                    // succeeded or not.
                    let proposal = proposals.lock().unwrap().pop_front().unwrap();
                    proposal.propose_success.send(res).unwrap();
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
        handle_messages(ready.take_persisted_messages(), send_txs);
    }

    // Call `RawNode::advance` interface to update position flags in the raft.
    let mut light_rd = raft_group.advance(ready);
    // Update commit index.
    if let Some(commit) = light_rd.commit_index() {
        store.wl().mut_hard_state().set_commit(commit);
    }
    // Send out the messages.
    handle_messages(light_rd.take_messages(), send_txs);
    // Apply all committed entries.
    handle_committed_entries(raft_group, light_rd.take_committed_entries());
    // Advance the apply index.
    raft_group.advance_apply();
}

fn example_config() -> Config {
    Config {
        election_tick: 10,
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
    propose_success: SyncSender<Option<MbMessage>>,
}

impl Proposal {
    fn conf_change(cc: &ConfChange) -> (Self, Receiver<Option<MbMessage>>) {
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

    pub fn normal(msg:MbMessage) -> (Self, Receiver<Option<MbMessage>>) {
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
            let data = msg.to_bytes();
            let _ = raft_group.propose(vec![], data.to_vec());
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
        proposal.propose_success.send(None).unwrap();
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
            // TODO: is_none()でいいのか
            if rx.recv().unwrap().is_none() {
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
    }
}
