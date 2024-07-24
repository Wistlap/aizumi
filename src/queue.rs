use super::Request;
use actix_web::web;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

pub type Queue = web::Data<MsgQueuePool>;

pub struct MsgQueuePool {
    hash: RwLock<HashMap<i32, Arc<Mutex<Vec<Request>>>>>,
}

impl MsgQueuePool {
    pub fn new() -> Self {
        Self {
            hash: RwLock::new(HashMap::new()),
        }
    }

    /// Add new vec to hash with specified queue_id.
    pub fn add_queue(&self, queue_id: i32) {
        let queue = Arc::new(Mutex::new(Vec::new()));
        self.hash.write().unwrap().insert(queue_id, queue);
    }

    ///Remove queue with specified queue_id.
    pub fn remove_queue(&self, queue_id: i32) {
        self.hash.write().unwrap().remove(&queue_id);
    }

    /// If specified queue_id's queue already exists, enqueue data (Request).
    /// Else, create queue with specified queue_id, and enqueue data.
    pub fn enqueue(&self, data: Request, queue_id: i32) {
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
    pub fn dequeue_with_id(&self, queue_id: i32, msg_id: i32) -> Option<Request> {
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

    // /// Return MsgQueueStat about all queues mapped.
    // fn status(&self) -> MsgQueueStat {
    //     let msgs_per_vecs = self.hash.read().unwrap().values().map(|queue| {
    //         queue
    //             .lock()
    //             .unwrap()
    //             .len()
    //             .try_into()
    //             .unwrap()
    //     });
    //     MsgQueueStat {
    //         stattype: 0,
    //         num_of_messages: msgs_per_vecs.clone().sum(),
    //         num_of_queues: self.hash.read().unwrap().len().try_into().unwrap(),
    //         max_messages: msgs_per_vecs.max().unwrap(),
    //     }
    // }
}
#[derive(Serialize, Debug)]
struct MsgQueueStat {
    stattype: i32,
    num_of_messages: i32,
    num_of_queues: i32,
    max_messages: i32,
}
