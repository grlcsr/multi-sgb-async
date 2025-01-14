pub(crate) mod global_data;
pub(crate) mod stream_reader;

use std::time::Duration;
use tokio::sync::mpsc;

use crate::raplibs::{
    base, flash::FlashData, ftdi_wrapper::FtdiBoard, sanity_checks, settings::RunSettings, sha256,
    RapLibErrors,
};
use global_data::{StreamData, FRESH_NIBBLES_AFTER_RESET};
use stream_reader::{FifoReader, PacketGenerator, TemperatureStabilizer};

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
    CheckSettings,
    Termination,
    ErrorHandler,
}

pub struct SingleGeneratorBoardFSM {
    state: StreamerState,
    serial_number: String,
    board: FtdiBoard,

    flash_default: FlashData,
    flash_calib: FlashData,
    run_settings_local: RunSettings,

    v_counter_last: i32,

    tx_channel: Option<mpsc::Sender<StreamData>>,
}

impl SingleGeneratorBoardFSM {
    pub fn new(serial: &'static str, tx_channel: Option<mpsc::Sender<StreamData>>) -> Self {
        Self {
            serial_number: serial.to_string(),
            tx_channel,
            ..Default::default()
        }
    }

    pub async fn sgb_mananger(&mut self) {
        let mut err: Result<_, RapLibErrors> = Ok(());

        loop {
            if err.is_err() {
                self.state = StreamerState::ErrorHandler;
            }

            match &self.state {
                StreamerState::OpenConnection => {
                    println!("Opening Connection.");
                    err = self.open_connection().await;

                    self.state = StreamerState::ReadFlash;
                }

                StreamerState::ReadFlash => {
                    println!("Initializing Flash data.");
                    err = self.read_flash().await;

                    self.state = StreamerState::PrepareInitialization;
                }

                StreamerState::PrepareInitialization => {
                    println!("Preparing Board for initialization.");
                    err = self.stop_device().await;
                    if !err.is_err() {
                        err = self.flush_device().await;
                    }
                    
                    self.state = StreamerState::Initalize;
                }

                StreamerState::Initalize => {
                    println!("Initializing Board.");
                    err = self.initialize_board().await;
                    if !err.is_err() {
                        err = self.reset_nibbles().await;
                    }

                    self.state = StreamerState::WriteSettings;
                }

                StreamerState::WriteSettings => {
                    println!("Writing settings to device.");
                    err = self.write_run_settings_to_device();
                    if !err.is_err() {
                        err = self.prepare_fifos().await;
                    }

                    self.state = StreamerState::TempStabilization;
                }

                StreamerState::TempStabilization => {
                    println!("Performing Temperature Stabilization.");
                    err = self.perform_temperature_stabilization().await;

                    self.state = StreamerState::ReadStream;
                }

                StreamerState::ReadStream => {
                    println!("Generating bits.");
                    err = self.generate_packet().await;

                    self.state = StreamerState::ReadTests;
                }

                StreamerState::ReadTests => {
                    println!("Reading Fifos from buffer.");
                    err = self.read_fifo_buffers().await;

                    self.state = StreamerState::TempCompensation;
                }

                StreamerState::TempCompensation => {
                    println!("Performing Temperature Compensation.");
                    match self.temperature_compensation() {
                        Ok(true) => err = self.prepare_fifos().await,
                        Err(_) => {
                            self.state = StreamerState::ErrorHandler;
                            continue;
                        }
                        _ => {}
                    }

                    self.state = StreamerState::CheckSettings;
                }

                StreamerState::CheckSettings => {
                    if let Ok(new_settings) = RunSettings::get_run_settings() {
                        if self.run_settings_local != new_settings {
                            self.run_settings_local = new_settings;
                            err = self.write_run_settings_to_device();
                            if !err.is_err() {
                                err = self.prepare_fifos().await;
                            }
                        }
                    }
                    
                    self.state = StreamerState::ReadStream;
                }

                StreamerState::Termination => {
                    println!("Terminating device.");
                    let _ = base::stop(&self.board);
                    let _ = self.flush_device().await;
                    let _ = base::close(&self.board);
                    break;
                }

                StreamerState::ErrorHandler => todo!(),
            }
        }
    }

    async fn flush_device(&mut self) -> Result<(), RapLibErrors> {
        println!("Flushing device, please wait.");
        let _timeout = Duration::from_secs(1);
        let flushed_bytes = stream_reader::FlushDevice::new(&self.board, _timeout)
            .flush_device()
            .await?;
        Ok(println!("Flushed {flushed_bytes} bytes!"))
    }

    async fn initialize_board(&mut self) -> Result<(), RapLibErrors> {
        let device = &mut self.board;
        base::check_board_communication(device)?;

        let hv_val = self.flash_default.hv();
        let dac = self.flash_default.dac();
        base::initialize_sipm_parameters(device, hv_val, dac)?;

        let afp_threshold: u16 = self.run_settings_local.get_afp_threshold();
        base::set_tdc_time_threshold(device, afp_threshold)?;
        Ok(())
    }

