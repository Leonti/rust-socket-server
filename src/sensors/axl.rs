use futures::sync::mpsc;
use std::sync::{Arc, Mutex};

use event::Event;
use std::time::{Duration, Instant};
use tokio::prelude::*;
use tokio::timer::Interval;

type Tx = mpsc::UnboundedSender<Event>;

pub struct Axl {
    tx: Arc<Mutex<Tx>>,
}

impl Axl {
    pub fn new(tx: Arc<Mutex<Tx>>) -> Axl {
        Axl { tx }
    }

    pub fn run(self) -> impl Future<Item = (), Error = ()> {
        Interval::new(Instant::now(), Duration::from_millis(1000))
            .for_each(move |_| {
                let event = Event::Generic {
                    message: "Axl sensor message".to_string(),
                };

                let s_tx = &self.tx.lock().unwrap();
                match s_tx.unbounded_send(event) {
                    Ok(_) => (),
                    Err(e) => println!("axl send error = {:?}", e),
                }

                Ok(())
            }).map_err(|e| print!("interval errored; err={:?}", e))
    }
}
