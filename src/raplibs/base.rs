use super::{
    ftdi_wrapper::FtdiBoard, write_commands::WriteCommands, RapLibErrors, CHECK_VALUE,
    MIN_SUPPORTED_FIRMWARE_VERSION, SOFTWARE_VERSION,
};

pub fn check_board_communication(device: &mut FtdiBoard) -> Result<(), RapLibErrors> {
    let cmd: u8 = 7;
    let value: u16 = 0x1234;
    let _ = write_pack(device, cmd, value)?;

    let check_value: u32 = device.read_32_bit_u32()?;
    let fw_version: u32 = device.read_32_bit_u32()?;

    if check_value == CHECK_VALUE && fw_version >= MIN_SUPPORTED_FIRMWARE_VERSION {
        println!("Communication OK: got value {:#010x}", check_value);
        println!(
            "Firmware {} supported: minimum version required: {}, software version: {}.",
            fw_version, MIN_SUPPORTED_FIRMWARE_VERSION, SOFTWARE_VERSION
        );
        Ok(())
    } else if check_value != CHECK_VALUE {
        Err(RapLibErrors::StreamerError(format!(
            "Communication NOT OK: received check_value: {:#010x} expected value: {:#010x}",
            check_value, CHECK_VALUE
        )))
    } else {
        Err(RapLibErrors::StreamerError(format!(
            "Firmware {:?} NOT SUPPORTED: minimum version required: {}.",
            fw_version, MIN_SUPPORTED_FIRMWARE_VERSION
        )))
    }
}

pub fn close(device: &mut FtdiBoard) -> Result<(), RapLibErrors> {
    Ok(device.close()?)
}

pub fn hv_compensate(temperature_now: f32, hv_val: f32, ref_temp: f32) -> f32 {
    hv_val + (temperature_now - ref_temp) * 0.054
}

pub fn initialize_sipm_parameters(
    device: &mut FtdiBoard,
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

pub fn request_raw_tdc_words(device: &mut FtdiBoard, value: u16) -> Result<usize, RapLibErrors> {
    let cmd: u8 = 6;
    write_pack(device, cmd, value)
}

pub fn req_read_dcr(device: &mut FtdiBoard) -> Result<usize, RapLibErrors> {
    let cmd: u8 = WriteCommands::ReqReadDCR.into();
    let value: u16 = 0;
    Ok(device.write(cmd, value)?)
}

pub fn req_temperature(device: &mut FtdiBoard) -> Result<f32, RapLibErrors> {
    let cmd: u8 = WriteCommands::ReqTemperature.into();
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
        Err(RapLibErrors::StreamerError(msg))
    } else {
        Ok(temperature)
    }
}

pub fn reset_fail_flag_latch(device: &mut FtdiBoard) -> Result<usize, RapLibErrors> {
    let cmd: u8 = WriteCommands::ResetFailFlagLatch.into();
    let value: u16 = 1;
    Ok(device.write(cmd, value)?)
}

pub fn reset_rap_values(
    device: &mut FtdiBoard,
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

pub fn set_gate_dcr(device: &mut FtdiBoard, value: u16) -> Result<usize, RapLibErrors> {
    let cmd: u8 = WriteCommands::SetGateDCR.into();
    Ok(device.write(cmd, value)?)
}

pub fn set_hvdac(device: &mut FtdiBoard, hv_val: f32) -> Result<usize, RapLibErrors> {
    let cmd: u8 = WriteCommands::SetHVDac.into();
    // Conversion of hv value formula
    let value: u16 = (1534.6 + -26.23 * f32::min(hv_val, 58.2)) as u16;
    if value > 0 {
        Ok(device.write(cmd, value)?)
    } else {
        Err(RapLibErrors::StreamerError(format!(
            "HV Value too small: {:?}",
            value
        )))
    }
}

pub fn set_tdc_time_threshold(
    device: &mut FtdiBoard,
    afp_threshold: u16,
) -> Result<usize, RapLibErrors> {
    let cmd: u8 = WriteCommands::SetTDCTimeThreshold.into();
    let value: u16 = afp_threshold;
    Ok(device.write(cmd, value)?)
}

fn set_thdac(device: &mut FtdiBoard, dac_val: u32) -> Result<usize, RapLibErrors> {
    let cmd: u8 = WriteCommands::SetThDac.into();
    let value: u16 = dac_val as u16;
    Ok(device.write(cmd, value)?)
}

pub fn stop(device: &mut FtdiBoard) -> Result<usize, RapLibErrors> {
    let cmd: u8 = WriteCommands::ReqStop.into();
    let value: u16 = 0;
    Ok(device.write(cmd, value)?)
}

pub fn write_pack(device: &mut FtdiBoard, cmd: u8, value: u16) -> Result<usize, RapLibErrors> {
    let cmd1: u8 = WriteCommands::ReqWritePackFirst.into();
    let cmd2: u8 = WriteCommands::ReqWritePackSecond.into();

    let value1 = cmd as u16;

    device.write(cmd1, value1)?;
    Ok(device.write(cmd2, value)?)
}
