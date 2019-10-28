use std::io::Read;
use std::io::Write;
use std::marker::PhantomData;
use std::marker::Send;
use std::net::SocketAddr;
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::channel;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::thread::JoinHandle;
use websocket::server;

pub struct ConnectedPlayer<P>
where
    P: Player + Send,
{
    pub socket: SocketStatus,
    pub stream: Option<TcpStream>,
    pub player: Option<P>,
    pub failure_count: usize,
}

pub enum SocketStatus {
    Uninitialized(TcpListener),
    Listening(JoinHandle<Result<TcpStream, ()>>, Receiver<bool>),
    Connected,
    Hold,
    ConnectionError,
}

pub trait Player {
    fn get_state(&self) -> String;
}

pub enum ConnectionStatus {
    NotConnected,
    Connected,
    Error,
}

impl<P> ConnectedPlayer<P>
where
    P: Player + Send + 'static,
{
    pub fn new() -> Self {
        ConnectedPlayer {
            socket: SocketStatus::Uninitialized(TcpListener::bind("127.0.0.1:00000").unwrap()),
            stream: None,
            player: None,
            failure_count: 0,
        }
    }

    pub fn open_connections(mut self) -> Self {
        let (send, recieve) = channel::<bool>();
        if let SocketStatus::Uninitialized(listener) = self.socket {
            self.socket =
                SocketStatus::Listening(thread::spawn(|| listen(listener, send)), recieve);
        }
        self
    }

    pub fn check_connections(&mut self) -> ConnectionStatus {
        let mut hold = SocketStatus::Hold;

        use std::mem;
        mem::swap(&mut self.socket, &mut hold);

        if self.failure_count > 100 {
            self.socket = SocketStatus::ConnectionError;
        }
        // Temp solution
        match hold {
            SocketStatus::Listening(handle, recieve) => {
                if recieve.try_iter().count() != 0 {
                    match handle.join().unwrap() {
                        Ok(stream) => {
                            self.stream = Some(stream);
                            self.socket = SocketStatus::Connected;
                            println!("Connected!");
                        }
                        Err(_) => {
                            self.socket = SocketStatus::ConnectionError;
                        }
                    }
                } else {
                    self.failure_count += 1;
                    self.socket = SocketStatus::Listening(handle, recieve);
                }
                return ConnectionStatus::NotConnected;
            }
            SocketStatus::Connected => {
                self.socket = SocketStatus::Connected;
                return ConnectionStatus::Connected;
            }
            SocketStatus::Uninitialized(listener) => {
                self.failure_count += 1;
                self.socket = SocketStatus::Uninitialized(listener);
                return ConnectionStatus::NotConnected;
            }
            SocketStatus::ConnectionError | SocketStatus::Hold => {
                println!("Error on socket!");
                if let Some(stream) = &self.stream {
                    let _ = stream.shutdown(std::net::Shutdown::Both);
                    self.stream = None;
                };
                self.socket = SocketStatus::ConnectionError
            }
        }
        return ConnectionStatus::Error;
    }

    pub fn send_state(&mut self, state: String) -> Result<(), ()> {
        //create json
        if let Some(stream) = &mut self.stream {
            let buffer = state.as_bytes();
            let length = buffer.len();
            let _ = stream.write_all(&length.to_be_bytes());
            let _ = stream.write_all(buffer);
            return Ok(());
        }
        Err(())
    }

    pub fn read_input(&mut self, buf: &mut Vec<u8>) -> Result<(), String> {
        if let Some(stream) = &mut self.stream {
            let _ = stream.read_exact(buf);
            *buf = vec![0; byte_be_to_usize(&buf)];
            let _ = stream.read_exact(buf);
        }
        Ok(())
    }
}

fn listen(stream: TcpListener, send: Sender<bool>) -> std::result::Result<TcpStream, ()> {
    if let Result::Ok((out_stream, _addr)) = stream.accept() {
        let _ = send.send(true);
        return std::result::Result::Ok(out_stream);
    }
    std::result::Result::Err(())
}

fn byte_be_to_usize(buf: &Vec<u8>) -> usize {
    let mut out = 0;
    for num in buf.iter() {
        out <<= 4;
        out |= *num as usize;
    }
    out
}
