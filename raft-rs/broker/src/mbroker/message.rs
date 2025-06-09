use serde::{Deserialize, Serialize};
use std::ffi::c_uint;
use std::io;
use std::mem::size_of;

pub const MSG_PAYLOAD_LEN: usize = 1024;
pub const MSG_HEADER_LEN: usize = size_of::<MessageHeader>();
pub const MSG_TOTAL_LEN: usize = MSG_PAYLOAD_LEN + MSG_HEADER_LEN;
pub const MSG_HEADER_END_INDEX: usize = MSG_HEADER_LEN;
pub const MSG_PAYLOAD_START_INDEX: usize = MSG_HEADER_LEN;

/// Packet to send to or reveice from client
#[derive(Debug, Clone)]
pub struct Message {
    pub header: MessageHeader,
    raw_msg: [u8; MSG_TOTAL_LEN],
}

impl Message {

    pub fn new(header: MessageHeader) -> Self {
        Self {
            header,
            raw_msg: [0; MSG_TOTAL_LEN],
        }
    }

    pub fn change_payload<S: Serialize>(&mut self, payload: S) {
        let payload_size = size_of::<S>();
        assert!(payload_size <= MSG_PAYLOAD_LEN);
        let raw_payload = bincode::serialize(&payload).unwrap();
        self.raw_msg[MSG_PAYLOAD_START_INDEX..(MSG_PAYLOAD_START_INDEX + payload_size)]
            .as_mut()
            .copy_from_slice(&raw_payload)
    }

    pub fn to_bytes(&mut self) -> &[u8] {
        let raw_header = bincode::serialize(&self.header).unwrap();
        self.raw_msg[0..MSG_HEADER_END_INDEX]
            .as_mut()
            .copy_from_slice(&raw_header);
        &self.raw_msg
    }

    pub fn from_bytes(raw_msg: &[u8]) -> Self {
        assert!(raw_msg.len() == MSG_TOTAL_LEN);
        let header = MessageHeader::from(&raw_msg[0..MSG_HEADER_END_INDEX]);
        Self {
            header,
            raw_msg: raw_msg.try_into().unwrap(),
        }
    }
}

impl From<[u8; MSG_TOTAL_LEN]> for Message {
    fn from(raw_msg: [u8; MSG_TOTAL_LEN]) -> Self {
        let header = MessageHeader::from(&raw_msg[0..MSG_HEADER_END_INDEX]);
        Self { header, raw_msg }
    }
}

/// The header of `Message`
///
/// * `tot_len` is total length of whole packet
/// * `msg_type` is [`MessageType`] as unsigned int
/// * `saddr` is source id
/// * `daddr` is destination id
/// * `id` is message id (In many cases, tihs is message's serial number)
#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct MessageHeader {
    pub tot_len: c_uint,
    msg_type: c_uint,
    pub saddr: c_uint,
    pub daddr: c_uint,
    pub id: c_uint,
}

impl MessageHeader {
    pub fn new<I1, I2, I3>(msg_type: MessageType, saddr: I1, daddr: I2, id: I3) -> Self
    where
        I1: Into<c_uint>,
        I2: Into<c_uint>,
        I3: Into<c_uint>,
    {
        Self {
            tot_len: MSG_TOTAL_LEN.try_into().unwrap(),
            msg_type: msg_type.into(),
            saddr: saddr.into(),
            daddr: daddr.into(),
            id: id.into(),
        }
    }

    pub fn msg_type(&self) -> MessageType {
        self.msg_type.try_into().unwrap()
    }

    pub fn change_msg_type(&mut self, new_msg_type: MessageType) {
        self.msg_type = new_msg_type.into()
    }
}

impl From<&[u8]> for MessageHeader {
    fn from(raw_msg_header: &[u8]) -> Self {
        assert!(raw_msg_header.len() == MSG_HEADER_LEN);
        bincode::deserialize(raw_msg_header).unwrap()
    }
}

/// Trait for enum that can be converted to `c_uint`
pub trait RecordableType {
    fn as_u32(&self) -> c_uint;
}

