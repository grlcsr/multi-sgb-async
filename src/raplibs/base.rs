use super::ftdi_wrapper::FtdiBoard;
use super::write_commands::WriteCommands;
use super::RapLibErrors;

pub const SOFTWARE_VERSION: u32 = 23061401;
pub const MIN_SUPPORTED_FIRMWARE_VERSION: u32 = 23060802;
pub const CHECK_VALUE: u32 = 0xabcd1234;

const REQ_WRITE_PACK_FIRST: u8 = 0xFE;
const REQ_WRITE_PACK_SECOND: u8 = 0xFF;

pub fn check_board_communication(device: &FtdiBoard) -> Result<(), RapLibErrors> {
    let cmd: u8 = 7;
    let value: u16 = 0x1234;
    let _ = write_pack(device, cmd, value)?;

    let check_value: u32 = device.read_32_bit_u32()?;
    let fw_version: u32 = device.read_32_bit_u32()?;

    if check_value == CHECK_VALUE && fw_version >= MIN_SUPPORTED_FIRMWARE_VERSION {
        println!("Communication OK: got value {:#010x}", check_value);
        println!(
            "Firmware {:?} supported: minimum version required: {:?}, software version: {:?}.",
            fw_version, MIN_SUPPORTED_FIRMWARE_VERSION, SOFTWARE_VERSION
        );
        Ok(())
    } else if check_value != CHECK_VALUE {
        Err(RapLibErrors::BaseError(format!(
            "Communication NOT OK: received check_value: {:#010x} expected value: {:#010x}",
            check_value, CHECK_VALUE
        )))
    } else {
        Err(RapLibErrors::BaseError(format!(
            "Firmware {:?} NOT SUPPORTED: minimum version required: {:?}.",
            fw_version, MIN_SUPPORTED_FIRMWARE_VERSION
        )))
    }
}

pub fn hv_compensate(temperature_now: f32, hv_val:f32, ref_temp: f32) -> f32 {
    hv_val + (temperature_now - ref_temp) * 0.054
}

pub fn initialize_sipm_parameters(
    device: &FtdiBoard,
    hv_val: f32,
    dac_val: u32,
) -> Result<(), RapLibErrors> {
    let _ = set_hvdac(device, hv_val)?;
    let _ = set_thdac(device, dac_val)?;
    Ok(())
}

pub fn open_with_serial(serial_number: &str) -> Result<FtdiBoard, RapLibErrors> {
    Ok(FtdiBoard::open_with_serial(serial_number)?)
}

pub fn request_raw_tdc_words(device: &FtdiBoard, value: u16) -> Result<usize, RapLibErrors> {
    let cmd: u8 = 6;
    write_pack(device, cmd, value)
}

pub fn req_read_dcr(device: &FtdiBoard) -> Result<usize, RapLibErrors> {
    let cmd: u8 = WriteCommands::ReqReadDCR as u8;
    let value: u16 = 0;
    Ok(device.write(cmd, value)?)
}

pub fn req_temperature(device: &FtdiBoard) -> Result<f32, RapLibErrors> {
    let cmd: u8 = WriteCommands::ReqTemperature as u8;
    let value: u16 = 0;
    device.write(cmd, value)?;
    let temperature_dac: u32 = device.read_32_bit_u32()?;

    let temperature: f32 = if temperature_dac <= 2048 {
        (128.0 / 2048.0) * temperature_dac as f32
    } else {
        -(128.0 / 2048.0) * (4096.0 - temperature_dac as f32)
    };

    if !(-20.0..=100.0).contains(&temperature) {
        let msg: String = format!("Read temperature error: temp measured = {:?}.", temperature);
        Err(RapLibErrors::BaseError(msg))
    } else {
        Ok(temperature)
    }
}

pub fn reset_fail_flag_latch(device: &FtdiBoard) -> Result<usize, RapLibErrors> {
    let cmd: u8 = WriteCommands::ResetFailFlagLatch as u8;
    let value: u16 = 1;
    Ok(device.write(cmd, value)?)
}

pub fn reset_rap_values(
    device: &FtdiBoard,
    reset_tdc: bool,
    reset_mono: bool,
    reset_sha256: bool,
) -> Result<usize, RapLibErrors> {
    let cmd: u8 = 5;
    let mut value: u16 = 0;
    if reset_sha256 {
        value += 1;
    }
    if reset_mono {
        value += 2;
    }
    if reset_tdc {
        value += 4;
    }
    write_pack(device, cmd, value)
}

pub fn set_gate_dcr(device: &FtdiBoard, value: u16) -> Result<usize, RapLibErrors> {
    let cmd: u8 = WriteCommands::SetGateDCR as u8;
    Ok(device.write(cmd, value)?)
}

pub fn set_hvdac(device: &FtdiBoard, hv_val: f32) -> Result<usize, RapLibErrors> {
    let cmd: u8 = WriteCommands::SetHVDac as u8;
    // Conversion of hv value formula
    let value: u16 = (1534.6 + -26.23 * f32::min(hv_val, 58.2)) as u16;
    if value > 0 {
        Ok(device.write(cmd, value)?)
    } else {
        Err(RapLibErrors::BaseError(format!(
            "HV Value too small: {:?}",
            value
        )))
    }
}

pub fn set_tdc_time_threshold(
    device: &FtdiBoard,
    afp_threshold: u16,
) -> Result<usize, RapLibErrors> {
    let cmd: u8 = WriteCommands::SetTDCTimeThreshold as u8;
    let value: u16 = afp_threshold;
    Ok(device.write(cmd, value)?)
}

fn set_thdac(device: &FtdiBoard, dac_val: u32) -> Result<usize, RapLibErrors> {
    let cmd: u8 = WriteCommands::SetThDac as u8;
    let value: u16 = dac_val as u16;
    Ok(device.write(cmd, value)?)
}

pub fn stop(device: &FtdiBoard) -> Result<usize, RapLibErrors> {
    let cmd: u8 = WriteCommands::ReqStop as u8;
    let value: u16 = 0;
    Ok(device.write(cmd, value)?)
}

pub fn write_pack(device: &FtdiBoard, cmd: u8, value: u16) -> Result<usize, RapLibErrors> {
    let cmd1: u8 = REQ_WRITE_PACK_FIRST;
    let cmd2: u8 = REQ_WRITE_PACK_SECOND;

    let value1 = cmd as u16;

    device.write(cmd1, value1)?;
    Ok(device.write(cmd2, value)?)
}
