use core::time::Duration;
use libftd2xx::{BitMode, DeviceStatus, FtStatus, Ftdi, FtdiCommon};
use std::sync::{Arc, Mutex, MutexGuard};

#[derive(Debug)]
pub struct FtdiBoard {
    device: Option<Arc<Mutex<Ftdi>>>,
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
                device: Some(Arc::new(Mutex::new(t))),
            },
        }
    }

    pub fn clean_buffer(&self) -> Result<(), FtdiBoardStatus> {
        Ok(self.get_device().purge_all()?)
    }

    pub fn get_queue_status(&self) -> Result<usize, FtdiBoardStatus> {
        Ok(self.get_device().queue_status()?)
    }

    pub fn get_status(&self) -> Result<DeviceStatus, FtdiBoardStatus> {
        Ok(self.get_device().status()?)
    }

    pub fn open_with_serial(serial_number: &str) -> Result<FtdiBoard, FtdiBoardStatus> {
        let board: FtdiBoard = FtdiBoard::new(Some(Ftdi::with_serial_number(serial_number)?));

        board.device_setup()?;
        board.clean_buffer()?;

        Ok(board)
    }

    pub fn read(&self, buf: &mut [u8]) -> Result<usize, FtdiBoardStatus> {
        Ok(self.get_device().read(buf)?)
    }

    pub fn read_32_bit_u32(&self) -> Result<u32, FtdiBoardStatus> {
        let mut buf_32b_u32: [u8; 4] = [0; 4];
        let _: usize = self.read(&mut buf_32b_u32)?;
        Ok(u32::from_be_bytes(buf_32b_u32))
    }

    pub fn write(&self, cmd: u8, value: u16) -> Result<usize, FtdiBoardStatus> {
        let mut tdc_command: [u8; 4] = [0; 4];
        tdc_command[0] = 0xa5;
        tdc_command[3] = cmd;

        let value_u8: [u8; 2] = value.to_be_bytes();
        tdc_command[1..3].copy_from_slice(&value_u8[..]);

        //println!("TDC command: {:?}", tdc_command);

        Ok(self.get_device().write(&tdc_command)?)
    }

    fn device_setup(&self) -> Result<(), FtdiBoardStatus> {
        self.get_device().reset()?;
        self.get_device().set_bit_mode(0xff, BitMode::from(0x00))?;
        self.get_device().set_bit_mode(0xff, BitMode::from(0x40))?;
        self.get_device().set_flow_control_rts_cts()?;
        self.get_device()
            .set_timeouts(Duration::from_millis(250), Duration::from_millis(250))?;
        Ok(())
    }

    fn get_device(&self) -> MutexGuard<'_, Ftdi> {
        match &self.device {
            Some(arc_mutex) => arc_mutex.as_ref().lock().expect("Failed to lock device."),
            None => panic!("Unhandled error: no device initialized!!"),
        }
    }
}

impl Clone for FtdiBoard {
    fn clone(&self) -> Self {
        FtdiBoard {
            device: self.device.as_ref().map(Arc::clone),
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
