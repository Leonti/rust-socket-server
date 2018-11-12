use futures::sync::mpsc;
//use std::sync::{Arc, Mutex};

use tokio::prelude::*;

use motor::Motor;
use command::MotorCommand;
use std::sync::{Arc, Mutex};

type Rx = mpsc::UnboundedReceiver<MotorCommand>;
type Tx = mpsc::UnboundedSender<MotorCommand>;

pub struct MotorHandler {
    rx: Rx,
    motor: Arc<Mutex<Option<Motor>>>
}

impl MotorHandler {

    pub fn new() -> (MotorHandler, Tx) {
        let (tx, rx) = mpsc::unbounded();

        match Motor::new() {
            Ok(motor) => (MotorHandler { rx, motor: Arc::new(Mutex::new(Some(motor))) }, tx),
            Err(e) => {
                println!("Error creating a motor {:?}", e);
                (MotorHandler { rx, motor: Arc::new(Mutex::new(None)) }, tx)
            }
        }

    }

    pub fn run(self) -> impl Future<Item = (), Error = ()> {
        let motor_arc = self.motor;
        self.rx.for_each(move |_| {
            println!("Received motor command");
            let mut motor_option = motor_arc.lock().unwrap();

            motor_option.as_mut().map(|motor| motor.set_speed());

            Ok(())
        }).map_err(|err| {
            println!("command reading error = {:?}", err);
        })
    }
}
