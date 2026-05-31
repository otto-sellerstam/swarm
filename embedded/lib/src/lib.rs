#![no_std]
#![no_main]

use cyw43::{Control, JoinOptions, NetDriver, aligned_bytes};
use cyw43_pio::{DEFAULT_CLOCK_DIVIDER, PioSpi};
use embassy_executor::Spawner;
use embassy_net::{Config, Stack, StackResources};
use embassy_rp::gpio::{Input, Level, Output, Pin, Pull};
use embassy_rp::peripherals::{DMA_CH0, DMA_CH1, PIN_23, PIN_24, PIN_25, PIN_29, PIO0, USB};
use embassy_rp::pio::{InterruptHandler, Pio};
use embassy_rp::usb::{Driver, InterruptHandler as UsbInterruptHandler};
use embassy_rp::{Peri, bind_interrupts, dma, rom_data};
use embassy_time::{Duration, Timer};
use log::info;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
    DMA_IRQ_0 => dma::InterruptHandler<DMA_CH0>, dma::InterruptHandler<DMA_CH1>;
    USBCTRL_IRQ => UsbInterruptHandler<USB>;
});

#[embassy_executor::task]
async fn cyw43_task(
    runner: cyw43::Runner<'static, cyw43::SpiBus<Output<'static>, PioSpi<'static, PIO0, 0>>>,
) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn logger_task(driver: Driver<'static, USB>) {
    embassy_usb_logger::run!(1024, log::LevelFilter::Info, driver);
}

pub struct Cyw43Pins {
    pub pin_pwr: Peri<'static, PIN_23>,
    pub pin_cs: Peri<'static, PIN_25>,
    pub pin_pio: Peri<'static, PIO0>,
    pub pin_dio: Peri<'static, PIN_24>,
    pub pin_clk: Peri<'static, PIN_29>,
    pub pin_dma_tx: Peri<'static, DMA_CH0>,
    pub pin_dma_rx: Peri<'static, DMA_CH1>,
}

async fn initialize_cyw43(
    spawner: &Spawner,
    cyw43_pins: Cyw43Pins,
) -> (Control<'static>, NetDriver<'static>) {
    let fw = aligned_bytes!("../../cyw43-firmware/43439A0.bin");
    let clm = aligned_bytes!("../../cyw43-firmware/43439A0_clm.bin");
    let nvram = aligned_bytes!("../../cyw43-firmware/nvram_rp2040.bin");

    let pwr = Output::new(cyw43_pins.pin_pwr, Level::Low);
    let cs = Output::new(cyw43_pins.pin_cs, Level::High);
    let mut pio = Pio::new(cyw43_pins.pin_pio, Irqs);
    let spi = PioSpi::new(
        &mut pio.common,
        pio.sm0,
        DEFAULT_CLOCK_DIVIDER,
        pio.irq0,
        cs,
        cyw43_pins.pin_dio,
        cyw43_pins.pin_clk,
        dma::Channel::new(cyw43_pins.pin_dma_tx, Irqs),
        //dma::Channel::new(cyw43_pins.pin_dma_rx, Irqs),
    );

    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());
    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw, nvram).await;
    spawner.spawn((cyw43_task(runner)).unwrap());

    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    (control, net_device)
}

async fn initialize_network(
    spawner: &Spawner,
    control: &mut Control<'static>,
    net_device: NetDriver<'static>,
) -> Stack<'static> {
    let ssid = "Tele2_ffae24";
    let password = "ywm4mtzj";

    let config = Config::dhcpv4(Default::default());

    static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
    let resources = RESOURCES.init(StackResources::new());

    let seed = 0x0123_4567_89ab_cdef;
    let (stack, runner) = embassy_net::new(net_device, config, resources, seed);

    spawner.spawn(net_task(runner).unwrap());

    control
        .join(ssid, JoinOptions::new(password.as_bytes()))
        .await
        .unwrap();

    // Wait for DHCP
    loop {
        if let Some(config) = stack.config_v4() {
            control.gpio_set(0, true).await;
            info!("Got IP {}", config.address);
            break;
        }
        Timer::after(Duration::from_millis(500)).await;
    }

    stack
}

pub async fn initialize_wifi_and_network(
    spawner: &Spawner,
    cyw43_pins: Cyw43Pins,
) -> (Control<'static>, Stack<'static>) {
    let (mut control, net_device) = initialize_cyw43(spawner, cyw43_pins).await;

    let stack = initialize_network(spawner, &mut control, net_device).await;

    (control, stack)
}

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

pub fn init_usb_logger(spawner: &Spawner, usb: Peri<'static, USB>) {
    let usb_driver = Driver::new(usb, Irqs);
    spawner.spawn(logger_task(usb_driver).unwrap());
}
