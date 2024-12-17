use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};
use tokio_stream::Stream;

use super::global_data::BUFFER_SIZE;
use super::FtdiBoard;
use crate::raplibs::{base, flash::FlashData};

#[derive(Debug)]
pub struct StreamResult {
    pub buf: [u8; BUFFER_SIZE],
    pub bytes_read: usize,
}

#[derive(Debug, Clone)]
pub struct DeviceStream {
    board: FtdiBoard,
    flash_default: FlashData,
    flash_calib: FlashData,

    last_poll_time: Instant,
    delay: Duration,
    timeout: Duration,
}

impl Default for DeviceStream {
    fn default() -> Self {
        Self {
            board: FtdiBoard::default(),
            flash_default: FlashData::default(),
            flash_calib: FlashData::default(),

            timeout: Duration::from_secs(1),
            last_poll_time: Instant::now(),
            delay: Duration::from_millis(2),
        }
    }
}

impl DeviceStream {
    pub fn new(serial_number: &str) -> Self {
        Self {
            board: base::open_with_serial(serial_number).unwrap(),
            flash_default: FlashData::default(),
            flash_calib: FlashData::default(),

            timeout: Duration::from_secs(1),
            last_poll_time: Instant::now(),
            delay: Duration::from_millis(2),
        }
    }

    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }

    pub fn initialize_board(&mut self) {
        base::check_board_communication(&mut self.board).unwrap();

        let hv_val = self.flash_default.get_hv();
        let dac = self.flash_default.get_dac();
        base::initialize_sipm_parameters(&mut self.board, hv_val, dac).unwrap();
    }

    pub fn initialize_flash(&mut self) {
        let board: &FtdiBoard = &self.board;
        let flash_data = FlashData::get_flash_info(board).expect("Error decoding Flash data.");
        self.flash_default = flash_data;
        self.flash_calib = flash_data;

        println!("{:?}", self.flash_default);
    }

    pub fn stop_device(&mut self) {
        base::stop(&mut self.board).unwrap();
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

        if self.board.get_queue_status().unwrap() > 0 {
            let mut buf: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];
            let bytes_read = self.board.read(&mut buf).unwrap();
            self.last_poll_time = Instant::now();
            return Poll::Ready(Some(StreamResult { buf, bytes_read }));
        } else {
            let waker = cx.waker().clone();
            let delay = self.delay;
            tokio::spawn(async move {
                tokio::time::sleep(delay).await;
                waker.wake();
            });
            return Poll::Pending;
        }
    }
}
