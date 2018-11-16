use futures::sync::mpsc;
use std::sync::{Arc, Mutex};

use event::Event;
use std::time::{Duration, Instant};
use tokio::prelude::*;
use tokio::timer::Interval;

type Tx = mpsc::UnboundedSender<Event>;

pub struct Gyro {
    tx: Arc<Mutex<Tx>>,
}

impl Gyro {
    pub fn new(tx: Arc<Mutex<Tx>>) -> Gyro {
        Gyro { tx }
    }

    pub fn run(self) -> impl Future<Item = (), Error = ()> {
        Interval::new(Instant::now(), Duration::from_millis(1000))
            .for_each(move |_| {
                let event = Event::Generic {
                    message: "Gyro sensor message".to_string(),
                };

                let s_tx = &self.tx.lock().unwrap();
                match s_tx.unbounded_send(event) {
                    Ok(_) => (),
                    Err(e) => println!("gyro send error = {:?}", e),
                }

                Ok(())
            })
            .map_err(|e| print!("interval errored; err={:?}", e))
    }
}
