use std::{
    pin::Pin,
    task::{Context, Poll},
    time::{Duration, Instant},
};
use tokio_stream::{Stream, StreamExt};

use crate::raplibs::{base, flash::FlashData, ftdi_wrapper::FtdiBoard, RapLibErrors};

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
