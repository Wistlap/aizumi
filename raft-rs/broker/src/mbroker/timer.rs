use super::MessageType;
use core::arch::x86_64::_rdtsc;
use std::{ffi::c_uint, fmt::Display, io::Write};

#[derive(Debug)]
pub struct TimerStorage {
    record: Vec<TimerRecord>,
}

#[derive(Debug)]
struct TimerRecord {
    msg_id: c_uint,
    msg_type: c_uint,
    tsc: u64,
}

impl TimerStorage {
    pub fn new() -> Self {
        Self { record: Vec::new() }
    }

    pub fn append(&mut self, msg_id: c_uint, msg_type: MessageType, tsc: u64) {
        let new_record = TimerRecord::new(msg_id, msg_type, tsc);
        self.record.push(new_record);
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
    fn new(msg_id: c_uint, msg_type: MessageType, tsc: u64) -> Self {
        Self {
            msg_id,
            msg_type: msg_type.into(),
            tsc,
        }
    }
}

impl Display for TimerRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{},{},{}", self.msg_id, self.msg_type, self.tsc)
    }
}

pub fn time_now() -> u64 {
    unsafe { _rdtsc() }
}
