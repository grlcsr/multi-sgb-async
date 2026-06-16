# multi-sgb-async

An asynchronous Rust application for managing multiple Random Power single-generator
boards (SiPM-based QRNGs) concurrently, built on Tokio.

This is a personal project I wrote to learn async Rust. The concurrency core is the
part I am most satisfied with. Some of the surrounding code (notably the settings
module) is more verbose and would benefit from refactoring.

## What it does

The application detects connected generator boards, manages each one independently,
and streams the generated bit-data out over a TCP socket. Boards can be hot-plugged
and removed at runtime, and the whole system shuts down cleanly on Ctrl-C.

## Architecture

- **Per-board finite state machine** (`streamer/`): each board is driven by an async
  FSM (`SingleGeneratorBoardFSM`) with explicit states for connection, initialization,
  temperature stabilization, bit generation, on-board statistical tests, and
  termination. Each board runs as an independent Tokio task.

- **Concurrency model** (`main.rs`): a `TaskTracker` manages the lifecycle of all
  per-board tasks; a bounded `mpsc` channel carries generated data to a single writer
  with backpressure; a `CancellationToken` propagates coordinated shutdown across every
  task; `select!` multiplexes data handling and cancellation.

- **Device runtime detection**: connected boards are diffed against available devices
  each cycle, spawning tasks for new boards and aborting tasks for removed ones.

- **Hardware abstraction** (`raplibs/`): wraps the FTDI interface, flash/calibration
  data, on-device settings, and SHA-256 post-processing.

## Running
``
cargo run
``
Then connect a board. To visualize the output stream, run the Python reader in a
separate terminal:
``
python socket_reader/reader.py
``
## Status

Functional and used to drive real hardware. Written as a learning project, so the code
quality is uneven by design: the async core is carefully structured, while the settings
layer is intentionally simple and repetitive.
