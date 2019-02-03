use futures::sync::mpsc;

use tokio::prelude::*;

use std::time::{Duration, Instant};
use tokio::timer::Interval;

use crate::command::{Direction, MotorCommand};
use crate::event::{EncoderEvent, Wheel, Event, MotorRunStat};
use crate::motor::{Dir, Motor, Side};
use std::sync::{Arc, Mutex};

type Tx = mpsc::UnboundedSender<Event>;

type RxCommand = mpsc::UnboundedReceiver<MotorCommand>;
type TxCommand = mpsc::UnboundedSender<MotorCommand>;

type RxEvent = mpsc::UnboundedReceiver<EncoderEvent>;
type TxEvent = mpsc::UnboundedSender<EncoderEvent>;

// http://www.robotc.net/wikiarchive/Tutorials/Arduino_Projects/Mobile_Robotics/VEX/Using_encoders_to_drive_straight
// http://brettbeauregard.com/blog/2011/04/improving-the-beginner%e2%80%99s-pid-sample-time/

const SAMPLE_TIME_MS: u64 = 100;

struct WheelState {
    i_term: f32,
    last_ticks: Option<isize>,
    current_ticks: isize,
    speed: f32,
}

impl WheelState {
    pub fn new(speed: f32) -> WheelState {
        WheelState {
            i_term: 0.0,
            last_ticks: None,
            current_ticks: 0,
            speed: speed,            
        }
    }
    pub fn tick(&mut self) {
        self.current_ticks += 1;
    }
}

struct BaseWheelState {
    current_ticks: isize,
    speed: f32,    
}

impl BaseWheelState {
    pub fn new(speed: f32) -> BaseWheelState {
        BaseWheelState {
            current_ticks: 0,
            speed: speed,            
        }
    }
    pub fn tick(&mut self) {
        self.current_ticks += 1;
    }
}

struct Pid {
    p: f32,
    i: f32,
    d: f32,    
}

struct MotorState {
    direction: Direction,
    is_moving: bool,
    ticks_to_move: isize,
    ticks_moved: isize,
    pid: Pid,
    wheel_left: WheelState,
    wheel_right: BaseWheelState,
    speed: u8,
    motor_stats: Vec<MotorRunStat>
}

pub struct MotorHandler {
    rx_command: RxCommand,
    rx_event: RxEvent,
    tx: Arc<Mutex<Tx>>,
    state: Arc<Mutex<MotorState>>,
    motor: Arc<Mutex<Option<Motor>>>,
}

// http://brettbeauregard.com/blog/2011/04/improving-the-beginner%e2%80%99s-pid-reset-windup/
fn next_wheel_state(ws: &WheelState, base_wheel: &BaseWheelState, pid: &Pid) -> (WheelState, MotorRunStat) {
    let error = (base_wheel.current_ticks - ws.current_ticks) as f32;
    let mut i_term = ws.i_term + pid.i * error;

    let out_min = -(base_wheel.speed as f32);
    let out_max = 100.0 - base_wheel.speed as f32;  

    if i_term > out_max {
        i_term = out_max;
    } else if i_term < out_min {
        i_term = out_min;
    }

    let input_delta = ws
        .last_ticks
        .map_or(0, |last| ws.current_ticks - last);
    
    let mut output = pid.p * error + ws.i_term - pid.d * input_delta as f32;

    if output > out_max {
        output = out_max;
    } else if output < out_min {
        output = out_min;
    }

    println!("speed {}, p_term: {}, i_term: {}, d_term: {}, error: {}, current_ticks: {}, base ticks: {}",
        output, pid.p * error, i_term, pid.d * input_delta as f32, error, ws.current_ticks, base_wheel.current_ticks);

    let wheel_state = WheelState {
        i_term: i_term,
        last_ticks: Some(ws.current_ticks),
        current_ticks: 0,
        speed: base_wheel.speed + output,
    };

    let stat = MotorRunStat {
        speed_base: base_wheel.speed,
        speed_slave: wheel_state.speed,
        ticks_base: base_wheel.current_ticks,
        ticks_slave: ws.current_ticks,
        error: error,
        p_term: pid.p * error,
        i_term: pid.d * input_delta as f32,
        d_term: i_term,
    };

    (wheel_state, stat)
}

impl MotorState {
    pub fn new() -> MotorState {
        MotorState {
            direction: Direction::Forward,
            is_moving: false,
            ticks_to_move: 0,
            ticks_moved: 0,
            pid: Pid {
                p: 0.0,
                i: 0.0,
                d: 0.0,
            },
            speed: 0,
            wheel_left: WheelState::new(0.0),
            wheel_right: BaseWheelState::new(0.0),
            motor_stats: Vec::new()
        }
    }

/* For dynamic sample time
    self.p = p;
    self.i = i * SAMPLE_TIME_MS as f32;
    self.d = d / SAMPLE_TIME_MS as f32;
*/

