pub(crate) mod base;
pub(crate) mod flash;
pub(crate) mod ftdi_wrapper;
pub(crate) mod sanity_checks;
pub(crate) mod settings;
pub(crate) mod sha256;
pub(crate) mod write_commands;

use ftdi_wrapper::FtdiBoardStatus;
use std::error::Error;
use std::fmt;

pub const SOFTWARE_VERSION: u32 = 23061401;
pub const MIN_SUPPORTED_FIRMWARE_VERSION: u32 = 23060802;
pub const CHECK_VALUE: u32 = 0xabcd1234;

pub enum RapLibErrors {
    FtdiStatus(FtdiBoardStatus),
    SettingsError(String),
    StreamerError(String),
    UnhandledError(String),
}

impl Error for RapLibErrors {}

impl fmt::Debug for RapLibErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RapLibErrors::FtdiStatus(x) => format!("FTDI Error: {}", x).fmt(f),
            RapLibErrors::StreamerError(x) => format!("Streamer Error: {}", x).fmt(f),
            RapLibErrors::SettingsError(x) => format!("Settings Error: {}", x).fmt(f),
            RapLibErrors::UnhandledError(..) => "Unhandled External Error. Please restart.".fmt(f),
        }
    }
}

impl fmt::Display for RapLibErrors {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                RapLibErrors::FtdiStatus(x) => format!("FTDI Error: {}", x),
                RapLibErrors::StreamerError(x) => format!("Streamer Error: {}", x),
                RapLibErrors::SettingsError(x) => format!("Settings Error: {}", x),
                RapLibErrors::UnhandledError(..) =>
                    format!("Unhandled External Error. Please restart."),
            }
        )
    }
}

impl From<FtdiBoardStatus> for RapLibErrors {
    fn from(err: FtdiBoardStatus) -> RapLibErrors {
        RapLibErrors::FtdiStatus(err)
    }
}
