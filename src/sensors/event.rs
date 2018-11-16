#![allow(unused)]

#[derive(Serialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum Wheel {
    Left,
    Right,
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ArduinoEvent {
    Power { load_voltage: f32, current_ma: f32 },
    Temp { room: f32, battery: f32 },
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub struct EncoderEvent {
    pub wheel: Wheel,
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Event {
    Encoder { event: EncoderEvent },
    Arduino { event: ArduinoEvent },
    Generic { message: String },
}
