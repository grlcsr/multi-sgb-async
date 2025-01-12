use binext::{BinaryRead, BinaryWrite};
use lazy_static::lazy_static;
use std::fs::OpenOptions;
use std::sync::Mutex;

use super::RapLibErrors;

#[derive(Debug, Copy, Clone)]
pub struct RunSettings {
    num_of_dwords: u16, // package size in dwords - number of 32 bit words, hardware limit is 0xffff, has to be a multiple of 0xff
    afp_threshold: u16,

    // ------------------------------------------
    // -------------- ACCELERATORS --------------
    // ------------------------------------------
    sanity_fail_flag_latch_event_alarm_thr: u16, // [1..15]

    // -------------- MONOBIT --------------        bit consumption:     (2^MONO_numOfSequencesPowerOf2 * 2^MONO_sequenceLengthPowerOf2)
    mono_num_of_sequences_power_of_2: u16, // [2...21]        greater than MONO_sequenceLengthPowerOf2, both have to be odd or even
    mono_sequence_length_power_of_2: u16,  // [2...11]
    mono_confidence_level_upper: u16,      // [1...16383]     k upper
    mono_confidence_level_lower: u16,      // [1...16383]     k lower

    // -------------- ASYMMETRY --------------      bit consumption:     (ASYM_sequenceLength_bits)
    asym_sequence_length_bits: u16, // [12, 16, 20, 24, ..., 2^30], has to be a multiple of 4

    // -------------- RUNS --------------           bit consumption:     (2^sequenceLength_RUNS * 2^numOfSequencesPowerOf2_RUNS)
    runs_sequence_length: u16,             // [5...11]
    runs_num_of_sequences_power_of_2: u16, // [2...21]
    runs_confidence_level: u16,            // [0...16383]

    // -------------- SHA256 --------------         bit consumption:     (512 * red_ratio/2)
    sha256_reduction_ratio: u16, // [2, 4, 6, 8, ..., 32]
}

lazy_static! {
    static ref RUN_SETTINGS: Mutex<RunSettings> = Mutex::new(RunSettings::default());
}

impl RunSettings {
    pub fn get_num_of_dwords(&self) -> u16 {
        self.num_of_dwords
    }

    pub fn set_num_of_dwords(&mut self, value: u16) {
        self.num_of_dwords = value;
    }

    pub fn get_afp_threshold(&self) -> u16 {
        self.afp_threshold
    }

    pub fn set_afp_threshold(&mut self, value: u16) {
        self.afp_threshold = value;
    }

    pub fn get_sanity_fail_flag_latch_event_alarm_thr(&self) -> u16 {
        self.sanity_fail_flag_latch_event_alarm_thr
    }

    pub fn set_sanity_fail_flag_latch_event_alarm_thr(&mut self, value: u16) {
        self.sanity_fail_flag_latch_event_alarm_thr = value;
    }

    pub fn get_mono_num_of_sequences_power_of_2(&self) -> u16 {
        self.mono_num_of_sequences_power_of_2
    }

    pub fn set_mono_num_of_sequences_power_of_2(&mut self, value: u16) {
        self.mono_num_of_sequences_power_of_2 = value;
    }

    pub fn get_mono_sequence_length_power_of_2(&self) -> u16 {
        self.mono_sequence_length_power_of_2
    }

    pub fn set_mono_sequence_length_power_of_2(&mut self, value: u16) {
        self.mono_sequence_length_power_of_2 = value;
    }

    pub fn get_mono_confidence_level_upper(&self) -> u16 {
        self.mono_confidence_level_upper
    }

    pub fn set_mono_confidence_level_upper(&mut self, value: u16) {
        self.mono_confidence_level_upper = value;
    }

    pub fn get_mono_confidence_level_lower(&self) -> u16 {
        self.mono_confidence_level_lower
    }

    pub fn set_mono_confidence_level_lower(&mut self, value: u16) {
        self.mono_confidence_level_lower = value;
    }

    pub fn get_asym_sequence_length_bits(&self) -> u16 {
        self.asym_sequence_length_bits
    }

    pub fn set_asym_sequence_length_bits(&mut self, value: u16) {
        self.asym_sequence_length_bits = value;
    }

    pub fn get_runs_sequence_length(&self) -> u16 {
        self.runs_sequence_length
    }

    pub fn set_runs_sequence_length(&mut self, value: u16) {
        self.runs_sequence_length = value;
    }

    pub fn get_runs_num_of_sequences_power_of_2(&self) -> u16 {
        self.runs_num_of_sequences_power_of_2
    }

    pub fn set_runs_num_of_sequences_power_of_2(&mut self, value: u16) {
        self.runs_num_of_sequences_power_of_2 = value;
    }

    pub fn get_runs_confidence_level(&self) -> u16 {
        self.runs_confidence_level
    }

    pub fn set_runs_confidence_level(&mut self, value: u16) {
        self.runs_confidence_level = value;
    }

    pub fn get_sha256_reduction_ratio(&self) -> u16 {
        self.sha256_reduction_ratio
    }

    pub fn set_sha256_reduction_ratio(&mut self, value: u16) {
        self.sha256_reduction_ratio = value;
    }

    pub fn initialize_run_settings() -> Result<(), RapLibErrors> {
        let saved_settings: Result<RunSettings, RapLibErrors> =
            RunSettings::read_run_settings_from_file();
        match saved_settings {
            Ok(arg) => RUN_SETTINGS.lock().unwrap().clone_from(&arg),
            Err(arg) => {
                println!(
                    "Run Settings file not found. Resetting to default. Error msg: {:?}.",
                    arg
                );
                RunSettings::reset_default_settings()?;
            }
        }
        Ok(())
    }

