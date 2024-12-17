#[allow(dead_code)]
#[repr(u8)]
pub enum WriteCommands {
    ReqStop = 0x00,
    SetHVDac = 0xA3,
    SetThDac = 0xA4,
    ReqTemperature = 0xA5,
    ReqReadDCR = 0xA6,
    SetGateDCR = 0xA8,
    ResetTDCFifo = 0xC2,
    SetTDCTimeThreshold = 0xC7,

    ReqInitFlash = 0xD0,
    ReqEraseFlash = 0xD1,
    ReqWriteFlash = 0xD2,
    ReqReadFlash = 0xD3,

    SetSequenceLengthRuns = 0xEA,
    SetFailFlagLatchEventAlarmThr = 0xEB,
    SetReportMode = 0xEC,
    SetSequenceLengthPowerOf2 = 0xED,
    SetNumOfSequencesPowerOf2 = 0xEE,
    SetConfidenceLevelUpper = 0xEF,
    SetConfidenceLevelLower = 0xF0,
    ResetFailFlagLatch = 0xF1,
    ReadFailFlagLatch = 0xF2,
    SetOperationMode = 0xF5,
    ReqReadMonoFifo = 0xF7,
    ReqReadRunsFlagLatches = 0xF9,
    ReqReadRunsZValFlag = 0xFA,
    SetConfidenceLevelRuns = 0xFB,
    SetNumOfSequencesPowerOf2Runs = 0xFC,

    ReqWritePackFirst = 0xFE,
    ReqWritePackSecond = 0xFF,
}

impl From<WriteCommands> for u8 {
    fn from(wc: WriteCommands) -> u8 {
        wc as u8
    }
}
