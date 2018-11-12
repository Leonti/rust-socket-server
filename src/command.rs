#![allow(unused)]

#[derive(Deserialize)]
pub enum ArduinoCommand {

    #[serde(rename = "off")]
    Off
}

#[derive(Deserialize)]
pub enum Command {
    #[serde(rename = "motor")]
    Motor {
        message: String
    },
    #[serde(rename = "arduino")]
    Arduino {
        command: ArduinoCommand
    }
}
