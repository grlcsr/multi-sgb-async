use libftd2xx::FtStatus;

#[derive(Debug)]
#[repr(u32)]
pub enum FtdiBoardStatus {
    InvalidHandle = 1,
    DeviceNotFound = 2,
    DeviceNotOpened = 3,
    IoError = 4,
    InsufficientResources = 5,
    InvalidParameter = 6,
    InvalidBaudRate = 7,
    DeviceNotOpenedForErase = 8,
    DeviceNotOpenedForWrite = 9,
    FailedToWriteDevice = 10,
    EepromReadFailed = 11,
    EepromWriteFailed = 12,
    EepromEraseFailed = 13,
    EepromNotPresent = 14,
    EepromNotProgrammed = 15,
    InvalidArgs = 16,
    NotSupported = 17,
    OtherError = 18,
    DeviceListNotReady = 19,
}

impl From<FtStatus> for FtdiBoardStatus {
    fn from(x: FtStatus) -> FtdiBoardStatus {
        match x {
            //FtStatus::OK => panic!("OK is not an error status"),
            FtStatus::INVALID_HANDLE => FtdiBoardStatus::InvalidHandle,
            FtStatus::DEVICE_NOT_FOUND => FtdiBoardStatus::DeviceNotFound,
            FtStatus::DEVICE_NOT_OPENED => FtdiBoardStatus::DeviceNotOpened,
            FtStatus::IO_ERROR => FtdiBoardStatus::IoError,
            FtStatus::INSUFFICIENT_RESOURCES => FtdiBoardStatus::InsufficientResources,
            FtStatus::INVALID_PARAMETER => FtdiBoardStatus::InvalidParameter,
            FtStatus::INVALID_BAUD_RATE => FtdiBoardStatus::InvalidBaudRate,
            FtStatus::DEVICE_NOT_OPENED_FOR_ERASE => FtdiBoardStatus::DeviceNotOpenedForErase,
            FtStatus::DEVICE_NOT_OPENED_FOR_WRITE => FtdiBoardStatus::DeviceNotOpenedForWrite,
            FtStatus::FAILED_TO_WRITE_DEVICE => FtdiBoardStatus::FailedToWriteDevice,
            FtStatus::EEPROM_READ_FAILED => FtdiBoardStatus::EepromReadFailed,
            FtStatus::EEPROM_WRITE_FAILED => FtdiBoardStatus::EepromWriteFailed,
            FtStatus::EEPROM_ERASE_FAILED => FtdiBoardStatus::EepromEraseFailed,
            FtStatus::EEPROM_NOT_PRESENT => FtdiBoardStatus::EepromNotPresent,
            FtStatus::EEPROM_NOT_PROGRAMMED => FtdiBoardStatus::EepromNotProgrammed,
            FtStatus::INVALID_ARGS => FtdiBoardStatus::InvalidArgs,
            FtStatus::NOT_SUPPORTED => FtdiBoardStatus::NotSupported,
            FtStatus::OTHER_ERROR => FtdiBoardStatus::OtherError,
            FtStatus::DEVICE_LIST_NOT_READY => FtdiBoardStatus::DeviceListNotReady,
            //_ => panic!("{x}"),
        }
    }
}
