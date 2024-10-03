use super::Request;
use actix_web::web;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Serialize, Debug, Deserialize, Clone, Default)]
pub struct MsgQueue {
    queue: Arc<Mutex<Vec<Request>>>,
}

#[derive(Serialize, Debug, Deserialize, Clone, Default)]
pub struct MsgQueuePool {
    hash: HashMap<i32, MsgQueue>,
}

pub type Queue = web::Data<MsgQueuePool>;

impl MsgQueue {
    pub fn new() -> Self {
        Self {
            queue: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Enqueue data (Request).
    pub fn enqueue(&self, data: Request) {
        self.queue.lock().unwrap().push(data);
    }

    /// Dequeue data (Request).
    /// If queue is empty, return None.
    pub fn dequeue(&self) -> Option<Request> {
        self.queue.lock().unwrap().pop()
    }

    // /// Return MsgQueueStat.
    // pub fn status(&self) -> MsgQueueStat {
    //     MsgQueueStat {
    //         stattype: 0,
    //         num_of_messages: self.queue.lock().unwrap().len() as i32,
    //     }
    // }
}

impl MsgQueuePool {
    pub fn new() -> Self {
        Self {
            hash: HashMap::new(),
        }
    }

    /// Add new vec to hash with specified queue_id.
    pub fn add_queue(&mut self, queue_id: i32, queue: MsgQueue) {
        self.hash.insert(queue_id, queue);
    }

    ///Remove queue with specified queue_id.
    pub fn remove_queue(&mut self, queue_id: i32) {
        self.hash.remove(&queue_id);
    }

    /// If specified queue_id's queue already exists, enqueue data (Request).
    /// Else, create queue with specified queue_id, and enqueue data.
    pub fn enqueue(&mut self, data: Request, queue_id: i32) {
        if let Some(queue) = self.hash.get(&queue_id) {
            queue.enqueue(data);
            return;
        }
        let queue = MsgQueue::new();
        queue.enqueue(data);
        self.add_queue(queue_id, queue)
    }

    /// Dequeue specified id's data (Request) from specified queue_id's queue.
    /// If that data does not exist, return None.
    #[allow(clippy::manual_map)]
    pub fn dequeue(&self, queue_id: i32) -> Option<Request> {
        if let Some(queue) = self.hash.get(&queue_id) {
            queue.dequeue()
        } else {
            None
        }
    }

    /// Return if specified queue_id's queue is empty.
    #[allow(clippy::manual_map)]
    pub fn is_empty(&self, queue_id: i32) -> bool {
        if let Some(queue) = self.hash.get(&queue_id) {
            queue.queue.lock().unwrap().is_empty()
        } else {
            true
        }
    }

    /// Return if specified queue_id's queue exists.
    pub fn is_exist(&self, queue_id: &i32) -> bool {
        self.hash.contains_key(queue_id)
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
#[allow(dead_code)]
struct MsgQueueStat {
    stattype: i32,
    num_of_messages: i32,
    num_of_queues: i32,
    max_messages: i32,
}
