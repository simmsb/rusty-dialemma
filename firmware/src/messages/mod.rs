use embassy_executor::Spawner;
use embassy_time::Duration;

pub mod device_to_device;
pub mod distributors;
pub mod transmissions;

pub use distributors::{send_to_host, try_send_to_host};

pub fn init(spawner: &Spawner) {
    spawner.must_spawn(distributors::from_usb_distributor());
    spawner.must_spawn(distributors::from_other_side_distributor());
}

#[derive(Debug)]
pub struct TransmittedMessage<T> {
    pub msg: T,
    pub timeout: Option<Duration>,
}

pub fn low_latency_msg<T>(msg: T) -> TransmittedMessage<T> {
    TransmittedMessage {
        msg,
        timeout: Some(Duration::from_micros(500)),
    }
}

pub fn reliable_msg<T>(msg: T) -> TransmittedMessage<T> {
    TransmittedMessage {
        msg,
        timeout: Some(Duration::from_millis(2)),
    }
}

pub fn unreliable_msg<T>(msg: T) -> TransmittedMessage<T> {
    TransmittedMessage { msg, timeout: None }
}
