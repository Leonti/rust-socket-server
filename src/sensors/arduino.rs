use bytes::{BufMut, Bytes, BytesMut};
use futures::sync::mpsc;
use std::sync::{Arc, Mutex};

use std::{f32, io, str};
use tokio::io::AsyncRead;
use tokio::prelude::*;
use tokio_io::codec::{Decoder, Encoder};

use futures::{future, Future, Stream};

use event::{ArduinoEvent, Event};

type Tx = mpsc::UnboundedSender<Event>;

use command::ArduinoCommand;

type CommandTx = mpsc::UnboundedSender<ArduinoCommand>;
type CommandRx = mpsc::UnboundedReceiver<ArduinoCommand>;

pub struct Arduino {
    tx: Arc<Mutex<Tx>>,
    command_rx: CommandRx,
}

struct LineCodec;

impl Decoder for LineCodec {
    type Item = BytesMut;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let newline = src.as_ref().iter().position(|b| *b == b'\n');
        if let Some(n) = newline {
            let line = src.split_to(n + 1);
            return Ok(Some(line));
        }
        Ok(None)
    }
}

impl Encoder for LineCodec {
    type Item = BytesMut;
    type Error = io::Error;

    fn encode(&mut self, item: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error> {
        dst.put(item);

        println!("Converting line {:?}", &dst);

        Ok(())
    }
}

fn parse_float(n: &str) -> Result<f32, io::Error> {
    n.parse().map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("Could not parse number '{}', {}", n, e),
        )
    })
}

fn parse_temp(temp: &str) -> Result<ArduinoEvent, io::Error> {
    match temp.split(",").collect::<Vec<&str>>().as_slice() {
        [room, battery] => Ok(ArduinoEvent::Temp {
            room: parse_float(room)?,
            battery: parse_float(battery)?,
        }),
        _ => Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Could not decode temp event {}", temp),
        )),
    }
}

fn parse_battery(battery: &str) -> Result<ArduinoEvent, io::Error> {
    match battery.split(",").collect::<Vec<&str>>().as_slice() {
        [v, c] => Ok(ArduinoEvent::Power {
            load_voltage: parse_float(v)?,
            current_ma: parse_float(c)?,
        }),
        _ => Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Could not decode battery event {}", battery),
        )),
    }
}

fn decode_event(bytes: Bytes) -> Result<ArduinoEvent, io::Error> {
    let event = str::from_utf8(&bytes).unwrap().trim();
    match event.split(":").collect::<Vec<&str>>().as_slice() {
        ["B", battery] => parse_battery(battery),
        ["T", temp] => parse_temp(temp),
        _ => Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Could not decode arduino event {}", event),
        )),
    }
}

fn encode_command(_command: ArduinoCommand) -> Result<BytesMut, io::Error> {
    let mut to_send = BytesMut::new();
    to_send.extend_from_slice(b"S90\n");

    Ok(to_send)
}

impl Arduino {
    pub fn new(tx: Arc<Mutex<Tx>>) -> (Arduino, CommandTx) {
        let (command_tx, command_rx) = mpsc::unbounded();

        (Arduino { tx, command_rx }, command_tx)
    }

    #[allow(deprecated)]
    fn read_from_port(
        self,
        mut port: tokio_serial::Serial,
    ) -> Box<Future<Item = (), Error = ()> + Send> {
        port.set_exclusive(false)
            .expect("Unable to set serial port exlusive");

        let (mut writer, reader) = port.framed(LineCodec).split();

        let command_handler = self
            .command_rx
            .for_each(move |command| {
                let send_result = encode_command(command).and_then(|c| writer.start_send(c));

                match send_result {
                    Ok(_) => println!("Sent line to serial port"),
                    Err(e) => println!("serial send error = {:?}", e),
                };
                Ok(())
            }).map_err(|err| {
                println!("command reading error = {:?}", err);
            });

        let tx_arc = self.tx.clone();
        let messages = reader
            .for_each(move |s| {
                let s_tx = tx_arc.lock().unwrap();
                let send_result = decode_event(s.freeze())
                    .map(|event| Event::Arduino { event })
                    .and_then(|event| {
                        s_tx.unbounded_send(event)
                            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("{}", e)))
                    });

                match send_result {
                    Ok(_) => (),
                    Err(e) => println!("event send error = {:?}", e),
                }

                Ok(())
            }).map_err(|e| println!("{}", e));

        Box::new(command_handler.join(messages).map(|_| ()))
    }

    fn print_not_connected(self) -> Box<Future<Item = (), Error = ()> + Send> {
        println!("Can't open serial port!");
        Box::new(future::ok(()).map_err(|e: std::io::Error| eprintln!("{}", e)))
    }

    pub fn run(self) -> impl Future<Item = (), Error = ()> {
        let mut settings = tokio_serial::SerialPortSettings::default();
        settings.baud_rate = 115200;
        match tokio_serial::Serial::from_path("/dev/ttyUSB0", &settings) {
            Ok(port) => self.read_from_port(port),
            Err(_e) => self.print_not_connected(),
        }
    }
}
