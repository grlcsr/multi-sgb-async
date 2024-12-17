pub(crate) mod global_data;
pub(crate) mod stream_reader;

use std::pin::Pin;
use std::future::Future;
use std::time::Duration;
use tokio_stream::Stream;
use std::task::{Context, Poll};

use super::raplibs::ftdi_wrapper::FtdiBoard;
use super::raplibs::{base, flash::FlashData};
use stream_reader::{DeviceStream, StreamResult};

enum StreamerState {
    OpenConnection,
    ReadFlash,
    PrepareInitialization,
    Initalize,
    TempStabilization,
    ReadStream,
    ReadTests,
    TempCompensation,
    Termination,
}

pub struct SGBStreamer<'a, 'b> {
    state: StreamerState,
    serial: String,

    board: &'a mut FtdiBoard,
    rx_stream: &'b mut DeviceStream,

    flash_default: FlashData,
    flash_calib: FlashData,

    total_streamed_bytes: usize,
    flushing: bool,
}

impl<'a, 'b> SGBStreamer<'a, 'b> {
    pub fn new(
        serial: &'static str,
        board: &'a mut FtdiBoard,
        rx_stream: &'b mut DeviceStream,
    ) -> Self {
        Self {
            serial: serial.to_string(),
            state: StreamerState::OpenConnection,
            board,
            rx_stream,

            flash_default: FlashData::default(),
            flash_calib: FlashData::default(),

            total_streamed_bytes: 0,
            flushing: false,
        }
    }

    fn flush_device(&mut self) {
        self.rx_stream.set_timeout(Duration::from_secs(1));
        self.flushing = true;
    }

    fn is_flushing(&self) -> bool {
        self.flushing
    }

    fn open_connection(&mut self) {
        *self.board = base::open_with_serial(&self.serial).unwrap();
        *self.rx_stream = DeviceStream::new(self.board.clone());
    }
}

impl<'a, 'b> Future for SGBStreamer<'a, 'b> {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        loop {
            if self.is_flushing() {
                while let item = Pin::new(&mut self.rx_stream).poll_next(cx) {
                    match item {
                        Poll::Ready(Some(buf)) => {
                            self.total_streamed_bytes += buf.bytes_read;
                        }
                        Poll::Ready(None) => {
                            println!(
                                "Flushing complete. Flushed {} bytes.",
                                self.total_streamed_bytes
                            );
                            self.flushing = false;
                            break;
                        }
                        Poll::Pending => {
                            cx.waker().wake_by_ref();
                            return Poll::Pending;
                        }
                    }
                }
                cx.waker().wake_by_ref();
                return Poll::Pending;
            } else {
                match &self.state {
                    StreamerState::OpenConnection => {
                        self.open_connection();
                        self.flush_device();

                        self.state = StreamerState::ReadFlash;
                        cx.waker().wake_by_ref();
                        return Poll::Pending;
                    }

                    StreamerState::ReadFlash => {
                        println!("Initializing Flash data.");
                        let board: &FtdiBoard = &self.board;
                        let flash_data =
                            FlashData::get_flash_info(board).expect("Error decoding Flash data.");
                        self.flash_default = flash_data;
                        self.flash_calib = flash_data;

                        println!("{:?}", self.flash_default);

                        self.state = StreamerState::PrepareInitialization;
                        cx.waker().wake_by_ref();
                        return Poll::Pending;
                    }

                    StreamerState::PrepareInitialization => {
                        println!("Preparing Board for initialization.");
                        base::stop(&mut self.board).unwrap();
                        self.flush_device();

                        self.state = StreamerState::Initalize;
                        cx.waker().wake_by_ref();
                        return Poll::Pending;
                    }

                    StreamerState::Initalize => {
                        println!("Initializing Board.");                        
                        base::check_board_communication(&mut self.board).unwrap();

                        let hv_val = self.flash_default.get_hv();
                        let dac = self.flash_default.get_dac();
                        base::initialize_sipm_parameters(&mut self.board, hv_val, dac).unwrap();

                        todo!("reset_everything_until_ok ---> needs run settings");

                        return Poll::Ready(());
                    }

                    StreamerState::TempStabilization => todo!(),
                    StreamerState::ReadStream => todo!(),
                    StreamerState::ReadTests => todo!(),
                    StreamerState::TempCompensation => todo!(),
                    StreamerState::Termination => todo!(),
                }
            }
        }
    }
}
