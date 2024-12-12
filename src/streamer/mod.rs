mod base;
mod ftdi_wrapper;
mod stream_reader;

use std::future::{Future, Pending};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use ftdi_wrapper::FtdiBoard;
use stream_reader::{DeviceStream, StreamResult};
use tokio_stream::{Stream, StreamExt};

enum StreamerState {
    OpenConnection,
    FlushInit(Pin<Box<DeviceStream>>),
    ReadFlash,
    Initalization,
    TempStabilization,
    ReadStream,
    ReadTests,
    TempCompensation,
    Termination,
}

pub struct SGBStreamer {
    state: StreamerState,
    serial: String,
    board: FtdiBoard,
}

impl SGBStreamer {
    pub fn new(serial: &'static str) -> Self {
        Self {
            serial: serial.to_string(),
            state: StreamerState::OpenConnection,
            board: FtdiBoard::new(None),
        }
    }

    fn flush_device(&mut self) -> Pin<Box<DeviceStream>> {
        let timeout = Duration::from_secs(1);
        let board_clone = self.board.clone();
        Box::pin(DeviceStream::new(board_clone, timeout))
    }

    fn open_connection(&mut self) {
        self.board = base::open_with_serial(&self.serial).unwrap();
    }
}

impl Future for SGBStreamer {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        loop {
            match &self.state {
                StreamerState::OpenConnection => {
                    self.open_connection();
                    self.state = StreamerState::FlushInit(self.flush_device());

                    cx.waker().wake_by_ref();
                    return Poll::Pending;
                }

                StreamerState::FlushInit(flusher) => {
                    while let item = flusher.clone().as_mut().poll_next(cx) {
                        match item {
                            Poll::Ready(Some(buf)) => {
                                println!("Reveived: {:?}", buf);
                            }
                            Poll::Ready(None) => {
                                println!("Flushing complete.");
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
