pub(crate) mod global_data;
pub(crate) mod stream_reader;

use std::pin::Pin;
use std::time::Duration;
use std::future::Future;
use tokio_stream::Stream;
use std::task::{Context, Poll};

use super::raplibs::ftdi_wrapper::FtdiBoard;
use super::raplibs::{base, flash::FlashData};
use stream_reader::{DeviceStream, StreamResult};

enum StreamerState {
    OpenConnection,
    FlushInit,
    ReadFlash,
    Initalization,
    TempStabilization,
    ReadStream,
    ReadTests,
    TempCompensation,
    Termination,
}

pub struct SGBStreamer<'a,'b> {
    state: StreamerState,
    serial: String,

    board: &'a mut FtdiBoard,
    rx_stream: &'b mut DeviceStream,

    flash_default: FlashData,
    flash_calib: FlashData,

    total_streamed_bytes: usize,
}

impl<'a,'b> SGBStreamer<'a,'b> {    
    pub fn new(serial: &'static str, board: &'a mut FtdiBoard, rx_stream: &'b mut DeviceStream) -> Self {
        Self {
            serial: serial.to_string(),
            state: StreamerState::OpenConnection,
            board,
            rx_stream,

            flash_default: FlashData::default(),
            flash_calib: FlashData::default(),

            total_streamed_bytes: 0,
        }
    }

    fn flush_device(&mut self) {
        self.rx_stream.set_timeout(Duration::from_secs(1));
    }

    fn open_connection(&mut self) {
        *self.board = base::open_with_serial(&self.serial).unwrap();
        *self.rx_stream = DeviceStream::new(self.board.clone());
        self.state = StreamerState::FlushInit;
    }
}

impl<'a,'b> Future for SGBStreamer<'a,'b> {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        loop {
            match &self.state {
                StreamerState::OpenConnection => {
                    self.open_connection();
                    self.flush_device();

                    cx.waker().wake_by_ref();
                    return Poll::Pending;
                }

                StreamerState::FlushInit => {
                    while let item = Pin::new(&mut self.rx_stream).poll_next(cx) {
                        match item {
                            Poll::Ready(Some(buf)) => {
                                self.total_streamed_bytes += buf.bytes_read;
                            }
                            Poll::Ready(None) => {
                                println!("Flushing complete. Flushed {} bytes.", self.total_streamed_bytes);
                                self.state = StreamerState::ReadFlash;
                                break;
                            }
                            Poll::Pending => {
                                return Poll::Pending;
                            }
                        }
                    }
                }

                StreamerState::ReadFlash => {
                    println!("Readflash");
                    return Poll::Ready(());
                }
                StreamerState::Initalization => todo!(),
                StreamerState::TempStabilization => todo!(),
                StreamerState::ReadStream => todo!(),
                StreamerState::ReadTests => todo!(),
                StreamerState::TempCompensation => todo!(),
                StreamerState::Termination => todo!(),
            }
        }
    }
}
