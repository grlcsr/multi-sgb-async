pub(crate) const BUFFER_SIZE: usize = 256;
pub(crate) const BUFFER_SIZE_32_BITS: usize = 4;
pub(crate) const BUFFER_SIZE_64_BITS: usize = 8;
pub(crate) const BUFFER_SIZE_FLUSHING: usize = 100000;


// From v_counter after reset if there is no hardware error (found experimentally, no idea why)
pub(crate) const FRESH_NIBBLES_AFTER_RESET: i32 = 8188;
