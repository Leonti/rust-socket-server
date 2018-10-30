//! Based on:
//! https://github.com/tokio-rs/tokio/blob/4ebaf18c2729ebc9e110e137682ecc9461c3659d/examples/chat.rs

#![deny(warnings)]

extern crate tokio;
#[macro_use]
extern crate futures;
extern crate bytes;

use tokio::io;
use tokio::net::{TcpListener, TcpStream};
use tokio::prelude::*;
use tokio::timer::Interval;
use std::time::{Duration, Instant};
use futures::sync::mpsc;
use bytes::{BytesMut, Bytes, BufMut};

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
type Tx = mpsc::UnboundedSender<Bytes>;
type Rx = mpsc::UnboundedReceiver<Bytes>;

struct Shared {
    clients: HashMap<SocketAddr, Tx>,
    server_tx: Tx
}

struct Client {
    lines: Lines,
    state: Arc<Mutex<Shared>>,
    rx: Rx,
    addr: SocketAddr,
}

#[derive(Debug)]
struct Lines {
    socket: TcpStream,
    rd: BytesMut,
    wr: BytesMut,
}

impl Shared {
    fn new(server_tx: Tx) -> Self {
        Shared {
            clients: HashMap::new(),
            server_tx
        }
    }
}

impl Client {

    fn new(state: Arc<Mutex<Shared>>,
           lines: Lines) -> Client
    {

        let addr = lines.socket.peer_addr().unwrap();
        let (tx, rx) = mpsc::unbounded();
        state.lock().unwrap()
            .clients.insert(addr, tx);

        Client {
            lines,
            state,
            rx,
            addr,
        }
    }
}

impl Future for Client {
    type Item = ();
    type Error = io::Error;

    fn poll(&mut self) -> Poll<(), io::Error> {
        const LINES_PER_TICK: usize = 10;

        for i in 0..LINES_PER_TICK {
            match self.rx.poll().unwrap() {
                Async::Ready(Some(v)) => {
                    self.lines.buffer(&v);

                    if i+1 == LINES_PER_TICK {
                        task::current().notify();
                    }
                }
                _ => break,
            }
        }

        let _ = self.lines.poll_flush()?;

        while let Async::Ready(line) = self.lines.poll()? {
            println!("Received line {:?}", line);

            if let Some(message) = line {
                let mut line = BytesMut::new();
                line.extend_from_slice(&message);

                let line = line.freeze();
                let is_closed = &self.state.lock().unwrap().server_tx.is_closed();
                println!("Is closed {:?}", is_closed);

                match &self.state.lock().unwrap().server_tx.unbounded_send(line.clone()) {
                    Ok(_) => println!("Message sent"),
                    Err(e) => println!("send error = {:?}", e)
                }
            } else {
                return Ok(Async::Ready(()));
            }
        }
        Ok(Async::NotReady)
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        self.state.lock().unwrap().clients
            .remove(&self.addr);
    }
}

impl Lines {
    fn new(socket: TcpStream) -> Self {
        Lines {
            socket,
            rd: BytesMut::new(),
            wr: BytesMut::new(),
        }
    }

    fn buffer(&mut self, line: &[u8]) {
        self.wr.reserve(line.len());
        self.wr.put(line);
    }

    fn poll_flush(&mut self) -> Poll<(), io::Error> {
        while !self.wr.is_empty() {
            let n = try_ready!(self.socket.poll_write(&self.wr));

            assert!(n > 0);

            let _ = self.wr.split_to(n);
        }

        Ok(Async::Ready(()))
    }

    fn fill_read_buf(&mut self) -> Poll<(), io::Error> {
        loop {
            self.rd.reserve(1024);
            let n = try_ready!(self.socket.read_buf(&mut self.rd));

            if n == 0 {
                return Ok(Async::Ready(()));
            }
        }
    }
}

impl Stream for Lines {
    type Item = BytesMut;
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {

        let sock_closed = self.fill_read_buf()?.is_ready();
        let pos = self.rd.windows(2).enumerate()
            .find(|&(_, bytes)| bytes == b"\r\n")
            .map(|(i, _)| i);

        if let Some(pos) = pos {
            let mut line = self.rd.split_to(pos + 2);
            line.split_off(pos);

            return Ok(Async::Ready(Some(line)));
        }

        if sock_closed {
            Ok(Async::Ready(None))
        } else {
            Ok(Async::NotReady)
        }
    }
}

fn process(socket: TcpStream, state: Arc<Mutex<Shared>>) {
    let lines = Lines::new(socket);

    let peer = Client::new(
        state,
        lines).map_err(|e| {
            println!("connection error = {:?}", e);
        });

    tokio::spawn(peer);
}

pub fn main() {

    let (server_tx, server_rx): (Tx, Rx) = mpsc::unbounded();
    let (sensors_tx, sensors_rx): (Tx, Rx) = mpsc::unbounded();
    let state = Arc::new(Mutex::new(Shared::new(server_tx)));

    let addr = "127.0.0.1:6142".parse().unwrap();

    let listener = TcpListener::bind(&addr).unwrap();

    let local_state = state.clone();
    let server = listener.incoming().for_each(move |socket| {
        process(socket, local_state.clone());
        Ok(())
    }).map_err(|err| {
        println!("accept error = {:?}", err);
    });

    println!("server running on localhost:6142");

    let receive_messages = server_rx.for_each(|line| {
        println!("Received line on server: {:?}", line);
        Ok(())
    }).map_err(|err| {
        println!("line reading error = {:?}", err);
    });

    let local_state = state.clone();
    let receive_sensor_messages = sensors_rx.for_each(move |line| {
        println!("Received sensor message, broadcasting: {:?}", line);

        let shared = local_state.lock().unwrap();
        for (_, tx) in &shared.clients {
            tx.unbounded_send(line.clone()).unwrap();
        }

        Ok(())
    }).map_err(|err| {
        println!("line reading error = {:?}", err);
    });

    let sensors = Interval::new(Instant::now(), Duration::from_millis(1000))
        .for_each(move |instant| {
            println!("fire; instant={:?}", instant);

            let mut line = BytesMut::new();
            line.extend_from_slice(b"Sensor message\r\n");
            let line = line.freeze();

            match sensors_tx.unbounded_send(line.clone()) {
                Ok(_) => println!("Sensor message sent"),
                Err(e) => println!("send error = {:?}", e)
            }

            Ok(())
        })
        .map_err(|e| panic!("interval errored; err={:?}", e));

    let joined = server
        .join(receive_messages)
        .join(receive_sensor_messages)
        .join(sensors).map(|_| ());

    tokio::run(joined);
}
