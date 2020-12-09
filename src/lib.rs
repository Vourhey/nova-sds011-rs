use serde::{Deserialize, Serialize};
use serialport::{DataBits, FlowControl, Parity, SerialPort, SerialPortSettings, StopBits};
use std::iter::FromIterator;
use std::mem::transmute;
use std::time::{Duration, SystemTime};

mod error;
pub use error::*;

const HEAD: u8 = b'\xaa';
const TAIL: u8 = b'\xab';
const CMD_ID: u8 = b'\xb4';

const READ: u8 = b'\x00';
const WRITE: u8 = b'\x01';

const REPORT_MODE_CMD: u8 = b'\x02';
const ACTIVE: u8 = b'\x00';
const PASSIVE: u8 = b'\x01';

const QUERY_CMD: u8 = b'\x04';

// The sleep command ID
// TODO
//const SLEEP_CMD: u8 = b'\x06';
// Sleep and work byte
// TODO
// const SLEEP: u8 = b'\x00';
// const WORK: u8= b'\x01';

// The work period command ID
const WORK_PERIOD_CMD: u8 = b'\x08';

/// Struct holds a link to a sensor and provides functions to interact with it
///
/// Example:
/// ```
/// use sds011::{SDS011};
/// use std::thread::sleep;
/// use std::time::{Duration};
///
/// match SDS011::new(port) {
///     Ok(mut sensor) => {
///         sensor.set_work_period(work_period).unwrap();
///
///         loop {
///             if let Some(m) = sensor.query() {
///                 println!("{:?}", m);
///             }
///
///             sleep(Duration::from_secs(5u64 * 60));
///         }
///     },
///     Err(e) => println!("{:?}", e.description),
/// };
/// ```
pub struct SDS011 {
    /// Link to a sensor, must be open via new()
    port: Box<dyn SerialPort>,
}

/// Represents a single measurement
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Message {
    /// A timestamp in UNIX format
    pub timestamp: String,
    /// PM2.5 particles
    pub pm25: f32,
    /// PM10 particles
    pub pm10: f32,
}

impl std::fmt::Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "[{}] PM10={} PM25={}", self.timestamp, self.pm10, self.pm25)
    }
}

impl SDS011 {
    /// Creates new instance of SDS011
    /// `port` is required, for example `/dev/ttyUSB0`
    ///
    /// # Example
    /// ```
    /// let mut sensor = SDS011::new("/dev/ttyUSB0").unwrap();
    /// ```
    pub fn new(port: &str) -> Result<SDS011> {
        let s = SerialPortSettings {
            baud_rate: 9600,
            data_bits: DataBits::Eight,
            flow_control: FlowControl::None,
            parity: Parity::None,
            stop_bits: StopBits::One,
            timeout: Duration::from_secs(2),
        };

        let opened = serialport::open_with_settings(port, &s);
        match opened {
            Ok(o) => {
                let mut s = SDS011 { port: o };
                s.set_report_mode()?;
                Ok(s)
            }
            Err(e) => Err(e.into()),
        }
    }

    /// Sets report mode
    /// TODO at the moment sets WRITE and PASSIVE mode only
    pub fn set_report_mode(&mut self) -> Result<()> {
        let read = false;
        let active = false;

        let mut cmd = self.cmd_begin();

        cmd.push(REPORT_MODE_CMD);
        cmd.push(if read { READ } else { WRITE });
        cmd.push(if active { ACTIVE } else { PASSIVE });
        cmd.append(vec![b'\x00'; 10].as_mut());

        self.finish_cmd(&mut cmd);
        self.execute(&cmd)?;
        self.get_reply()?;
        Ok(())
    }

    /// Reads data from the sensor and returns as `Message`
    pub fn query(&mut self) -> Result<Message> {
        let mut cmd = self.cmd_begin();

        cmd.push(QUERY_CMD);
        cmd.append(vec![b'\x00'; 12].as_mut());

        self.finish_cmd(&mut cmd);
        self.execute(&cmd)?;

        let raw = self.get_reply()?;

        let pm25_ar = [raw[2], raw[3]];
        let pm10_ar = [raw[4], raw[5]];
        let pm25: u16 = unsafe { transmute::<[u8; 2], u16>(pm25_ar) }.to_le();
        let pm10: u16 = unsafe { transmute::<[u8; 2], u16>(pm10_ar) }.to_le();

        Ok(Message {
            timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs()
                .to_string(),
            pm25: pm25 as f32 / 10.0,
            pm10: pm10 as f32 / 10.0,
        })
    }

    /// Returns command header and command ID bytes
    pub fn cmd_begin(&self) -> Vec<u8> {
        let mut vec = Vec::new();
        vec.push(HEAD);
        vec.push(CMD_ID);
        vec
    }

    /// Sets working period
    /// `work_time` must be between 0 and 30
    pub fn set_work_period(&mut self, work_time: u8) -> Result<()> {
        if work_time > 30 {
            return Err(Error::TooLongWorkTime);
        }
        let read = false;
        let mut cmd = self.cmd_begin();

        cmd.push(WORK_PERIOD_CMD);
        cmd.push(if read { READ } else { WRITE });
        cmd.push(work_time);
        cmd.append(vec![b'\x00'; 10].as_mut());

        self.finish_cmd(&mut cmd);
        self.execute(&cmd)?;
        self.get_reply()?;
        Ok(())
    }

    fn finish_cmd(&self, cmd: &mut Vec<u8>) {
        let id1 = b'\xff';
        let id2 = b'\xff';

        cmd.push(id1);
        cmd.push(id2);

        let ch = Vec::from_iter(cmd[2..].iter().cloned());
        let mut checksum: u32 = 0;
        for i in ch {
            checksum += i as u32;
        }
        checksum = checksum % 256;

        cmd.push(checksum as u8);
        cmd.push(TAIL);
    }

    fn execute(&mut self, cmd_bytes: &Vec<u8>) -> Result<()> {
        self.port.write_all(cmd_bytes)?;
        Ok(())
    }

    fn get_reply(&mut self) -> Result<[u8; 10]> {
        let mut buf = [0u8; 10];
        self.port.read_exact(buf.as_mut())?;

        let data = &buf[2..8];
        if data.len() == 0 {
            return Err(Error::EmptyDataFrame);
        }

        let mut checksum: u32 = 0;
        for i in data.iter() {
            checksum += *i as u32;
        }
        checksum = checksum & 255;

        if checksum as u8 != buf[8] {
            return Err(Error::BadChecksum);
        }

        Ok(buf)
    }
}
