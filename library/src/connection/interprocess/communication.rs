use std::fs;
use std::thread;
use std::time::Duration;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use interprocess::local_socket::{LocalSocketStream, LocalSocketListener};

#[derive(Debug)]
pub enum Error {
    FailCreateConnect,
    FailDestroyConnect,
    FailLockSharedResource,
    FailTransmission,
    ConnectionRefused,
}

pub struct Sender {
    socket_name: String,
    is_connected: bool,
    thread_handle: Option<JoinHandle<Result<(), Error>>>,
    stop_signal: Arc<AtomicBool>,
    send_data: Arc<Mutex<VecDeque<String>>>,
}

impl Sender {
    pub fn new(socket_name: String) -> Sender {
        Sender {
            socket_name,
            is_connected: false,
            thread_handle: None,
            stop_signal: Arc::new(AtomicBool::new(false)),
            send_data: Arc::new(Mutex::new(VecDeque::new()))
        }
    }

    pub fn connect(&mut self) -> Result<(), Error> {
        if self.is_connected() {
            return Ok(())
        }
        let listener = LocalSocketListener::bind(format!("/tmp/{}.sock", self.socket_name)).map_err(|_| Error::FailCreateConnect)?;
        self.stop_signal.store(false, Ordering::Relaxed);
        let stop_signal = self.stop_signal.clone();
        let send_data = self.send_data.clone();
        let thread_handle: JoinHandle<Result<(), Error>> = thread::spawn(move || {
            let mut stream = listener.accept().map_err(|_| Error::ConnectionRefused)?;
            while !stop_signal.load(Ordering::Relaxed) {
                let mut send_data = send_data.lock().map_err(|_| Error::FailLockSharedResource)?;
                if let Some(message) = send_data.pop_front() {
                    stream.write_all(message.as_bytes()).map_err(|_| Error::FailTransmission)?;
                }
            }
            Ok(())
        });
        self.thread_handle = Some(thread_handle);
        self.is_connected = true;
        Ok(())
    }

    pub fn disconnect(&mut self) -> Result<(), Error> {
        if !self.is_connected() {
            return Ok(());
        }
        self.stop_signal.store(true, Ordering::Relaxed);
        self.is_connected = false;
        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        self.is_connected
    }

    pub fn send(&mut self, message: String) -> Result<(), Error> {
        let mut send_data = self.send_data.lock().map_err(|_| Error::FailLockSharedResource)?;
        send_data.push_back(message);
        Ok(())
    }
}

pub struct Receiver {
    socket_name: String,
    is_connected: bool,
    thread_handle: Option<JoinHandle<Result<(), Error>>>,
    stop_signal: Arc<AtomicBool>,
    received_data: Arc<Mutex<VecDeque<String>>>
}

impl Receiver {
    pub fn new(socket_name: String) -> Receiver {
        let connector = Receiver {
            socket_name,
            is_connected: false,
            thread_handle: None,
            stop_signal: Arc::new(AtomicBool::new(false)),
            received_data: Arc::new(Mutex::new(VecDeque::new())),
        };
        connector
    }

    pub fn connect(&mut self) -> Result<(), Error> {
        if self.is_connected() {
            return Ok(());
        }
        let mut connection = LocalSocketStream::connect(format!("/tmp/{}.sock", self.socket_name)).map_err(|_| Error::FailCreateConnect)?;
        self.stop_signal.store(false, Ordering::Relaxed);
        let stop_signal = self.stop_signal.clone();
        let received_data = self.received_data.clone();
        let thread_handle: JoinHandle<Result<(), Error>> = thread::spawn(move || {
            while !stop_signal.load(Ordering::Relaxed) {
                let mut buffer = [0; 1024];
                match connection.read(&mut buffer) {
                    Ok(bytes_read) => {
                        if bytes_read == 0 {
                            thread::sleep(Duration::from_millis(10));
                        } else {
                            let message = String::from_utf8_lossy(&buffer[..bytes_read]);
                            let mut received_data = received_data.lock().map_err(|_| Error::FailLockSharedResource)?;
                            received_data.push_back(message.to_string());
                        }
                    },
                    Err(_) => {
                        return Err(Error::FailTransmission);
                    }
                }
            }
            Ok(())
        });
        self.thread_handle = Some(thread_handle);
        self.is_connected = true;
        Ok(())
    }

    pub fn disconnect(&mut self) -> Result<(), Error> {
        if !self.is_connected() {
            return Ok(());
        }
        self.stop_signal.store(true, Ordering::Relaxed);
        fs::remove_file(format!("/tmp/{}.sock", self.socket_name)).map_err(|_| Error::FailDestroyConnect)?;
        self.is_connected = false;
        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        self.is_connected
    }

    pub fn receive(&mut self) -> Result<String, Error> {
        let mut received_data = self.received_data.lock().map_err(|_| Error::FailLockSharedResource)?;
        Ok(received_data.pop_front().map_or("".to_string(), |str| str))
    }
}