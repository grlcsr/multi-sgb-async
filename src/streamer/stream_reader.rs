use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio_stream::{Stream, StreamExt};

use super::{base, global_data::*, sanity_checks, sha256, FlashData, FtdiBoard};

// TODO: HANDLING OF ERRORS -> PROPAGATE BACK TO MOD.RS AND IN CASE OF ERROR SHUT DOWN STREAM

#[derive(Debug, Clone)]
pub struct PacketGenerator<'a, 'b> {
    serial_number: String,
    board: &'a FtdiBoard,
    channel: &'b mpsc::Sender<StreamData>,

    max_dwords: u16,
    last_poll_time: Instant,
    delay: Duration,
    timeout: Duration,
}

impl<'a, 'b> PacketGenerator<'a, 'b> {
    pub fn new(
        serial_number: String,
        board: &'a FtdiBoard,
        channel: &'b mpsc::Sender<StreamData>,
        max_dwords: u16,
    ) -> Self {
        Self {
            serial_number,
            board,
            channel,
            max_dwords,

            delay: Duration::from_millis(1),
            timeout: Duration::from_secs(1),
            last_poll_time: Instant::now(),
        }
    }

    pub async fn generate_packet(&mut self) {
        base::request_raw_tdc_words(self.board, self.max_dwords);
        let mut num_seeds = self.max_dwords as i32 * 4 / SEED_LENGTH as i32;

        loop {
            if num_seeds == 0 {
                break;
            }

            match self.next().await {
                //TODO! Change to try next and return Result<usize, Err>
                Some(read_buf) => {
                    let _nist_tests = self.nist_tests(&read_buf).await;
                    let stream_results = StreamData {
                        serial: self.serial_number.clone(),
                        data: Some(DataType::RawStream(RawStream::new(
                            read_buf,
                            _nist_tests[0],
                            _nist_tests[1],
                        ))),
                    };

                    self.channel.send(stream_results).await;

                    num_seeds -= 1;
                    println!("Missing seeds: {}", num_seeds);
                }
                None => break,
            }
        }
    }

    async fn nist_tests(&mut self, raw_bits: &[u8; BUFFER_SIZE]) -> [bool; 2] {
        let apt_init_sym = raw_bits[0] >> 4;
        let mut rct_prev = apt_init_sym;

        let mut apt_count = 0;
        let mut rct_count = 0;

        let mut apt_fail = false;
        let mut rct_fail = false;

        for sym in raw_bits {
            // RCT
            if (sym >> 4) == rct_prev {
                rct_count += 1;

                if rct_count >= RCT_THR {
                    rct_fail = true;
                }
            } else {
                rct_count = 0;
            }

            rct_prev = sym >> 4;

            if (sym & 15) == rct_prev {
                rct_count += 1;

                if rct_count >= RCT_THR {
                    rct_fail = true;
                }
            } else {
                rct_count = 0;
            }

            rct_prev = sym & 15;

            // APT
            if (sym >> 4) == apt_init_sym {
                apt_count += 1;
            }

            if (sym & 15) == apt_init_sym {
                apt_count += 1;
            }
        }

        if apt_count >= APT_THR_UP || apt_count <= APT_THR_DOWN {
            apt_fail = true;
        }

        [rct_fail, apt_fail]
    }
}

impl<'a, 'b> Stream for PacketGenerator<'a, 'b> {
    type Item = [u8; BUFFER_SIZE];

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if Instant::now().duration_since(self.last_poll_time) > self.timeout {
            println!("Generating packet: Timeout exceeded!!");
            return Poll::Ready(None);
        }

        let rx = self.board.get_queue_status().unwrap();
        if rx >= 0x100 {
            let mut read_buf: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];
            let _bytes_read = self.board.read(&mut read_buf).unwrap();

            self.last_poll_time = Instant::now();
            return Poll::Ready(Some(read_buf));
        }

        let waker = cx.waker().clone();
        let delay = self.delay;
        tokio::spawn(async move {
            tokio::time::sleep(delay).await;
            waker.wake();
        });

        cx.waker().wake_by_ref();
        Poll::Pending
    }
}

#[derive(Debug, Clone)]
pub struct FifoReader<'a, 'b> {
    serial_number: String,
    board: &'a FtdiBoard,
    channel: &'b mpsc::Sender<StreamData>,
}

