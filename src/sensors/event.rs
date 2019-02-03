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

#[derive(Serialize, Clone)]
#[serde(rename_all = "lowercase")]
pub struct MotorRunStat {
    pub speed_base: f32,
    pub speed_slave: f32,
    pub ticks_base: isize,
    pub ticks_slave: isize,
    pub error: f32,
    pub p_term: f32,
    pub i_term: f32,
    pub d_term: f32,
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Event {
    Encoder { event: EncoderEvent },
    Arduino { event: ArduinoEvent },
    MotorRunStats {
        stats: Vec<MotorRunStat>,
        p: f32,
        i: f32,
        d: f32,
    },
    Generic { message: String },
}
