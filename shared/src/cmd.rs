use core::{
    hash::{Hash, Hasher},
    u8,
};

use serde::{Deserialize, Serialize};

#[cfg(feature = "defmt")]
use defmt::debug;
#[cfg(not(feature = "defmt"))]
use log::debug;

#[derive(Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Reliabilty {
    Reliable { id: u8, csum: u8 },
    Unreliable,
}

#[derive(Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Command<T> {
    pub reliability: Reliabilty,
    pub cmd: T,
}

pub fn calc_csum<T: Hash>(v: T) -> u8 {
    let mut hasher = StableHasher::new(fnv::FnvHasher::default());
    v.hash(&mut hasher);
    let checksum = hasher.finish();

    let bytes = checksum.to_le_bytes();

    bytes.iter().fold(0, core::ops::BitXor::bitxor)
}

impl<T: Hash> Command<T> {
    pub fn new_reliable(cmd: T, id: u8) -> Self {
        let csum = calc_csum((&cmd, id));

        Self {
            reliability: Reliabilty::Reliable { id, csum },
            cmd,
        }
    }

    pub fn new_unreliable(cmd: T) -> Self {
        Self {
            reliability: Reliabilty::Unreliable,
            cmd,
        }
    }

    /// validate the data of the command
    /// though the data will probably fail to deserialize if it has been corrupted, this just makes sure
    pub fn validate(&self) -> bool {
        if let Reliabilty::Reliable { id, csum } = self.reliability {
            let expected_csum = calc_csum((&self.cmd, id));
            if csum == expected_csum {
                true
            } else {
                debug!(
                    "Invalid csum on {}, expected: {}, computed: {}",
                    core::any::type_name::<Self>(),
                    expected_csum,
                    csum
                );
                false
            }
        } else {
            true
        }
    }

    pub fn ack(&self) -> Option<Ack> {
        if let Reliabilty::Reliable { id, .. } = self.reliability {
            let csum = calc_csum(id);
            Some(Ack { id, csum })
        } else {
            None
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Ack {
    pub id: u8,
    pub csum: u8,
}

#[derive(Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
pub enum CmdOrAck<T> {
    Cmd(Command<T>),
    Ack(Ack),
}

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct AckValidationError {
    pub id: u8,
    pub expected_csum: u8,
    pub given_csum: u8,
}

impl Ack {
    pub fn validate(self) -> Result<Self, AckValidationError> {
        let csum = calc_csum(self.id);
        if csum == self.csum {
            Ok(self)
        } else {
            Err(AckValidationError {
                id: self.id,
                expected_csum: self.csum,
                given_csum: csum,
            })
        }
    }
}

#[derive(Debug, Default)]
struct StableHasher<T> {
    inner: T,
}

impl<T: Hasher> Hasher for StableHasher<T> {
    fn write_u8(&mut self, i: u8) {
        self.write(&[i])
    }

    fn write_u16(&mut self, i: u16) {
        self.write(&i.to_le_bytes())
    }

    fn write_u32(&mut self, i: u32) {
        self.write(&i.to_le_bytes())
    }

    fn write_u64(&mut self, i: u64) {
        self.write(&i.to_le_bytes())
    }

    fn write_u128(&mut self, i: u128) {
        self.write(&i.to_le_bytes())
    }

    fn write_usize(&mut self, i: usize) {
        let bytes = i.to_le_bytes().iter().fold(0, core::ops::BitXor::bitxor);
        self.write(&[bytes])
    }

    fn write_i8(&mut self, i: i8) {
        self.write_u8(i as u8)
    }

    fn write_i16(&mut self, i: i16) {
        self.write_u16(i as u16)
    }

    fn write_i32(&mut self, i: i32) {
        self.write_u32(i as u32)
    }

    fn write_i64(&mut self, i: i64) {
        self.write_u64(i as u64)
    }

    fn write_i128(&mut self, i: i128) {
        self.write_u128(i as u128)
    }

    fn write_isize(&mut self, i: isize) {
        self.write_usize(i as usize)
    }

    fn finish(&self) -> u64 {
        self.inner.finish()
    }

    fn write(&mut self, bytes: &[u8]) {
        self.inner.write(bytes);
    }
}

impl<T> StableHasher<T> {
    fn new(inner: T) -> Self {
        Self { inner }
    }
}
