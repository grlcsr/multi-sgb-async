pub(crate) mod global_data;
pub(crate) mod stream_reader;

use global_data::FRESH_NIBBLES_AFTER_RESET;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio_stream::Stream;

use super::raplibs::ftdi_wrapper::FtdiBoard;
use stream_reader::{DeviceStream, StreamResult};

enum StreamerState {
    OpenConnection,
    ReadFlash,
    PrepareInitialization,
    Initalize,
    WaitingNibbles,
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
    flushing: bool,

    waiting_end_of_generation: bool,
    v_counter_last: i32,
    nibble_polls: u8
}

impl SGBStreamer {
    pub fn new(serial: &'static str) -> Self {
        Self {
            serial: serial.to_string(),
            state: StreamerState::OpenConnection,
            rx_stream: DeviceStream::default(),

            total_streamed_bytes: 0,
            flushing: false,

            waiting_end_of_generation: false,
            v_counter_last: 0,
            nibble_polls: 0
        }
    }

    fn flush_device(&mut self) {
        self.get_stream().set_timeout(Duration::from_secs(1));
        self.get_stream().set_last_poll_time();
        self.flushing = true;
    }

    fn get_stream(&mut self) -> &mut DeviceStream {
        &mut self.rx_stream
    }

    fn handle_flushing(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        let item = Pin::new(&mut self.get_stream()).poll_next(cx);
        match item {
            Poll::Ready(Some(buf)) => self.total_streamed_bytes += buf.bytes_read,
            Poll::Ready(None) => {
                self.flushing = false;
                println!("Flushing complete. Flushed {} bytes.", self.total_streamed_bytes);
                self.reset_total_streamed_bytes();
            }
            _ => {}
        }
        cx.waker().wake_by_ref();
        return Poll::Pending;
    }

    fn handle_waiting_end_of_generation(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        let mut v_counter: i32 = 0;

        let _ = self.get_stream().write_pack(4, 0);
        let item = Pin::new(&mut self.get_stream()).poll_next(cx);        
        match item {
            Poll::Ready(Some(buf)) => {
                if buf.bytes_read == 4 {
                    v_counter = u32::from_be_bytes([buf.buf[0], buf.buf[1], buf.buf[2], buf.buf[3]]) as i32;
                } else {
                    panic!("DIDNT READ 32 BITS??");
                }
            }
            _ => {}
        }

        let mut v_counter_diff = v_counter - self.v_counter_last;
        self.v_counter_last = v_counter;

        if v_counter_diff < 0 {
            v_counter_diff += 2_i32.pow(30);
            println!(
                "v_counter_diff less than zero. New val: {:?}",
                v_counter_diff
            );
        }

        self.total_streamed_bytes += v_counter_diff as usize;

        if v_counter_diff == 0 {
            self.waiting_end_of_generation = false;
        }
        
        cx.waker().wake_by_ref();
        return Poll::Pending;
    }

    fn is_flushing(&self) -> bool {
        self.flushing
    }

    fn open_stream(&mut self) {
        self.rx_stream = DeviceStream::new(&self.serial);
    }

    fn set_wait_end_of_generation(&mut self) {
        self.get_stream().set_timeout(Duration::from_secs(2));
        self.get_stream().set_last_poll_time();
        self.waiting_end_of_generation = true;
    }

    fn is_waiting_end_of_generation(&self) -> bool {
        self.waiting_end_of_generation
    }

    fn reset_total_streamed_bytes(&mut self) {
        self.total_streamed_bytes = 0;
    }
}

impl Future for SGBStreamer {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        loop {
            if self.is_flushing() {
                return self.handle_flushing(cx);
            } else if self.is_waiting_end_of_generation() {
                return self.handle_waiting_end_of_generation(cx);
            }

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

                    self.state = StreamerState::WaitingNibbles;
                    cx.waker().wake_by_ref();
                    return Poll::Pending;
                }

                StreamerState::WaitingNibbles => {
                    println!("Waiting Nibbles.");
                    let generated_nibbles: i32 = self.total_streamed_bytes as i32;
                    if generated_nibbles != FRESH_NIBBLES_AFTER_RESET {
                        self.get_stream().set_read_32_bits_stream(true);
                        self.get_stream().reset_rap_values(true, true, true);
                        self.set_wait_end_of_generation();
                        self.nibble_polls += 1;
                    } else {
                        self.state = StreamerState::TempStabilization;
                    }

                    self.reset_total_streamed_bytes();
                    cx.waker().wake_by_ref();
                    return Poll::Pending;
                }

                StreamerState::TempStabilization => {

                    self.state = StreamerState::ReadStream;
                    cx.waker().wake_by_ref();
                    return Poll::Pending;
                }

                StreamerState::ReadStream => {
                    return Poll::Ready(());
                }
                StreamerState::ReadTests => todo!(),
                StreamerState::TempCompensation => todo!(),
                StreamerState::Termination => todo!(),
            }
        }
    }
}
