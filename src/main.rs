//! Based on:
//! https://github.com/tokio-rs/tokio/blob/4ebaf18c2729ebc9e110e137682ecc9461c3659d/examples/chat.rs

#![deny(warnings)]

use futures::try_ready;
use std::io::{Error, ErrorKind};

#[macro_use]
extern crate serde_derive;

mod pca9685;

use bytes::{BufMut, Bytes, BytesMut};
use futures::sync::mpsc;
use tokio::io;
use tokio::net::{TcpListener, TcpStream};
use tokio::prelude::*;

use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;

use std::process;
use std::str;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
type Tx = mpsc::UnboundedSender<Bytes>;
type Rx = mpsc::UnboundedReceiver<Bytes>;

mod sensors;
use crate::sensors::event::{ArduinoEvent, Event, TimedEvent};
use crate::sensors::*;
mod command;
use crate::command::Command;

mod motor;
mod motor_handler;
use crate::motor_handler::MotorHandler;

type EventTx = mpsc::UnboundedSender<TimedEvent>;
type EventRx = mpsc::UnboundedReceiver<TimedEvent>;
type CommandTx = mpsc::UnboundedSender<Command>;
type CommandRx = mpsc::UnboundedReceiver<Command>;

type WsTx = mpsc::UnboundedSender<Message>;

struct Shared {
    clients: HashMap<SocketAddr, Tx>,
    ws_clients: HashMap<SocketAddr, WsTx>,
    server_tx: Tx,
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
            ws_clients: HashMap::new(),
            server_tx,
        }
    }
}

