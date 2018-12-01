use i2c_pca9685::PCA9685;
use i2cdev::linux::*;
use sysfs_gpio::{Direction, Pin};

const DEFAULT_PCA9685_ADDRESS: u16 = 0x40;

pub struct Motor {
    pca: PCA9685<LinuxI2CDevice>,
    in1_pin: Pin,
    in2_pin: Pin,
    in3_pin: Pin,
    in4_pin: Pin,
}

pub enum Side {
    Left,
    Right,
}

#[allow(dead_code)]
pub enum Dir {
    Forward,
    Backward,
}

fn prepare_pin(pin: &Pin) {
    pin.export().unwrap();
    pin.set_direction(Direction::Low).unwrap();
}

impl Motor {
    pub fn new() -> Result<Motor, LinuxI2CError> {
        let i2cdevice = LinuxI2CDevice::new("/dev/i2c-1", DEFAULT_PCA9685_ADDRESS)?;

        let mut pca = PCA9685::new(i2cdevice)?;
        pca.set_pwm_freq(100.0)?;

        let in1_pin = Pin::new(6);
        prepare_pin(&in1_pin);

        let in2_pin = Pin::new(5);
        prepare_pin(&in2_pin);

        let in3_pin = Pin::new(27);
        prepare_pin(&in3_pin);

        let in4_pin = Pin::new(17);
        prepare_pin(&in4_pin);

        Ok(Motor {
            pca,
            in1_pin,
            in2_pin,
            in3_pin,
            in4_pin,
        })
    }

    pub fn set_direction(&mut self, side: Side, direction: Dir) {
        match side {
            Side::Left => match direction {
                Dir::Forward => {
                    self.in1_pin.set_value(1).unwrap();
                    self.in2_pin.set_value(0).unwrap();
                }
                Dir::Backward => {
                    self.in1_pin.set_value(0).unwrap();
                    self.in2_pin.set_value(1).unwrap();
                }
            },
            Side::Right => match direction {
                Dir::Forward => {
                    self.in3_pin.set_value(1).unwrap();
                    self.in4_pin.set_value(0).unwrap();
                }
                Dir::Backward => {
                    self.in3_pin.set_value(0).unwrap();
                    self.in4_pin.set_value(1).unwrap();
                }
            },
        }
    }

    pub fn set_speed(&mut self, side: Side, speed: u8) -> () {
        println!("Setting speed to {}", speed);

        let scaled_speed: f32 = (speed as f32) / 100f32 * 82f32 + 18f32;
        let duty_cycle = 4095f32;
        let on = (duty_cycle * scaled_speed / 100f32).round();

        let pwm_pin = match side {
            Side::Left => 0,
            Side::Right => 1,
        };
        println!("Setting pwm to {}", on);
        self.pca.set_pwm(pwm_pin, on as u8, 0).unwrap();
        ()
    }

    pub fn stop(&mut self) {
        self.pca.set_pwm(0, 0, 0).unwrap();
        self.pca.set_pwm(1, 0, 0).unwrap();
        ()
    }
}
