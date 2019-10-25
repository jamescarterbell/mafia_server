use std::marker::PhantomData;
use std::marker::Send;
use std::net::SocketAddr;
use std::sync::mpsc::channel;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::thread::JoinHandle;
use ws::*;

pub struct ConnectedPlayer<P>
where
    P: Player + Send,
{
    pub socket: SocketStatus<P>,
    pub stream: Option<ws::Sender>,
    pub pending: Vec<Message>,
    pub player: Option<P>,
}

impl<P> Handler for ConnectedPlayer<P>
where
    P: Player + Send,
{
    fn on_message(&mut self, msg: Message) -> Result<()> {
        self.pending.push(msg);
        Result::Ok(())
    }
}

pub enum SocketStatus<P>
where
    P: Player,
    ConnectionFactory<P>: Factory,
{
    Uninitialized(WebSocket<ConnectionFactory<P>>),
    Listening(
        (
            JoinHandle<std::result::Result<WebSocket<ConnectionFactory<P>>, ()>>,
            Receiver<bool>,
        ),
    ),
    Connected(WebSocket<ConnectionFactory<P>>),
    Hold,
    ConnectionError,
    ClientConnection,
    ServerConnection,
}

pub trait Player {}

pub struct ConnectionFactory<P>
where
    P: Player,
{
    _phantom: PhantomData<P>,
}

impl<P> Factory for ConnectionFactory<P>
where
    P: Player + Send,
{
    type Handler = ConnectedPlayer<P>;

    fn connection_made(&mut self, ws: ws::Sender) -> ConnectedPlayer<P> {
        ConnectedPlayer {
            socket: SocketStatus::ServerConnection,
            stream: Some(ws),
            pending: vec![],
            player: None,
        }
    }

    fn client_connected(&mut self, ws: ws::Sender) -> ConnectedPlayer<P> {
        println!("Dab");
        ConnectedPlayer {
            socket: SocketStatus::ClientConnection,
            stream: Some(ws),
            pending: vec![],
            player: None,
        }
    }
}

impl<P> ConnectedPlayer<P>
where
    P: Player + Send + 'static,
{
    pub fn new() -> Self {
        ConnectedPlayer {
            socket: SocketStatus::Uninitialized(
                ws::WebSocket::new(ConnectionFactory {
                    _phantom: PhantomData,
                })
                .unwrap(),
            ),
            stream: None,
            pending: vec![],
            player: None,
        }
    }

    pub fn open_connections(mut self) -> Self {
        let (send, recieve) = channel::<bool>();
        if let SocketStatus::Uninitialized(listener) = self.socket {
            self.socket = SocketStatus::Listening((
                thread::spawn(|| {
                    let addr = listener.local_addr().unwrap();
                    ConnectedPlayer::listen(listener, addr, send)
                }),
                recieve,
            ));
        }
        self
    }

    pub fn listen(
        stream: WebSocket<ConnectionFactory<P>>,
        address: SocketAddr,
        send: Sender<bool>,
    ) -> std::result::Result<WebSocket<ConnectionFactory<P>>, ()> {
        if let Result::Ok(out_stream) = stream.listen(address) {
            let _ = send.send(true);
            return std::result::Result::Ok(out_stream);
        }
        std::result::Result::Err(())
    }

    pub fn check_connections(&mut self) -> bool {
        let mut hold = SocketStatus::Hold;

        use std::mem;
        mem::swap(&mut self.socket, &mut hold);

        // Temp solution
        match hold {
            SocketStatus::Listening((handle, recieve)) => {
                if recieve.try_iter().count() != 0 {
                    match handle.join().unwrap() {
                        Ok(stream) => {
                            self.stream = Some(stream.broadcaster());
                            self.socket = SocketStatus::Connected(stream);
                            println!("Connected!");
                            return true;
                        }
                        Err(_) => {
                            self.socket = SocketStatus::ConnectionError;
                        }
                    }
                } else {
                    self.socket = SocketStatus::Listening((handle, recieve));
                }
            }
            SocketStatus::Connected(stream) => {
                self.socket = SocketStatus::Connected(stream);
                return true;
            }
            SocketStatus::Uninitialized(listener) => {
                self.socket = SocketStatus::Uninitialized(listener)
            }
            _ => self.socket = SocketStatus::ConnectionError,
        }
        return false;
    }
}
