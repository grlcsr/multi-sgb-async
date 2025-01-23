pub mod raplibs;
pub mod streamer;

use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};

use raplibs::{ftdi_wrapper::list_devices, settings::RunSettings, RapLibErrors};
use streamer::{
    global_data::{DataType, StreamData},
    SingleGeneratorBoardFSM,
};
use tokio::{
    runtime::Runtime,
    select, signal,
    sync::mpsc,
    task::{JoinError, JoinHandle},
    time::sleep,
};
use tokio_util::{sync::CancellationToken, task::TaskTracker};

//const LOCAL_ADDRESS: &str = "127.69.42.0:1412";

fn main() {
    if let Err(err) = initialize_settings() {
        println!("Settings initialization failed! {}", err);
        return;
    }

    let runtime = Runtime::new().expect("Failed to create Tokio runtime");
    runtime.block_on(async_main());
}

async fn async_main() {
    let (tx, rx) = mpsc::channel::<StreamData>(1000);
    let cancellation_token = CancellationToken::new();
    
    let signal_handler = start_signal_handler(cancellation_token.clone());
    let message_handler =
        start_message_handler(rx, cancellation_token.clone());

    let mut task_tracker = TaskTracker::new();
    let mut device_list = HashMap::new();
    manage_devices(&mut task_tracker, &mut device_list, &tx, &cancellation_token).await;

    signal_handler.await.ok();
    message_handler.await.ok();
    task_tracker.wait().await;
    println!("Completed Tokio!");
}

fn start_signal_handler(cancellation_token: CancellationToken) -> JoinHandle<()> {
    tokio::spawn(async move {
        select! {
            _ = signal::ctrl_c() => cancellation_token.cancel(),
            _ = cancellation_token.cancelled() => return,
        }
    })
}

fn start_message_handler(
    mut rx: mpsc::Receiver<StreamData>,
    cancellation_token: CancellationToken,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            select! {
                message = rx.recv() => {
                    match message {
                        Some(data) => {
                            println!("GOT = {:?}", data);
                            //let serialized = bincode::serialize(&data).unwrap();
                            //println!("{:?}", serialized);
                        }
                        None => break
                    }
                }
                _ = cancellation_token.cancelled() => rx.close(),
            }
        }
    })
}

async fn manage_devices(
    task_tracker: &mut TaskTracker,
    device_list: &mut HashMap<String, JoinHandle<Result<(), JoinError>>>,
    tx: &mpsc::Sender<StreamData>,
    cancellation_token: &CancellationToken,
) {
    let mut timeout_check = Instant::now();
    loop {
        update_device_list(task_tracker, device_list, tx, cancellation_token).await;        
        sleep(Duration::from_secs(1)).await;

        if !device_list.is_empty() {
            timeout_check = Instant::now();
        } else if timeout_check.elapsed() >= Duration::from_secs(10) {
            println!("No devices connected for 10 seconds, cancelling...");
            cancellation_token.cancel();
        }

        if cancellation_token.is_cancelled() {
            task_tracker.close();
            break;
        }
    }
}

async fn update_device_list(
    task_tracker: &mut TaskTracker,
    device_list: &mut HashMap<String, JoinHandle<Result<(), JoinError>>>,
    tx: &mpsc::Sender<StreamData>,
    cancellation_token: &CancellationToken,
) {
    // Comparing which devices are connected and not yet initiated
    // and which ones have been disconnected without proper shutdown
    if let Ok(serial_list) = list_devices() {
        let connected_devices_set: HashSet<_> = device_list.keys().cloned().collect();
        let serial_set: HashSet<_> = serial_list.iter().cloned().collect();

        let not_in_serial_set: HashSet<_> = connected_devices_set.difference(&serial_set).cloned().collect();
        let only_in_serial_set: HashSet<_> = serial_set.difference(&connected_devices_set).cloned().collect();

        for serial_number in only_in_serial_set {
            println!("Adding new board with serial {}", &serial_number);
            let handle = task_tracker.spawn(start_device(
                serial_number.clone(),
                tx.clone(),
                cancellation_token.clone(),
            ));
            device_list.insert(serial_number, handle);
        }

        for serial_number in not_in_serial_set {
            if let Some(handle) = device_list.get(&serial_number) {
                if cancellation_token.is_cancelled() {
                    handle.abort();
                }
                if handle.is_finished() {
                    println!("Removed board with serial {}", serial_number);
                    device_list.remove(&serial_number);
                }
            }            
        }
    }
}

fn start_device(
    serial_number: String,
    tx: mpsc::Sender<StreamData>,
    cancellation_token: CancellationToken,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut board_fsm =
            SingleGeneratorBoardFSM::new(serial_number, Some(tx), cancellation_token);
        board_fsm.run().await;
    })
}

fn initialize_settings() -> Result<(), RapLibErrors> {
    RunSettings::initialize_run_settings().map(|_| {
        println!("Initialized settings:");
        if let Ok(settings) = RunSettings::get_run_settings() {
            println!("{:?}", settings);
        }
    })
}
