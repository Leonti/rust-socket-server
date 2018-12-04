use pca9685::PCA9685;
use i2cdev::linux::*;
use sysfs_gpio::{Direction, Pin};

const DEFAULT_PCA9685_ADDRESS: u16 = 0x40;

pub struct Motor {
    pca: PCA9685,
    in1_pin: Pin,
    in2_pin: Pin,
    in3_pin: Pin,
    in4_pin: Pin,
}

#[derive(Debug)]
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

        // 50 is an unused value
        let mut pca = PCA9685::new(i2cdevice, 50)?;
        pca.set_frequency(100)?;

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

    pub fn set_speed(&mut self, side: Side, speed: f32) -> () {
        println!("Setting speed to {} on side {:?}", speed, side);

        let scaled_speed = speed / 100.0 * 82.0 + 18.0;
        let duty_cycle = 4095f32;
        let pulse_length = duty_cycle * scaled_speed / 100f32;

        let pwm_pin = match side {
            Side::Left => 0,
            Side::Right => 1,
        };

        println!("Setting pwm to {}", pulse_length);
        self.pca.set_pulse_length(pwm_pin, pulse_length).unwrap();
        ()
    }

    pub fn stop(&mut self) {
        self.pca.set_pulse_length(0, 0.0).unwrap();
        self.pca.set_pulse_length(1, 0.0).unwrap();
        ()
    }
}
