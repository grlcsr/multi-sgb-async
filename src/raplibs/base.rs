use super::RapLibErrors;
use super::settings::RunSettings;
use super::write_commands::WriteCommands;
use super::ftdi_wrapper::FtdiBoard;

pub const SOFTWARE_VERSION: u32 = 23061401;
pub const MIN_SUPPORTED_FIRMWARE_VERSION: u32 = 23060802;
pub const CHECK_VALUE: u32 = 0xabcd1234;

pub fn check_board_communication(device: &mut FtdiBoard) -> Result<(), RapLibErrors> {
    let cmd: u8 = 7;
    let value: u16 = 0x1234;
    let _ = device.write_pack(cmd, value)?;

    let check_value: u32 = device.read_32_bit_u32()?;
    let fw_version: u32 = device.read_32_bit_u32()?;

    if check_value == CHECK_VALUE && fw_version >= MIN_SUPPORTED_FIRMWARE_VERSION {
        println!("Communication OK: got value {:#010x}", check_value);
        println!("Firmware {:?} supported: minimum version required: {:?}.", fw_version, MIN_SUPPORTED_FIRMWARE_VERSION);
        Ok(())
    } else if check_value != CHECK_VALUE {
        panic!("Communication NOT OK: received check_value: {:#010x} expected value: {:#010x}", check_value, CHECK_VALUE);
    } else {
        panic!("Firmware {:?} NOT SUPPORTED: minimum version required: {:?}.", fw_version, MIN_SUPPORTED_FIRMWARE_VERSION);
    }
}

pub fn initialize_sipm_parameters(device: &mut FtdiBoard, hv_val: f32, dac_val: u32) -> Result<(), RapLibErrors> {
    let _ = set_hvdac(device, hv_val)?;
    let _ = set_thdac(device, dac_val)?;
    Ok(())
}

pub fn open_with_serial(serial_number: &str) -> Result<FtdiBoard, RapLibErrors> {
    Ok(FtdiBoard::open_with_serial(serial_number)?)
}

fn set_hvdac(device: &mut FtdiBoard, hv_val: f32) -> Result<usize, RapLibErrors> {
    let cmd: u8 = WriteCommands::SetHVDac as u8;
    // Conversion of hv value formula
    let value: u16 = (1534.6 + -26.23 * f32::min(hv_val, 58.2)) as u16;
    if value > 0 {
        Ok(device.write(cmd, value)?)
    } else {
        panic!("HV Value too small: {:?}", value);
    }
}

pub fn set_tdc_time_threshold(device: &mut FtdiBoard, afp_threshold: u16) -> Result<usize, RapLibErrors> {
    let cmd: u8 = WriteCommands::SetTDCTimeThreshold as u8;
    let value: u16 = afp_threshold;
    Ok(device.write(cmd, value)?)
}

fn set_thdac(device: &mut FtdiBoard, dac_val: u32) -> Result<usize, RapLibErrors> {
    let cmd: u8 = WriteCommands::SetThDac as u8;
    let value: u16 = dac_val as u16;
    Ok(device.write(cmd, value)?)
}

pub fn stop(device: &mut FtdiBoard) -> Result<usize, RapLibErrors> {
    let cmd: u8 = WriteCommands::ReqStop as u8;
    let value: u16 = 0;
    Ok(device.write(cmd, value)?)
}