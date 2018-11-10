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
        self.port_listen(12, Wheel::Left, self.tx.clone())
        .join(self.port_listen(13, Wheel::Right, self.tx.clone()))
        .map(|_| ())
    }

    fn port_listen(&self, pin_number: u64, wheel: Wheel, tx: Arc<Mutex<Tx>>) -> Box<Future<Item = (), Error = ()> + Send> {
        let pin = Pin::new(pin_number);
        pin.export().unwrap();
        pin.set_direction(Direction::In).unwrap();
        pin.set_edge(Edge::BothEdges).unwrap();

        let l = Core::new().unwrap();
        let handle = l.handle();

        let stream = pin.get_value_stream(&handle).unwrap().for_each(move |val| {
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

        Box::new(stream)
    }
}
