use super::settings::RunSettings;
use super::RapLibErrors;
use super::ftdi_wrapper::FtdiBoard;
use super::write_commands::WriteCommands;

use lazy_static::lazy_static;

lazy_static! {
    static ref FRACTIONAL_PART_SIZE: i32 = 10;
    pub static ref SF_FXP: f32 = 2.0_f32.powf(-10.0); // Should be -FRACTIONAL_PART_SIZE but not implemented in rust
}

pub fn update_fpga_settings(device: &FtdiBoard, run_settings: RunSettings) -> Result<(), RapLibErrors> {
    set_operation_mode(device, 0)?;
    set_report_mode(device, 1)?;
    set_sequence_length_power_of_2(device, run_settings.get_mono_sequence_length_power_of_2())?;
    set_num_of_sequences_power_of_2(device, run_settings.get_mono_num_of_sequences_power_of_2())?;
    set_confidence_level_upper(device, run_settings.get_mono_confidence_level_upper())?;
    set_confidence_level_lower(device, run_settings.get_mono_confidence_level_lower())?;
    set_fail_flag_latch_event_alarm_thr(device, run_settings.get_sanity_fail_flag_latch_event_alarm_thr())?;
    set_sequence_length_runs(device, run_settings.get_runs_sequence_length())?;
    set_num_of_sequences_power_of_2_runs(device, run_settings.get_runs_num_of_sequences_power_of_2())?;
    set_confidence_level_runs(device, run_settings.get_runs_confidence_level())?;
    set_asym_nos(device, run_settings.get_asym_sequence_length_bits() / 4)?;
    
    Ok(())
}

fn set_operation_mode(device: &FtdiBoard, value: u16) -> Result<usize, RapLibErrors> {
    let cmd: u8 = WriteCommands::SetOperationMode as u8;
    Ok(device.write(cmd, value)?)
}

fn set_report_mode(device: &FtdiBoard, value: u16) -> Result<usize, RapLibErrors> {
    let cmd: u8 = WriteCommands::SetReportMode as u8;
    Ok(device.write(cmd, value)?)
}

fn set_sequence_length_power_of_2(device: &FtdiBoard, value: u16) -> Result<usize, RapLibErrors> {
    let cmd: u8 = WriteCommands::SetSequenceLengthPowerOf2 as u8;
    Ok(device.write(cmd, value)?)
}

fn set_num_of_sequences_power_of_2(device: &FtdiBoard, value: u16) -> Result<usize, RapLibErrors> {
    let cmd: u8 = WriteCommands::SetNumOfSequencesPowerOf2 as u8;
    Ok(device.write(cmd, value)?)
}

fn set_confidence_level_upper(device: &FtdiBoard, value: u16) -> Result<usize, RapLibErrors> {
    let cmd: u8 = WriteCommands::SetConfidenceLevelUpper as u8;
    let value_adjusted: u16 = (value as f32 / *SF_FXP) as u16;
    Ok(device.write(cmd, value_adjusted)?)
}

fn set_confidence_level_lower(device: &FtdiBoard, value: u16) -> Result<usize, RapLibErrors> {
    let cmd: u8 = WriteCommands::SetConfidenceLevelLower as u8;
    let value_adjusted: u16 = (value as f32 / *SF_FXP) as u16;
    Ok(device.write(cmd, value_adjusted)?)
}

fn set_fail_flag_latch_event_alarm_thr(device: &FtdiBoard, value: u16) -> Result<usize, RapLibErrors> {
    let cmd: u8 = WriteCommands::SetFailFlagLatchEventAlarmThr as u8;
    Ok(device.write(cmd, value)?)
}

fn set_sequence_length_runs(device: &FtdiBoard, value: u16) -> Result<usize, RapLibErrors> {
    let cmd: u8 = WriteCommands::SetSequenceLengthRuns as u8;
    let value_adjusted: u16 = value - 5;
    Ok(device.write(cmd, value_adjusted)?)
}

fn set_num_of_sequences_power_of_2_runs(device: &FtdiBoard, value: u16) -> Result<usize, RapLibErrors> {
    let cmd: u8 = WriteCommands::SetNumOfSequencesPowerOf2Runs as u8;
    Ok(device.write(cmd, value)?)
}

fn set_confidence_level_runs(device: &FtdiBoard, value: u16) -> Result<usize, RapLibErrors> {
    let cmd: u8 = WriteCommands::SetConfidenceLevelRuns as u8;
    Ok(device.write(cmd, value)?)
}

fn set_asym_nos(device: &FtdiBoard, value: u16) -> Result<usize, RapLibErrors> {
    let cmd: u8 = 8;
    crate::raplibs::base::write_pack(device, cmd, value)
}

pub fn req_read_asym_fifo(device: &FtdiBoard) -> Result<usize, RapLibErrors> {
    let cmd: u8 = 9;
    let value: u16 = 0;
    crate::raplibs::base::write_pack(device, cmd, value)
}

pub fn req_read_monobit_fifo(device: &FtdiBoard) -> Result<usize, RapLibErrors> {
    let cmd: u8 = WriteCommands::ReqReadMonoFifo as u8;
    let value: u16 = 0;
    Ok(device.write(cmd, value)?)
}

pub fn req_read_runs_fifo(device: &FtdiBoard) -> Result<usize, RapLibErrors> {
    let cmd: u8 = WriteCommands::ReqReadRunsZValFlag as u8;
    let value: u16 = 0;
    Ok(device.write(cmd, value)?)
}

pub fn req_read_runs_stats(device: &FtdiBoard) -> Result<usize, RapLibErrors> {
    let cmd: u8 = WriteCommands::ReqReadRunsFlagLatches as u8;
    let value: u16 = 0;
    Ok(device.write(cmd, value)?)
}

pub fn signed_int_to_dec(value: u32) -> i32 {
    let max_positive_value = i32::pow(2, 29) - 1;
    if value as i32 > max_positive_value {
        value as i32 - max_positive_value * 2 - 2
    } else {
        value as i32
    }
}

pub fn fxp_to_flp_smpl(num: i32, e_pos: f32) -> f32 {
    if (num >> 24) == 0 {
        num as f32 / f32::powf(2.0, e_pos)
    } else {
        -1.0 * ((!num + 1) & 0xffffff) as f32 / f32::powf(2.0, e_pos)
    }
}

pub fn fixed_to_float(fixed_input_num: u64, total_bits: i32, fractional_bits: i32) -> f64 {
    let tmp_u64: u64 = 1;

    // Handle negative numbers
    let sign_bit: u64 = (fixed_input_num >> (total_bits - 1)) & 1;
    let fixed_num: i64 = if sign_bit == 1 {
        let sbtrct: i64 = (tmp_u64 << total_bits) as i64;
        fixed_input_num as i64 - sbtrct
    } else {
        fixed_input_num as i64
    };

    let integer_part: f64 = (fixed_num >> fractional_bits) as f64;
    let fractional_part: f64 = (fixed_num as u64 & ((tmp_u64 << fractional_bits) - 1)) as f64;

    // Convert the integer and fractional parts to floating-point
    let floating_point_num: f64 = integer_part + fractional_part / ((tmp_u64 << fractional_bits) as f64);
    floating_point_num
}