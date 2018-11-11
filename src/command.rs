#![allow(unused)]

#[derive(Deserialize)]
pub enum Command {
    #[serde(rename = "motor")]
    Motor {
        message: String
    }
}
