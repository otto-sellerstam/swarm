use crate::config::ENV_CONFIG;
use cyw43::{Control, JoinOptions, NetDriver, aligned_bytes};
use cyw43_pio::{DEFAULT_CLOCK_DIVIDER, PioSpi};
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_net::{Config, Stack, StackResources};
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH1, PIN_23, PIN_24, PIN_25, PIN_29, PIO0};
use embassy_rp::pio::{Common, Irq, StateMachine};
use embassy_rp::{Peri, dma};
use embassy_time::{Duration, Timer};
use log::{error, info};
use static_cell::StaticCell;

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

pub struct Cyw43Pins {
    pub pin_pwr: Peri<'static, PIN_23>,
    pub pin_cs: Peri<'static, PIN_25>,
    pub pin_dio: Peri<'static, PIN_24>,
    pub pin_clk: Peri<'static, PIN_29>,
}

async fn initialize_cyw43(
    spawner: &Spawner,
    mut common: Common<'static, PIO0>,
    sm0: StateMachine<'static, PIO0, 0>,
    irq0: Irq<'static, PIO0, 0>,
    dma_ch: dma::Channel<'static>,
    cyw43_pins: Cyw43Pins,
) -> (Control<'static>, NetDriver<'static>) {
    let fw = aligned_bytes!("../../cyw43-firmware/43439A0.bin");
    let clm = aligned_bytes!("../../cyw43-firmware/43439A0_clm.bin");
    let nvram = aligned_bytes!("../../cyw43-firmware/nvram_rp2040.bin");

    let pwr = Output::new(cyw43_pins.pin_pwr, Level::Low);
    let cs = Output::new(cyw43_pins.pin_cs, Level::High);
    let spi = PioSpi::new(
        &mut common,
        sm0,
        DEFAULT_CLOCK_DIVIDER,
        irq0,
        cs,
        cyw43_pins.pin_dio,
        cyw43_pins.pin_clk,
        dma_ch,
    );

    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());
    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw, nvram).await;
    spawner.spawn(cyw43_task(runner).unwrap());

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
    let config = Config::dhcpv4(Default::default());

    static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
    let resources = RESOURCES.init(StackResources::new());

    let seed = 0x0123_4567_89ab_cdef;
    let (stack, runner) = embassy_net::new(net_device, config, resources, seed);

    spawner.spawn(net_task(runner).unwrap());

    loop {
        match control
            .join(
                ENV_CONFIG.wifi_ssid,
                JoinOptions::new(ENV_CONFIG.wifi_password.as_bytes()),
            )
            .await
        {
            Ok(_) => {
                info!("Joined WiFi");
                break;
            }
            Err(err) => {
                error!("Failed to join WiFi: {:?}", err);
                Timer::after(Duration::from_secs(2)).await;
            }
        };
    }

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
    common: Common<'static, PIO0>,
    sm0: StateMachine<'static, PIO0, 0>,
    irq0: Irq<'static, PIO0, 0>,
    dma_ch: dma::Channel<'static>,
    cyw43_pins: Cyw43Pins,
) -> (Control<'static>, Stack<'static>) {
    let (mut control, net_device) =
        initialize_cyw43(spawner, common, sm0, irq0, dma_ch, cyw43_pins).await;

    let stack = initialize_network(spawner, &mut control, net_device).await;

    (control, stack)
}
