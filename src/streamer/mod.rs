pub(crate) mod global_data;
pub(crate) mod stream_readers;

use std::time::Duration;
use tokio::{select, sync::mpsc};
use tokio_util::sync::CancellationToken;

use crate::raplibs::{
    base, flash::FlashData, ftdi_wrapper::FtdiBoard, sanity_checks, settings::RunSettings, sha256,
    RapLibErrors,
};
use global_data::{StreamData, FRESH_NIBBLES_AFTER_RESET};
use stream_readers::{
    device_flusher::FlushDevice, fifo_reader::FifoReader, packet_generator::PacketGenerator,
    temperature_stabilizer::TemperatureStabilizer,
};

#[derive(PartialEq)]
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
    serial_number: &'static str,
    board: FtdiBoard,

    flash_default: FlashData,
    flash_calib: FlashData,
    run_settings_local: RunSettings,
    cancellation_token: CancellationToken,

    v_counter_last: i32,

    tx_channel: Option<mpsc::Sender<StreamData>>,
}

impl SingleGeneratorBoardFSM {
    pub fn new(
        serial: &'static str,
        tx_channel: Option<mpsc::Sender<StreamData>>,
        cancellation_token: CancellationToken,
    ) -> Self {
        Self {
            serial_number: serial,
            tx_channel,
            cancellation_token,
            ..Default::default()
        }
    }

    pub async fn run(&mut self) {
        let token = self.cancellation_token.clone();
        loop {
            select! {
                _ = token.cancelled() => self.state = StreamerState::Termination,
                e = self.handle_current_state() => {
                    if e.is_err() {
                        eprintln!("Error encountered: {:?}", e);
                        self.state = StreamerState::ErrorHandler;
                    }
                }
            }

            if self.state == StreamerState::Termination {
                if let Err(x) = self.handle_termination().await {
                    format!("Error during termination of device. Error code: {}", x);
                }
                break;
            }
        }
    }

    async fn handle_current_state(&mut self) -> Result<(), RapLibErrors> {
        match self.state {
            StreamerState::OpenConnection => self.handle_open_connection().await,
            StreamerState::ReadFlash => self.handle_read_flash().await,
            StreamerState::PrepareInitialization => self.handle_prepare_initialization().await,
            StreamerState::Initalize => self.handle_initialize().await,
            StreamerState::WriteSettings => self.handle_write_settings().await,
            StreamerState::TempStabilization => self.handle_temp_stabilization().await,
            StreamerState::ReadStream => self.handle_read_stream().await,
            StreamerState::ReadTests => self.handle_read_tests().await,
            StreamerState::TempCompensation => self.handle_temp_compensation().await,
            StreamerState::CheckSettings => self.handle_check_settings().await,
            StreamerState::Termination => Ok(()),
            StreamerState::ErrorHandler => self.handle_error().await,
        }
    }

    async fn handle_open_connection(&mut self) -> Result<(), RapLibErrors> {
        println!("Opening Connection.");
        self.board = base::open_with_serial(&self.serial_number)?;
        self.run_settings_local = RunSettings::get_run_settings()?;
        self.flush_device().await?;

        self.state = StreamerState::ReadFlash;
        Ok(())
    }

    async fn handle_read_flash(&mut self) -> Result<(), RapLibErrors> {
        println!("Reading flash data.");
        self.flash_default = FlashData::get_flash_info(&self.board)?;
        self.flash_calib = self.flash_default.clone();

        println!("Flash data initialized: {:?}", self.flash_default);

        self.state = StreamerState::PrepareInitialization;
        Ok(())
    }

    async fn handle_prepare_initialization(&mut self) -> Result<(), RapLibErrors> {
        println!("Preparing board for initialization.");
        self.stop_device().await?;
        self.flush_device().await?;

        self.state = StreamerState::Initalize;
        Ok(())
    }

    async fn handle_initialize(&mut self) -> Result<(), RapLibErrors> {
        println!("Initializing board.");
        base::check_board_communication(&self.board)?;

        let hv_val = self.flash_default.hv();
        let dac = self.flash_default.dac();
        base::initialize_sipm_parameters(&self.board, hv_val, dac)?;

        let afp_threshold = self.run_settings_local.get_afp_threshold();
        base::set_tdc_time_threshold(&self.board, afp_threshold)?;

        self.reset_nibbles().await?;

        self.state = StreamerState::WriteSettings;
        Ok(())
    }

