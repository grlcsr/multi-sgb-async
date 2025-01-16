pub mod raplibs;
pub mod streamer;

use std::collections::HashMap;

use raplibs::ftdi_wrapper::list_devices;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;
use tokio::{select, signal};
use tokio::task::JoinHandle;

use raplibs::settings::RunSettings;
use streamer::global_data::{DataType, StreamData};
use streamer::SingleGeneratorBoardFSM;
use tokio_util::sync::CancellationToken;

fn main() {
    println!("{:?}", list_devices());

    let x = RunSettings::initialize_run_settings();
    match x {
        Ok(_) => {
            println!("INITIALIZED RUN SETTINGS!\nPRINTING RUN SETTINGS:\n");
            println!("{:?}", RunSettings::get_run_settings().unwrap());
        }
        Err(arg) => println!("RUN SETTING INITIALIZATION FAILED! {:?}", arg),
    }

    let runtime: Runtime = Runtime::new().unwrap();
    runtime.block_on(runtime.spawn(async_main())).unwrap();
}

async fn async_main() {
    //let serial = "RNG46856";
    let serial = "RNG0013";

    let mut device_list: HashMap<String, JoinHandle<()>> = HashMap::new();
    let (tx, mut rx) = mpsc::channel::<StreamData>(1000);
    let token = CancellationToken::new();

    

    loop {
        if let Ok(serial_list) = list_devices() {
            for serial_number in serial_list {
                if device_list.contains_key(&serial_number) {
                    if let Some(handle) = device_list.get(&serial_number) {
                        todo!();
                    }
                } else {
                    let mut serial_stream = SingleGeneratorBoardFSM::new(serial, Some(tx.clone()), token.clone());
                    let handle = tokio::spawn(async move {
                        serial_stream.run().await;
                    });
                    device_list.insert(serial_number, handle);
                }
            }
        }
    }

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

    use tokio::time::Duration;
    tokio::time::sleep(Duration::from_secs(1)).await;

    println!("Completed tokio!!")
}
