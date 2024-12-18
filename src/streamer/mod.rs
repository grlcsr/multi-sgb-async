pub(crate) mod global_data;
pub(crate) mod stream_reader;

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio_stream::Stream;

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

pub struct SGBStreamer {
    state: StreamerState,
    serial: String,

    rx_stream: DeviceStream,

    total_streamed_bytes: usize,
    v_counter_last: i32,
    flushing: bool,
}

impl SGBStreamer {
    pub fn new(serial: &'static str) -> Self {
        Self {
            serial: serial.to_string(),
            state: StreamerState::OpenConnection,
            rx_stream: DeviceStream::default(),

            total_streamed_bytes: 0,
            v_counter_last: 0,
            flushing: false,
        }
    }

    fn flush_device(&mut self) {
        self.rx_stream.set_timeout(Duration::from_secs(1));
        self.flushing = true;
    }

    fn get_stream(&mut self) -> &mut DeviceStream {
        &mut self.rx_stream
    }

    fn get_v_counter_last(&mut self) -> &mut i32 {
        return &mut self.v_counter_last;
    }

    fn is_flushing(&self) -> bool {
        self.flushing
    }

    fn open_stream(&mut self) {
        self.rx_stream = DeviceStream::new(&self.serial);
    }
}

impl Future for SGBStreamer {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        loop {
            if self.is_flushing() {
                loop {
                    let item = Pin::new(&mut self.get_stream()).poll_next(cx);
                    match item {
                        Poll::Ready(Some(buf)) => {
                            println!("Flushing: {:?}", buf.bytes_read);
                            self.total_streamed_bytes += buf.bytes_read;

                            cx.waker().wake_by_ref();
                            return Poll::Pending;
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
                        self.open_stream();
                        self.flush_device();

                        self.state = StreamerState::ReadFlash;
                        cx.waker().wake_by_ref();
                        return Poll::Pending;
                    }

                    StreamerState::ReadFlash => {
                        println!("Initializing Flash data.");
                        self.rx_stream.initialize_flash();

                        self.state = StreamerState::PrepareInitialization;
                        cx.waker().wake_by_ref();
                        return Poll::Pending;
                    }

                    StreamerState::PrepareInitialization => {
                        println!("Preparing Board for initialization.");
                        self.get_stream().stop_device();
                        self.flush_device();

                        self.state = StreamerState::Initalize;
                        cx.waker().wake_by_ref();
                        return Poll::Pending;
                    }

                    StreamerState::Initalize => {
                        println!("Initializing Board.");
                        self.get_stream().initialize_board();

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