impl Client {
    fn new(state: Arc<Mutex<Shared>>, lines: Lines) -> Client {
        let addr = lines.socket.peer_addr().unwrap();
        let (tx, rx) = mpsc::unbounded();
        state.lock().unwrap().clients.insert(addr, tx);

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

                    if i + 1 == LINES_PER_TICK {
                        task::current().notify();
                    }
                }
                _ => break,
            }
        }

        let _ = self.lines.poll_flush()?;

        while let Async::Ready(line) = self.lines.poll()? {
            if let Some(message) = line {
                let mut line = BytesMut::new();
                line.extend_from_slice(&message);

                let line = line.freeze();

                match &self
                    .state
                    .lock()
                    .unwrap()
                    .server_tx
                    .unbounded_send(line.clone())
                {
                    Ok(_) => (),
                    Err(e) => println!("send error = {:?}", e),
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
        self.state.lock().unwrap().clients.remove(&self.addr);
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
        let pos = self
            .rd
            .windows(2)
            .enumerate()
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

    let peer = Client::new(state, lines).map_err(|e| {
        println!("connection error = {:?}", e);
    });

    tokio::spawn(peer);
}

fn process_ws(
    socket: TcpStream,
    state: Arc<Mutex<Shared>>,
) -> Box<Future<Item = (), Error = io::Error> + Send> {
    let addr = socket
        .peer_addr()
        .expect("connected streams should have a peer address");
    let state_clone = state.clone();
    let future = accept_async(socket)
        .and_then(move |ws_stream| {
            println!("New WebSocket connection: {}", addr);

            let (tx, rx) = futures::sync::mpsc::unbounded();
            state.lock().unwrap().ws_clients.insert(addr, tx);
            let (sink, source) = ws_stream.split();

            let ws_reader = source.for_each(move |message| {
                println!("Received a ws message: {}", message);

                let mut line = BytesMut::new();
                line.extend_from_slice(&message.into_data());
                let line = line.freeze();

                match state_clone
                    .lock()
                    .unwrap()
                    .server_tx
                    .unbounded_send(line.clone())
                {
                    Ok(_) => (),
                    Err(e) => println!("send error = {:?}", e),
                }

                Ok(())
            });

            let ws_writer = rx.fold(sink, |mut sink, msg| {
                use futures::Sink;
                sink.start_send(msg).unwrap();
                Ok(sink)
            });

            let connection = ws_reader
                .map(|_| ())
                .map_err(|_| ())
                .select(ws_writer.map(|_| ()).map_err(|_| ()));

            tokio::spawn(connection.then(move |_| {
                // remove socket from state here
                //  connections_inner.lock().unwrap().remove(&addr);
                state.lock().unwrap().ws_clients.remove(&addr);
                println!("Websocket connection closed: {}", addr);
                Ok(())
            }));

            Ok(())
        })
        .map_err(|e| Error::new(ErrorKind::Other, e));

    Box::new(future)
}

pub fn main() {
    let i2c_output = process::Command::new("i2cdetect")
        .arg("-y")
        .arg("1")
        .output();
    match i2c_output {
        Ok(output) => {
            println!("{}", str::from_utf8(&output.stdout).unwrap());
            println!("{}", str::from_utf8(&output.stderr).unwrap())
        }
        Err(_) => println!("Could not read from i2cdetect"),
    };

    let (server_tx, server_rx): (Tx, Rx) = mpsc::unbounded();
    let (sensors_tx, sensors_rx): (EventTx, EventRx) = mpsc::unbounded();
    let (_commands_tx, _commands_rx): (CommandTx, CommandRx) = mpsc::unbounded();
    let state = Arc::new(Mutex::new(Shared::new(server_tx)));

    let addr = "0.0.0.0:5000".parse().unwrap();
    let ws_addr = "0.0.0.0:5001".parse().unwrap();

    let listener = TcpListener::bind(&addr).unwrap();
    let ws_listener = TcpListener::bind(&ws_addr).unwrap();

    let local_state = state.clone();
    let ws_server = ws_listener
        .incoming()
        .for_each(move |socket| process_ws(socket, local_state.clone()))
        .map_err(|err| {
            println!("ws accept error = {:?}", err);
        });

    let local_state = state.clone();
    let server = listener
        .incoming()
        .for_each(move |socket| {
            process(socket, local_state.clone());
            Ok(())
        })
        .map_err(|err| {
            println!("ws accept error = {:?}", err);
        });

    println!("server running on localhost:5000");

    let sensors_tx_arc = Arc::new(Mutex::new(sensors_tx));

    let (motor_handler, motor_handler_tx_command, motor_handler_tx_event) =
        MotorHandler::new(sensors_tx_arc.clone());
    let (arduino, arduino_tx) = arduino::Arduino::new(sensors_tx_arc.clone());
    let receive_messages = server_rx
        .for_each(move |line| {
            println!("Received line on server: {:?}", line);

            let command_result: Result<Command, serde_json::Error> = serde_json::from_slice(&line);

            match command_result {
                Ok(Command::Arduino { command }) => {
                    arduino_tx.unbounded_send(command).unwrap();
                }
                Ok(Command::Motor { command }) => {
                    motor_handler_tx_command.unbounded_send(command).unwrap();
                }
                Err(e) => println!("could not deserialize command = {:?}", e),
            };

            Ok(())
        })
        .map_err(|err| {
            println!("line reading error = {:?}", err);
        });

    let local_state = state.clone();
    let receive_sensor_messages = sensors_rx
        .for_each(move |event| {
            let event_json = serde_json::to_string(&event).unwrap();
            //            println!("Received sensor message, broadcasting: {:?}", &event_json);

            match event {
                TimedEvent {
                    event: Event::Arduino { event: e },
                    time: _u128,
                } => match e {
                    ArduinoEvent::Encoders { encoders } => {
                        motor_handler_tx_event.unbounded_send(encoders).unwrap();
                        ()
                    }
                    _ => (),
                },
                _ => (),
            };

            let mut line = BytesMut::new();
            line.extend_from_slice(event_json.as_bytes());
            line.extend_from_slice(b"\r\n");
            let line = line.freeze();

            let shared = local_state.lock().unwrap();
            for (_, tx) in &shared.clients {
                tx.unbounded_send(line.clone()).unwrap();
            }

            for (_, ws_tx) in &shared.ws_clients {
                ws_tx
                    .unbounded_send(Message::Text(
                        str::from_utf8(&line).unwrap().trim().to_string(),
                    ))
                    .unwrap();
            }

            Ok(())
        })
        .map_err(|err| {
            println!("line reading error = {:?}", err);
        });

    let ir = ir::Ir::new(sensors_tx_arc.clone());
    let encoder = encoder::Encoder::new(sensors_tx_arc.clone());
    let gyro = gyro::Gyro::new(sensors_tx_arc.clone());
    let lidar = lidar::Lidar::new(sensors_tx_arc.clone());
    let compass = compass::Compass::new(sensors_tx_arc.clone());
    let axl = axl::Axl::new(sensors_tx_arc.clone());
    encoder.run();

    let joined = server
        .join(ws_server)
        .join(receive_messages)
        .join(receive_sensor_messages)
        .join(ir.run())
        .join(gyro.run())
        .join(lidar.run())
        .join(compass.run())
        .join(axl.run())
        .join(arduino.run())
        .join(motor_handler.run())
        .map(|_| ());

    tokio::run(joined);
}
