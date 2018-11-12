#![allow(unused)]

#[derive(Deserialize)]
pub enum ArduinoCommand {

    #[serde(rename = "off")]
    Off
}

#[derive(Deserialize)]
pub enum MotorCommand {

    #[serde(rename = "move")]
    Move,
    #[serde(rename = "stop")]
    Stop,
}

#[derive(Deserialize)]
pub enum Command {
    #[serde(rename = "motor")]
    Motor {
        command: MotorCommand
    },
    #[serde(rename = "arduino")]
    Arduino {
        command: ArduinoCommand
    }
}
