use futures::sync::mpsc;
//use std::sync::{Arc, Mutex};

use tokio::prelude::*;

use command::Command;

type Rx = mpsc::UnboundedReceiver<Command>;
type Tx = mpsc::UnboundedSender<Command>;

pub struct MotorHandler {
    rx: Rx
}

impl MotorHandler {

    pub fn new() -> (MotorHandler, Tx) {
        let (tx, rx) = mpsc::unbounded();
        (MotorHandler { rx }, tx)
    }

    pub fn run(self) -> impl Future<Item = (), Error = ()> {
        self.rx.for_each(|_| {
            println!("Received command");
            Ok(())
        }).map_err(|err| {
            println!("command reading error = {:?}", err);
        })
    }
}
