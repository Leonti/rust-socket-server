use futures::sync::mpsc;

use tokio::prelude::*;

use std::time::{Duration, Instant};
use tokio::timer::Interval;

use crate::command::MotorCommand;
use crate::event::{EncoderEvent, Wheel};
use crate::motor::{Dir, Motor, Side};
use std::sync::{Arc, Mutex};

type RxCommand = mpsc::UnboundedReceiver<MotorCommand>;
type TxCommand = mpsc::UnboundedSender<MotorCommand>;

type RxEvent = mpsc::UnboundedReceiver<EncoderEvent>;
type TxEvent = mpsc::UnboundedSender<EncoderEvent>;

// http://www.robotc.net/wikiarchive/Tutorials/Arduino_Projects/Mobile_Robotics/VEX/Using_encoders_to_drive_straight
// http://brettbeauregard.com/blog/2011/04/improving-the-beginner%e2%80%99s-pid-sample-time/

const SAMPLE_TIME_MS: u64 = 100;

struct MotorState {
    is_moving: bool,
    ticks_to_move: isize,
    p: f32,
    i: f32,
    d: f32,
    i_term: f32,
    last_left_ticks: isize,    
    speed: u8,
    left_ticks: isize,
    right_ticks: isize,
    left_speed: f32,
    sum_ticks: isize,
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
            is_moving: false,
            ticks_to_move: 0,
            p: 0.0,
            i: 0.0,
            d: 0.0,
            i_term: 0.0,
            last_left_ticks: 0,
            speed: 0,
            left_speed: 0.0,
            left_ticks: 0,
            right_ticks: 0,
            sum_ticks: 0,
        }
    }

    // http://brettbeauregard.com/blog/2011/04/improving-the-beginner%e2%80%99s-pid-reset-windup/
    pub fn update_left_speed(&mut self) {
        let error = (self.right_ticks - self.left_ticks) as f32;
        self.i_term += self.i * error;

        let out_min = - (self.speed as f32);
        let out_max = 100.0 - self.speed as f32;

        if self.i_term > out_max {
            self.i_term = out_max;
        } else if self.i_term < out_min {
            self.i_term = out_min;
        }

        let input_delta = self.left_ticks - self.last_left_ticks;
        self.last_left_ticks = self.left_ticks;

        let mut output = self.p * error + self.i_term - self.d * input_delta as f32;

        if output > out_max {
            output = out_max;
        } else if output < out_min {
            output = out_min;
        } 

        println!("speed {}, p_term: {}, i_term: {}, d_term: {}, error: {}, left_ticks: {}, right_ticks: {}",
            output, self.p * error, self.i_term, self.d * input_delta as f32, error, self.left_ticks, self.right_ticks);

        self.left_speed = self.speed as f32 + output;
    }

    pub fn new_command(&mut self, ticks_to_move: u32, speed: u8, p: f32, i: f32, d: f32) {
        self.is_moving = true;
        self.ticks_to_move = ticks_to_move as isize;
        self.p = p;
        self.i = i * SAMPLE_TIME_MS as f32;
        self.d = d / SAMPLE_TIME_MS as f32;
        self.i_term = 0.0;
        self.last_left_ticks = 0;
        self.speed = speed;
        self.sum_ticks = 0;
    }

    pub fn reset_loop(&mut self) {
        self.left_ticks = 0;
        self.right_ticks = 0;
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
                let mut motor_option = motor_command_arc.lock().unwrap();
                let mut state = state_command_arc.lock().unwrap();
                match command {
                    MotorCommand::Move {
                        speed,
                        ticks,
                        p,
                        i,
                        d,
                    } => {
                        println!("Received motor Move command ");

                        motor_option.as_mut().map(|motor| {
                            motor.set_direction(Side::Left, Dir::Forward);
                            motor.set_direction(Side::Right, Dir::Forward);
                            motor.set_speed(Side::Left, speed as f32);
                            motor.set_speed(Side::Right, speed as f32);
                        });

                        state.new_command(ticks, speed, p, i, d);
                        ()
                    }
                    MotorCommand::Stop => {
                        println!("Received motor stop command ");
                        state.is_moving = false;
                        motor_option.as_mut().map(|motor| motor.stop());
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
                    Wheel::Right => {
                        state.right_ticks += 1;
                        state.sum_ticks += 1
                        },
                };

                Ok(())
            }).map_err(|err| {
                println!("envoder event error = {:?}", err);
            });

        let motor_pid_arc = self.motor.clone();
        let state_pid_arc = self.state.clone();
        let pid_loop = Interval::new(Instant::now(), Duration::from_millis(SAMPLE_TIME_MS))
            .for_each(move |_| {
                let mut state = state_pid_arc.lock().unwrap();

                if !state.is_moving {
                    return Ok(())
                }

                if state.sum_ticks >= state.ticks_to_move {
                    let mut motor_option = motor_pid_arc.lock().unwrap();
                    motor_option.as_mut().map(|motor| motor.stop());

                    println!("Finished moving");
                    state.is_moving = false;
                    return Ok(())
                }

                state.update_left_speed();

                let mut motor_option = motor_pid_arc.lock().unwrap();
                motor_option.as_mut().map(|motor| motor.set_speed(Side::Left, state.left_speed));

                state.reset_loop();
                Ok(())
            }).map_err(|e| print!("interval errored; err={:?}", e));

        command_handler
            .join(encoder_handler)
            .join(pid_loop)
            .map(|_| ())
    }
}
