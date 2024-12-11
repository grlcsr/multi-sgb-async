mod ftdi_wrapper;
mod base;
mod stream_reader;

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use ftdi_wrapper::FtdiBoard;
use stream_reader::{DeviceStream, StreamResult};
use tokio_stream::StreamExt;

enum StreamerState {
    OpenConnection,
    ReadFlash,
    Initalization,
    TempStabilization,
    ReadStream,
    ReadTests,
    TempCompensation,
    Termination
}

pub struct SGBStreamer {
    state: StreamerState,
    serial: String,
    board: FtdiBoard
}

impl SGBStreamer {
    pub fn new(serial: &'static str) -> Self {
        Self {
            serial: serial.to_string(),
            state: StreamerState::OpenConnection,
            board: FtdiBoard::new()
        }
    }

    fn flush_device(&mut self) {
        let timeout = Duration::from_secs(1);

        let board_clone = self.board.clone();

        tokio::spawn(async move {
            let mut flush = DeviceStream::new(board_clone, timeout);

            loop {
                match flush.next().await {
                    Some(stream) => {
                        println!("{:?}", stream);
                    }
                    None => {
                        println!("Stream completed. Restarting...");
                    }
                }

            }
        });
    }

    fn open_connection(&mut self) {
        println!("Poll22");
        self.board = base::open_with_serial(&self.serial).unwrap();
        println!("Poll3");
        self.state = StreamerState::ReadFlash;
    }
    
}

impl Future for SGBStreamer {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<()> {
        println!("Poll1");
        match self.state {
            StreamerState::OpenConnection => {
                println!("Poll2");
                self.open_connection();
                self.flush_device();


                Poll::Pending
            }

            StreamerState::ReadFlash => todo!(),
            StreamerState::Initalization => todo!(),
            StreamerState::TempStabilization => todo!(),
            StreamerState::ReadStream => todo!(),
            StreamerState::ReadTests => todo!(),
            StreamerState::TempCompensation => todo!(),
            StreamerState::Termination => todo!(),
        }
    }
}