/// Message Type
#[derive(Debug, Copy, Clone)]
pub enum MessageType {
    SendReq,
    SendAck,
    RecvReq,
    RecvAck,
    FreeReq,
    FreeAck,
    PushReq,
    PushAck,
    HeloReq,
    HeloAck,
    StatReq,
    StatRes,
}

impl MessageType {
    /// Return *Ack type for own type
    ///
    /// If own type is already *Ack type, return None.
    pub fn ack(&self) -> Option<Self> {
        match self {
            MessageType::SendReq => Some(MessageType::SendAck),
            MessageType::RecvReq => Some(MessageType::RecvAck),
            MessageType::FreeReq => Some(MessageType::FreeAck),
            MessageType::PushReq => Some(MessageType::PushAck),
            MessageType::HeloReq => Some(MessageType::HeloAck),
            MessageType::StatReq => Some(MessageType::StatRes),
            _ => None,
        }
    }
}

impl TryFrom<c_uint> for MessageType {
    type Error = io::Error;

    fn try_from(value: c_uint) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(MessageType::SendReq),
            2 => Ok(MessageType::SendAck),
            3 => Ok(MessageType::RecvReq),
            4 => Ok(MessageType::RecvAck),
            5 => Ok(MessageType::FreeReq),
            6 => Ok(MessageType::FreeAck),
            7 => Ok(MessageType::PushReq),
            8 => Ok(MessageType::PushAck),
            9 => Ok(MessageType::HeloReq),
            10 => Ok(MessageType::HeloAck),
            11 => Ok(MessageType::StatReq),
            12 => Ok(MessageType::StatRes),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid number for Message Type",
            )),
        }
    }
}

impl From<MessageType> for c_uint {
    fn from(msg_type: MessageType) -> Self {
        (msg_type as Self) + 1
    }
}

impl RecordableType for MessageType {
    fn as_u32(&self) -> c_uint {
        (*self as c_uint) + 1
    }
}

/// Raft timestamp type
/// 各要素は，あるメッセージにおける Raft 処理のタイミングを表す
#[derive(Debug, Copy, Clone)]
#[repr(u64)]
pub enum RaftTimestampType {
    BeforeProposalEnqueue = 101,  //プロポーズに追加前
    BeforeLogAppend = 102,    // proposalsからのメッセージをRaftログに追加前
    BeforeMessageSend = 103,  // 送信処理部にメッセージ受け渡し前
    AfterRPCSent = 104, // RPC 送信後
    AfterRPCReceived = 105,  // RPC 受信後
    BeforeReceivedLogAppend = 106, // 受信部で受信したメッセージをRaftログに追加前
    BeforeStateMachineApply = 107,    // コミット済みエントリのステートマシン適用前
    AfterStateMachineApply = 108, // コミット済みエントリのステートマシン適用後
    AfterRaftProcessComplete = 109,   // Raft処理終了後
}

impl TryFrom<c_uint> for RaftTimestampType {
    type Error = io::Error;

    fn try_from(value: c_uint) -> Result<Self, Self::Error> {
        match value {
            101 => Ok(RaftTimestampType::BeforeProposalEnqueue),
            102 => Ok(RaftTimestampType::BeforeLogAppend),
            103 => Ok(RaftTimestampType::BeforeMessageSend),
            104 => Ok(RaftTimestampType::AfterRPCSent),
            105 => Ok(RaftTimestampType::AfterRPCReceived),
            106 => Ok(RaftTimestampType::BeforeReceivedLogAppend),
            107 => Ok(RaftTimestampType::BeforeStateMachineApply),
            108 => Ok(RaftTimestampType::AfterStateMachineApply),
            109 => Ok(RaftTimestampType::AfterRaftProcessComplete),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid number for Raft Timestamp Type",
            )),
        }
    }
}

impl From<RaftTimestampType> for c_uint {
    fn from(msg_type: RaftTimestampType) -> Self {
        msg_type as Self
    }
}

impl RecordableType for RaftTimestampType {
    fn as_u32(&self) -> c_uint {
        *self as c_uint
    }
}