use futures::sync::mpsc;
use bytes::{BytesMut, BufMut};
use std::sync::{Arc, Mutex};

use std::{io};
use tokio::io::AsyncRead;
use tokio_io::codec::{Decoder, Encoder};
use tokio::prelude::*;

use futures::{Future, Stream, future};

use event::Event;

type Tx = mpsc::UnboundedSender<Event>;

use command::ArduinoCommand;

type CommandTx = mpsc::UnboundedSender<ArduinoCommand>;
type CommandRx = mpsc::UnboundedReceiver<ArduinoCommand>;

pub struct Arduino {
    tx: Arc<Mutex<Tx>>,
    command_rx: CommandRx
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

impl Arduino {

    pub fn new(tx: Arc<Mutex<Tx>>) -> (Arduino, CommandTx) {
        let (command_tx, command_rx) = mpsc::unbounded();

        (Arduino { tx, command_rx }, command_tx)
    }

    #[allow(deprecated)]
    fn read_from_port(self, mut port: tokio_serial::Serial) -> Box<Future<Item = (), Error = ()> + Send> {
        port.set_exclusive(false).expect("Unable to set serial port exlusive");

        let (mut writer, reader) = port.framed(LineCodec).split();


        let command_handler = self.command_rx.for_each(move |_| {
            println!("Received serial command");
            let mut to_send = BytesMut::new();
            to_send.extend_from_slice(b"S90\n");

            match writer.start_send(to_send) {
                Ok(_) => println!("Sent line to serial port"),
                Err(e) => println!("serial send error = {:?}", e)
            };
            Ok(())
        }).map_err(|err| {
            println!("command reading error = {:?}", err);
        });

        let tx_arc = self.tx.clone();
        let messages = reader
        .for_each(move |s| {

            let mut line = BytesMut::new();
            line.extend_from_slice(b"Serial sensor message\r\n");
            line.extend_from_slice(&s);
            let _line = line.freeze();

            let event = Event::Generic { message: "Arduino sensor message".to_string() };

            let s_tx = tx_arc.lock().unwrap();
            match s_tx.unbounded_send(event) {
                Ok(_) => (),
                Err(e) => println!("serial send error = {:?}", e)
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
            Err(_e) => self.print_not_connected()
        }
    }
}
