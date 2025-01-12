#![allow(dead_code)]
pub(crate) const SEED_LENGTH: usize = 2048 / 8;
pub(crate) const BUFFER_SIZE: usize = SEED_LENGTH;
pub(crate) const BUFFER_SIZE_FLUSHING: usize = 100000;
pub(crate) const MAXIMUM_NUM_OF_DWORDS: usize = 0xffff;

// From v_counter after reset if there is no hardware error (found experimentally, no idea why)
pub(crate) const FRESH_NIBBLES_AFTER_RESET: i32 = 8188;

pub(crate) const RCT_THR: usize = 6;
pub(crate) const APT_THR_UP: usize = 62;
pub(crate) const APT_THR_DOWN: usize = 8;

#[derive(Debug)]
pub enum DataType {
    DeviceError(String),
    RawStream(RawStream),
    Asym(Vec<i32>),
    Monobit(Vec<(f32, u32, u32)>),
    Runs(Vec<(f64, u32, u32)>),
    Sha256(Vec<u8>)
}

#[derive(Debug)]
pub struct StreamData {
    pub serial: String,
    pub data: Option<DataType>,
}

#[derive(Debug)]
pub struct RawStream {
    buf: [u8; BUFFER_SIZE],
    rct_fail: bool,
    apt_fail: bool
}

impl RawStream {
    pub fn new(buf: [u8; BUFFER_SIZE], rct: bool, apt: bool) -> Self {
        Self {
            buf,
            rct_fail: rct,
            apt_fail: apt
        }
    }

    pub fn get_buf(&self) -> [u8; BUFFER_SIZE] {
        self.buf
    }

    pub fn get_rct_fail(&self) -> bool {
        self.rct_fail
    }

    pub fn get_apt_fail(&self) -> bool {
        self.apt_fail
    }
}