    pub fn set_run_settings(self) -> Result<(), RapLibErrors> {
        self.check_run_settings_validity();
        RUN_SETTINGS.lock().unwrap().clone_from(&self);
        RunSettings::write_settings_to_file();
        Ok(())
    }

    pub fn get_run_settings() -> Result<RunSettings, RapLibErrors> {
        Ok(*RUN_SETTINGS.lock().unwrap())
    }

    pub fn reset_default_settings() -> Result<(), RapLibErrors> {
        RunSettings::default().set_run_settings()?;
        RunSettings::write_settings_to_file();
        Ok(())
    }

    fn read_run_settings_from_file() -> Result<RunSettings, RapLibErrors> {
        let run_settings_path: &str = "./run_settings.bin";
        let mut read_file = OpenOptions::new().read(true).open(run_settings_path)?;

        Ok(read_file
            .read_binary::<RunSettings>()
            .expect("Failed to unwrap binary run_settings.bin"))
    }

    fn write_settings_to_file() {
        let run_settings: RunSettings = *RUN_SETTINGS.lock().unwrap();
        let run_settings_path: &str = "./run_settings.bin";
        let mut write_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(run_settings_path)
            .expect("Failed to open run_settings.bin");

        write_file
            .write_binary(&run_settings)
            .expect("Cannot write run_settings to file.");
    }

    fn check_run_settings_validity(self) -> Result<(), RapLibErrors> {
        let mut msg: String = "".to_string();
        let mut err: bool = false;

        if self.num_of_dwords % 0xff != 0 {
            err = true;
            msg += "- num_of_dwords must be a multiple of 0xff.\n";
        }

        if self.mono_sequence_length_power_of_2 < 2 || self.mono_sequence_length_power_of_2 > 11 {
            err = true;
            msg += "- mono_sequence_length_power_of_2 out of range: it must be within range <2; 11>.\n";
        }

        if self.mono_num_of_sequences_power_of_2 < 2 || self.mono_num_of_sequences_power_of_2 > 21 {
            err = true;
            msg += "- mono_num_of_sequences_power_of_2 out of range: it must be within range <2; 21>. \n";
        }

        if (self.mono_sequence_length_power_of_2 % 2 == 0
            && self.mono_num_of_sequences_power_of_2 % 2 != 0)
            || (self.mono_sequence_length_power_of_2 % 2 != 0
                && self.mono_num_of_sequences_power_of_2 % 2 == 0)
        {
            err = true;
            msg += "- mono_num_of_sequences_power_of_2 and mono_sequence_length_power_of_2 need to both be either even or odd. \n";
        }

        if self.mono_confidence_level_upper > 16 {
            err = true;
            msg +=
                "- mono_confidence_level_upper out of range: it must be within range <0; 16>. \n";
        }

        if self.mono_confidence_level_lower > 16 {
            err = true;
            msg +=
                "- mono_confidence_level_lower out of range: it must be within range <0; 16>. \n";
        }

        if self.sanity_fail_flag_latch_event_alarm_thr < 1
            || self.sanity_fail_flag_latch_event_alarm_thr > 15
        {
            err = true;
            msg += "- sanity_fail_flag_latch_event_alarm_thr out of range: it must be within range <1; 15>. \n";
        }

        if self.runs_sequence_length < 5 || self.runs_sequence_length > 11 {
            err = true;
            msg += "- runs_sequence_length out of range: it must be within range <5; 11>. \n";
        }

        if self.runs_confidence_level > 0x3fff {
            err = true;
            msg += "- runs_confidence_level out of range: it must be within range <0; 0x3fff>. \n";
        }

        if self.sha256_reduction_ratio % 2 == 1 {
            err = true;
            msg += "- sha256_red_ratio must be of even values.\n";
        }

        if err {
            return Err(RapLibErrors::RunSettingsError(msg.to_string()));
        }

        Ok(())
    }
}

impl Default for RunSettings {
    fn default() -> Self {
        RunSettings {
            num_of_dwords: 0xffc0,
            afp_threshold: 50,
            sanity_fail_flag_latch_event_alarm_thr: 3,

            mono_num_of_sequences_power_of_2: 8,
            mono_sequence_length_power_of_2: 6,
            mono_confidence_level_upper: 3,
            mono_confidence_level_lower: 3,

            asym_sequence_length_bits: 600,

            runs_sequence_length: 5,
            runs_num_of_sequences_power_of_2: 8,
            runs_confidence_level: 3,

            sha256_reduction_ratio: 16,
        }
    }
}

impl PartialEq for RunSettings {
    fn eq(&self, other: &Self) -> bool {
        self.num_of_dwords == other.num_of_dwords &&
        self.afp_threshold == other.afp_threshold &&
        self.sanity_fail_flag_latch_event_alarm_thr == other.sanity_fail_flag_latch_event_alarm_thr &&
        self.mono_num_of_sequences_power_of_2 == other.mono_num_of_sequences_power_of_2 &&
        self.mono_sequence_length_power_of_2 == other.mono_sequence_length_power_of_2 &&
        self.mono_confidence_level_upper == other.mono_confidence_level_upper &&
        self.mono_confidence_level_lower == other.mono_confidence_level_lower &&
        self.asym_sequence_length_bits == other.asym_sequence_length_bits &&
        self.runs_sequence_length == other.runs_sequence_length &&
        self.runs_num_of_sequences_power_of_2 == other.runs_num_of_sequences_power_of_2 &&
        self.runs_confidence_level == other.runs_confidence_level &&
        self.sha256_reduction_ratio == other.sha256_reduction_ratio
    }
}