use futures::sync::mpsc;

use tokio::prelude::*;

use std::time::{Duration, Instant};
use tokio::timer::Interval;

use command::MotorCommand;
use event::{EncoderEvent, Wheel};
use motor::Motor;
use std::sync::{Arc, Mutex};

type RxCommand = mpsc::UnboundedReceiver<MotorCommand>;
type TxCommand = mpsc::UnboundedSender<MotorCommand>;

type RxEvent = mpsc::UnboundedReceiver<EncoderEvent>;
type TxEvent = mpsc::UnboundedSender<EncoderEvent>;

struct MotorState {
    ticks_to_move: u32,
    speed: u8,
    left_ticks: u32,
    right_ticks: u32,
}

pub struct MotorHandler {
    rx_command: RxCommand,
    rx_event: RxEvent,
    state: Arc<Mutex<MotorState>>,
    motor: Arc<Mutex<Option<Motor>>>,
}

impl MotorState {
    pub fn new() -> MotorState {
        MotorState {
            ticks_to_move: 0,
            speed: 0,
            left_ticks: 0,
            right_ticks: 0,
        }
    }

    pub fn new_command(&mut self, ticks_to_move: u32, speed: u8) {
        self.ticks_to_move = ticks_to_move;
        self.speed = speed;
    }
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
                    state: Arc::new(Mutex::new(MotorState::new())),
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
                        state: Arc::new(Mutex::new(MotorState::new())),
                        motor: Arc::new(Mutex::new(None)),
                    },
                    tx_command,
                    tx_event,
                )
            }
        }
    }

    pub fn run(self) -> impl Future<Item = (), Error = ()> {
        let motor_command_arc = self.motor.clone();
        let state_command_arc = self.state.clone();

        let command_handler = self
            .rx_command
            .for_each(move |command| {
                println!("Received motor command");
                let mut motor_option = motor_command_arc.lock().unwrap();
                let mut state = state_command_arc.lock().unwrap();
                match command {
                    MotorCommand::Move { speed, ticks } => {
                        motor_option.as_mut().map(|motor| motor.set_speed(speed));
                        state.new_command(ticks, speed);
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

        let state_encoder_arc = self.state.clone();
        let encoder_handler = self
            .rx_event
            .for_each(move |encoder_event| {
                let mut state = state_encoder_arc.lock().unwrap();
                match encoder_event.wheel {
                    Wheel::Left => state.left_ticks += 1,
                    Wheel::Right => state.right_ticks += 1,
                };

                Ok(())
            }).map_err(|err| {
                println!("envoder event error = {:?}", err);
            });

        let motor_pid_arc = self.motor.clone();
        let state_pid_arc = self.state.clone();
        let pid_loop = Interval::new(Instant::now(), Duration::from_millis(50))
            .for_each(move |_| {
                let state = state_pid_arc.lock().unwrap();

                if state.left_ticks >= state.ticks_to_move
                    || state.right_ticks >= state.ticks_to_move
                {
                    let mut motor_option = motor_pid_arc.lock().unwrap();
                    motor_option.as_mut().map(|motor| motor.set_speed(0));
                }

                Ok(())
            }).map_err(|e| print!("interval errored; err={:?}", e));

        command_handler
            .join(encoder_handler)
            .join(pid_loop)
            .map(|_| ())
    }
}