    async fn generate_packet(&mut self) -> Result<(), RapLibErrors> {
        if let Some(tx_channel) = self.tx_channel.clone() {
            let serial_number = self.serial_number.clone();
            let max_dwords = self.run_settings_local.get_num_of_dwords();

            let mut packet_generator =
                PacketGenerator::new(serial_number, &self.board, &tx_channel, max_dwords);
            return Ok(packet_generator.generate_packet().await?);
        }
        Err(RapLibErrors::UnhandledError("No transmitter channel found. Restart the application.".to_string()))
    }

    async fn open_connection(&mut self) -> Result<(), RapLibErrors> {
        self.board = base::open_with_serial(&self.serial_number)?;
        self.run_settings_local = RunSettings::get_run_settings()?;
        //.expect("Panic initializing DeviceStream: cannot get runsettings.");
        Ok(self.flush_device().await?)
    }

    async fn perform_temperature_stabilization(&mut self) -> Result<(), RapLibErrors> {
        let mut flash_calib = self.flash_calib;
        let timeout = Duration::from_secs(20);

        let mut temperature_stabilizer =
            TemperatureStabilizer::new(&self.board, &mut flash_calib, timeout);
        temperature_stabilizer
            .perform_temperature_stabilization()
            .await?;

        self.flash_calib = flash_calib;
        Ok(())
    }

    async fn prepare_fifos(&mut self) -> Result<(), RapLibErrors> {
        base::reset_fail_flag_latch(&self.board)?;
        base::reset_rap_values(&self.board, true, true, true)?;
        let _ = self.wait_for_end_of_generation().await;
        self.flush_device().await?;
        Ok(())
    }

    async fn read_flash(&mut self) -> Result<(), RapLibErrors> {
        let device: &FtdiBoard = &self.board;
        let flash_data = FlashData::get_flash_info(device)?;
        self.flash_default = flash_data;
        self.flash_calib = flash_data;

        Ok(println!("{:?}", self.flash_default))
    }

    async fn read_fifo_buffers(&mut self) -> Result<(), RapLibErrors> {
        if let Some(tx_channel) = self.tx_channel.clone() {
            let serial_number = self.serial_number.clone();

            let mut fifo_reader = FifoReader::new(serial_number, &self.board, &tx_channel);
            return Ok(fifo_reader.read_fifo_results().await?);
        }
        Err(RapLibErrors::UnhandledError("No transmitter channel found. Restart the application.".to_string()))
    }


    async fn reset_nibbles(&mut self) -> Result<(), RapLibErrors> {
        for _i in 0..5 {
            base::reset_rap_values(&mut self.board, true, true, true)?;

            if let FRESH_NIBBLES_AFTER_RESET = self.wait_for_end_of_generation().await? {
                return Ok(());
            }
        }
        Err(RapLibErrors::StreamerError(
            "Cannot reset to a known state".to_string(),
        ))
    }

    async fn stop_device(&mut self) -> Result<(), RapLibErrors> {
        let _ = base::stop(&mut self.board)?;
        Ok(())
    }

    async fn wait_for_end_of_generation(&mut self) -> Result<i32, RapLibErrors> {
        let mut v_counter_total: i32 = 0;

        loop {
            let v_counter_diff: Result<i32, RapLibErrors> = async {
                base::write_pack(&mut self.board, 4, 0)?;
                let v_counter: i32 = self.board.read_32_bit_u32()? as i32;
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

                Ok(v_counter_diff)
            }
            .await;

            if let Ok(0) = v_counter_diff {
                break;
            }

            if let Err(err) = v_counter_diff {
                return Err(err);
            }
        }

        Ok(v_counter_total)
    }

    fn write_run_settings_to_device(&self) -> Result<(), RapLibErrors> {
        let afp_threshold: u16 = self.run_settings_local.get_afp_threshold();

        sanity_checks::update_fpga_settings(&self.board, self.run_settings_local)?;
        sha256::perform_accelerator_initialization(&self.board)?;
        sha256::set_reduction_ratio(&self.board, self.run_settings_local)?;
        base::set_tdc_time_threshold(&self.board, afp_threshold)?;
        Ok(())
    }

    fn temperature_compensation(&mut self) -> Result<bool, RapLibErrors> {
        let flash_default = self.flash_default;
        let temperature_now = base::req_temperature(&self.board)?;
        let hv_now = base::hv_compensate(
            temperature_now,
            flash_default.hv(),
            flash_default.ref_temp(),
        );
        self.flash_calib.set_hv(hv_now);
        base::set_hvdac(&self.board, hv_now)?;

        let delta_t = (self.flash_calib.ref_temp() - temperature_now).abs();
        if delta_t > 2.0 {
            self.flash_calib.set_ref_temp(temperature_now);
            return Ok(true);
        }
        return Ok(false);
    }
}

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
            tx_channel: None,
        }
    }
}
