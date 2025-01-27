// Modules
pub mod raplibs;
pub mod streamer;

// Standard Library Imports
use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};

// External Crates
use raplibs::{ftdi_wrapper::list_devices, settings::RunSettings, RapLibErrors};
use streamer::{global_data::StreamData, SingleGeneratorBoardFSM};
use tokio::{
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream},
    runtime::Runtime,
    select, signal,
    sync::mpsc,
    task::{JoinError, JoinHandle},
    time::sleep,
};
use tokio_util::{sync::CancellationToken, task::TaskTracker};

// Constants
const LOCAL_ADDRESS: &str = "127.69.42.0:1412";

// Main Entry Point
fn main() {
    if let Err(err) = initialize_settings() {
        eprintln!("Settings initialization failed: {}", err);
        return;
    }

    let runtime = Runtime::new().expect("Failed to create Tokio runtime");
    runtime.block_on(async_main());
}

// Asynchronous Main Function
async fn async_main() {
    let cancellation_token = CancellationToken::new();

    // Start signal handler
    let signal_handler = start_signal_handler(cancellation_token.clone());

    // Create message handler channel
    let (tx, rx) = mpsc::channel::<StreamData>(1000);
    let message_handler = start_message_handler(rx, cancellation_token.clone());

    let mut task_tracker = TaskTracker::new();
    let mut device_list = HashMap::new();

    // Manage devices and wait for tasks to complete
    manage_devices(
        &mut task_tracker,
        &mut device_list,
        &tx,
        &cancellation_token,
    )
    .await;

    // Await tasks
    signal_handler.await.ok();
    message_handler.await.ok();
    task_tracker.wait().await;

    println!("Completed Tokio!");
}

// Signal Handler
fn start_signal_handler(cancellation_token: CancellationToken) -> JoinHandle<()> {
    tokio::spawn(async move {
        select! {
            _ = signal::ctrl_c() => cancellation_token.cancel(),
            _ = cancellation_token.cancelled() => (),
        }
    })
}

// Message Handler
fn start_message_handler(
    mut rx: mpsc::Receiver<StreamData>,
    cancellation_token: CancellationToken,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let tcp_listener = TcpListener::bind(LOCAL_ADDRESS).await.unwrap();
        let mut tcp_stream: Option<TcpStream> = None;

        loop {
            select! {
                Ok((socket, addr)) = tcp_listener.accept() => {
                    println!("New client connected: {}", addr);
                    tcp_stream = Some(socket);
                },
                message = rx.recv() => {
                    match message {
                        Some(data) => {
                            if let Ok(data_serialized) = serde_json::to_string(&data) {
                                if let Some(ref mut socket) = tcp_stream {
                                    if socket.write_all(data_serialized.as_bytes()).await.is_err() {
                                        tcp_stream = None;
                                        eprintln!("Error writing to the socket.");
                                    } else if socket.write_all(b"\n").await.is_err() {
                                        tcp_stream = None;
                                        eprintln!("Error writing delimiter to the socket.");
                                    }
                                } else {
                                    println!("No connected socket. Data: {:?}", data_serialized);
                                }
                            }
                        }
                        None => break,
                    }
                },
                _ = cancellation_token.cancelled() => {
                    rx.close();
                    break;
                },
            }
        }
    })
}

// Device Management
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

// Update Device List
async fn update_device_list(
    task_tracker: &mut TaskTracker,
    device_list: &mut HashMap<String, JoinHandle<Result<(), JoinError>>>,
    tx: &mpsc::Sender<StreamData>,
    cancellation_token: &CancellationToken,
) {
    if let Ok(serial_list) = list_devices() {
        let connected_devices: HashSet<_> = device_list.keys().cloned().collect();
        let available_devices: HashSet<_> = serial_list.iter().cloned().collect();

        let disconnected_devices = connected_devices.difference(&available_devices);
        let new_devices = available_devices.difference(&connected_devices);

        for serial in new_devices {
            println!("Adding new board: {}", serial);
            let handle = task_tracker.spawn(start_device(
                serial.clone(),
                tx.clone(),
                cancellation_token.clone(),
            ));
            device_list.insert(serial.clone(), handle);
        }

        for serial in disconnected_devices {
            if let Some(handle) = device_list.get(serial) {
                if handle.is_finished() {
                    println!("Removing disconnected board: {}", serial);
                    device_list.remove(serial);
                } else {
                    handle.abort();
                }
            }
        }
    }
}

// Start Device Task
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

// Initialize Settings
fn initialize_settings() -> Result<(), RapLibErrors> {
    RunSettings::initialize_run_settings().map(|_| {
        println!("Initialized settings:");
        if let Ok(settings) = RunSettings::get_run_settings() {
            println!("{:?}", settings);
        }
    })
}
