mod base;
mod ftdi_wrapper;
mod stream_reader;

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use ftdi_wrapper::FtdiBoard;
use stream_reader::{DeviceStream, StreamResult};
use tokio_stream::{Stream, StreamExt};

enum StreamerState {
    OpenConnection,
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
    flushing: bool,
}

impl SGBStreamer {
    pub fn new(serial: &'static str) -> Self {
        Self {
            serial: serial.to_string(),
            state: StreamerState::OpenConnection,
            board: FtdiBoard::new(None),
            flushing: false,
        }
    }

    fn set_flushing(&mut self, is_flushing: bool) {
        self.flushing = is_flushing;
    }

    fn is_flushing(&mut self) -> bool {
        self.flushing
    }

    fn flush_device(&mut self) {
        let timeout = Duration::from_secs(1);
        let board_clone = self.board.clone();

        /*let flush_Stream = tokio::spawn(async move {
            let mut flush = DeviceStream::new(board_clone, timeout);

            loop {
                match flush.next().await {
                    Some(stream) => {
                        println!("{:?}", stream);
                    }
                    None => {
                        println!("Stream completed. Restarting...");
                        break;
                    }
                }
            }
        });

        if flush_Stream.is_finished() {
            self.set_flushing(false);
        }*/

    }

    fn open_connection(&mut self) {
        self.board = base::open_with_serial(&self.serial).unwrap();
    }
}

impl Future for SGBStreamer {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        loop {
            if !self.is_flushing() {
                match self.state {
                    StreamerState::OpenConnection => {
                        println!("Poll2");
                        self.open_connection();
                        self.set_flushing(true);
                        println!("Poll3");
                        self.state = StreamerState::ReadFlash;

                        cx.waker().wake_by_ref();
                        return Poll::Pending;
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
            } else {
                self.flush_device();
            }
        }
    }
}
