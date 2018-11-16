use futures::sync::mpsc;

use tokio::prelude::*;

use command::MotorCommand;
use event::{EncoderEvent, Wheel};
use motor::Motor;
use std::sync::{Arc, Mutex};

type RxCommand = mpsc::UnboundedReceiver<MotorCommand>;
type TxCommand = mpsc::UnboundedSender<MotorCommand>;

type RxEvent = mpsc::UnboundedReceiver<EncoderEvent>;
type TxEvent = mpsc::UnboundedSender<EncoderEvent>;

pub struct MotorHandler {
    rx_command: RxCommand,
    rx_event: RxEvent,
    motor: Arc<Mutex<Option<Motor>>>,
}

impl MotorHandler {
    pub fn new() -> (MotorHandler, TxCommand, TxEvent) {
        let (tx_command, rx_command) = mpsc::unbounded();
        let (tx_event, rx_event) = mpsc::unbounded();

        match Motor::new() {
            Ok(motor) => (
                MotorHandler {
                    rx_command,
                    rx_event,
                    motor: Arc::new(Mutex::new(Some(motor))),
                },
                tx_command,
                tx_event,
            ),
            Err(e) => {
                println!("Error creating a motor {:?}", e);
                (
                    MotorHandler {
                        rx_command,
                        rx_event,
                        motor: Arc::new(Mutex::new(None)),
                    },
                    tx_command,
                    tx_event,
                )
            }
        }
    }

    pub fn run(self) -> impl Future<Item = (), Error = ()> {
        let motor_arc = self.motor;

        let command_handler = self
            .rx_command
            .for_each(move |command| {
                println!("Received motor command");
                let mut motor_option = motor_arc.lock().unwrap();
                match command {
                    MotorCommand::Move {
                        speed,
                        ticks: _ticks,
                    } => {
                        motor_option.as_mut().map(|motor| motor.set_speed(speed));
                        ()
                    }
                    MotorCommand::Stop => {
                        motor_option.as_mut().map(|motor| motor.set_speed(0));
                        ()
                    }
                };

                Ok(())
            }).map_err(|err| {
                println!("command reading error = {:?}", err);
            });

        let encoder_handler = self
            .rx_event
            .for_each(move |encoder_event| {
                match encoder_event.wheel {
                    Wheel::Left => (),
                    Wheel::Right => (),
                };

                Ok(())
            }).map_err(|err| {
                println!("envoder event error = {:?}", err);
            });

        command_handler.join(encoder_handler).map(|_| ())
    }
}
