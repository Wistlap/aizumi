use super::message::{RaftTimestampType, RecordableType};
use core::arch::x86_64::_rdtsc;
use std::{collections::BTreeMap, ffi::c_uint, fmt::Display, io::Write};


#[derive(Debug)]
pub struct TimerStorage {
    record: Vec<TimerRecord>,
}

#[derive(Debug)]
struct TimerRecord {
    msg_id: c_uint,
    node_id_from: c_uint, // For logging raft messages
    node_id_to: c_uint,   // For logging raft messages
    msg_type: c_uint,
    tsc: u64,
}

impl TimerStorage {
    pub fn new() -> Self {
        Self { record: Vec::new() }
    }

    pub fn append<T>(&mut self, msg_id: c_uint, node_id_from: c_uint, node_id_to: c_uint, msg_type: T, tsc: u64)
    where
        T: RecordableType,
    {
        let new_record = TimerRecord::new(msg_id, node_id_from, node_id_to, msg_type, tsc);
        self.record.push(new_record);
    }

    pub fn merge_from(&mut self, mut other: TimerStorage) {
        self.record.append(&mut other.record);
    }

    pub fn len(&self) -> usize {
        self.record.len()
    }

    pub fn dump<W: Write>(&self, writer: &mut W) {
        let len = self.len();
        self.record
            .iter()
            .enumerate()
            .for_each(|(count, record)| writeln!(writer, "{},{},{}", count, len, record).unwrap())
    }
}

impl TimerRecord {
    fn new<T>(msg_id: c_uint, node_id_from: c_uint, node_id_to: c_uint, msg_type: T, tsc: u64) -> Self
    where
        T: RecordableType,
    {
        Self {
            msg_id,
            node_id_from,
            node_id_to,
            msg_type: msg_type.as_u32(),
            tsc,
        }
    }
}

impl Display for TimerRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{},{},{},{},{}", self.msg_id, self.node_id_from, self.node_id_to, self.msg_type, self.tsc)
    }
}

pub fn time_now() -> u64 {
    unsafe { _rdtsc() }
}

pub struct RaftTimerStorage {
    timestamps: BTreeMap<c_uint, TimerStorage>, // msg_id -> [(event, timestamp)]
}

impl RaftTimerStorage {
    pub fn new() -> Self {
        Self {
            timestamps: BTreeMap::new(),
        }
    }

    pub fn append(&mut self, msg_id: c_uint, node_id_from: c_uint, node_id_to: c_uint, msg_type: RaftTimestampType, tsc: u64){
        let new_record = TimerRecord::new(msg_id, node_id_from, node_id_to, msg_type, tsc);
        self
            .timestamps
            .entry(msg_id)
            .or_insert_with(TimerStorage::new)
            .record
            .push(new_record);
    }

    pub fn take(&mut self, msg_id: c_uint) -> Option<TimerStorage> {
        self.timestamps.remove(&msg_id)
    }

    pub fn take_all(&mut self) -> Option<TimerStorage> {
        if self.timestamps.is_empty() {
            None
        } else {
            let timestamps = std::mem::take(&mut self.timestamps);
            Some(TimerStorage {
                record: timestamps.into_iter().flat_map(|(_, storage)| storage.record).collect(),
            })
        }
    }
}
