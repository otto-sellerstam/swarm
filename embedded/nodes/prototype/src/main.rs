#![no_std]
#![no_main]

use defmt::*;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_net::{Stack, tcp::TcpSocket};
use embassy_rp::bind_interrupts;
use embassy_rp::dma;
use embassy_rp::dma::InterruptHandler as DmaInterruptHandler;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, DMA_CH1, PIO0, PIO1, USB};
use embassy_rp::pio::InterruptHandler as PioInterruptHandler;
use embassy_rp::pio::Pio;
use embassy_rp::usb::Driver;
use embassy_rp::usb::InterruptHandler as UsbInterruptHandler;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::pubsub::{PubSubChannel, WaitResult};
use embassy_time::Duration;
use embedded_io_async::Write;
use log::info;
use swarm_lib::network::{Cyw43Pins, initialize_wifi_and_network};
use swarm_lib::sensors::ky_040::{Event as RotaryEvent, RotaryEncoder};
use swarm_lib::usb::init_usb_logger;

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => PioInterruptHandler<PIO0>;
    PIO1_IRQ_0 => PioInterruptHandler<PIO1>;
    DMA_IRQ_0 => DmaInterruptHandler<DMA_CH0>, DmaInterruptHandler<DMA_CH1>;
    USBCTRL_IRQ => UsbInterruptHandler<USB>;
});

static BUS: PubSubChannel<CriticalSectionRawMutex, CommandEvent, 4, 3, 1> = PubSubChannel::new();

#[derive(Clone, Debug)]
enum CommandEvent {
    Solid,
    Off,
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    let led_pin = Output::new(p.PIN_12, Level::Low);

    init_usb_logger(&spawner, Driver::new(p.USB, Irqs));
    //setup_bootsel_button(&spawner, p.PIN_16);

    let pio = Pio::new(p.PIO0, Irqs);
    let dma_ch = dma::Channel::new(p.DMA_CH0, Irqs);

    let (_control, stack) = initialize_wifi_and_network(
        &spawner,
        pio.common,
        pio.sm0,
        pio.irq0,
        dma_ch,
        Cyw43Pins {
            pin_pwr: p.PIN_23,
            pin_cs: p.PIN_25,
            pin_dio: p.PIN_24,
            pin_clk: p.PIN_29,
        },
    )
    .await;

    spawner.spawn(tcp_server(stack).unwrap());
    spawner.spawn(handle_led(led_pin).unwrap());

    let Pio {
        mut common, sm0, ..
    } = Pio::new(p.PIO1, Irqs);

    let mut ky_040 = RotaryEncoder::new(&mut common, sm0, p.PIN_14, p.PIN_15, p.PIN_16);
    loop {
        match ky_040.next_event().await {
            RotaryEvent::PressDown => info!("PressDown"),
            RotaryEvent::PressUp => info!("PressUp"),
            RotaryEvent::RotationClockwise => info!("RotationClockwise"),
            RotaryEvent::RotationAntiClockwise => info!("RotationAntiClockwise"),
        }
    }
}

fn get_event_from_waitresult(wait_result: WaitResult<CommandEvent>) -> CommandEvent {
    match wait_result {
        WaitResult::Message(e) => e,
        WaitResult::Lagged(_) => defmt::panic!("We missed messages :("),
    }
}

#[embassy_executor::task]
async fn handle_led(mut led_pin: Output<'static>) {
    let mut event: CommandEvent = CommandEvent::Off;
    let mut sub = BUS.subscriber().unwrap();

    loop {
        match event {
            CommandEvent::Solid => led_pin.set_high(),
            CommandEvent::Off => led_pin.set_low(),
        }
        let wr = sub.next_message().await;
        event = get_event_from_waitresult(wr);
    }
}

#[embassy_executor::task]
async fn tcp_server(stack: Stack<'static>) -> ! {
    let mut rx_buffer = [0u8; 1024];
    let mut tx_buffer = [0u8; 1024];

    loop {
        let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
        socket.set_timeout(Some(Duration::from_secs(60)));

        info!("Waiting for connection on port 8000");
        if let Err(e) = socket.accept(8000).await {
            warn!("Accept error: {:?}", e);
            continue;
        }

        let remote = socket.remote_endpoint();
        info!("Connection from {:?}", remote);

        let pubr = BUS.publisher().unwrap();
        let mut buf = [0u8; 64];

        loop {
            let n = match socket.read(&mut buf).await {
                Ok(0) => {
                    info!("Connection closed by peer");
                    break;
                }
                Ok(n) => n,
                Err(e) => {
                    warn!("Read error: {:?}", e);
                    break;
                }
            };

            let command = core::str::from_utf8(&buf[..n]).unwrap_or("").trim();

            info!("Recevied: {}", command);

            let event = match command {
                "on" => Some(CommandEvent::Solid),
                "off" => Some(CommandEvent::Off),
                _ => {
                    let _ = socket.write_all(b"Unkown command\r\n").await;
                    None
                }
            };

            if let Some(event) = event {
                pubr.publish(event).await;
                let _ = socket.write_all(b"OK\r\n").await;
            }
        }

        socket.close();
    }
}
