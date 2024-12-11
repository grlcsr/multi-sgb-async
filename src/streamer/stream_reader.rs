use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};
use tokio_stream::Stream;

use super::ftdi_wrapper::FtdiBoard;

use std::sync::Arc;
use tokio::sync::Mutex;

const BUFFER_SIZE: usize = 256;

#[derive(Debug)]
pub struct StreamResult {
    buf: [u8; BUFFER_SIZE],
    bytes_read: usize
}

pub struct DeviceStream {
    board: FtdiBoard, //Arc<Mutex<&'a mut FtdiBoard>>,
    last_poll_time: Instant,
    delay: Duration,
    timeout: Duration,
}

impl DeviceStream {
    pub fn new(board: FtdiBoard, timeout: Duration) -> Self {
        Self {
            board,
            timeout,
            last_poll_time: Instant::now(),
            delay: Duration::from_millis(2),
        }
    }
}

impl Stream for DeviceStream {
    type Item = StreamResult;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // First check timeout
        if Instant::now().duration_since(self.last_poll_time) > self.timeout {
            println!("Timeout exceeded!!");
            return Poll::Ready(None);
        }

        /*
        TODO: As fo now just read whatevre, for the future create an enum that decides what kind of data to read so we can
        have correct buffer sizes for raw data stream
        */
        
        if self.board.get_queue_status().unwrap() > 0 {
            let mut buf: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];
            let bytes_read = self.board.read(&mut buf).unwrap();            
            self.last_poll_time = Instant::now();
            return Poll::Ready(Some(StreamResult{
                buf,
                bytes_read
            }));
        } else {
            let waker = cx.waker().clone();
            let delay = self.delay;
            tokio::spawn(async move {
                tokio::time::sleep(delay).await;
                waker.wake();
            });
        }

        Poll::Pending
    }
}

/*

pub fn flush_device(&mut self) -> Result<usize, FtdiBoardStatus> {
            let time_out: Duration = Duration::from_secs(1);
            let mut local_buf: [u8; 1000] = [0; 1000];

            let mut total_read_bytes: usize = 0;
            let _ = loop {
                let mut start_time: Instant = Instant::now();
                let mut amount_read: usize = 0;
                let result: Result<usize, FtdiBoardStatus> = loop {
                    if self.get_queue_status()? > 0 {
                        amount_read = self.read_comm(&mut local_buf)?;
                        total_read_bytes += amount_read;
                        start_time = Instant::now();
                    }

                    if Instant::now().checked_duration_since(start_time) > Some(time_out) {
                        println!("Flush device timeout! Amount read: {}. Total read: {}.", amount_read, total_read_bytes);
                        break Ok::<usize, FtdiBoardStatus>(amount_read);
                    }
                };

                if result? == 0 {
                    let tmp: usize = 0;
                    break Ok::<usize, FtdiBoardStatus>(tmp);
                };
            };
            Ok(total_read_bytes)
        }
        */
