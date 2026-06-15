use crate::Irqs;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_rp::Peri;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::Driver;

pub fn init_usb_logger(spawner: &Spawner, usb: Peri<'static, USB>) {
    let usb_driver = Driver::new(usb, Irqs);
    spawner.spawn(logger_task(usb_driver).unwrap());
}

#[embassy_executor::task]
async fn logger_task(driver: Driver<'static, USB>) {
    embassy_usb_logger::run!(1024, log::LevelFilter::Info, driver);
}
