use futures::sync::mpsc;
use bytes::{Bytes, BytesMut};
use std::sync::{Arc, Mutex};

use std::{io};
use tokio::io::AsyncRead;
use tokio_io::codec::{Decoder, Encoder};

use futures::{Future, Stream};

type Tx = mpsc::UnboundedSender<Bytes>;

pub struct Arduino {
    tx: Arc<Mutex<Tx>>
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

    fn encode(&mut self, _item: Self::Item, _dst: &mut BytesMut) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl Arduino {

    pub fn new(tx: Arc<Mutex<Tx>>) -> Arduino {
        Arduino { tx }
    }

    #[allow(deprecated)]
    pub fn run(self) -> impl Future<Item = (), Error = ()> {

        let mut settings = tokio_serial::SerialPortSettings::default();
        settings.baud_rate = 115200;
        let mut port = tokio_serial::Serial::from_path("/dev/ttyUSB0", &settings).unwrap();
        port.set_exclusive(false).expect("Unable to set serial port exlusive");

        let (_, reader) = port.framed(LineCodec).split();

        reader
        .for_each(move |s| {
        //    println!("{:?}", s);

            let mut line = BytesMut::new();
            line.extend_from_slice(b"Serial sensor message\r\n");
            line.extend_from_slice(&s);
            let line = line.freeze();

            let s_tx = &self.tx.lock().unwrap();
            match s_tx.unbounded_send(line.clone()) {
                Ok(_) => (),
                Err(e) => println!("axl send error = {:?}", e)
            }

            Ok(())
        }).map_err(|e| eprintln!("{}", e))
    }
}
