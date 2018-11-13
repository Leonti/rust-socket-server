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
    // B:loadvoltage,current_ma
    Power {
        load_voltage: f32,
        current_ma: f32
    },
    // T:room,battery
    Temp {
        room: f32,
        battery: f32
    }
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Event {
    Encoder {
        wheel: Wheel
    },
    Arduino {
        event: ArduinoEvent
    },
    Generic {
        message: String
    }
}
