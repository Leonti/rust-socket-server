#![allow(unused)]
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum Wheel {
    Left,
    Right,
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub struct EncodersSnapshot {
    pub left: u8,
    pub right: u8,
    pub duration: isize,
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ArduinoEvent {
    Power { load_voltage: f32, current_ma: f32 },
    Temp { room: f32, battery: f32 },
    Encoders { encoders: EncodersSnapshot },
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
    pub duration: isize,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "lowercase")]
pub struct LidarScanPoint {
    pub angle: f32,
    pub distance: f32,
    pub quality: u8,
    pub is_sync: bool,
    pub is_valid: bool
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Event {
    Encoder {
        event: EncoderEvent,
    },
    Arduino {
        event: ArduinoEvent,
    },
    MotorRunStats {
        stats: Vec<MotorRunStat>,
        p: f32,
        i: f32,
        d: f32,
    },
    Lidar {
        scan_points: Vec<LidarScanPoint>,
    },
    Generic {
        message: String,
    },
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub struct TimedEvent {
    pub event: Event,
    pub time: u128,
}

impl TimedEvent {
    #[repr(u128)]
    pub fn new(event: Event) -> TimedEvent {
        let start = SystemTime::now();
        let since_the_epoch = start
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        let time =
            since_the_epoch.as_secs() as u128 * 1000 + since_the_epoch.subsec_millis() as u128;

        TimedEvent { event, time }
    }
}
