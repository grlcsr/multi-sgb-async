pub mod raplibs;
pub mod streamer;

use tokio::runtime::Runtime;
use tokio::{select, signal};
use tokio::sync::mpsc;

use raplibs::settings::RunSettings;
use streamer::global_data::{DataType, StreamData};
use streamer::SingleGeneratorBoardFSM;
use tokio_util::sync::CancellationToken;

fn main() {
    use libftd2xx::list_devices;
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

    let (tx, mut rx) = mpsc::channel::<StreamData>(1000);
    let token = CancellationToken::new();

    let mut serial_stream = SingleGeneratorBoardFSM::new(serial, Some(tx.clone()), token.clone());

    tokio::spawn(async move {
        serial_stream.run().await;
    });

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
