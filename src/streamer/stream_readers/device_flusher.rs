use std::{
    pin::Pin,
    task::{Context, Poll},
    time::{Duration, Instant},
};
use tokio_stream::{Stream, StreamExt};

use crate::{
    raplibs::{ftdi_wrapper::FtdiBoard, RapLibErrors},
    streamer::global_data::BUFFER_SIZE_FLUSHING,
};

#[derive(Debug)]
pub struct FlushDevice<'a> {
    board: &'a mut FtdiBoard,

    last_poll_time: Instant,
    timeout: Duration,
}

impl<'a> FlushDevice<'a> {
    pub fn new(board: &'a mut FtdiBoard, timeout: Duration) -> Self {
        Self {
            board,
            timeout,
            last_poll_time: Instant::now(),
        }
    }

    pub async fn flush_device(&mut self) -> Result<usize, RapLibErrors> {
        let mut total_cleaned_bytes: usize = 0;

        while let Some(read_bytes) = self.try_next().await? {
            total_cleaned_bytes += read_bytes;
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
