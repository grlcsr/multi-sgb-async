use binext::{BinaryRead, BinaryWrite};
use lazy_static::lazy_static;
use std::fs::OpenOptions;
use std::sync::Mutex;

use super::RapLibErrors;

pub(crate) const MAXIMUM_NUM_OF_DWORDS: usize = 0xffff;

lazy_static! {
    static ref RUN_SETTINGS: Mutex<RunSettings> = Mutex::new(RunSettings::default());
    static ref HW_LIMITS: Mutex<HwLimits> = Mutex::new(HwLimits::new());
}

#[derive(Debug, Copy, Clone)]
pub struct RunSettings {
    num_of_dwords: u16, // package size in dwords - number of 32 bit words, hardware limit is 0xffff, has to be a multiple of 0xff
    afp_threshold: u16,

    // ------------------------------------------
    // -------------- ACCELERATORS --------------
    // ------------------------------------------
    sanity_fail_flag_latch_event_alarm_thr: u16, // [1..15]

    // -------------- MONOBIT --------------        bit consumption:     (2^MONO_numOfSequencesPowerOf2 * 2^MONO_sequenceLengthPowerOf2)
    mono_active: bool,
    mono_num_of_sequences_power_of_2: u16, // [2...21]        greater than MONO_sequenceLengthPowerOf2, both have to be odd or even
    mono_sequence_length_power_of_2: u16,  // [2...11]
    mono_confidence_level_upper: u16,      // [1...16383]     k upper
    mono_confidence_level_lower: u16,      // [1...16383]     k lower

    // -------------- ASYMMETRY --------------      bit consumption:     (ASYM_sequenceLength_bits)
    asym_active: bool,
    asym_sequence_length_bits: u16, // [12, 16, 20, 24, ..., 2^30], has to be a multiple of 4

    // -------------- RUNS --------------           bit consumption:     (2^sequenceLength_RUNS * 2^numOfSequencesPowerOf2_RUNS)
    runs_active: bool,
    runs_sequence_length: u16,             // [5...11]
    runs_num_of_sequences_power_of_2: u16, // [2...21]
    runs_confidence_level: u16,            // [0...16383]

    // -------------- SHA256 --------------         bit consumption:     (512 * red_ratio/2)
    sha256_active: bool,
    sha256_reduction_ratio: u16, // [2, 4, 6, 8, ..., 32]
}

impl Default for RunSettings {
    fn default() -> Self {
        RunSettings {
            num_of_dwords: 0xffc0,
            afp_threshold: 50,
            sanity_fail_flag_latch_event_alarm_thr: 3,

            mono_active: true,
            mono_num_of_sequences_power_of_2: 8,
            mono_sequence_length_power_of_2: 6,
            mono_confidence_level_upper: 3,
            mono_confidence_level_lower: 3,

            asym_active: true,
            asym_sequence_length_bits: 600,

            runs_active: true,
            runs_sequence_length: 5,
            runs_num_of_sequences_power_of_2: 8,
            runs_confidence_level: 3,

            sha256_active: true,
            sha256_reduction_ratio: 16,
        }
    }
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

    pub fn get_mono(&self) -> bool {
        self.mono_active
    }

