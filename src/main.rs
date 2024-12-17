mod streamer;
mod raplibs;

use tokio::runtime::Runtime;

use streamer::SGBStreamer;
use raplibs::ftdi_wrapper::FtdiBoard;
use streamer::stream_reader::DeviceStream;

fn main() {

    use libftd2xx::list_devices;
    println!("{:?}", list_devices());

    let runtime: Runtime = Runtime::new().unwrap();
    runtime.block_on(
        runtime.spawn(async_main())
    ).unwrap();
}

async fn async_main() {
    let serial = "RNG46856";

    //TODO spostare openconnection qui e creare lo stream da passare con la connessione aperta

    //let mut board = FtdiBoard::default();
    //let mut strim = DeviceStream::default();

    let serial_stream = SGBStreamer::new(serial);

    serial_stream.await;


    use tokio::time::Duration;
    tokio::time::sleep(Duration::from_secs(3)).await;

    println!("Completed tokio!!")

}