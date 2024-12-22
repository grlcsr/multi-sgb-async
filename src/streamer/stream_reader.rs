use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};
use tokio_stream::Stream;

use super::global_data::*;
use super::FtdiBoard;
use crate::raplibs::settings::RunSettings;
use crate::raplibs::{base, flash::FlashData};

// TODO: HANDLING OF ERRORS -> PROPAGATE BACK TO MOD.RS AND IN CASE OF ERROR SHUT DOWN STREAM

#[derive(Debug)]
pub struct StreamResult {
    pub buf: Vec<u8>,
    pub bytes_read: usize,
}

#[derive(Debug, Clone)]
pub struct DeviceStream {
    board: FtdiBoard,
    flash_default: FlashData,
    flash_calib: FlashData,
    run_settings_local: RunSettings,

    last_poll_time: Instant,
    delay: Duration,
    timeout: Duration,

    read_32_bits_stream: bool,
}

impl Default for DeviceStream {
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
        }
    }
}

impl DeviceStream {
    pub fn new(serial_number: &str) -> Self {
        Self {
            board: base::open_with_serial(serial_number).unwrap(),
            run_settings_local: RunSettings::get_run_settings()
                .expect("Panic initializing DeviceStream: cannot get runsettings.")
                .clone(),
            ..Default::default()
        }
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

    pub fn is_read_32_bits_stream(&self) -> bool {
        self.read_32_bits_stream
    }

    pub fn reset_rap_values(&mut self, reset_tdc: bool, reset_mono: bool, reset_sha256: bool) {
        base::reset_rap_values(&mut self.board, reset_tdc, reset_mono, reset_sha256);
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

    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }

    pub fn stop_device(&mut self) {
        base::stop(&mut self.board).unwrap();
    }

    pub fn write_pack(&mut self, cmd: u8, value: u16) {
        base::write_pack(&mut self.board, cmd, value);
    }
}

impl Stream for DeviceStream {
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
            let mut read_buf: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];
            bytes_read = self.board.read(&mut read_buf).unwrap();
            buf = read_buf.to_vec();

            self.set_last_poll_time();
            return Poll::Ready(Some(StreamResult { buf, bytes_read }));
        } else if self.board.get_queue_status().unwrap() > 0 {
            let mut read_buf: [u8; BUFFER_SIZE_32_BITS] = [0; BUFFER_SIZE_32_BITS];
            println!("ee\t{:?}", self.board.get_status().unwrap());
            bytes_read = self.board.read(&mut read_buf).unwrap();
            buf = read_buf.to_vec();

            self.set_last_poll_time();
            return Poll::Ready(Some(StreamResult { buf, bytes_read }));
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
