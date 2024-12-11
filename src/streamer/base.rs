use super::ftdi_wrapper::FtdiBoard;
use super::ftdi_wrapper::ft_status::FtdiBoardStatus;

use super::stream_reader::DeviceStream;

pub fn open_with_serial(serial_number: &str) -> Result<FtdiBoard, FtdiBoardStatus> {
    Ok(FtdiBoard::open_with_serial(serial_number)?)
}

pub fn open_with_idx(index: i32) -> Result<FtdiBoard, FtdiBoardStatus> {
    Ok(FtdiBoard::open_with_idx(index)?)
}

