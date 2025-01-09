use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};
use tokio_stream::{Stream, StreamExt};

use super::global_data::*;
use super::FtdiBoard;
use crate::raplibs::{base, flash::FlashData, sanity_checks, settings::RunSettings, sha256};

// TODO: HANDLING OF ERRORS -> PROPAGATE BACK TO MOD.RS AND IN CASE OF ERROR SHUT DOWN STREAM

#[derive(Debug)]
pub struct StreamResult {
    pub buf: Vec<u8>,
    pub bytes_read: usize,
}

#[derive(Debug, Clone)]
pub struct SGBStreamer {
    board: FtdiBoard,
    flash_default: FlashData,
    flash_calib: FlashData,
    run_settings_local: RunSettings,

    last_poll_time: Instant,
    delay: Duration,
    timeout: Duration,

    read_32_bits_stream: bool,

    flushing: bool,
}

impl Default for SGBStreamer {
    fn default() -> Self {
        Self {
            board: FtdiBoard::default(),
            flash_default: FlashData::default(),
            flash_calib: FlashData::default(),
            run_settings_local: RunSettings::default(),

            timeout: Duration::from_secs(1),
            last_poll_time: Instant::now(),
            delay: Duration::from_millis(1),
            read_32_bits_stream: false,
            flushing: false,
        }
    }
}

impl SGBStreamer {
    pub fn new(serial_number: &str) -> Self {
        Self {
            board: base::open_with_serial(serial_number).unwrap(),
            run_settings_local: RunSettings::get_run_settings()
                .expect("Panic initializing DeviceStream: cannot get runsettings.")
                .clone(),
            ..Default::default()
        }
    }

    pub fn flush_device(&mut self) {
        self.set_timeout(Duration::from_secs(1));
        self.set_last_poll_time();
        self.set_flushing(true);
    }

    pub fn get_flash_calib(&self) -> FlashData {
        self.flash_calib
    }

    pub fn initialize_board(&mut self) {
        let device = &mut self.board;
        base::check_board_communication(device);

        let hv_val = self.flash_default.get_hv();
        let dac = self.flash_default.get_dac();
        base::initialize_sipm_parameters(device, hv_val, dac);

        let afp_threshold: u16 = self.run_settings_local.get_afp_threshold();
        base::set_tdc_time_threshold(device, afp_threshold);
    }

    pub fn initialize_flash(&mut self) {
        let board: &FtdiBoard = &self.board;
        let flash_data = FlashData::get_flash_info(board).expect("Error decoding Flash data.");
        self.flash_default = flash_data;
        self.flash_calib = flash_data;

        println!("{:?}", self.flash_default);
    }

    pub fn is_flushing(&self) -> bool {
        self.flushing
    }

    pub fn is_read_32_bits_stream(&self) -> bool {
        self.read_32_bits_stream
    }

    pub fn req_temperature(&mut self) -> f32 {
        base::req_temperature(&mut self.board).unwrap()
    }

    pub fn reset_rap_values(&mut self, reset_tdc: bool, reset_mono: bool, reset_sha256: bool) {
        base::reset_rap_values(&mut self.board, reset_tdc, reset_mono, reset_sha256);
    }

    pub fn set_gate_dcr(&mut self) {
        // read the DCR; 2 = 1 second gate for pulse counting
        //               1 = 10 seconds
        let value = 1;
        base::set_gate_dcr(&mut self.board, value);
    }

    pub fn set_last_poll_time(&mut self) {
        self.last_poll_time = Instant::now();
    }

    pub fn set_read_32_bits_stream(&mut self, val: bool) {
        self.read_32_bits_stream = val;
    }

    pub fn set_delay(&mut self, delay: Duration) {
        self.delay = delay;
    }

