use std::{future::Future, time::Duration};
use tokio::{sync::mpsc, time};

use crate::{
    raplibs::{
        ftdi_wrapper::FtdiBoard, sanity_checks, settings::MAXIMUM_NUM_OF_DWORDS, sha256,
        RapLibErrors,
    },
    streamer::global_data::{DataType, StreamData},
};

#[derive(Debug, Clone)]
pub struct FifoReader<'a, 'b> {
    serial_number: &'static str,
    board: &'a FtdiBoard,
    channel: &'b mpsc::Sender<StreamData>
}

impl<'a, 'b> FifoReader<'a, 'b> {
    pub fn new(
        serial_number: &'static str,
        board: &'a FtdiBoard,
        channel: &'b mpsc::Sender<StreamData>,
    ) -> Self {
        Self {
            serial_number,
            board,
            channel
        }
    }

    pub async fn read_fifo_results(&mut self) -> Result<(), RapLibErrors> {
        self.send_result(self.request_asymmetry_results()).await?;
        self.send_result(self.request_monobit_results()).await?;
        self.send_result(self.request_runs_results()).await?;
        self.send_result(self.request_sha256_results()).await?;

        Ok(())
    }

    async fn send_result<T: Into<DataType>>(
        &self,
        request: impl Future<Output = Result<T, RapLibErrors>>,
    ) -> Result<(), RapLibErrors> {
        let result = request.await?;
        let stream_data = StreamData {
            serial: self.serial_number.to_string(),
            data: Some(result.into()),
        };
        self.channel.send(stream_data).await.map_err(|err| {
            RapLibErrors::UnhandledError(format!("Error sending stream data: {:?}", err))
        })
    }

    async fn timeout_with_err<F, T>(&self, f: F) -> Result<T, RapLibErrors>
    where
        F: Fn() -> T + Send,
    {
        /* 
            This timeout will actually never happen because from the perspective of time::timeout
            the future and the timeout end at the same time (since the future doesn't poll and yield)
            even if the timeout is exceeded on the blocking part. In that case, it prefers 
            to return the output instead of an error.
        */
        let timeout = Duration::from_millis(500);
        time::timeout(timeout, async { f() })
            .await
            .map_err(|e| RapLibErrors::UnhandledError(format!("fifo_reader timeout: {:?}", e)))
    }

    async fn request_asymmetry_results(&self) -> Result<Vec<i32>, RapLibErrors> {
        let mut asym_buffer: Vec<i32> = Vec::new();
        sanity_checks::req_read_asym_fifo(self.board)?;

        while let Ok(value_read) = self
            .timeout_with_err(&|| self.board.read_32_bit_u32())
            .await?
        {
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

        while let Ok(value_read) = self
            .timeout_with_err(&|| self.board.read_32_bit_u32())
            .await?
        {
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

        while let Ok(value_read) = self
            .timeout_with_err(&|| self.board.read_64_bit_u64())
            .await?
        {
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
