use i2cdev::linux::*;
use i2c_pca9685::PCA9685;

const DEFAULT_PCA9685_ADDRESS: u16 = 0x40;

pub struct Motor {
    pca: PCA9685<LinuxI2CDevice>
}

impl Motor {

    pub fn new() -> Result<Motor, LinuxI2CError> {
        let i2cdevice = LinuxI2CDevice::new("/dev/i2c-1", DEFAULT_PCA9685_ADDRESS)?;

        let mut pca = PCA9685::new(i2cdevice)?;
        pca.set_pwm_freq(60.0)?;

        Ok(Motor {
            pca
        })
    }

    pub fn set_speed(&mut self) -> () {

        self.pca.set_pwm(1, 0, 200).unwrap();
        ()
    }

}
