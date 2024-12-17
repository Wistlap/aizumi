use super::message::{Message, MSG_TOTAL_LEN};
use nix::sys::socket::{setsockopt, sockopt};
use std::io::{self, Read, Write};
use std::net::ToSocketAddrs;
use std::net::{TcpListener, TcpStream};
use std::os::fd::AsFd;
use std::os::unix::io::AsRawFd;

/// A TCP socket server, listening for connections
#[derive(Debug)]
pub struct NetService {
    pub listener: TcpListener,
}

impl NetService {
    /// Creates a new TcpListener which will be bound to the specified address
    pub fn bind<T>(addr: T) -> Result<Self, io::Error>
    where
        T: ToSocketAddrs,
    {
        let listener = TcpListener::bind(addr)?;
        setsockopt(&listener, sockopt::ReuseAddr, &true)?;
        Ok(Self { listener })
    }

    /// Accept a new incoming connection from this listener
    pub fn accept(&self) -> Result<NetStream, io::Error> {
        let (stream, _) = self.listener.accept()?;
        stream.set_nodelay(true)?;
        stream.set_nonblocking(true)?;
        Ok(NetStream { stream })
    }
}

/// A TCP stream between a local and a remote socket
#[derive(Debug)]
pub struct NetStream {
    stream: TcpStream,
}

impl NetStream {
    /// Sets the read timeout to the timeout specified.
    ///
    /// Return [`io::ErrorKind::WouldBlock`] when a read times out
    // pub fn set_timeout(&mut self, duration: time::Duration) -> Result<(), io::Error> {
    //     self.stream.set_read_timeout(Some(duration))
    // }

    /// Receive [`Message`] from client
    pub fn recv_msg(&mut self) -> Result<Message, std::io::Error> {
        let mut buf: [u8; MSG_TOTAL_LEN] = [0; MSG_TOTAL_LEN];
        self.stream.read_exact(&mut buf)?;
        let msg = Message::from(buf);
        Ok(msg)
    }

    /// Send [`Message`] to client
    pub fn send_msg(&mut self, msg: &mut Message) -> Result<(), io::Error> {
        self.stream.write_all(msg.to_bytes())
    }
}

impl AsRawFd for NetStream {
    fn as_raw_fd(&self) -> std::os::unix::prelude::RawFd {
        self.stream.as_raw_fd()
    }
}

impl AsFd for NetStream {
    fn as_fd(&self) -> std::os::unix::prelude::BorrowedFd<'_> {
        self.stream.as_fd()
    }
}
