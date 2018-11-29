//! Based on:
//! https://github.com/tokio-rs/tokio/blob/4ebaf18c2729ebc9e110e137682ecc9461c3659d/examples/chat.rs

#![deny(warnings)]

extern crate tokio;
#[macro_use]
extern crate futures;
extern crate bytes;
extern crate sysfs_gpio;
extern crate tokio_codec;
extern crate tokio_io;
extern crate tokio_serial;

extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate serde_derive;
extern crate i2c_pca9685;
extern crate i2cdev;

use bytes::{BufMut, Bytes, BytesMut};
use futures::sync::mpsc;
use tokio::io;
use tokio::net::{TcpListener, TcpStream};
use tokio::prelude::*;

use std::io::Write;
use std::process;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
type Tx = mpsc::UnboundedSender<Bytes>;
type Rx = mpsc::UnboundedReceiver<Bytes>;

mod sensors;
use sensors::event::Event;
use sensors::*;
mod command;
use command::Command;

mod motor;
mod motor_handler;
use motor_handler::MotorHandler;

type EventTx = mpsc::UnboundedSender<Event>;
type EventRx = mpsc::UnboundedReceiver<Event>;
type CommandTx = mpsc::UnboundedSender<Command>;
type CommandRx = mpsc::UnboundedReceiver<Command>;

struct Shared {
    clients: HashMap<SocketAddr, Tx>,
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

pub fn main() {
    let i2c_output = process::Command::new("i2cdetect").arg("-y 1").output();
    match i2c_output {
        Ok(output) => {
            std::io::stdout()
                .write(&output.stdout)
                .expect("Could not wrtie to stdout");
            ()
        }
        Err(_) => println!("Could not read from i2cdetect"),
    };

    let (server_tx, server_rx): (Tx, Rx) = mpsc::unbounded();
    let (sensors_tx, sensors_rx): (EventTx, EventRx) = mpsc::unbounded();
    let (_commands_tx, _commands_rx): (CommandTx, CommandRx) = mpsc::unbounded();
    let state = Arc::new(Mutex::new(Shared::new(server_tx)));

    let addr = "0.0.0.0:5000".parse().unwrap();

    let listener = TcpListener::bind(&addr).unwrap();

    let local_state = state.clone();
    let server = listener
        .incoming()
        .for_each(move |socket| {
            process(socket, local_state.clone());
            Ok(())
        }).map_err(|err| {
            println!("accept error = {:?}", err);
        });

    println!("server running on localhost:6142");

    let sensors_tx_arc = Arc::new(Mutex::new(sensors_tx));

    let (motor_handler, motor_handler_tx_command, motor_handler_tx_event) = MotorHandler::new();
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
        }).map_err(|err| {
            println!("line reading error = {:?}", err);
        });

    let local_state = state.clone();
    let receive_sensor_messages = sensors_rx
        .for_each(move |event| {
            let event_json = serde_json::to_string(&event).unwrap();
            println!("Received sensor message, broadcasting: {:?}", &event_json);

            match event {
                Event::Encoder { event: e } => {
                    motor_handler_tx_event.unbounded_send(e).unwrap();
                    ()
                }
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

            Ok(())
        }).map_err(|err| {
            println!("line reading error = {:?}", err);
        });

    let ir = ir::Ir::new(sensors_tx_arc.clone());
    let encoder = encoder::Encoder::new(sensors_tx_arc.clone());
    let gyro = gyro::Gyro::new(sensors_tx_arc.clone());
    let compass = compass::Compass::new(sensors_tx_arc.clone());
    let axl = axl::Axl::new(sensors_tx_arc.clone());
    encoder.run();

    let joined = server
        .join(receive_messages)
        .join(receive_sensor_messages)
        .join(ir.run())
        .join(gyro.run())
        .join(compass.run())
        .join(axl.run())
        .join(arduino.run())
        .join(motor_handler.run())
        .map(|_| ());

    tokio::run(joined);
}
