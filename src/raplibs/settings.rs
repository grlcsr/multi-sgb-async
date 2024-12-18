use std::sync::Mutex;
use std::fs::OpenOptions;
use lazy_static::lazy_static;
use binext::{BinaryRead, BinaryWrite};

use super::RapLibErrors;

#[derive(Debug, Copy, Clone)]
pub struct RunSettings {
    pub num_of_dwords: u16,                     // package size in dwords - number of 32 bit words, hardware limit is 0xffff, has to be a multiple of 0xff
    pub afp_threshold: u16,

    // ------------------------------------------
    // -------------- ACCELERATORS --------------
    // ------------------------------------------
    pub sanity_fail_flag_latch_event_alarm_thr: u16,  // [1..15]

    // -------------- MONOBIT --------------        bit consumption:     (2^MONO_numOfSequencesPowerOf2 * 2^MONO_sequenceLengthPowerOf2)
    pub mono_num_of_sequences_power_of_2: u16,       // [2...21]        greater than MONO_sequenceLengthPowerOf2, both have to be odd or even
    pub mono_sequence_length_power_of_2: u16,        // [2...11]
    pub mono_confidence_level_upper: u16,            // [1...16383]     k upper
    pub mono_confidence_level_lower: u16,            // [1...16383]     k lower
 
    // -------------- ASYMMETRY --------------      bit consumption:     (ASYM_sequenceLength_bits)
    pub asym_sequence_length_bits: u16,              // [12, 16, 20, 24, ..., 2^30], has to be a multiple of 4
 
    // -------------- RUNS --------------           bit consumption:     (2^sequenceLength_RUNS * 2^numOfSequencesPowerOf2_RUNS)
    pub runs_sequence_length: u16,                   // [5...11]
    pub runs_num_of_sequences_power_of_2: u16,       // [2...21]
    pub runs_confidence_level: u16,                  // [0...16383]
 
    // -------------- SHA256 --------------         bit consumption:     (512 * red_ratio/2)
    pub sha256_reduction_ratio: u16,                 // [2, 4, 6, 8, ..., 32]
}

lazy_static! {
    static ref RUN_SETTINGS: Mutex<RunSettings> = Mutex::new(
        {
            RunSettings::default()
            });
}

impl RunSettings {

    pub fn initialize_run_settings() -> Result<(), RapLibErrors> {
        let saved_settings: Result<RunSettings, RapLibErrors> = RunSettings::read_run_settings_from_file();
        match saved_settings {
            Ok(arg) => RUN_SETTINGS.lock().unwrap().clone_from(&arg),
            Err(arg) => {
                println!("Run Settings file not found. Resetting to default. Error msg: {:?}.", arg);
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
        let mut read_file = OpenOptions::new()
            .read(true)
            .open(run_settings_path)?;
        
        Ok(read_file.read_binary::<RunSettings>()
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

        write_file.write_binary(&run_settings)
            .expect("Cannot write run_settings to file.");
    }

    fn check_run_settings_validity(self) {
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
        
        if (self.mono_sequence_length_power_of_2 % 2 == 0 &&
            self.mono_num_of_sequences_power_of_2 % 2 != 0) ||
           (self.mono_sequence_length_power_of_2 % 2 != 0 &&
            self.mono_num_of_sequences_power_of_2 % 2 == 0) {
            err = true;
            msg += "- mono_num_of_sequences_power_of_2 and mono_sequence_length_power_of_2 need to both be either even or odd. \n";
        }

        if self.mono_confidence_level_upper > 16 {
            err = true;
            msg += "- mono_confidence_level_upper out of range: it must be within range <0; 16>. \n";
        }
        
        if self.mono_confidence_level_lower > 16 {
            err = true;
            msg += "- mono_confidence_level_lower out of range: it must be within range <0; 16>. \n";
        }

        if self.sanity_fail_flag_latch_event_alarm_thr < 1 || self.sanity_fail_flag_latch_event_alarm_thr > 15 {
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
            panic!("{:}", msg);
        }
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