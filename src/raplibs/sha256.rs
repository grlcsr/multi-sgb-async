#![allow(dead_code)]
use super::{ftdi_wrapper::FtdiBoard, settings::RunSettings, RapLibErrors};

#[derive(Debug)]
#[repr(u32)]
enum ShaAcceleratorStatus {
    Ok = 0,
    ErrorFifo = 1,
    OkFifo = 2,
    Sha256Timeout = 3,
    Sha256Error = 4,
    Sha256Special = 5,
    ResetOk = 6,
    ResetError = 7,
    ResetTimeout = 8,
    CompError = 9,
}

pub fn perform_accelerator_initialization(device: &FtdiBoard) -> Result<(), RapLibErrors> {
    req_init_self_test_sha256(device)?;
    let status: u32 = device.read_32_bit_u32()?;
    let fifo_status: u32 = status >> 4;
    let sha256_selftest: u32 = status & 0xf;

    if fifo_status == ShaAcceleratorStatus::OkFifo as u32
        && sha256_selftest == ShaAcceleratorStatus::Ok as u32
    {
        println!("SHA256: Initialization and self test passed.");
        Ok(())
    } else {
        let msg: String = format!(
            "SHA256 initialization and self test failed. Fifo_status = {:}; SHA256_selftest = {:}",
            fifo_status, sha256_selftest
        );
        Err(RapLibErrors::StreamerError(msg))
    }
}

fn req_init_self_test_sha256(device: &FtdiBoard) -> Result<usize, RapLibErrors> {
    let cmd: u8 = 0;
    let value: u16 = 0;
    crate::raplibs::base::write_pack(device, cmd, value)
}

pub fn req_read_sha256_fifo(device: &FtdiBoard) -> Result<usize, RapLibErrors> {
    let cmd: u8 = 3;
    let value: u16 = 0;
    crate::raplibs::base::write_pack(device, cmd, value)
}

pub fn set_reduction_ratio(
    device: &FtdiBoard,
    run_settings: RunSettings,
) -> Result<(), RapLibErrors> {
    let cmd: u8 = 2;
    let value: u16 = (run_settings.get_sha256_reduction_ratio() / 2) - 1;

    crate::raplibs::base::write_pack(device, cmd, value)?;
    let result: u32 = device.read_32_bit_u32()?;

    println!("Reduction ratio setting result value: {:}", result);
    Ok(())
}
