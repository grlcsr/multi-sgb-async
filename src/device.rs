use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

enum DeviceState {
    OpenConnection,
    ReadFlash,
    Initalization,
    TempStabilization,
    ReadStream,
    ReadTests,
    TempCompensation,
    Termination
}

struct Device {
    state: DeviceState,
}

impl Future for Device {
    type Output = &'static str;

    fn Poll(mut self: Pin<&mut self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.state {
            DeviceState::OpenConnection => todo!(),
            DeviceState::ReadFlash => todo!(),
            DeviceState::Initalization => todo!(),
            DeviceState::TempStabilization => todo!(),
            DeviceState::ReadStream => todo!(),
            DeviceState::ReadTests => todo!(),
            DeviceState::TempCompensation => todo!(),
            DeviceState::Termination => todo!(),
        }
    }
}