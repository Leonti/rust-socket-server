use futures::sync::mpsc;
use std::sync::{Arc, Mutex};

use futures::{Future, Stream, future};
use sysfs_gpio::{Direction, Edge, Pin};
use tokio_core::reactor::Core;

use event::{Event, Wheel};

type Tx = mpsc::UnboundedSender<Event>;

pub struct Encoder {
    tx: Arc<Mutex<Tx>>
}

fn print_error(e: sysfs_gpio::Error) -> Box<Future<Item = (), Error = ()> + Send> {
    println!("Failed to start encoders: {:?}", e);
    Box::new(future::ok(()).map_err(|_:std::io::Error| ()))
}

impl Encoder {

    pub fn new(tx: Arc<Mutex<Tx>>) -> Encoder {
        Encoder { tx }
    }

    pub fn run(self) -> impl Future<Item = (), Error = ()> {

        match (self.port_listen(12, Wheel::Left, self.tx.clone()), self.port_listen(13, Wheel::Right, self.tx.clone())) {
            (Ok(left_encoder), Ok(right_encoder)) => {
                Box::new(left_encoder
                .join(right_encoder)
                .map(|_| ()))
            },
            (Err(e), _) => print_error(e),
            (_, Err(e)) => print_error(e)
        }
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
