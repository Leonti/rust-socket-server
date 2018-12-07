
use i2cdev::core::I2CDevice;
pub use i2cdev::linux::{LinuxI2CDevice,LinuxI2CError};
use std::thread::sleep;
use std::time::Duration;

const MODE_1_REG: u8 = 0x00;
const MODE_2_REG: u8 = 0x01;
const LED0_ON_L: u8 = 0x06;
const LED0_ON_H: u8 = 0x07;
const LED0_OFF_L: u8 = 0x08;
const LED0_OFF_H: u8 = 0x09;
const PRE_SCALE_REG: u8 = 0xFE;

const AUTO_INCREMENT: u8 = 0b1 << 5;

pub struct PCA9685 {
	pub device: LinuxI2CDevice
}

impl PCA9685 {
	pub fn new(device: LinuxI2CDevice, frequency: u16) -> Result<PCA9685, LinuxI2CError> {
        assert!(frequency >= 40 && frequency <= 1000);

		// Setting auto-increment lets us write independent PWM values to all
		// channels using only one i2c write.
		let mut mode1 = 0x01 | AUTO_INCREMENT;
		let mut pca9685 = PCA9685 { device: device };

		pca9685.device.smbus_write_byte_data(MODE_2_REG, 0x04)?;
		pca9685.device.smbus_write_byte_data(MODE_1_REG, mode1)?;
		sleep(Duration::from_millis(6));
		mode1 &= !0x01;
		pca9685.device.smbus_write_byte_data(MODE_1_REG, mode1)?;
		sleep(Duration::from_millis(6));

        // set frequency
        let mut prescalelevel = 25000000.0;
		prescalelevel /= 4096.0;
		prescalelevel /= frequency as f32;
        prescalelevel -= 1.0;

        let mode = 0x01;
        pca9685.device.smbus_write_byte_data(MODE_1_REG, (mode & 0x7F) | 0x10)?;
		pca9685.device.smbus_write_byte_data(PRE_SCALE_REG, prescalelevel as u8)?;
		pca9685.device.smbus_write_byte_data(MODE_1_REG, mode)?;
		sleep(Duration::from_millis(6));
        pca9685.device.smbus_write_byte_data(MODE_1_REG, mode | 0x80)?;

		Ok(pca9685)
	}

    pub fn set_duty_cycle(&mut self, channel: u8, duty_cycle: u16) -> Result<(), LinuxI2CError> {
		assert!(duty_cycle < 4096);
		// let off = 4095 - duty_cycle;
		self.device.smbus_write_byte_data(LED0_ON_L+4*channel, 0)?;
		self.device.smbus_write_byte_data(LED0_ON_H+4*channel, 0)?;
		self.device.smbus_write_byte_data(LED0_OFF_L+4*channel, (duty_cycle & 0xFF) as u8)?;
		self.device.smbus_write_byte_data(LED0_OFF_H+4*channel, (duty_cycle >> 8) as u8)?;
		Ok(())
    }

}

/* 

    __MODE1 = 0x00,
__PRESCALE = 0xFE,

    pwm.setFreq = function (freq, correctionFactor) {
        if (!isValidFreq(freq)) throw new Error("Frequency must be between 40 and 1000 Hz");
        var oldmode, newmode, prescale, prescaleval;
        correctionFactor = correctionFactor || 1.0;
        prescaleval = 25000000;
        prescaleval /= 4096.0;
        prescaleval /= freq;
        prescaleval -= 1.0;
        prescale = Math.floor(prescaleval * correctionFactor + 0.5);
        oldmode = i2cRead(this.i2c, __MODE1, 1);
        newmode = (oldmode & 0x7F) | 0x10;
        i2cSend(this.i2c, __MODE1, newmode);
        i2cSend(this.i2c, __PRESCALE, Math.floor(prescale));
        i2cSend(this.i2c, __MODE1, oldmode);
        sleep.usleep(10000);
        i2cSend(this.i2c, __MODE1, oldmode | 0x80);
}
*/


