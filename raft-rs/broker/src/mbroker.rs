/// Module of message sent/received between clients
mod message;
/// Module that binds port or sends/receives message
mod network;
/// Module of queue for storing message
mod queue;
/// Module for mesuring processing time
mod timer;
/// Module for Raft consensus algorithm
pub mod raftrs;

// use log::*;
use message::{Message, MessageHeader, MessageType};
use network::{NetService, NetStream};
use nix::sys::epoll::*;
use queue::{MQueue, MQueuePool};
use raftrs::{start_raft, Proposal};
use serde::Serialize;
use simplelog::{LevelFilter, WriteLogger};
use std::collections::VecDeque;
use std::ffi::c_uint;
use std::fs::File;
use std::io::{self, stderr, stdout, Write};
use std::net::ToSocketAddrs;
use std::ops::DerefMut;
use std::os::unix::io::AsRawFd;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use timer::{time_now, TimerStorage};


const FIRST_MSG_ID: c_uint = 1;
const ACK_MSG_ID: c_uint = 0;

/// `MBroker` has data for performing processing of message broker
pub struct MBroker {
    service: NetService,
    myid: c_uint,
    timeout: u64,
    log_target: Box<dyn Write + Send>,
    raft_nodes: u32,
    my_address: String,
}

impl MBroker {
    /// Create `MBroker`
    ///
    /// `sockaddr` is the set of IP address and port which message broker binds.
    /// Check [ToSocketAddrs] for how to specify.
    ///
    /// `writer` and `log_level` is for [`Logger`].
    /// * `writer` is stream which logger writes log.
    /// * `log_level` is default log_level of message broker.
    pub fn new<S, T, U, V, W>(
        myid: S,
        sockaddr: T,
        timeout: U,
        log_target_name: V,
        log_filter_level: u32,
        pid_file_path: W,
        raft_nodes: u32,
    ) -> Result<Self, io::Error>
    where
        S: Into<c_uint>,
        T: ToSocketAddrs + ToString,
        U: Into<u64>,
        V: ToString,
        W: ToString,
    {
        WriteLogger::init(
            int_to_loglevel(log_filter_level),
            simplelog::Config::default(),
            get_writable(log_target_name.to_string()).unwrap(),
        )
        .unwrap();

        if !pid_file_path.to_string().is_empty() {
            create_pid_file(pid_file_path).unwrap()
        }

        Ok(Self {
            service: NetService::bind(sockaddr.to_string())?,
            myid: myid.into(),
            timeout: timeout.into(),
            log_target: get_writable(log_target_name.to_string()).unwrap(),
            raft_nodes,
            my_address: sockaddr.to_string(),
        })
    }

    /// Start the processing of message broker.
    ///
    /// This function is blocked by `accept` and never terminate with `Ok`
    pub fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        let proposals = Arc::new(Mutex::new(VecDeque::<Proposal>::new()));
        let proposals_clone = Arc::clone(&proposals);
        let mq_pool = MQueuePool::new();
        let mq_pool = Arc::new(RwLock::new(mq_pool));
        let mq_pool_clone = Arc::clone(&mq_pool);
        thread::spawn(move || {start_raft(proposals_clone, mq_pool_clone, self.raft_nodes, self.my_address);});
        let msg_id = FIRST_MSG_ID;
        let msg_id = Arc::new(Mutex::new(msg_id));
        let timeout_len = self.timeout;
        let log_target = Arc::new(Mutex::new(self.log_target));

        log_target.lock().unwrap().write_all(b"proc_count,total_msg_count,id,msg_type,tsc\n").unwrap();

        // info!("start");
        // {
        //     info!("The following sentence is for stat_from_csv.py.");
        //     info!("To use it, you must raise the log level to 3 or higher.");
        //     writeln!(
        //         log_target.lock().unwrap(),
        //         "proc_count,total_msg_count,id,msg_type,tsc"
        //     )
        //     .unwrap();
        // }
        loop {
            let stream = self.service.accept()?;
            //stream.set_timeout(timeout_len)?;
            let mq_pool = Arc::clone(&mq_pool);
            let msg_id = Arc::clone(&msg_id);
            let log_target = Arc::clone(&log_target);
            let proposals = Arc::clone(&proposals);
            thread::spawn(move || {
                treat_client(stream, mq_pool, msg_id, self.myid, log_target, timeout_len, proposals);
            });
        }

        // unreachable
    }
}

fn get_writable<S: AsRef<str>>(file_name: S) -> Result<Box<dyn Write + Send>, ()> {
    match file_name.as_ref() {
        "" | "stdout" => Ok(Box::new(stdout())),
        "stderr" => Ok(Box::new(stderr())),
        _ => Ok(Box::new(
            File::options()
                .append(true)
                .create(true)
                .open(file_name.as_ref())
                .unwrap(),
        )),
    }
}

fn int_to_loglevel<U: Into<u64>>(loglevel_int: U) -> LevelFilter {
    match loglevel_int.into() {
        0 => LevelFilter::Trace,
        1 => LevelFilter::Debug,
        2 => LevelFilter::Info,
        3 => LevelFilter::Warn,
        4 => LevelFilter::Error,
        _ => LevelFilter::Off,
    }
}

fn create_pid_file<S: ToString>(path: S) -> Result<(), ()> {
    let mut pid_file = File::create(path.to_string()).unwrap();
    write!(pid_file, "{}", std::process::id()).unwrap();
    Ok(())
}

