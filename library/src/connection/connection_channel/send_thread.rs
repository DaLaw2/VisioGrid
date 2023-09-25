use std::rc::Weak;
use std::sync::Arc;
use std::cell::RefCell;
use std::net::TcpStream;
use std::sync::atomic::AtomicBool;
use crate::connection::connection_channel::definition::ConnectChannel;

pub struct SendThread<T: ConnectChannel> {
    socket: TcpStream,
    stop_signal: Arc<AtomicBool>,
    connection_channel: Weak<RefCell<T>>
}

impl<T: ConnectChannel> SendThread<T> {
    pub fn new(socket: TcpStream, connection_channel: Weak<RefCell<T>>) -> Self {
        let stop_signal = connection_channel.upgrade().unwrap()
        Self {
            socket,
            stop_signal,
            connection_channel
        }
    }

    pub fn
}