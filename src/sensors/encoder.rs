use futures::sync::mpsc;
use std::sync::{Arc, Mutex};

use std::thread;
use sysfs_gpio::{Direction, Edge, Pin};

use crate::event::{EncoderEvent, Event, TimedEvent, Wheel};

type Tx = mpsc::UnboundedSender<TimedEvent>;

pub struct Encoder {
    tx: Arc<Mutex<Tx>>,
}

fn port_listen(pin_number: u64, wheel: Wheel, tx: Arc<Mutex<Tx>>) -> sysfs_gpio::Result<()> {
    let input = Pin::new(pin_number);

    input.with_exported(|| {
        input.set_direction(Direction::In)?;
        input.set_edge(Edge::RisingEdge)?;
        let mut poller = input.get_poller()?;
        loop {
            match poller.poll(1000)? {
                Some(_val) => {
                    let encoder_event = EncoderEvent {
                        wheel: wheel.clone(),
                    };
                    let event = Event::Encoder {
                        event: encoder_event,
                    };

                    let s_tx = tx.lock().unwrap();
                    match s_tx.unbounded_send(TimedEvent::new(event)) {
                        Ok(_) => (),
                        Err(e) => println!("encoder send error = {:?}", e),
                    }
                }
                None => (),
            }
        }
    })
}

impl Encoder {
    pub fn new(tx: Arc<Mutex<Tx>>) -> Encoder {
        Encoder { tx }
    }

    pub fn run(self) -> () {
        let left_tx = self.tx.clone();
        thread::spawn(move || match port_listen(23, Wheel::Left, left_tx) {
            Ok(_) => (),
            Err(e) => println!("Interrupt failed on pin {} {}", 23, e),
        });

        let right_tx = self.tx.clone();
        thread::spawn(move || match port_listen(22, Wheel::Right, right_tx) {
            Ok(_) => (),
            Err(e) => println!("Interrupt failed on pin {} {}", 22, e),
        });

        ()
    }
}
