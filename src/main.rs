mod streamer;
mod raplibs;

use tokio::runtime::Runtime;

use streamer::SingleGeneratorBoardFSM;
use raplibs::settings::RunSettings;

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

    let mut serial_stream = SingleGeneratorBoardFSM::new(serial);

    serial_stream.sgb_mananger().await;
    println!("Completed");


    use tokio::time::Duration;
    tokio::time::sleep(Duration::from_secs(3)).await;

    println!("Completed tokio!!")

}