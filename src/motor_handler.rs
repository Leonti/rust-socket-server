use futures::sync::mpsc;
//use std::sync::{Arc, Mutex};

use tokio::prelude::*;

use motor::Motor;
use command::Command;
use std::sync::{Arc, Mutex};

type Rx = mpsc::UnboundedReceiver<Command>;
type Tx = mpsc::UnboundedSender<Command>;

pub struct MotorHandler {
    rx: Rx,
    motor: Arc<Mutex<Motor>>
}

impl MotorHandler {

    pub fn new() -> (MotorHandler, Tx) {
        let (tx, rx) = mpsc::unbounded();

        let motor = Motor::new();
        (MotorHandler { rx, motor: Arc::new(Mutex::new(motor)) }, tx)
    }

    pub fn run(self) -> impl Future<Item = (), Error = ()> {
        let motor_arc = self.motor;
        self.rx.for_each(move |_| {
            println!("Received command");
            let mut motor = motor_arc.lock().unwrap();
            motor.set_speed();
            Ok(())
        }).map_err(|err| {
            println!("command reading error = {:?}", err);
        })
    }
}
