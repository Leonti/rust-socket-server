use futures::sync::mpsc;
use std::sync::{Arc, Mutex};

extern crate rplidar_drv;
extern crate serialport;

use rplidar_drv::{Health, RplidarDevice, ScanOptions};
use serialport::prelude::*;

use crate::event::{Event, LidarScanPoint, TimedEvent};
use std::time::{Duration, Instant};
use tokio::prelude::*;
use tokio::timer::Interval;

type Tx = mpsc::UnboundedSender<TimedEvent>;

pub struct Lidar {
    tx: Arc<Mutex<Tx>>,
}

impl Lidar {
    pub fn new(tx: Arc<Mutex<Tx>>) -> Lidar {
        Lidar { tx }
    }

    pub fn run(self) -> impl Future<Item = (), Error = ()> {
        let s = SerialPortSettings {
            baud_rate: 115200,
            data_bits: DataBits::Eight,
            flow_control: FlowControl::None,
            parity: Parity::None,
            stop_bits: StopBits::One,
            timeout: Duration::from_millis(1),
        };

        let mut serial_port =
            serialport::open_with_settings("/dev/ttyUSB0", &s).expect("failed to open serial port");

        serial_port
            .write_data_terminal_ready(false)
            .expect("failed to clear DTR");
        /*
            let channel = Channel::<RplidarHostProtocol, serialport::SerialPort>::new(
                RplidarHostProtocol::new(),
                serial_port,
            );
        */
        //    let mut rplidar = RplidarDevice::new(channel);

        //    let serial_port = serialport::open("/dev/ttyUSB0").unwrap();
        let mut rplidar = RplidarDevice::with_stream(serial_port);

        //   rplidar.stop_motor().unwrap();
        let device_info = rplidar.get_device_info();

        match device_info {
            Ok(info) => println!("Rplidar info = {:?}", info),
            Err(e) => println!("Error getting info = {:?}", e),
        }

        let scan_modes = rplidar.get_all_supported_scan_modes();

        match scan_modes {
            Ok(modes) => println!("Scan modes = {:?}", modes),
            Err(e) => println!("Error getting scan modes = {:?}", e),
        }

        //   rplidar.start_scan().unwrap();

        //    println!("Rplidar info = {:?}", device_info);

        let device_health = rplidar
            .get_device_health()
            .expect("failed to get device health");

        match device_health {
            Health::Healthy => {
                println!("LIDAR is healthy.");
            }
            Health::Warning(error_code) => {
                println!("LIDAR is unhealthy, warn: {:04X}", error_code);
            }
            Health::Error(error_code) => {
                println!("LIDAR is unhealthy, error: {:04X}", error_code);
            }
        }

        let typical_scan_mode = rplidar
            .get_typical_scan_mode()
            .expect("failed to get typical scan mode");

        println!("Typical scan mode: {}", typical_scan_mode);

        match rplidar.check_motor_ctrl_support() {
            Ok(support) if support == true => {
                println!(
                    "Accessory board is detected and support motor control, starting motor..."
                );
                rplidar.set_motor_pwm(600).expect("failed to start motor");
            }
            Ok(_) => {
                println!("Accessory board is detected, but doesn't support motor control");
            }
            Err(_) => {
                println!("Accessory board isn't detected");
            }
        }

        println!("Starting LIDAR in typical mode...");

        let actual_mode = rplidar
            .start_scan_with_options(&ScanOptions::with_mode(0))
            .expect("failed to start scan in standard mode");

        println!("Started scan in mode `{}`", actual_mode.name);

        Interval::new(Instant::now(), Duration::from_millis(1000))
            .for_each(move |_| {
                match rplidar.grab_scan() {
                    Ok(scan) => {
                        let event = Event::Lidar {
                            scan_points: scan
                                .iter()
                                .map(|point| LidarScanPoint {
                                    angle: point.angle(),
                                    distance: point.distance(),
                                    quality: point.quality,
                                    is_sync: point.is_sync(),
                                    is_valid: point.is_valid(),
                                })
                                .collect(),
                        };

                        let s_tx = &self.tx.lock().unwrap();
                        match s_tx.unbounded_send(TimedEvent::new(event)) {
                            Ok(_) => (),
                            Err(e) => println!("lidar send error = {:?}", e),
                        }
                    }
                    Err(err) => {
                        println!("Error: {:?}", err);
                    }
                }

                Ok(())
            })
            .map_err(|e| print!("interval errored; err={:?}", e))
    }
}