    async fn handle_write_settings(&mut self) -> Result<(), RapLibErrors> {
        println!("Writing settings to device.");
        self.write_run_settings_to_device()?;
        self.prepare_fifos().await?;

        self.state = StreamerState::TempStabilization;
        Ok(())
    }

    async fn handle_temp_stabilization(&mut self) -> Result<(), RapLibErrors> {
        println!("Performing temperature stabilization.");
        let timeout = Duration::from_secs(20);
        let mut stabilizer =
            TemperatureStabilizer::new(&self.board, &mut self.flash_calib, timeout);
        stabilizer.perform_temperature_stabilization().await?;

        self.state = StreamerState::ReadStream;
        Ok(())
    }

    async fn handle_read_stream(&mut self) -> Result<(), RapLibErrors> {
        println!("Generating bits.");
        self.generate_packet().await?;

        self.state = StreamerState::ReadTests;
        Ok(())
    }

    async fn handle_read_tests(&mut self) -> Result<(), RapLibErrors> {
        println!("Reading FIFOs from buffer.");
        self.read_fifo_buffers().await?;

        self.state = StreamerState::TempCompensation;
        Ok(())
    }

    async fn handle_temp_compensation(&mut self) -> Result<(), RapLibErrors> {
        println!("Performing temperature compensation.");
        if self.temperature_compensation()? {
            self.prepare_fifos().await?;
        }

        self.state = StreamerState::CheckSettings;
        Ok(())
    }

    async fn handle_check_settings(&mut self) -> Result<(), RapLibErrors> {
        println!("Checking settings.");
        if let Ok(new_settings) = RunSettings::get_run_settings() {
            if self.run_settings_local != new_settings {
                self.run_settings_local = new_settings;
                self.write_run_settings_to_device()?;
                self.prepare_fifos().await?;
            }
        }

        self.state = StreamerState::ReadStream;
        Ok(())
    }

    async fn handle_error(&mut self) -> Result<(), RapLibErrors> {
        eprintln!("Handling error state.");

        if let Err(e) = self.handle_termination().await {
            self.state = StreamerState::Termination;
            eprintln!("Unable to handle error. Terminating device.\nError code: {:?}", e);
            return Ok(());
        }

        if let Err(e) = self.handle_open_connection().await {
            self.state = StreamerState::Termination;
            eprintln!("Unable to handle error. Terminating device.\nError code: {:?}", e);
        }
        Ok(())
    }

    async fn handle_termination(&mut self) -> Result<(), RapLibErrors> {
        println!("Terminating device.");
        base::stop(&self.board)?;
        self.flush_device().await?;
        Ok(base::close(&self.board)?)
    }

    async fn flush_device(&mut self) -> Result<(), RapLibErrors> {
        println!("Flushing device, please wait.");
        let _timeout = Duration::from_secs(1);
        let flushed_bytes = FlushDevice::new(&self.board, _timeout)
            .flush_device()
            .await?;
        Ok(println!("Flushed {flushed_bytes} bytes!"))
    }

    async fn generate_packet(&mut self) -> Result<(), RapLibErrors> {
        if let Some(tx_channel) = self.tx_channel.clone() {
            let serial_number = self.serial_number;
            let max_dwords = self.run_settings_local.get_num_of_dwords();

            let mut packet_generator =
                PacketGenerator::new(serial_number, &self.board, &tx_channel, max_dwords);
            return Ok(packet_generator.generate_packet().await?);
        }
        Err(RapLibErrors::UnhandledError(
            "No transmitter channel found. Restart the application.".to_string(),
        ))
    }

    async fn prepare_fifos(&mut self) -> Result<(), RapLibErrors> {
        base::reset_fail_flag_latch(&self.board)?;
        base::reset_rap_values(&self.board, true, true, true)?;
        let _ = self.wait_for_end_of_generation().await;
        self.flush_device().await?;
        Ok(())
    }

    async fn read_fifo_buffers(&mut self) -> Result<(), RapLibErrors> {
        if let Some(tx_channel) = self.tx_channel.clone() {
            let serial_number = self.serial_number;

            let mut fifo_reader = FifoReader::new(serial_number, &self.board, &tx_channel);
            return Ok(fifo_reader.read_fifo_results().await?);
        }
        Err(RapLibErrors::UnhandledError(
            "No transmitter channel found. Restart the application.".to_string(),
        ))
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
            serial_number: "default",
            board: FtdiBoard::default(),
            flash_default: FlashData::default(),
            flash_calib: FlashData::default(),
            run_settings_local: RunSettings::default(),
            v_counter_last: 0,
            tx_channel: None,
            cancellation_token: CancellationToken::default(),
        }
    }
}
