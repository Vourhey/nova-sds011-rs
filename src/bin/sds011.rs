extern crate sds011;
use sds011::SDS011;

use clap::{App, Arg};
use std::thread::sleep;
use std::time::Duration;

fn main() {
    let matches = App::new("SDS011 Driver")
        .version("0.1.3")
        .author("Vadim Manaenko <vadim.razorq@gmail.com>")
        .about("Reads data from Nova SDS011 Sensor")
        .arg(
            Arg::with_name("port")
                .short("p")
                .long("port")
                .takes_value(true)
                .default_value("/dev/ttyUSB0")
                .help("Specify port a sensor is connected to"),
        )
        .arg(
            Arg::with_name("work_period")
                .short("w")
                .long("work")
                .takes_value(true)
                .default_value("5")
                .help("Work period in minutes"),
        )
        .get_matches();

    let port = matches.value_of("port").unwrap();
    let work_period_str = matches.value_of("work_period").unwrap();
    let work_period = work_period_str.parse::<u8>().unwrap();

    match SDS011::new(port) {
        Ok(mut sensor) => {
            sensor.set_work_period(work_period).unwrap();

            loop {
                if let Ok(m) = sensor.query() {
                    println!("{:?}", m);
                }

                sleep(Duration::from_secs(work_period as u64 * 60));
            }
        }
        Err(e) => println!("{:?}", e.description),
    };
}
