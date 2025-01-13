pub(crate) mod ftdi_wrapper;
pub(crate) mod base;
pub(crate) mod flash;
pub(crate) mod sanity_checks;
pub(crate) mod write_commands;
pub(crate) mod settings;
pub(crate) mod sha256;

use std::fmt;
use std::error::Error;
use ftdi_wrapper::FtdiBoardStatus;

pub enum RapLibErrors {
    FtdiStatus(FtdiBoardStatus),
    FlashError(String),
    BaseError(String),
    Sha256Error(String),
    RunSettingsError(String),
    UnhandledError(String)
}

impl Error for RapLibErrors {}

impl fmt::Debug for RapLibErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RapLibErrors::FtdiStatus(x) =>
                format!("FTDI Error: {}", x).fmt(f),
            RapLibErrors::FlashError(x) =>
                format!("Flash Error: {}", x).fmt(f),
            RapLibErrors::BaseError(x) =>
                format!("Base Error: {}", x).fmt(f),
            RapLibErrors::Sha256Error(x) =>
                format!("Sha256 Error: {}", x).fmt(f),
            RapLibErrors::RunSettingsError(x) =>
                format!("Run Settings Error: {}", x).fmt(f),
            RapLibErrors::UnhandledError(..) => "Unhandled External Error. Please restart.".fmt(f),
        }
    }
}

impl fmt::Display for RapLibErrors {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write! (f, 
            "{}", 
            match self {
                RapLibErrors::FtdiStatus(x) => format!("FTDI Error: {}", x),
                RapLibErrors::FlashError(x) => format!("Flash Error: {}", x),
                RapLibErrors::BaseError(x) => format!("Base Error: {}", x),
                RapLibErrors::Sha256Error(x) => format!("Sha256 Error: {}", x),
                RapLibErrors::RunSettingsError(x) => format!("Run Settings Error: {}", x),
                RapLibErrors::UnhandledError(..) => format!("Unhandled External Error. Please restart."),
        })
    }
}

impl From<FtdiBoardStatus> for RapLibErrors {
    fn from(err: FtdiBoardStatus) -> RapLibErrors {
        RapLibErrors::FtdiStatus(err)
    }
}