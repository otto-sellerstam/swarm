#![no_std]
#![no_main]

use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_rp::gpio::{Input, Pin, Pull};
use embassy_rp::peripherals::{DMA_CH0, DMA_CH1, PIO0, USB};
use embassy_rp::pio::InterruptHandler;
use embassy_rp::usb::InterruptHandler as UsbInterruptHandler;
use embassy_rp::{Peri, bind_interrupts, dma, rom_data};

pub mod config;
pub mod network;
pub mod sensors;
pub mod usb;

mod panic_impl {
    #[panic_handler]
    fn panic(_info: &core::panic::PanicInfo) -> ! {
        embassy_rp::rom_data::reset_to_usb_boot(0, 0);
        loop {
            cortex_m::asm::nop();
        }
    }
}

bind_interrupts!(pub(crate) struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
    DMA_IRQ_0 => dma::InterruptHandler<DMA_CH0>, dma::InterruptHandler<DMA_CH1>;
    USBCTRL_IRQ => UsbInterruptHandler<USB>;
});

#[embassy_executor::task]
async fn bootsel_task(mut bootsel_button_pin: Input<'static>) {
    bootsel_button_pin.wait_for_falling_edge().await;
    rom_data::reset_to_usb_boot(0, 0);
}
/**
Creates listening task for an input GPIO to trigger `reset_to_usb_boot`.
 */
pub fn setup_bootsel_button<T>(spawner: &Spawner, bootsel_pin: Peri<'static, T>)
where
    T: Pin,
{
    let bootsel_button_pin = Input::new(bootsel_pin, Pull::Up);

    spawner.spawn(bootsel_task(bootsel_button_pin).unwrap());
}
