#![no_std]
#![feature(type_alias_impl_trait, trait_alias)]

use embassy_executor::Spawner;
use embassy_rp::gpio::Output;
use embassy_rp::interrupt;
use embassy_rp::peripherals::PIN_25;
use embassy_rp::usb::Driver;
use embassy_time::{Duration, Timer};
use shared::side::KeyboardSide;

#[cfg(not(feature = "probe"))]
use panic_reset as _;
#[cfg(feature = "probe")]
use {defmt_rtt as _, panic_probe as _};

#[cfg(feature = "probe")]
use defmt::info;
#[cfg(not(feature = "probe"))]
use log::info;

pub mod event;
pub mod fw_update;
pub mod logger;
pub mod messages;
pub mod side;
pub mod usb;
pub mod utils;

fn detect_usb() -> bool {
    return true;
    let regs = embassy_rp::pac::USBCTRL_REGS;
    unsafe { regs.sie_status().read().connected() }
}

#[embassy_executor::task]
async fn blinky(mut pin: Output<'static, PIN_25>) {
    loop {
        pin.set_high();
        Timer::after(Duration::from_secs(1)).await;

        pin.set_low();
        Timer::after(Duration::from_secs(1)).await;
    }
}

pub async fn main(spawner: Spawner, side: KeyboardSide) {
    let p = embassy_rp::init(Default::default());

    info!("Hello!");

    side::init(side, detect_usb());

    if side::this_side_has_usb() {
        let irq = interrupt::take!(USBCTRL_IRQ);
        let usb_driver = Driver::new(p.USB, irq);

        usb::init(&spawner, usb_driver);
    } else {
        info!("No usb connected");
    }

    logger::setup_logger();
    messages::init(&spawner);
    fw_update::init(&spawner, p.WATCHDOG, p.FLASH);

    spawner.must_spawn(blinky(Output::new(p.PIN_25, embassy_rp::gpio::Level::Low)));

    let mut counter = 0;
    loop {
        counter += 1;

        log::info!("Tick {}", counter);

        #[cfg(feature = "probe")]
        defmt::info!("Tick {}", counter);

        Timer::after(Duration::from_secs(1)).await;
    }
}