    pub fn new_command(
        &mut self,
        direction: Direction,
        speed: u8,
        ticks_to_move: u32,
        p: f32,
        i: f32,
        d: f32,
    ) {
        self.is_moving = true;
        self.ticks_to_move = ticks_to_move as isize;
        self.direction = direction;
        self.pid = Pid {
            p: p,
            i: i,
            d: d,
        };

        self.wheel_left = WheelState::new(speed as f32);
        self.wheel_right = BaseWheelState::new(speed as f32);
        self.speed = speed;
        self.ticks_moved = 0;
    }
}

impl MotorHandler {
    pub fn new(tx: Arc<Mutex<Tx>>) -> (MotorHandler, TxCommand, TxEvent) {
        let (tx_command, rx_command) = mpsc::unbounded();
        let (tx_event, rx_event) = mpsc::unbounded();

        match Motor::new() {
            Ok(motor) => (
                MotorHandler {
                    rx_command,
                    rx_event,
                    tx,
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
                        tx,
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
                        direction,
                        ticks,
                        p,
                        i,
                        d,
                    } => {
                        println!("Received motor Move command ");

                        motor_option.as_mut().map(|motor| {
                            match direction {
                                Direction::Forward => {
                                    motor.set_direction(Side::Left, Dir::Forward);
                                    motor.set_direction(Side::Right, Dir::Forward);
                                }
                                Direction::Backward => {
                                    motor.set_direction(Side::Left, Dir::Backward);
                                    motor.set_direction(Side::Right, Dir::Backward);
                                }
                                Direction::Right => {
                                    motor.set_direction(Side::Left, Dir::Forward);
                                    motor.set_direction(Side::Right, Dir::Backward);
                                }
                                Direction::Left => {
                                    motor.set_direction(Side::Left, Dir::Backward);
                                    motor.set_direction(Side::Right, Dir::Forward);
                                }
                            };
                            motor.set_speed(Side::Left, speed as f32);
                            motor.set_speed(Side::Right, speed as f32);
                        });

                        state.new_command(direction, speed, ticks, p, i, d);
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
            })
            .map_err(|err| {
                println!("command reading error = {:?}", err);
            });

        let state_encoder_arc = self.state.clone();
        let encoder_handler = self
            .rx_event
            .for_each(move |encoder_event| {
                let mut state = state_encoder_arc.lock().unwrap();
                match encoder_event.wheel {
                    Wheel::Left => state.wheel_left.tick(),
                    Wheel::Right => {
                        state.wheel_right.tick();
                        state.ticks_moved += 1;
                    },
                };

                Ok(())
            })
            .map_err(|err| {
                println!("encoder event error = {:?}", err);
            });

        let motor_pid_arc = self.motor.clone();
        let state_pid_arc = self.state.clone();
        let tx = self.tx.clone();
        let pid_loop = Interval::new(Instant::now(), Duration::from_millis(SAMPLE_TIME_MS))
            .for_each(move |_| {
                let mut state = state_pid_arc.lock().unwrap();

                if !state.is_moving {
                    return Ok(());
                }

                if state.ticks_moved >= state.ticks_to_move {
                    let mut motor_option = motor_pid_arc.lock().unwrap();
                    motor_option.as_mut().map(|motor| motor.stop());

                    println!("Finished moving");
                    state.is_moving = false;
                    let tx = tx.lock().unwrap();
                    match tx.unbounded_send(Event::MotorRunStats {
                        stats: state.motor_stats.clone(),
                        p: state.pid.p,
                        i: state.pid.i,
                        d: state.pid.d,
                    }) {
                        Ok(_) => (),
                        Err(e) => println!("motor stats send error = {:?}", e),
                    };
                    state.motor_stats = Vec::new();
                    return Ok(());
                }

                println!("left ticks: {}, right ticks: {}", state.wheel_left.current_ticks, state.wheel_right.current_ticks);

                println!("Left state:");
                let (next_state, stat) = next_wheel_state(&state.wheel_left, &state.wheel_right, &state.pid);
                state.wheel_left = next_state;
                state.wheel_right.current_ticks = 0;
                state.motor_stats.push(stat);

                let mut motor_option = motor_pid_arc.lock().unwrap();
                motor_option
                    .as_mut()
                    .map(|motor| {
                        motor.set_speed(Side::Left, state.wheel_left.speed);
                        motor.set_speed(Side::Right, state.wheel_right.speed);
                    });

                Ok(())
            })
            .map_err(|e| print!("interval errored; err={:?}", e));

        command_handler
            .join(encoder_handler)
            .join(pid_loop)
            .map(|_| ())
    }
}
