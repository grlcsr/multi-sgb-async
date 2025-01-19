pub mod raplibs;
pub mod streamer;

use std::collections::HashMap;

use raplibs::ftdi_wrapper::list_devices;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::{select, signal};

use tokio::time::Duration;

use raplibs::settings::RunSettings;
use streamer::global_data::{DataType, StreamData};
use streamer::SingleGeneratorBoardFSM;
use tokio_util::sync::CancellationToken;

fn main() {

    match RunSettings::initialize_run_settings() {
        Ok(_) => {
            println!("Initialized settings:");
            println!("{:?}", RunSettings::get_run_settings().unwrap());
        }
        Err(arg) => println!("Settings initialization failed! {}", arg),
    }

    let runtime: Runtime = Runtime::new().unwrap();
    runtime.block_on(runtime.spawn(async_main())).unwrap();
}

async fn async_main() {
    let mut device_list: HashMap<String, JoinHandle<()>> = HashMap::new();
    let (tx, mut rx) = mpsc::channel::<StreamData>(1000);
    let token = CancellationToken::new();
    let token_clone = token.clone();

    tokio::spawn(async move {
        loop {
            select! {
                    _ = signal::ctrl_c() => {
                        token.cancel();
                        break;
                    },
                    Some(message) = rx.recv() => {
                        match message.data {
                            Some(DataType::RawStream(x)) => continue, //println!("GOT = {:?}", x)
                            Some(x) => println!("GOT = {:?}", x),
                            None => continue
                        }
                }
            }
        }
    });

    loop {
        check_devices(&mut device_list, &tx, &token_clone).await;
        if token_clone.is_cancelled() {
            break;
        }
    }

    println!("Completed tokio!!")
}

/*
    Checks if device is still runinng and if new devices were connected
*/
async fn check_devices(
    device_list: &mut HashMap<String, JoinHandle<()>>,
    tx: &mpsc::Sender<StreamData>,
    token: &CancellationToken,
) {
    if let Ok(serial_list) = list_devices() {
        for serial_number in serial_list {
            if device_list.contains_key(&serial_number) {
                if let Some(handle) = device_list.get(&serial_number) {
                    if handle.is_finished() {
                        println!("Removed board with serial {}", serial_number); 
                        device_list.remove(&serial_number);
                    }
                }
            } else {
                let mut serial_stream = SingleGeneratorBoardFSM::new(
                    serial_number.clone(),
                    Some(tx.clone()),
                    token.clone(),
                );
                let handle = tokio::spawn(async move {
                    serial_stream.run().await;
                });
                device_list.insert(serial_number, handle);
            }
        }
    }
    tokio::time::sleep(Duration::from_secs(1)).await;
}
