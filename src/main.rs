pub mod raplibs;
pub mod streamer;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use raplibs::{ftdi_wrapper::list_devices, settings::RunSettings, RapLibErrors};
use streamer::{
    global_data::{DataType, StreamData},
    SingleGeneratorBoardFSM,
};
use tokio::{
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream},
    runtime::Runtime,
    select, signal,
    sync::mpsc,
    task::JoinHandle,
    time::sleep,
};
use tokio_util::sync::CancellationToken;

const LOCAL_ADDRESS: &str = "127.69.42.0:1412";

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
    let listener = TcpListener::bind(LOCAL_ADDRESS).await.unwrap();
    let socket_wrapper: Arc<Mutex<Option<TcpStream>>> = Arc::new(Mutex::new(None));

    let socket_listener_handler =
        start_socket_listener_handler(listener, socket_wrapper.clone(), cancellation_token.clone());
    let signal_handler = start_signal_handler(cancellation_token.clone());
    let message_handler =
        start_message_handler(rx, socket_wrapper.clone(), cancellation_token.clone());

    let mut device_list = HashMap::new();
    manage_devices(&mut device_list, &tx, &cancellation_token).await;

    signal_handler.await.ok();
    socket_listener_handler.await.ok();
    message_handler.await.ok();
    println!("Completed Tokio!");
}

fn start_signal_handler(cancellation_token: CancellationToken) -> JoinHandle<()> {
    tokio::spawn(async move {
        if signal::ctrl_c().await.is_ok() {
            cancellation_token.cancel();
        }
    })
}

fn start_socket_listener_handler(
    listener: TcpListener,
    socket_wrapper: Arc<Mutex<Option<TcpStream>>>,
    cancellation_token: CancellationToken,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            select! {
                connection = listener.accept() => {
                    match connection {
                        Ok((socket, addr)) => {
                            println!("New client: {:?}", addr);
                            let mut socket_arc = socket_wrapper.lock().unwrap();
                            match *socket_arc {
                                Some(_) => println!("Connection already open."),
                                None => *socket_arc = Some(socket)
                            }
                        }
                        Err(e) => println!("Couldn't get client: {:?}", e),
                    }
                }
                _ = cancellation_token.cancelled() => {
                    println!("socket_handler terminated.");
                    break;
                }
            }
        }
    })
}

fn start_message_handler(
    mut rx: mpsc::Receiver<StreamData>,
    socket_wrapper: Arc<Mutex<Option<TcpStream>>>,
    cancellation_token: CancellationToken,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            select! {
                _ = cancellation_token.cancelled() => rx.close(),
                message = rx.recv() => {
                    match message {
                        Some(data) => println!("GOT = {:?}", data),
                        None => break
                    }
                }
            }
        }
    })
}

async fn manage_devices(
    device_list: &mut HashMap<String, JoinHandle<()>>,
    tx: &mpsc::Sender<StreamData>,
    cancellation_token: &CancellationToken,
) {
    loop {
        update_device_list(device_list, tx, cancellation_token).await;
        sleep(Duration::from_secs(1)).await;
    }
}

async fn update_device_list(
    device_list: &mut HashMap<String, JoinHandle<()>>,
    tx: &mpsc::Sender<StreamData>,
    cancellation_token: &CancellationToken,
) {
    if let Ok(serial_list) = list_devices() {
        for serial_number in serial_list {
            match device_list.get(&serial_number) {
                Some(handle) => {
                    if cancellation_token.is_cancelled() {
                        handle.abort();
                    }
                    if handle.is_finished() {
                        println!("Removed board with serial {}", serial_number);
                        device_list.remove(&serial_number);
                    }
                }
                None if !cancellation_token.is_cancelled() => {
                    println!("Adding new board with serial {}", &serial_number);
                    let handle = start_device(
                        serial_number.clone(),
                        tx.clone(),
                        cancellation_token.clone(),
                    );
                    device_list.insert(serial_number, handle);
                }
                _ => {}
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
