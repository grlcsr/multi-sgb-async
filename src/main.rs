mod streamer;
mod raplibs;

use tokio::runtime::Runtime;

use streamer::SGBStreamer;
use raplibs::settings::RunSettings;

fn main() {

    use libftd2xx::list_devices;
    println!("{:?}", list_devices());
    
    println!("TEST RUN SETTINGS!");
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

    let serial_stream = SGBStreamer::new(serial);

    serial_stream.await;


    use tokio::time::Duration;
    tokio::time::sleep(Duration::from_secs(3)).await;

    println!("Completed tokio!!")

}