impl<'a, 'b> FifoReader<'a, 'b> {
    pub fn new(
        serial_number: String,
        board: &'a FtdiBoard,
        channel: &'b mpsc::Sender<StreamData>,
    ) -> Self {
        Self {
            serial_number,
            board,
            channel,
        }
    }

    pub async fn read_fifo_results(&mut self) {
        let asym = self.request_asymmetry_results().await;
        let asym_results = StreamData {
            serial: self.serial_number.clone(),
            data: Some(DataType::Asym(asym)),
        };
        println!("{:?}", asym_results);
        self.channel.send(asym_results).await;
        
        let monobit = self.request_monobit_results().await;
        let monobit_results = StreamData {
            serial: self.serial_number.clone(),
            data: Some(DataType::Monobit(monobit)),
        };
        self.channel.send(monobit_results).await;
        
        let runs = self.request_runs_results().await;
        let runs_results = StreamData {
            serial: self.serial_number.clone(),
            data: Some(DataType::Runs(runs)),
        };
        self.channel.send(runs_results).await;

        let sha = self.request_sha256_results().await;
        let sha_results = StreamData {
            serial: self.serial_number.clone(),
            data: Some(DataType::Sha256(sha)),
        };
        self.channel.send(sha_results).await;
    }

    async fn request_asymmetry_results(&mut self) -> Vec<i32> {
        let mut asym_buffer: Vec<i32> = Vec::new();
        sanity_checks::req_read_asym_fifo(self.board);

        loop {
            let value_read = async { self.board.read_32_bit_u32().unwrap() }.await;

            if (value_read & 0x80000000) == 0 {
                asym_buffer.push(sanity_checks::signed_int_to_dec(value_read));
            } else {
                break;
            }
        }
        asym_buffer
    }

    async fn request_monobit_results(&mut self) -> Vec<(f32, u32, u32)> {
        let mut monobit_buffer: Vec<(f32, u32, u32)> = Vec::new();
        sanity_checks::req_read_monobit_fifo(self.board);

        loop {
            let value_read: u32 = async { self.board.read_32_bit_u32().unwrap() }.await;

            if (value_read & 0x80000000) == 0 {
                let sn_mean_value: f32 =
                    sanity_checks::fxp_to_flp_smpl((value_read & 0x1ffffff) as i32, 10.0);
                let fail_flag: u32 = (value_read >> 25) & 0xf;
                let fail_flag_latch: u32 = (value_read >> 29) & 0x1;

                monobit_buffer.push((sn_mean_value, fail_flag, fail_flag_latch));
            } else {
                break;
            }
        }
        monobit_buffer
    }

    async fn request_runs_results(&mut self) -> Vec<(f64, u32, u32)> {
        let mut runs_buffer: Vec<(f64, u32, u32)> = Vec::new();
        sanity_checks::req_read_runs_fifo(self.board);

        loop {
            let value_read: u64 = async { self.board.read_64_bit_u64().unwrap() }.await;

            if (value_read & 0x8000000000000000) == 0 {
                let signed_z_val_fxp: u64 = value_read & 0xFFFFFFFFFFFFF;

                let z_value: f64 = sanity_checks::fixed_to_float(signed_z_val_fxp, 52, 44);
                let fail_flag: u32 = ((value_read & 0x3c0000000000000) >> (13 * 4 + 2)) as u32;
                let fail_flag_latch: u32 = ((value_read & 0x10000000000000) >> (13 * 4)) as u32;

                runs_buffer.push((z_value, fail_flag, fail_flag_latch));
            } else {
                break;
            }
        }
        runs_buffer
    }

    async fn request_sha256_results(&mut self) -> Vec<u8> {
        sha256::req_read_sha256_fifo(self.board);
        let words_in_fpga: usize = self.board.read_32_bit_u32().unwrap() as usize;
        let mut sha_results: [u8; MAXIMUM_NUM_OF_DWORDS*4] = [0; MAXIMUM_NUM_OF_DWORDS*4];
        let _ = self.board.read(&mut sha_results);
        sha_results[..words_in_fpga].to_vec()
    }
}

#[derive(Debug, Clone)]
pub struct FlushDevice<'a> {
    board: &'a FtdiBoard,

    last_poll_time: Instant,
    timeout: Duration,
}

