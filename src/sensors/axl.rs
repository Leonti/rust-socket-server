use futures::sync::mpsc;
use bytes::{Bytes, BytesMut};
use std::sync::{Arc, Mutex};

use tokio::timer::Interval;
use std::time::{Duration, Instant};
use tokio::prelude::*;

type Tx = mpsc::UnboundedSender<Bytes>;

pub struct Axl {
    tx: Arc<Mutex<Tx>>
}

impl Axl {

    pub fn new(tx: Arc<Mutex<Tx>>) -> Axl {
        Axl { tx }
    }

    pub fn run(self) -> impl Future<Item = (), Error = ()> {
        Interval::new(Instant::now(), Duration::from_millis(1000))
            .for_each(move |_| {

                let mut line = BytesMut::new();
                line.extend_from_slice(b"Axl sensor message\r\n");
                let line = line.freeze();

                let s_tx = &self.tx.lock().unwrap();
                match s_tx.unbounded_send(line.clone()) {
                    Ok(_) => (),
                    Err(e) => println!("axl send error = {:?}", e)
                }

                Ok(())
            })
            .map_err(|e| print!("interval errored; err={:?}", e))
    }
}
