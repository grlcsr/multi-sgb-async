use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};
use tokio_stream::Stream;

use super::ftdi_wrapper::FtdiBoard;

const BUFFER_SIZE: usize = 256;

#[derive(Debug)]
pub struct StreamResult {
    pub buf: [u8; BUFFER_SIZE],
    pub bytes_read: usize
}

#[derive(Debug, Clone)]
pub struct DeviceStream {
    board: FtdiBoard,
    last_poll_time: Instant,
    delay: Duration,
    timeout: Duration,
}

impl Default for DeviceStream {
    fn default() -> Self {
        Self::new(FtdiBoard::default())
    }
}

impl DeviceStream {
    pub fn new(board: FtdiBoard) -> Self {
        Self {
            board,
            timeout: Duration::from_secs(1),
            last_poll_time: Instant::now(),
            delay: Duration::from_millis(2),
        }
    }

    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }
}

impl Stream for DeviceStream {
    type Item = StreamResult;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {        
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
            return Poll::Pending;
        }
    }
}
