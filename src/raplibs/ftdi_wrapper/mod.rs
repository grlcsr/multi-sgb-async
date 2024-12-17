use core::time::Duration;
use libftd2xx::{
    list_devices as ftdi_ld, BitMode, DeviceInfo, DeviceStatus, FtStatus, Ftdi, FtdiCommon,
};
use std::fmt;

/*
    Returns the list of devices currently connected to the computer
*/
pub fn list_devices() -> Result<Vec<String>, FtdiBoardStatus> {
    Ok((ftdi_ld()?)
        .iter()
        .map(|device_info: &DeviceInfo| device_info.serial_number.clone())
        .collect())
}

#[derive(Debug)]
pub struct FtdiBoard {
    device: Option<Ftdi>,
}

impl Default for FtdiBoard {
    fn default() -> Self {
        Self::new(None)
    }
}

impl FtdiBoard {
    pub fn new(t: Option<Ftdi>) -> Self {
        match t {
            None => Self { device: None },
            Some(t) => Self {
                device: Some(t),
            },
        }
    }

    pub fn clean_buffer(&mut self) -> Result<(), FtdiBoardStatus> {
        Ok(self.get_device().purge_all()?)
    }

    pub fn close(&mut self) -> Result<(), FtdiBoardStatus> {
        Ok(self.get_device().close()?)
    }

    pub fn get_queue_status(&mut self) -> Result<usize, FtdiBoardStatus> {
        Ok(self.get_device().queue_status()?)
    }

    pub fn get_status(&mut self) -> Result<DeviceStatus, FtdiBoardStatus> {
        Ok(self.get_device().status()?)
    }

    pub fn open_with_serial(serial_number: &str) -> Result<FtdiBoard, FtdiBoardStatus> {
        let mut board: FtdiBoard = FtdiBoard::new(Some(Ftdi::with_serial_number(serial_number)?));

        board.device_setup()?;
        board.clean_buffer()?;

        Ok(board)
    }

    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, FtdiBoardStatus> {
        Ok(self.get_device().read(buf)?)
    }

    pub fn read_32_bit_u32(&mut self) -> Result<u32, FtdiBoardStatus> {
        let mut buf_32b_u32: [u8; 4] = [0; 4];
        let _: usize = self.read(&mut buf_32b_u32)?;
        Ok(u32::from_be_bytes(buf_32b_u32))
    }

    pub fn read_64_bit_u64(&mut self) -> Result<u64, FtdiBoardStatus> {
        let mut buf_64b_u64: [u8; 8] = [0; 8];
        let _: usize = self.read(&mut buf_64b_u64)?;
        Ok(u64::from_be_bytes(buf_64b_u64))
    }

    pub fn write(&mut self, cmd: u8, value: u16) -> Result<usize, FtdiBoardStatus> {
        let mut tdc_command: [u8; 4] = [0; 4];
        tdc_command[0] = 0xa5;
        tdc_command[3] = cmd;

        let value_u8: [u8; 2] = value.to_be_bytes();
        tdc_command[1..3].copy_from_slice(&value_u8[..]);

        // println!("TDC command: {:?}", tdc_command);

        Ok(self.get_device().write(&tdc_command)?)
    }

    fn device_setup(&mut self) -> Result<(), FtdiBoardStatus> {
        self.get_device().reset()?;
        self.get_device().set_bit_mode(0xff, BitMode::from(0x00))?;
        self.get_device().set_bit_mode(0xff, BitMode::from(0x40))?;
        self.get_device().set_flow_control_rts_cts()?;
        self.get_device()
            .set_timeouts(Duration::from_millis(250), Duration::from_millis(250))?;
        Ok(())
    }

    fn get_device(&mut self) -> &mut Ftdi {
        match &mut self.device {
            Some(dev) => dev,
            None => panic!("Unhandled error: no device initialized!!"),
        }
    }
}

#[derive(Debug)]
pub struct FtdiBoardStatus {
    err: String,
}

impl From<FtStatus> for FtdiBoardStatus {
    fn from(x: FtStatus) -> FtdiBoardStatus {
        FtdiBoardStatus { err: x.to_string() }
    }
}

impl std::fmt::Display for FtdiBoardStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error code: {}", self.err)
    }
}
