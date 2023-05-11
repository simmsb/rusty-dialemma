#![no_std]
#![allow(incomplete_features)]
#![feature(type_alias_impl_trait, trait_alias, async_fn_in_trait)]

use embassy_executor::Spawner;
use embassy_rp::dma::Channel;
use embassy_rp::gpio::{AnyPin, Input, Pin};
use embassy_rp::interrupt;
use embassy_rp::peripherals::{PIN_25, PIN_24, PIN_19};
use embassy_rp::rom_data::reset_to_usb_boot;
use embassy_rp::usb::Driver;
use embassy_rp::{gpio::Output, pio::PioPeripheral};
use embassy_time::{Duration, Timer};
use shared::side::KeyboardSide;

#[cfg(not(feature = "probe"))]
use panic_reset as _;
#[cfg(feature = "probe")]
use {defmt_rtt as _, panic_probe as _};

use utils::log;

use crate::messages::reliable_msg;

pub mod event;
#[cfg(feature = "bootloader")]
pub mod fw_update;
pub mod interboard;
pub mod logger;
pub mod messages;
pub mod side;
pub mod trackpad;
pub mod usb;
pub mod utils;

pub static VERSION: &str = "0.1.0";

fn detect_usb(pin: Input<'_, PIN_19>) -> bool {
    let connected = pin.is_high();
    log::info!("Usb connected? {}", connected);
    connected
}

#[embassy_executor::task]
async fn blinky(mut pin: Output<'static, AnyPin>) {
    loop {
        pin.set_high();
        Timer::after(Duration::from_secs(1)).await;

        pin.set_low();
        Timer::after(Duration::from_secs(1)).await;
    }
}

#[link_section = ".ram0.bootloader_magic"]
#[used]
static mut BOOTLOADER_MAGIC: u32 = 0;

const MAGIC_TOKEN: u32 = 0xCAFEB0BA;

unsafe fn check_bootloader() {
    const CYCLES_PER_US: usize = 125;
    const WAIT_CYCLES: usize = 500 * 1000 * CYCLES_PER_US;

    if BOOTLOADER_MAGIC != MAGIC_TOKEN {
        BOOTLOADER_MAGIC = MAGIC_TOKEN;

        cortex_m::asm::delay(WAIT_CYCLES as u32);
        BOOTLOADER_MAGIC = 0;
        return;
    }

    BOOTLOADER_MAGIC = 0;

    reset_to_usb_boot(1 << 17, 0);
}

pub async fn main(spawner: Spawner, side: KeyboardSide) {
    let p = embassy_rp::init(Default::default());
    unsafe {
        check_bootloader();
    }

    log::info!("Just a whisper. I hear it in my ghost.");

    // not sure if this makes the usb detection happier
    Timer::after(Duration::from_micros(100)).await;

    side::init(side, detect_usb(Input::new(p.PIN_19, embassy_rp::gpio::Pull::Down)));

    if side::this_side_has_usb() {
        let irq = interrupt::take!(USBCTRL_IRQ);
        let usb_driver = Driver::new(p.USB, irq);

        usb::init(&spawner, usb_driver);
    } else {
        log::info!("No usb connected");
    }

    logger::setup_logger();
    messages::init(&spawner);
    #[cfg(feature = "bootloader")]
    fw_update::init(&spawner, p.WATCHDOG, p.FLASH);

    let (_pio0, sm0, sm1, _sm2, _sm3) = p.PIO0.split();
    let usart_pin = p.PIN_1.into();
    // let usart_pin = p.PIN_25.into();

    interboard::init(&spawner, sm0, sm1, usart_pin);

    if side::get_side().is_right() {
        trackpad::init(
            &spawner,
            p.SPI0,
            p.PIN_22,
            p.PIN_23,
            p.PIN_20,
            p.PIN_21,
            p.DMA_CH0.degrade(),
            p.DMA_CH1.degrade(),
        );
    }

    spawner.must_spawn(blinky(Output::new(
        p.PIN_17.degrade(),
        embassy_rp::gpio::Level::Low,
    )));

    let mut counter = 0u8;
    loop {
        counter = counter.wrapping_add(1);

        Timer::after(Duration::from_secs(1)).await;

        log::info!("tick");

        if side::get_side().is_left() {
            interboard::send_msg(reliable_msg(shared::device_to_device::DeviceToDevice::Ping))
                .await;
        }
    }
}
