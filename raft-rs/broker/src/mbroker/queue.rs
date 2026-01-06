use serde::Serialize;

use super::message::Message;
use std::{
    collections::{HashMap, VecDeque},
    ffi::c_int,
    sync::{RwLock, Arc},
};

/// A set of queues one-on-one with client
///
/// * `client_id` is an id of the client
/// * `queue` is the queue to store [`Message`] for sending to client
/// * `deliverd_queue` is the queue to store [`Message`] which has not been confirmed as complete for transmission
#[derive(Debug)]
pub struct MQueuePool {
    pool: HashMap<u32, Arc<RwLock<MQueue>>>,
}

impl MQueuePool {
    pub fn new() -> Self {
        Self {
            pool: HashMap::new(),
        }
    }

    pub fn add(&mut self, client_id: u32, mqueue: MQueue) -> &Arc<RwLock<MQueue>> {
        let mqueue= Arc::new(RwLock::new(mqueue));
        self.pool.insert(client_id, mqueue);
        self.pool.get(&client_id).unwrap()
    }

    pub fn find_by_id(&self, id: u32) -> Option<&Arc<RwLock<MQueue>>> {
        self.pool.get(&id)
    }

    pub fn status(&self) -> MQueuePoolStat {
        let messages_per_queue = self.pool.values().map(|queue| {
            queue
                .read()
                .unwrap()
                .waiting_queue
                .len()
                .try_into()
                .unwrap()
        });
        MQueuePoolStat {
            stat_type: 0,
            num_of_messages: messages_per_queue.clone().sum(),
            num_of_queues: self.pool.len().try_into().unwrap(),
            max_messages: messages_per_queue.max().unwrap(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct MQueuePoolStat {
    pub stat_type: c_int,
    pub num_of_messages: c_int,
    pub num_of_queues: c_int,
    pub max_messages: c_int,
}

/// A set of queues one-on-one with client
///
/// * `client_id` is an id of the client
/// * `queue` is the queue to store [`Message`] for sending to client
/// * `deliverd_queue` is the queue to store [`Message`] which has not been confirmed as complete for transmission
#[derive(Debug)]
pub struct MQueue {
    pub client_id: u32,
    pub waiting_queue: Queue<Message>,
    pub delivered_queue: Queue<Message>,
}

impl MQueue {
    pub fn new<U: Into<u32>>(client_id: U) -> Self {
        Self {
            client_id: client_id.into(),
            waiting_queue: Queue::new(),
            delivered_queue: Queue::new(),
        }
    }
}

#[derive(Debug)]
pub struct Queue<T> {
    queue: VecDeque<T>,
}

impl<T> Queue<T> {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn enqueue(&mut self, msg: T) {
        self.queue.push_back(msg)
    }

    pub fn dequeue(&mut self) -> Option<T> {
        self.queue.pop_front()
    }

    pub fn dequeue_by<P>(&mut self, predicate: P) -> Option<T>
    where
        P: FnMut(&T) -> bool,
    {
        let pos = self.queue.iter().position(predicate);
        pos.map(|pos| self.queue.remove(pos).unwrap())
    }

    pub fn get_front(&self) -> Option<T>
    where
        T: Clone,
    {
        self.queue.front().cloned()
    }
}
