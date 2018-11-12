use futures::sync::mpsc;
use std::sync::{Arc, Mutex};

use futures::{Future, Stream};
use sysfs_gpio::{Direction, Edge, Pin};
use tokio_core::reactor::Core;

use event::{Event, Wheel};

type Tx = mpsc::UnboundedSender<Event>;

pub struct Encoder {
    tx: Arc<Mutex<Tx>>
}

impl Encoder {

    pub fn new(tx: Arc<Mutex<Tx>>) -> Encoder {
        Encoder { tx }
    }

    pub fn run(self) -> impl Future<Item = (), Error = ()> {
        let left_encoder = self.port_listen(12, Wheel::Left, self.tx.clone()).unwrap();
        let right_encoder = self.port_listen(13, Wheel::Right, self.tx.clone()).unwrap();

        left_encoder
        .join(right_encoder)
        .map(|_| ())
    }

    fn port_listen(&self, pin_number: u64, wheel: Wheel, tx: Arc<Mutex<Tx>>) -> Result<Box<Future<Item = (), Error = ()> + Send>, sysfs_gpio::Error> {
        let pin = Pin::new(pin_number);
        pin.export()?;
        pin.set_direction(Direction::In)?;
        pin.set_edge(Edge::RisingEdge)?;

        let l = Core::new().unwrap();
        let handle = l.handle();

        let stream = pin.get_value_stream(&handle)?.for_each(move |val| {
                                       println!("Pin {} changed value to {}", pin_number, val);
                                       let event = Event::Encoder { wheel: wheel.clone() };

                                       let s_tx = tx.lock().unwrap();
                                       match s_tx.unbounded_send(event) {
                                           Ok(_) => (),
                                           Err(e) => println!("encoder send error = {:?}", e)
                                       }

                                       Ok(())
                                   })
                                   .map_err(|e| print!("interrupt errored; err={:?}", e));

        Ok(Box::new(stream))
    }
}
