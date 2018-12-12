use futures::sync::mpsc;
use std::sync::{Arc, Mutex};

use crate::event::Event;
use std::time::{Duration, Instant};
use tokio::prelude::*;
use tokio::timer::Interval;

type Tx = mpsc::UnboundedSender<Event>;

pub struct Compass {
    tx: Arc<Mutex<Tx>>,
}

impl Compass {
    pub fn new(tx: Arc<Mutex<Tx>>) -> Compass {
        Compass { tx }
    }

    pub fn run(self) -> impl Future<Item = (), Error = ()> {
        Interval::new(Instant::now(), Duration::from_millis(1000))
            .for_each(move |_| {
                let event = Event::Generic {
                    message: "Compass sensor message".to_string(),
                };

                let s_tx = &self.tx.lock().unwrap();
                match s_tx.unbounded_send(event) {
                    Ok(_) => (),
                    Err(e) => println!("compass send error = {:?}", e),
                }

                Ok(())
            })
            .map_err(|e| print!("interval errored; err={:?}", e))
    }
}
