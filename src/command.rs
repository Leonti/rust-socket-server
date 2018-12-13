#![allow(unused)]

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ArduinoCommand {
    Off,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Forward,
    Backward,
    Right,
    Left,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MotorCommand {
    Move {
        speed: u8,
        direction: Direction,
        ticks: u32,
        p: f32,
        i: f32,
        d: f32,
    },
    Stop,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Command {
    Motor { command: MotorCommand },
    Arduino { command: ArduinoCommand },
}
