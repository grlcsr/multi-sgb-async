pub mod ft_status;

use ft_status::*;
use core::time::Duration;
use libftd2xx::{BitMode, DeviceInfo, DeviceStatus, Ftdi, FtdiCommon};
use std::sync::{Arc, Mutex, MutexGuard};


#[derive(Debug)]
pub struct FtdiBoard  {
    device: Option<Arc<Mutex<Ftdi>>>,
}

impl Default for FtdiBoard {
    fn default() -> Self {
        Self::new(None)
    }
}

impl FtdiBoard {
    const REQ_WRITE_PACK_FIRST: u8 = 0xFE;
    const REQ_WRITE_PACK_SECOND: u8 = 0xFF;

    pub fn new(t: Option<Ftdi>) -> Self {        
        match t {
            None => Self {
                device: None
            },
            Some(t) => Self {
                device: Some(Arc::new(Mutex::new(t)))
            }
        }
        
    }

    pub fn clean_buffer(&mut self) -> Result<(), FtdiBoardStatus> {
        Ok(self.get_device().purge_all()?)
    }

    pub fn get_queue_status(&mut self) -> Result<usize, FtdiBoardStatus> {
        Ok(self.get_device().queue_status()?)
    }
    
    pub fn open_with_idx(index: i32) -> Result<FtdiBoard, FtdiBoardStatus> {
        let mut board: FtdiBoard = FtdiBoard::new(Some(Ftdi::with_index(index)?));

        board.device_setup()?;
        board.clean_buffer()?;
        
        //board.flush_device()?; -> remove from here: we need to create out first stream
        Ok(board)
    }

    pub fn open_with_serial(serial_number: &str) -> Result<FtdiBoard, FtdiBoardStatus> {
        let mut board: FtdiBoard = FtdiBoard::new(Some(Ftdi::with_serial_number(serial_number)?));

        board.device_setup()?;
        board.clean_buffer()?;

        //board.flush_device()?;
        Ok(board)
    }

    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, FtdiBoardStatus> {
        Ok(self.get_device().read(buf)?)
    }

    fn device_setup(&mut self) -> Result<(), FtdiBoardStatus> {
        self.get_device().reset()?;
        self.get_device().set_bit_mode(0xff, BitMode::from(0x00))?;
        self.get_device().set_bit_mode(0xff, BitMode::from(0x40))?;
        self.get_device().set_flow_control_rts_cts()?;
        self.get_device().set_timeouts(Duration::from_millis(250), Duration::from_millis(250))?;
        Ok(())
    }

    fn get_device(&mut self) -> MutexGuard<'_, Ftdi> {
        match &self.device {
            Some(arc_mutex) => {
                arc_mutex.as_ref().lock().expect("Failed to lock device.")
            }
            None => panic!("No device was found.")
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
