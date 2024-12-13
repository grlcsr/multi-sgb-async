#[derive(Debug, Clone, Copy)]
pub struct FlashData {
    hv_val: f32,
    dac: u32,
    ref_temp: f32,
}

impl Default for FlashData {
    fn default() -> Self {
        Self::new(0.0, 0, 0.0)
    }
}

impl FlashData {
    pub fn new(hv_val: f32, dac: u32, ref_temp: f32) -> Self {
        Self {
            hv_val,
            dac,
            ref_temp
        }
    }

    pub fn get_hv(&self) -> f32 {
        self.hv_val
    }

    pub fn set_hv(&mut self, val: f32) {
        self.hv_val = val;
    }

    pub fn get_dac(&self) -> u32 {
        self.dac
    }

    pub fn set_dac(&mut self, val: u32) {
        self.dac = val;
    }

    pub fn get_ref_temp(&self) -> f32 {
        self.ref_temp
    }

    pub fn set_ref_temp(&mut self, val: f32) {
        self.ref_temp = val;
    }
}