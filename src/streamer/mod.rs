pub(crate) mod global_data;
pub(crate) mod stream_reader;

use std::time::Duration;

use super::raplibs::ftdi_wrapper::FtdiBoard;
use crate::raplibs::{base, flash::FlashData, sanity_checks, settings::RunSettings, sha256};

enum StreamerState {
    OpenConnection,
    ReadFlash,
    PrepareInitialization,
    Initalize,
    WaitingNibbles,
    WriteSettings,
    TempStabilization,
    ReadStream,
    ReadTests,
    TempCompensation,
    Termination,
}

pub struct SingleGeneratorBoardFSM {
    serial_number: String,
    board: FtdiBoard,

    flash_default: FlashData,
    flash_calib: FlashData,
    run_settings_local: RunSettings,
}

impl SingleGeneratorBoardFSM {
    pub fn new(serial: &'static str) -> Self {
        Self {
            serial_number: serial.to_string(),
            ..Default::default()
        }
    }

    async fn open_connection(&mut self) {
        self.board = base::open_with_serial(&self.serial_number).unwrap();
        self.run_settings_local = RunSettings::get_run_settings()
            .expect("Panic initializing DeviceStream: cannot get runsettings.")
            .clone();

        let _timeout = Duration::from_secs(1);
        let flushed_bytes = stream_reader::FlushDevice::new(&self.board, _timeout).flush_device().await;

        println!("Connection Opened. Flushed {flushed_bytes} bytes!");
    }

    async fn read_flash(&mut self) {
        println!("Initializing Flash data.");
                    self.rx_stream.initialize_flash();
    }

    /*fn get_stream(&mut self) -> &mut SGBStreamer {
        &mut self.rx_stream
    }

    fn handle_flushing(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        let item = Pin::new(&mut self.get_stream()).poll_next(cx);
        match item {
            Poll::Ready(Some(buf)) => self.total_streamed_bytes += buf.bytes_read,
            Poll::Ready(None) => {
                self.get_stream().set_flushing(false);
                println!(
                    "Flushing complete. Flushed {} bytes.",
                    self.total_streamed_bytes
                );
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
                    v_counter =
                        u32::from_be_bytes([buf.buf[0], buf.buf[1], buf.buf[2], buf.buf[3]]) as i32;
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

        println!("v_counter {}, vcounter_diff {}", v_counter, v_counter_diff);

        self.total_streamed_bytes += v_counter_diff as usize;

        if v_counter_diff == 0 {
            self.waiting_end_of_generation = false;
        }

        cx.waker().wake_by_ref();
        return Poll::Pending;
    }

    fn open_stream(&mut self) {
        self.rx_stream = SGBStreamer::new(&self.serial);
    }

    fn set_wait_end_of_generation(&mut self) {
        self.get_stream().set_timeout(Duration::from_secs(1));
        self.get_stream().set_last_poll_time();
        self.waiting_end_of_generation = true;
    }

    fn is_waiting_end_of_generation(&self) -> bool {
        self.waiting_end_of_generation
    }

    fn reset_total_streamed_bytes(&mut self) {
        self.total_streamed_bytes = 0;
    }*/
}

/*impl Future for SingleGeneratorBoardFSM {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        loop {
            if self.get_stream().is_flushing() {
                self.get_stream().set_read_32_bits_stream(false);
                return self.handle_flushing(cx);
            } else if self.is_waiting_end_of_generation() {
                self.get_stream().set_read_32_bits_stream(true);
                return self.handle_waiting_end_of_generation(cx);
            }

            match &self.state {
                StreamerState::OpenConnection => {
                    self.open_stream();
                    self.get_stream().flush_device();

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
                    self.get_stream().flush_device();

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
                    self.reset_total_streamed_bytes();

                    let generated_nibbles: i32 = self.total_streamed_bytes as i32;
                    if generated_nibbles != FRESH_NIBBLES_AFTER_RESET {
                        self.get_stream().reset_rap_values(true, true, true);
                        self.set_wait_end_of_generation();
                        
                        self.nibble_polls += 1;
                        if self.nibble_polls >= 5 {
                            panic!("Can't reset board to known state.");
                        }
                    } else {
                        self.nibble_polls = 0;
                    }

                    self.state = StreamerState::TempStabilization;
                    cx.waker().wake_by_ref();
                    return Poll::Pending;
                }

                StreamerState::TempStabilization => {
                    let flash_calib = self.get_stream().get_flash_calib();
                    let temperature_now: f32 = self.get_stream().req_temperature();
                    let delta_t = f32::abs(flash_calib.get_ref_temp() - temperature_now);

                    self.get_stream().set_gate_dcr();


                    self.state = StreamerState::WriteSettings;
                    cx.waker().wake_by_ref();
                    return Poll::Pending;
                }
                
                StreamerState::WriteSettings => {
                    self.get_stream().write_run_settings_to_device();
                    self.get_stream().reset_rap_values(true, true, true);
                    self.set_wait_end_of_generation();
                    //self.get_stream().flush_device();

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
*/



impl Default for SingleGeneratorBoardFSM {
    fn default() -> Self {
        Self {
            serial_number: "defalt".to_string(),
            board: FtdiBoard::default(),
            flash_default: FlashData::default(),
            flash_calib: FlashData::default(),
            run_settings_local: RunSettings::default(),
        }
    }
}