    pub fn set_flushing(&mut self, val: bool) {
        self.flushing = val;
    }

    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }

    pub fn stop_device(&mut self) {
        base::stop(&mut self.board).unwrap();
    }

    pub fn write_pack(&mut self, cmd: u8, value: u16) {
        base::write_pack(&mut self.board, cmd, value);
    }

    pub fn write_run_settings_to_device(&mut self) {
        let afp_threshold: u16 = self.run_settings_local.get_afp_threshold();

        sanity_checks::update_fpga_settings(&mut self.board, self.run_settings_local);
        sha256::perform_accelerator_initialization(&mut self.board);
        sha256::set_reduction_ratio(&mut self.board, self.run_settings_local);
        base::set_tdc_time_threshold(&mut self.board, afp_threshold);
        base::reset_fail_flag_latch(&mut self.board);
    }
}

impl Stream for SGBStreamer {
    type Item = StreamResult;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if Instant::now().duration_since(self.last_poll_time) > self.timeout {
            println!("Timeout exceeded!!");
            return Poll::Ready(None);
        }

        /*
        TODO: As fo now just read whatevre, for the future create an enum that decides what kind of data to read so we can
        have correct buffer sizes for raw data stream
        */

        let buf: Vec<u8>;
        let bytes_read: usize;

        if self.is_read_32_bits_stream() {
            let mut read_buf: [u8; BUFFER_SIZE_32_BITS] = [0; BUFFER_SIZE_32_BITS];
            bytes_read = self.board.read(&mut read_buf).unwrap();
            
            if bytes_read == 4 {
                buf = read_buf.to_vec();
                self.set_last_poll_time();
                return Poll::Ready(Some(StreamResult { buf, bytes_read }));
            }
        } else if self.is_flushing() && self.board.get_queue_status().unwrap() > 0 {
            let mut read_buf: [u8; BUFFER_SIZE_FLUSHING] = [0; BUFFER_SIZE_FLUSHING];

            bytes_read = self.board.read(&mut read_buf).unwrap();
            buf = read_buf[..bytes_read].to_vec();

            self.set_last_poll_time();
            return Poll::Ready(Some(StreamResult { buf, bytes_read }));
        } else if self.board.get_queue_status().unwrap() > 0x100 {
        }

        if self.delay > Duration::from_millis(0) {
            let waker = cx.waker().clone();
            let delay = self.delay;
            tokio::spawn(async move {
                tokio::time::sleep(delay).await;
                waker.wake();
            });
        }
        return Poll::Pending;
    }
}


#[derive(Debug, Clone)]
pub struct FlushDevice<'a> {
    board: &'a FtdiBoard,

    last_poll_time: Instant,
    timeout: Duration,
}

impl<'a> FlushDevice<'a> {
    pub fn new(board: &'a FtdiBoard, timeout: Duration) -> Self {
        Self {
            board,
            timeout,
            last_poll_time: Instant::now(),
        }
    }

    async fn flush_device(&mut self) -> usize {
        let mut total_cleaned_bytes: usize = 0;

        loop {
            match self.next().await { //TODO! Change to try next and return Result<usize, Err>
                Some(read_bytes) => total_cleaned_bytes += read_bytes,
                None => break,
            }
        }

        return total_cleaned_bytes;
    }

    fn set_last_poll_time(&mut self) {
        self.last_poll_time = Instant::now();
    }
}

impl<'a> Stream for FlushDevice<'a> {
    type Item = usize;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if Instant::now().duration_since(self.last_poll_time) > self.timeout {
            println!("Cleaning buffer: Timeout exceeded!!");
            return Poll::Ready(None);
        }

        if self.board.get_queue_status().unwrap() > 0 {
            let mut read_buf: [u8; BUFFER_SIZE_FLUSHING] = [0; BUFFER_SIZE_FLUSHING];

            let bytes_read = self.board.read(&mut read_buf).unwrap();

            self.set_last_poll_time();
            return Poll::Ready(Some(bytes_read));
        }

        return Poll::Pending;
    }
}