impl<'a> FlushDevice<'a> {
    pub fn new(board: &'a FtdiBoard, timeout: Duration) -> Self {
        Self {
            board,
            timeout,
            last_poll_time: Instant::now(),
        }
    }

    pub async fn flush_device(&mut self) -> usize {
        let mut total_cleaned_bytes: usize = 0;

        loop {
            match self.next().await {
                //TODO! Change to try next and return Result<usize, Err>
                Some(read_bytes) => total_cleaned_bytes += read_bytes,
                None => break,
            }
        }
        total_cleaned_bytes
    }
}

impl<'a> Stream for FlushDevice<'a> {
    type Item = usize;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if Instant::now().duration_since(self.last_poll_time) > self.timeout {
            println!("Cleaning buffer: Timeout exceeded!!");
            return Poll::Ready(None);
        }

        if self.board.get_queue_status().unwrap() > 0 {
            let mut read_buf: [u8; BUFFER_SIZE_FLUSHING] = [0; BUFFER_SIZE_FLUSHING];
            let bytes_read = self.board.read(&mut read_buf).unwrap();

            self.last_poll_time = Instant::now();
            return Poll::Ready(Some(bytes_read));
        }

        cx.waker().wake_by_ref();
        Poll::Pending
    }
}

pub struct TemperatureStabilizer<'a, 'b> {
    board: &'a FtdiBoard,
    flash_data: &'b mut FlashData,

    last_poll_time: Instant,
    delay: Duration,
    timeout: Duration,
}

impl<'a, 'b> TemperatureStabilizer<'a, 'b> {
    pub fn new(board: &'a FtdiBoard, flash_data: &'b mut FlashData, timeout: Duration) -> Self {
        Self {
            board,
            flash_data,
            timeout,
            delay: Duration::from_millis(2),
            last_poll_time: Instant::now(),
        }
    }

    pub async fn perform_temperature_stabilization(&mut self) {
        let mut temperature_now: f32 = base::req_temperature(self.board).unwrap();
        let mut delta_t = f32::abs(self.flash_data.get_ref_temp() - temperature_now);

        println!(
            "Initial values: temperature_now = {}; ref_temperature = {}; delta_t = {}",
            temperature_now,
            self.flash_data.get_ref_temp(),
            delta_t
        );

        self.set_gate_dcr();

        while delta_t > 0.5 {
            async {
                let temperature_old: f32 = temperature_now;
                let mut dcr_now: f32 = 0.0;

                for _i in 0..5 {
                    base::req_read_dcr(self.board);
                    match self.next().await {
                        //TODO! Change to try next and return Result<usize, Err>
                        Some(dcr) => {
                            println!("Temperature stabilization iteration {}; got DCR: {}.", _i, dcr);
                            dcr_now = dcr as f32 / 10000.0;
                        }
                        None => break, // TODO this should not happen
                    }
                }

                temperature_now = base::req_temperature(self.board).unwrap();
                delta_t = temperature_now - temperature_old;
                println!("Temperature stabilization: DCR [KHz] = {}; temperature_old = {}; temperature_now = {}, delta_t = {}.", dcr_now, temperature_old, temperature_now, delta_t);
            }.await
        }

        self.flash_data.set_ref_temp(temperature_now);
    }

    fn set_gate_dcr(&mut self) {
        // read the DCR; 2 = 1 second gate for pulse counting
        //               1 = 10 seconds
        let value = 1;
        base::set_gate_dcr(self.board, value);
    }
}

impl<'a, 'b> Stream for TemperatureStabilizer<'a, 'b> {
    type Item = u32;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if Instant::now().duration_since(self.last_poll_time) > self.timeout {
            println!("Temperature Stabilization: Timeout Exceeded!!");
            return Poll::Ready(None);
        }

        let dword: u32 = self.board.read_32_bit_u32().unwrap();
        if dword > 0 {
            self.last_poll_time = Instant::now();
            return Poll::Ready(Some(dword));
        }

        let waker = cx.waker().clone();
        let delay = self.delay;
        tokio::spawn(async move {
            tokio::time::sleep(delay).await;
            waker.wake();
        });

        cx.waker().wake_by_ref();
        Poll::Pending
    }
}
