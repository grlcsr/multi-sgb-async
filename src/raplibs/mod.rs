pub(crate) mod ftdi_wrapper;
pub(crate) mod base;
pub(crate) mod flash;
pub(crate) mod write_commands;
pub(crate) mod settings;

use std::fmt;
use std::io::Error;
use ftdi_wrapper::FtdiBoardStatus;

#[derive(Debug)]
pub enum RapLibErrors {
    FtdiStatus(FtdiBoardStatus),
    FlashError(String),
    BaseError(String),
    SanityChecksError(String),
    Sha256Error(String),
    RunSettingsError(String),

    UnhandledError(Error)
}

impl fmt::Display for RapLibErrors {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RapLibErrors::FtdiStatus(x) =>
                write!(f, "FTDI Error: {:?}", x),
            RapLibErrors::FlashError(x) =>
                write!(f, "Flash Error: {:?}", x),
            RapLibErrors::BaseError(x) =>
                write!(f, "Base Error: {:?}", x),
            RapLibErrors::SanityChecksError(x) =>
                write!(f, "Sanity Check Error: {:?}", x),
            RapLibErrors::Sha256Error(x) =>
                write!(f, "Sha256 Error: {:?}", x),
            RapLibErrors::RunSettingsError(x) =>
                write!(f, "Run Settings Error: {:?}", x),
            
            
            RapLibErrors::UnhandledError(x)=>
                write!(f, "Unhandled Error: origin unknown. Error code: {:?}", x),
        }
    }
}

impl From<Error> for RapLibErrors {
    fn from(err: Error) -> RapLibErrors {
        RapLibErrors::UnhandledError(err)
    }
}

impl From<FtdiBoardStatus> for RapLibErrors {
    fn from(err: FtdiBoardStatus) -> RapLibErrors {
        RapLibErrors::FtdiStatus(err)
    }
}