    pub fn set_mono(&mut self, active: bool) {
        self.mono_active = active
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

    pub fn get_asym(&self) -> bool {
        self.asym_active
    }

    pub fn set_asym(&mut self, active: bool) {
        self.asym_active = active
    }

    pub fn get_asym_sequence_length_bits(&self) -> u16 {
        self.asym_sequence_length_bits
    }

    pub fn set_asym_sequence_length_bits(&mut self, value: u16) {
        self.asym_sequence_length_bits = value;
    }

    pub fn get_runs(&self) -> bool {
        self.runs_active
    }

    pub fn set_runs(&mut self, active: bool) {
        self.runs_active = active
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

    pub fn get_sha256(&self) -> bool {
        self.sha256_active
    }

    pub fn set_sha256(&mut self, active: bool) {
        self.sha256_active = active
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

    pub fn set_run_settings(&mut self) -> Result<(), RapLibErrors> {
        match self.check_run_settings_validity() {
            Ok(_) => {
                RUN_SETTINGS.lock().unwrap().clone_from(&self);
                RunSettings::write_settings_to_file()?;
            }
            Err(f) => println!("{}", f),
        }
        Ok(())
    }

    pub fn get_run_settings() -> Result<RunSettings, RapLibErrors> {
        Ok(*RUN_SETTINGS.lock().unwrap())
    }

    pub fn reset_default_settings() -> Result<(), RapLibErrors> {
        RunSettings::default().set_run_settings()?;
        RunSettings::write_settings_to_file()?;
        Ok(())
    }

    fn read_run_settings_from_file() -> Result<RunSettings, RapLibErrors> {
        let run_settings_path: &str = "./run_settings.bin";
        if let Ok(mut read_file) = OpenOptions::new().read(true).open(run_settings_path) {
            if let Ok(run_settings) = read_file.read_binary::<RunSettings>() {
                Ok(run_settings)
            } else {
                Err(RapLibErrors::SettingsError(
                    "Cannot read run_settings.bin".to_string(),
                ))
            }
        } else {
            Err(RapLibErrors::SettingsError(
                "Cannot read run_settings.bin".to_string(),
            ))
        }
    }

    fn write_settings_to_file() -> Result<(), RapLibErrors> {
        let run_settings: RunSettings = *RUN_SETTINGS.lock().unwrap();
        let run_settings_path: &str = "./run_settings.bin";
        let mut write_file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(run_settings_path)
            .map_err(|err| {
                RapLibErrors::SettingsError(format!(
                    "Failed to open run_settings.bin. Error code: {:?}",
                    err
                ))
            })?;

        write_file.write_binary(&run_settings).map_err(|err| {
            RapLibErrors::SettingsError(format!(
                "Cannot write run_settings to file. Error code: {:?}",
                err
            ))
        })?;
        Ok(())
    }

    fn check_run_settings_validity(&mut self) -> Result<(), RapLibErrors> {
        let mut msg: String = "".to_string();
        let mut err: bool = false;

        if self.num_of_dwords % 0x100 != 0 {
            println!("- num_of_dwords must be a multiple of 0x100. Computing value.");
            let optimal_dwords = self.calculate_optimal_num_of_dwords()?;
            self.set_num_of_dwords(optimal_dwords);
            println!("- num_of_dwords value computed to be (0x{:04x})", optimal_dwords);
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
            return Err(RapLibErrors::SettingsError(msg.to_string()));
        }

        Ok(())
    }

    fn calculate_optimal_num_of_dwords(& mut self) -> Result<u16, RapLibErrors> {
        let mut step = 0x100;

        let bits_per_entry_mono = 2_i32.pow(self.get_mono_sequence_length_power_of_2() as u32)
            * 2_i32.pow(self.get_mono_num_of_sequences_power_of_2() as u32);
        let bits_per_entry_runs = 22_i32.pow(self.get_runs_sequence_length() as u32)
            * 2_i32.pow(self.get_runs_num_of_sequences_power_of_2() as u32);
        let bits_per_entry_asym = 4 * self.get_asym_sequence_length_bits() as i32;
        let bits_per_entry_sha256 = 256 * self.get_sha256_reduction_ratio() as i32;

        let mono_words_per_entry = 1;
        let runs_words_per_entry = 1;
        let asym_words_per_entry = 1;
        let sha256_words_per_entry = 8;

        let mut current_dw = 0x0000;

        let mut limit_due_to_mono = false;
        let mut limit_due_to_runs = false;
        let mut limit_due_to_asym = false;
        let mut limit_due_to_sha256 = false;
        let mut limit_due_to_hw = false;

        let mut state = "search_coarse";

        let mut generated_bits: i32;
        let mut mono_fifo_usage: i32;
        let mut runs_fifo_usage: i32;
        let mut asym_fifo_usage: i32;
        let mut sha256_fifo_usage: i32;

        loop {
            let mut limit_reached = false;
            generated_bits = current_dw * 32;
            mono_fifo_usage = generated_bits / bits_per_entry_mono * mono_words_per_entry;
            runs_fifo_usage = generated_bits / bits_per_entry_runs * runs_words_per_entry;
            asym_fifo_usage = generated_bits / bits_per_entry_asym * asym_words_per_entry;
            sha256_fifo_usage = generated_bits / bits_per_entry_sha256 * sha256_words_per_entry;

            if mono_fifo_usage > (HwLimits::get_hw_limits()?).mono() {
                // && enabled_accelerators_dict["mono"]["acq"]:
                limit_due_to_mono = true;
                limit_reached = true;
            }
            if runs_fifo_usage > (HwLimits::get_hw_limits()?).runs() {
                // && enabled_accelerators_dict["runs"]["acq"]:
                limit_due_to_runs = true;
                limit_reached = true;
            }
            if asym_fifo_usage > (HwLimits::get_hw_limits()?).asym() {
                // && enabled_accelerators_dict["asym"]["acq"]:
                limit_due_to_asym = true;
                limit_reached = true;
            }

            if sha256_fifo_usage > (HwLimits::get_hw_limits()?).sha256() {
                // &&enabled_accelerators_dict["sha256"]["acq_save"]:
                limit_due_to_sha256 = true;
                limit_reached = true;
            }

            if current_dw > MAXIMUM_NUM_OF_DWORDS as i32 {
                limit_due_to_hw = true;
                limit_reached = true;
            }

            if state == "search_coarse" && limit_reached {
                state = "backsearch_fine";
                step = -step;
            }

            if state == "backsearch_fine" && !limit_reached {
                break;
            }

            current_dw += step
        }

        println!(
            "Found max number of dwords: {} (0x{:04x})",
            current_dw, current_dw,
        );

        fn is_active(val: bool) -> String {
            if val {
                return "+".to_string();
            }
            return " ".to_string();
        }

        println!(
            " + HW    : {},\tusage: {},\tlimit: {}",
            limit_due_to_hw, current_dw, MAXIMUM_NUM_OF_DWORDS
        );
        println!(
            " {} SHA256: {},\tusage: {},\tlimit: {}",
            is_active(self.sha256_active),
            limit_due_to_sha256,
            sha256_fifo_usage,
            (HwLimits::get_hw_limits()?).sha256()
        );
        println!(
            " {} MONO  : {},\tusage: {},\tlimit: {}",
            is_active(self.mono_active),
            limit_due_to_mono,
            mono_fifo_usage,
            (HwLimits::get_hw_limits()?).mono()
        );
        println!(
            " {} RUNS  : {},\tusage: {},\tlimit: {}",
            is_active(self.runs_active),
            limit_due_to_runs,
            runs_fifo_usage,
            (HwLimits::get_hw_limits()?).runs()
        );
        println!(
            " {} ASYM  : {},\tusage: {},\tlimit: {}",
            is_active(self.asym_active),
            limit_due_to_asym,
            asym_fifo_usage,
            (HwLimits::get_hw_limits()?).asym()
        );

        Ok(current_dw as u16)
    }
}

impl PartialEq for RunSettings {
    fn eq(&self, other: &Self) -> bool {
        self.num_of_dwords == other.num_of_dwords
            && self.afp_threshold == other.afp_threshold
            && self.sanity_fail_flag_latch_event_alarm_thr
                == other.sanity_fail_flag_latch_event_alarm_thr
            && self.mono_num_of_sequences_power_of_2 == other.mono_num_of_sequences_power_of_2
            && self.mono_sequence_length_power_of_2 == other.mono_sequence_length_power_of_2
            && self.mono_confidence_level_upper == other.mono_confidence_level_upper
            && self.mono_confidence_level_lower == other.mono_confidence_level_lower
            && self.asym_sequence_length_bits == other.asym_sequence_length_bits
            && self.runs_sequence_length == other.runs_sequence_length
            && self.runs_num_of_sequences_power_of_2 == other.runs_num_of_sequences_power_of_2
            && self.runs_confidence_level == other.runs_confidence_level
            && self.sha256_reduction_ratio == other.sha256_reduction_ratio
    }
}

#[derive(Copy, Clone, Debug)]
pub struct HwLimits {
    sha256_fifo: i32, // minus 2 full entries for safety
    mono_fifo: i32,   // minus 4 full entries for safety
    runs_fifo: i32,   // minus 4 full entries for safety
    asym_fifo: i32,   // minus 4 full entries for safety
}

impl HwLimits {
    pub const fn new() -> Self {
        Self {
            sha256_fifo: 512 - 8 * 2,
            mono_fifo: 1024 - 4,
            runs_fifo: 1024 - 4,
            asym_fifo: 1024 - 4,
        }
    }

    pub fn get_hw_limits() -> Result<HwLimits, RapLibErrors> {
        Ok(*HW_LIMITS.lock().unwrap())
    }

    pub fn sha256(&self) -> i32 {
        self.sha256_fifo
    }

    pub fn mono(&self) -> i32 {
        self.mono_fifo
    }

    pub fn runs(&self) -> i32 {
        self.runs_fifo
    }

    pub fn asym(&self) -> i32 {
        self.asym_fifo
    }
}
