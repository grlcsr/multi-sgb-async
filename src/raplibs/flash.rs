use super::ftdi_wrapper::FtdiBoard;
use super::write_commands::WriteCommands;
use super::RapLibErrors;

const FLASH_SUCCESS: u32 = 0x00004F4B;
#[allow(dead_code)]
const FLASH_FAILURE: u32 = 0x00455252;
const FLASH_PAGESIZE: usize = 256;

#[derive(Default, Debug, Clone, Copy)]
pub struct FlashData {
    hv_val: f32,
    dac: u32,
    ref_temp: f32,
}

impl FlashData {
    pub fn new(hv_val: f32, dac: u32, ref_temp: f32) -> Self {
        Self {
            hv_val,
            dac,
            ref_temp,
        }
    }

    pub fn hv(&self) -> f32 {
        self.hv_val
    }

    pub fn set_hv(&mut self, val: f32) {
        self.hv_val = val;
    }

    pub fn dac(&self) -> u32 {
        self.dac
    }

    pub fn set_dac(&mut self, val: u32) {
        self.dac = val;
    }

    pub fn ref_temp(&self) -> f32 {
        self.ref_temp
    }

    pub fn set_ref_temp(&mut self, val: f32) {
        self.ref_temp = val;
    }

    pub fn get_flash_info(device: &FtdiBoard) -> Result<FlashData, RapLibErrors> {
        Self::inititialize_flash(device)?;
        let flash_data_page: [u8; FLASH_PAGESIZE] = Self::req_read_flash(device)?;
        Ok(Self::decode_flash_read_data(&flash_data_page))
    }

    fn decode_flash_read_data(read_data: &[u8]) -> FlashData {
        let mut tmp: [u8; 4] = [0; 4];

        tmp[1..].copy_from_slice(&read_data[17..20]);
        let hv_val: f32 = (u32::from_be_bytes(tmp) as f32) / 100.0;

        tmp[1..].copy_from_slice(&read_data[21..24]);
        let dac: u32 = u32::from_be_bytes(tmp);

        tmp[1..].copy_from_slice(&read_data[25..28]);
        let ref_temp: f32 = (u32::from_be_bytes(tmp) as f32) / 10.0;

        FlashData::new(hv_val, dac, ref_temp)
    }

    fn inititialize_flash(device: &FtdiBoard) -> Result<(), RapLibErrors> {
        let cmd: u8 = WriteCommands::ReqInitFlash.into();
        let val: u16 = 0;
        Self::read_write_cmd_value_and_validate(device, cmd, val)
    }

    fn read_write_cmd_value_and_validate(
        device: &FtdiBoard,
        cmd: u8,
        val: u16,
    ) -> Result<(), RapLibErrors> {
        device.write(cmd, val)?;
        let command: u8 = (device.read_32_bit_u32()?) as u8;

        if command == cmd {
            let status: u32 = device.read_32_bit_u32()?;

            if status == FLASH_SUCCESS {
                Ok(())
            } else {
                Err(RapLibErrors::StreamerError(
                    "FLASH communication: status mismatch.".to_string(),
                ))
            }
        } else {
            Err(RapLibErrors::StreamerError(
                "FLASH communication failed: cmd mismatch.".to_string(),
            ))
        }
    }

    fn req_read_flash(device: &FtdiBoard) -> Result<[u8; FLASH_PAGESIZE], RapLibErrors> {
        let cmd: u8 = WriteCommands::ReqReadFlash.into();
        let val: u16 = 0;
        let mut flash_data: [u8; FLASH_PAGESIZE] = [0; FLASH_PAGESIZE];

        match Self::read_write_cmd_value_and_validate(device, cmd, val) {
            Ok(_) => {
                let _ = device.read(&mut flash_data);
                Ok(flash_data)
            }
            Err(x) => Err(RapLibErrors::StreamerError(x.to_string())),
        }
    }
}
