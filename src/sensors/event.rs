#![allow(unused)]

#[derive(Serialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum Wheel {
    Left,
    Right,
}

#[derive(Serialize)]
pub enum Event {
    #[serde(rename = "encoder")]
    Encoder {
        wheel: Wheel
    },
    Generic {
        message: String
    }
}
