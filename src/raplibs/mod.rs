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
    UnhandledError(String),
}

impl Error for RapLibErrors {}

impl fmt::Debug for RapLibErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RapLibErrors::FtdiStatus(x) =>
                write!(f, "FTDI Error: {}", x),
            RapLibErrors::FlashError(x) =>
                write!(f, "Flash Error: {}", x),
            RapLibErrors::BaseError(x) =>
                write!(f, "Base Error: {}", x),
            RapLibErrors::Sha256Error(x) =>
                write!(f, "Sha256 Error: {}", x),
            RapLibErrors::RunSettingsError(x) =>
                write!(f, "Run Settings Error: {}", x),
            RapLibErrors::UnhandledError(..) =>
                    write!(f, "Unhandled External Error. Please restart."),
        }
    }
}

impl fmt::Display for RapLibErrors {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RapLibErrors::FtdiStatus(x) =>
                write!(f, "FTDI Error: {}", x),
            RapLibErrors::FlashError(x) =>
                write!(f, "Flash Error: {}", x),
            RapLibErrors::BaseError(x) =>
                write!(f, "Base Error: {}", x),
            RapLibErrors::Sha256Error(x) =>
                write!(f, "Sha256 Error: {}", x),
            RapLibErrors::RunSettingsError(x) =>
                write!(f, "Run Settings Error: {}", x),
            RapLibErrors::UnhandledError(..) =>
                    write!(f, "Unhandled External Error. Please restart."),
        }
    }
}

impl From<FtdiBoardStatus> for RapLibErrors {
    fn from(err: FtdiBoardStatus) -> RapLibErrors {
        RapLibErrors::FtdiStatus(err)
    }
}
