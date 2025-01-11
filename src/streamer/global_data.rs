pub(crate) const SEED_LENGTH: usize = 248;
pub(crate) const BUFFER_SIZE: usize = 256;
pub(crate) const BUFFER_SIZE_32_BITS: usize = 4;
pub(crate) const BUFFER_SIZE_64_BITS: usize = 8;
pub(crate) const BUFFER_SIZE_FLUSHING: usize = 100000;

// From v_counter after reset if there is no hardware error (found experimentally, no idea why)
pub(crate) const FRESH_NIBBLES_AFTER_RESET: i32 = 8188;

pub(crate) const RCT_THR: usize = 6;
pub(crate) const APT_THR_UP: usize = 62;
pub(crate) const APT_THR_DOWN: usize = 8;

#[derive(Debug)]
pub enum DataType {
    DEVICE_ERROR(String),
    RAW_STREAM(RawStream),
    MONOBIT,
    RUNS,
    ASYM,
    SHA256
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