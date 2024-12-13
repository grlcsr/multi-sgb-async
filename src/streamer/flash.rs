#[derive(Debug, Clone, Copy)]
pub struct FlashData {
    pub hv_val: f32,
    pub dac: u32,
    pub ref_temp: f32,
}

impl Default for FlashData {
    fn default() -> Self {
        Self::new()
    }
}

impl FlashData {
    pub fn new() -> Self {
        Self {
            hv_val: 0.0,
            dac: 0,
            ref_temp: 0.0
        }
    }
}