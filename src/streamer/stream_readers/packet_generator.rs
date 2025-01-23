use std::{
    pin::Pin,
    task::{Context, Poll},
    time::{Duration, Instant},
};

use tokio::sync::mpsc;
use tokio_stream::{Stream, StreamExt};

use crate::{
    raplibs::{base, ftdi_wrapper::FtdiBoard, RapLibErrors},
    streamer::global_data::{
        DataType, RawStream, StreamData, APT_THR_DOWN, APT_THR_UP, BUFFER_SIZE, RCT_THR,
        SEED_LENGTH,
    },
};

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
                        serial: self.serial_number.to_string(),
                        data: Some(DataType::RawStream(RawStream::new(
                            read_buf,
                            _nist_tests[0],
                            _nist_tests[1],
                        ))),
                    };

                    match self.channel.send(stream_results).await {
                        Ok(_) => {
                            num_seeds -= 1;
                        }
                        Err(x) => {
                            return Err(RapLibErrors::UnhandledError(format!(
                                "generate_packet: unhandled error while generating. Error code: {}", x
                            )))
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

        let mut apt_count = 1;
        let mut rct_count = 1;

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
                rct_count = 1;
            }

            rct_prev = sym >> 4;

            if (sym & 15) == rct_prev {
                rct_count += 1;

                if rct_count >= RCT_THR {
                    rct_fail = true;
                }
            } else {
                rct_count = 1;
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
