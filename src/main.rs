mod streamer;
mod raplibs;

use tokio::runtime::Runtime;
use tokio::sync::mpsc;

use raplibs::settings::RunSettings;
use streamer::global_data::StreamData;
use streamer::SingleGeneratorBoardFSM;

fn main() {

    use libftd2xx::list_devices;
    println!("{:?}", list_devices());
    
    let x = RunSettings::initialize_run_settings();
    match x {
        Ok(_) => {
            println!("INITIALIZED RUN SETTINGS!\nPRINTING RUN SETTINGS:\n");
            println!("{:?}", RunSettings::get_run_settings().unwrap());
        },
        Err(arg) => println!("RUN SETTING INITIALIZATION FAILED! {:?}", arg)
    }

    let runtime: Runtime = Runtime::new().unwrap();
    runtime.block_on(
        runtime.spawn(async_main())
    ).unwrap();
}

async fn async_main() {
    let serial = "RNG46856";

    let (tx, mut rx) = mpsc::channel::<StreamData>(1000);

    let mut serial_stream = SingleGeneratorBoardFSM::new(serial, Some(tx.clone()));

    tokio::spawn(async move {
        serial_stream.sgb_mananger().await;
    });

    while let Some(message) = rx.recv().await {
        //println!("GOT = {:?}", message.serial);
        continue;
    }

    use tokio::time::Duration;
    tokio::time::sleep(Duration::from_secs(3)).await;

    println!("Completed tokio!!")

}