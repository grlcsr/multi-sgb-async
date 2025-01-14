use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::{Duration, Instant},
};
use tokio::sync::mpsc;
use tokio_stream::{Stream, StreamExt};

use super::{base, global_data::*, sanity_checks, sha256, FlashData, FtdiBoard};
use crate::raplibs::{settings::MAXIMUM_NUM_OF_DWORDS, RapLibErrors};

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

    pub async fn generate_packet(&mut self) -> Result<(), RapLibErrors> {
        base::request_raw_tdc_words(self.board, self.max_dwords)?;
        let mut num_seeds = self.max_dwords as i32 * 4 / SEED_LENGTH as i32;

        loop {
            if num_seeds == 0 {
                return Ok(());
            }

            match self.try_next().await? {
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

                    match self.channel.send(stream_results).await {
                        Ok(_) => {
                            num_seeds -= 1;
                            println!("Missing seeds: {}", num_seeds);
                        }
                        Err(_) => {
                            return Err(RapLibErrors::UnhandledError(
                                "generate_packet: unhandled error while generating.".to_string(),
                            ))
                        }
                    }
                }
                None => {
                    return Err(RapLibErrors::StreamerError(
                        "Generation timed out.".to_string(),
                    ))
                }
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
    type Item = Result<[u8; BUFFER_SIZE], RapLibErrors>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if Instant::now().duration_since(self.last_poll_time) > self.timeout {
            println!("Generating packet: Timeout exceeded!!");
            return Poll::Ready(None);
        }

        let rx = self.board.get_queue_status()?;
        if rx >= 0x100 {
            let mut read_buf: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];
            let _bytes_read = self.board.read(&mut read_buf)?;

            self.last_poll_time = Instant::now();
            return Poll::Ready(Some(Ok(read_buf)));
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

    pub async fn read_fifo_results(&mut self) -> Result<(), RapLibErrors> {
        async fn send_result<T: Into<DataType>>(
            serial: &str,
            request: impl Future<Output = Result<T, RapLibErrors>>,
            channel: &mpsc::Sender<StreamData>,
        ) -> Result<(), RapLibErrors> {
            let result = request.await?;
            let stream_data = StreamData {
                serial: serial.to_string(),
                data: Some(result.into()),
            };
            channel.send(stream_data).await.map_err(|f| {
                RapLibErrors::UnhandledError(format!("Unhandled external error: {:?}", f))
            })
        }

        send_result(
            &self.serial_number,
            self.request_asymmetry_results(),
            &self.channel,
        )
        .await?;
        send_result(
            &self.serial_number,
            self.request_monobit_results(),
            &self.channel,
        )
        .await?;
        send_result(
            &self.serial_number,
            self.request_runs_results(),
            &self.channel,
        )
        .await?;
        send_result(
            &self.serial_number,
            self.request_sha256_results(),
            &self.channel,
        )
        .await?;

        Ok(())
    }

    async fn request_asymmetry_results(&self) -> Result<Vec<i32>, RapLibErrors> {
        let mut asym_buffer: Vec<i32> = Vec::new();
        sanity_checks::req_read_asym_fifo(self.board)?;

        while let Ok(value_read) = async { self.board.read_32_bit_u32() }.await {
            if (value_read & 0x80000000) == 0 {
                asym_buffer.push(sanity_checks::signed_int_to_dec(value_read));
            } else {
                break;
            }
        }
        Ok(asym_buffer)
    }

    async fn request_monobit_results(&self) -> Result<Vec<(f32, u32, u32)>, RapLibErrors> {
        let mut monobit_buffer: Vec<(f32, u32, u32)> = Vec::new();
        sanity_checks::req_read_monobit_fifo(self.board)?;

        while let Ok(value_read) = async { self.board.read_32_bit_u32() }.await {
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
        Ok(monobit_buffer)
    }

    async fn request_runs_results(&self) -> Result<Vec<(f64, u32, u32)>, RapLibErrors> {
        let mut runs_buffer: Vec<(f64, u32, u32)> = Vec::new();
        sanity_checks::req_read_runs_fifo(self.board)?;

        while let Ok(value_read) = async { self.board.read_64_bit_u64() }.await {
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
        Ok(runs_buffer)
    }

    async fn request_sha256_results(&self) -> Result<Vec<u8>, RapLibErrors> {
        sha256::req_read_sha256_fifo(self.board)?;
        let words_in_fpga: usize = self.board.read_32_bit_u32()? as usize;
        let mut sha_results: [u8; MAXIMUM_NUM_OF_DWORDS * 4] = [0; MAXIMUM_NUM_OF_DWORDS * 4];
        let _ = self.board.read(&mut sha_results)?;
        Ok(sha_results[..words_in_fpga].to_vec())
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

    pub async fn flush_device(&mut self) -> Result<usize, RapLibErrors> {
        let mut total_cleaned_bytes: usize = 0;

        loop {
            match self.try_next().await? {
                Some(read_bytes) => total_cleaned_bytes += read_bytes,
                None => break,
            }
        }
        Ok(total_cleaned_bytes)
    }
}

impl<'a> Stream for FlushDevice<'a> {
    type Item = Result<usize, RapLibErrors>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if Instant::now().duration_since(self.last_poll_time) > self.timeout {
            println!("Cleaning buffer: Timeout exceeded!!");
            return Poll::Ready(None);
        }

        if self.board.get_queue_status()? > 0 {
            let mut read_buf: [u8; BUFFER_SIZE_FLUSHING] = [0; BUFFER_SIZE_FLUSHING];
            let bytes_read = self.board.read(&mut read_buf)?;

            self.last_poll_time = Instant::now();
            return Poll::Ready(Some(Ok(bytes_read)));
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

    pub async fn perform_temperature_stabilization(&mut self) -> Result<(), RapLibErrors> {
        let mut temperature_now: f32 = base::req_temperature(self.board)?;
        let mut delta_t = f32::abs(self.flash_data.ref_temp() - temperature_now);

        println!(
            "Initial values: temperature_now = {}; ref_temperature = {}; delta_t = {}",
            temperature_now,
            self.flash_data.ref_temp(),
            delta_t
        );

        self.set_gate_dcr()?;

        while delta_t > 0.5 {
            async {
                let temperature_old: f32 = temperature_now;
                let mut dcr_now: f32 = 0.0;

                for _i in 0..5 {
                    base::req_read_dcr(self.board)?;
                    match self.try_next().await? {
                        Some(dcr) => {
                            println!("Temperature stabilization iteration {}; got DCR: {}.", _i, dcr);
                            dcr_now = dcr as f32 / 10000.0;
                        }
                        None => return Err(RapLibErrors::StreamerError("Temperature Stabilization failed.".to_string()))
                    }
                }

                temperature_now = base::req_temperature(self.board)?;
                delta_t = temperature_now - temperature_old;
                println!("Temperature stabilization: DCR [KHz] = {}; temperature_old = {}; temperature_now = {}, delta_t = {}.", dcr_now, temperature_old, temperature_now, delta_t);
                Ok::<(), RapLibErrors>(())
            }.await?;
        }

        self.flash_data.set_ref_temp(temperature_now);
        Ok(())
    }

    fn set_gate_dcr(&mut self) -> Result<usize, RapLibErrors> {
        // read the DCR; 2 = 1 second gate for pulse counting
        //               1 = 10 seconds
        let value = 1;
        Ok(base::set_gate_dcr(self.board, value)?)
    }
}

impl<'a, 'b> Stream for TemperatureStabilizer<'a, 'b> {
    type Item = Result<u32, RapLibErrors>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if Instant::now().duration_since(self.last_poll_time) > self.timeout {
            println!("Temperature Stabilization: Timeout Exceeded!!");
            return Poll::Ready(None);
        }

        let dword: u32 = self.board.read_32_bit_u32()?;
        if dword > 0 {
            self.last_poll_time = Instant::now();
            return Poll::Ready(Some(Ok(dword)));
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