/// Send/Receive `Message` one-on-one with clients.
fn treat_client(
    mut stream: NetStream,
    mq_pool: Arc<RwLock<MQueuePool>>,
    msg_id: Arc<Mutex<c_uint>>,
    myid: c_uint,
    log_target: Arc<Mutex<Box<dyn Write + Send>>>,
    timeout: u64,
    proposals: Arc<Mutex<VecDeque<Proposal>>>,
) {
    let mut client_id = 0;
    let mut _counter = 0;
    let mut timer = TimerStorage::new();
    let epoll = Epoll::new(EpollCreateFlags::empty()).unwrap();
    let event = EpollEvent::new(EpollFlags::EPOLLIN, stream.as_raw_fd().try_into().unwrap());
    epoll.add(&stream, event).unwrap();
    let mut events = [EpollEvent::empty()];
    loop {
        let nfd = epoll.wait(&mut events, timeout as isize).unwrap();
        if nfd > 0 {
            match stream.recv_msg() {
                Ok(mut msg) => match msg.header.msg_type() {
                    MessageType::SendReq => {
                        // TODO: Raftによる処理を追加
                        timer.append(msg.header.id, msg.header.msg_type(), time_now());
                        let mut ack = into_normal_ack(msg.clone(), myid);
                        msg.header.id = get_msg_id(Arc::clone(&msg_id));

                        let (proposal, rx) = Proposal::normal(msg.clone());
                        proposals.lock().unwrap().push_back(proposal);
                        rx.recv().unwrap();
                        // println!("consensus result(SendReq): {:?}", res.header);
                        stream.send_msg(&mut ack).unwrap();
                        _counter += 1;
                    }
                    MessageType::RecvReq => {
                        // do nothing
                    }
                    MessageType::FreeReq => {
                        let (proposal, rx) = Proposal::normal(msg.clone());
                        proposals.lock().unwrap().push_back(proposal);
                        rx.recv().unwrap();
                        stream.send_msg(&mut into_normal_ack(msg.clone(), myid)).unwrap();

                        if is_ready_to_send(&mq_pool.read().unwrap().find_by_id(msg.header.saddr).unwrap().clone().read().unwrap()) {
                            msg.header.change_msg_type(MessageType::PushReq);
                            let (proposal, rx) = Proposal::normal(msg.clone());
                            proposals.lock().unwrap().push_back(proposal);
                            // After we got a response from `rx`, we can assume the put succeeded and following
                            // `get` operations can find the key-value pair.
                            let mut res = match rx.recv().unwrap() {
                                Some(msg) => msg,
                                None => continue,
                            };
                            // println!("consensus result(FreeReq): {:?}", res.header);
                            timer.append(res.header.id, res.header.msg_type(), time_now());
                            stream.send_msg(&mut res).unwrap();
                        }
                    }
                    MessageType::PushAck => {
                        // do nothing
                    }
                    MessageType::HeloReq => {
                        let (proposal, rx) = Proposal::normal(msg.clone());
                        proposals.lock().unwrap().push_back(proposal);
                        rx.recv().unwrap();
                        // println!("consensus result(HeloReq): {:?}", res.header);

                        client_id = msg.header.saddr;
                        let mut ack = into_normal_ack(msg, myid);
                        stream.send_msg(&mut ack).unwrap();
                    }
                    MessageType::StatReq => {
                        let queue_stat = {
                            let mq_pool = mq_pool.read().unwrap();
                            mq_pool.status()
                        };
                        let mut ack = into_ack(msg, myid, queue_stat);
                        stream.send_msg(&mut ack).unwrap();
                    }
                    _ => {
                        // The other MessageType will never be received
                    }
                },
                Err(_) => break,
            }
        } else {
            let mqueue = {
                let mq_pool = mq_pool.read().unwrap();
                mq_pool.find_by_id(client_id).cloned()
            };
            if let Some(mqueue) = mqueue {
                // let mut mqueue = mqueue.write().unwrap();
                if is_ready_to_send(&mqueue.read().unwrap()) {
                    let msg = Message::new(MessageHeader::new(MessageType::PushReq, client_id as c_uint, 0 as c_uint, 0 as c_uint));
                let (proposal, rx) = Proposal::normal(msg.clone());
                proposals.lock().unwrap().push_back(proposal);
                let mut res = match rx.recv().unwrap() {
                    Some(msg) => msg,
                    None => continue,
                };
                // println!("consensus result(Timeout): {:?}", res.header);
                timer.append(res.header.id, res.header.msg_type(), time_now());
                stream.send_msg(&mut res).unwrap();
                }
            }
            _counter += 1;
        }
    }
    timer.dump(log_target.lock().unwrap().deref_mut());
    // info!("counter: {}, tsc: {}", counter, timer.len());
}

fn into_normal_ack(msg: Message, myid: c_uint) -> Message {
    let msg_id = msg.header.id;
    into_ack(msg, myid, msg_id)
}

fn into_ack<S: Serialize>(mut msg: Message, myid: c_uint, payload: S) -> Message {
    let current_header = msg.header;
    let new_header = MessageHeader::new(
        current_header.msg_type().ack().unwrap(),
        myid,
        current_header.saddr,
        ACK_MSG_ID,
    );
    msg.header = new_header;
    msg.change_payload(payload);
    msg
}

fn is_ready_to_send(mqueue: &MQueue) -> bool {
    (!mqueue.waiting_queue.is_empty()) && (mqueue.delivered_queue.is_empty())
}

fn get_msg_id(msg_id: Arc<Mutex<c_uint>>) -> c_uint {
    let mut msg_id_locked = msg_id.lock().unwrap();
    let current_msg_id = *msg_id_locked;
    *msg_id_locked += 1;
    current_msg_id
}
