use tokio::time::sleep;
use std::time::{Duration, Instant};

use crate::raplibs::{base, flash::FlashData, ftdi_wrapper::FtdiBoard, RapLibErrors};

pub struct TemperatureStabilizer<'a, 'b> {
    board: &'a FtdiBoard,
    flash_data: &'b mut FlashData,

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
        }
    }

    pub async fn perform_temperature_stabilization(&mut self) -> Result<(), RapLibErrors> {
        let mut temperature_now = base::req_temperature(self.board)?;
        let mut delta_t = (self.flash_data.ref_temp() - temperature_now).abs();

        println!(
            "Initial values: temperature_now = {}; ref_temperature = {}; delta_t = {}",
            temperature_now,
            self.flash_data.ref_temp(),
            delta_t
        );

        self.set_gate_dcr()?;

        while delta_t > 0.5 {
            self.perform_stabilization_step(&mut temperature_now, &mut delta_t)
                .await?;
        }

        self.flash_data.set_ref_temp(temperature_now);
        Ok(())
    }

    async fn perform_stabilization_step(
        &self,
        temperature_now: &mut f32,
        delta_t: &mut f32,
    ) -> Result<(), RapLibErrors> {
        let temperature_old = *temperature_now;
        let mut dcr_now = 0.0;

        for i in 0..5 {
            base::req_read_dcr(self.board)?;
            let dcr = self.await_next().await?;
            println!(
                "Temperature stabilization iteration {}; got DCR: {}.",
                i, dcr
            );
            dcr_now = dcr as f32 / 10000.0;
        }

        *temperature_now = base::req_temperature(&self.board)?;
        *delta_t = *temperature_now - temperature_old;

        println!(
            "Temperature stabilization: DCR [KHz] = {}; temperature_old = {}; temperature_now = {}, delta_t = {}.",
            dcr_now, temperature_old, temperature_now, delta_t
        );

        Ok(())
    }

    fn set_gate_dcr(&mut self) -> Result<usize, RapLibErrors> {
        // read the DCR; 2 = 1 second gate for pulse counting
        //               1 = 10 seconds
        let value = 1;
        Ok(base::set_gate_dcr(self.board, value)?)
    }

    async fn await_next(&self) -> Result<f32, RapLibErrors> {
        let start_time = Instant::now();
        loop {
            if start_time.elapsed() > self.timeout {
                return Err(RapLibErrors::StreamerError(
                    "Temperature Stabilization: timeout exceeded.".to_string(),
                ));
            }

            match self.board.read_32_bit_u32()? {
                dword if dword > 0 => {
                    return Ok(dword as f32);
                }
                _ => {
                    sleep(self.delay).await;
                }
            }
        }
    }
}
