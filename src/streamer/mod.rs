pub(crate) mod global_data;
pub(crate) mod stream_reader;

use std::{fmt::Error, time::Duration};

use global_data::FRESH_NIBBLES_AFTER_RESET;
use stream_reader::TemperatureStabilizer;

use super::raplibs::ftdi_wrapper::FtdiBoard;
use crate::raplibs::{
    base, flash::FlashData, sanity_checks, settings::RunSettings, sha256, RapLibErrors,
};

enum StreamerState {
    OpenConnection,
    ReadFlash,
    PrepareInitialization,
    Initalize,
    TempStabilization,
    WriteSettings,
    ReadStream,
    ReadTests,
    TempCompensation,
    Termination,
}

pub struct SingleGeneratorBoardFSM {
    state: StreamerState,
    serial_number: String,
    board: FtdiBoard,

    flash_default: FlashData,
    flash_calib: FlashData,
    run_settings_local: RunSettings,

    v_counter_last: i32,
}

impl SingleGeneratorBoardFSM {
    pub fn new(serial: &'static str) -> Self {
        Self {
            serial_number: serial.to_string(),
            ..Default::default()
        }
    }

    pub async fn sgb_mananger(&mut self) {
        loop {
            match &self.state {
                StreamerState::OpenConnection => {
                    println!("Opening Connection.");
                    self.open_connection().await;
                    self.state = StreamerState::ReadFlash;
                }

                StreamerState::ReadFlash => {
                    println!("Initializing Flash data.");
                    self.read_flash().await;
                    self.state = StreamerState::PrepareInitialization;
                }

                StreamerState::PrepareInitialization => {
                    println!("Preparing Board for initialization.");
                    self.stop_device().await;
                    self.flush_device().await;

                    self.state = StreamerState::Initalize;
                }

                StreamerState::Initalize => {
                    println!("Initializing Board.");
                    self.initialize_board().await;
                    self.reset_nibbles().await;

                    self.state = StreamerState::TempStabilization;
                }

                StreamerState::TempStabilization => {
                    println!("Performing Temperature Stabilization.");
                    self.perform_temperature_stabilization().await;

                    self.state = StreamerState::WriteSettings;
                }

                StreamerState::WriteSettings => {
                    println!("Writing settings to device.");
                    self.write_run_settings_to_device();
                    base::reset_rap_values(&self.board, true, true, true);
                    self.flush_device().await;
                    self.wait_for_end_of_generation().await;

                    self.state = StreamerState::ReadStream;
                }

                StreamerState::ReadStream => {
                    todo!()
                }
                StreamerState::ReadTests => todo!(),
                StreamerState::TempCompensation => todo!(),
                StreamerState::Termination => todo!(),
            }
        }
    }

    async fn flush_device(&mut self) {
        let _timeout = Duration::from_secs(1);
        let flushed_bytes = stream_reader::FlushDevice::new(&self.board, _timeout)
            .flush_device()
            .await;
        println!("Flushed {flushed_bytes} bytes!");
    }

    async fn initialize_board(&mut self) {
        let device = &mut self.board;
        base::check_board_communication(device);

        let hv_val = self.flash_default.get_hv();
        let dac = self.flash_default.get_dac();
        base::initialize_sipm_parameters(device, hv_val, dac);

        let afp_threshold: u16 = self.run_settings_local.get_afp_threshold();
        base::set_tdc_time_threshold(device, afp_threshold);
    }

    async fn open_connection(&mut self) {
        self.board = base::open_with_serial(&self.serial_number).unwrap();
        self.run_settings_local = RunSettings::get_run_settings()
            .expect("Panic initializing DeviceStream: cannot get runsettings.")
            .clone();
        self.flush_device().await;
    }

    async fn perform_temperature_stabilization(&mut self) {
        let mut flash_calib = self.flash_calib;
        let timeout = Duration::from_secs(20);

        let mut temperature_stabilizer =
            TemperatureStabilizer::new(&self.board, &mut flash_calib, timeout);
        temperature_stabilizer
            .perform_temperature_stabilization()
            .await;

        self.flash_calib = flash_calib;
    }

    async fn read_flash(&mut self) {
        println!("Initializing Flash data.");
        let device: &FtdiBoard = &self.board;
        let flash_data = FlashData::get_flash_info(device).expect("Error decoding Flash data.");
        self.flash_default = flash_data;
        self.flash_calib = flash_data;

        println!("{:?}", self.flash_default);
    }

    async fn reset_nibbles(&mut self) -> Result<(), RapLibErrors> {
        for _i in 0..5 {
            base::reset_rap_values(&mut self.board, true, true, true);

            if let FRESH_NIBBLES_AFTER_RESET = self.wait_for_end_of_generation().await {
                return Ok(());
            }
        }
        Err(RapLibErrors::BaseError(
            "Cannot reset to a known state".to_string(),
        ))
    }

    async fn stop_device(&mut self) {
        base::stop(&mut self.board).unwrap();
    }

    async fn wait_for_end_of_generation(&mut self) -> i32 {
        let mut v_counter_total: i32 = 0;

        loop {
            let v_counter_diff = async {
                base::write_pack(&mut self.board, 4, 0);
                let v_counter: i32 = self.board.read_32_bit_u32().unwrap() as i32; // TODO Error handling
                let mut v_counter_diff = v_counter - self.v_counter_last;

                if v_counter_diff < 0 {
                    v_counter_diff += 2_i32.pow(30);
                    println!(
                        "v_counter_diff less than zero. New val: {:?}",
                        v_counter_diff
                    );
                }

                v_counter_total += v_counter_diff;
                self.v_counter_last = v_counter;

                v_counter_diff
            }
            .await;

            if v_counter_diff == 0 {
                break;
            }
        }

        return v_counter_total;
    }

    fn write_run_settings_to_device(&self) {
        let afp_threshold: u16 = self.run_settings_local.get_afp_threshold();

        sanity_checks::update_fpga_settings(&self.board, self.run_settings_local);
        sha256::perform_accelerator_initialization(&self.board);
        sha256::set_reduction_ratio(&self.board, self.run_settings_local);
        base::set_tdc_time_threshold(&self.board, afp_threshold);
        base::reset_fail_flag_latch(&self.board);
    }
}

/*impl Future for SingleGeneratorBoardFSM {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
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
            state: StreamerState::OpenConnection,
            serial_number: "defalt".to_string(),
            board: FtdiBoard::default(),
            flash_default: FlashData::default(),
            flash_calib: FlashData::default(),
            run_settings_local: RunSettings::default(),
            v_counter_last: 0,
        }
    }
}
