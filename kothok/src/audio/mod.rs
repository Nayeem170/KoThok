pub mod driver;
pub(crate) mod glue;
mod synth;
pub(crate) mod types;

pub use driver::DriverConfig;
pub use types::{Cmd, Event, Utterance};

use std::sync::mpsc;

pub fn spawn(
    utterances: Vec<Utterance>,
    config: DriverConfig,
) -> (mpsc::Sender<Cmd>, mpsc::Receiver<Event>) {
    let (cmd_tx, cmd_rx) = mpsc::channel::<Cmd>();
    let (evt_tx, evt_rx) = mpsc::channel::<Event>();
    std::thread::Builder::new()
        .name("kobo-audio".into())
        .spawn(move || run(cmd_rx, evt_tx, utterances, config))
        .expect("spawn kobo-audio thread");
    (cmd_tx, evt_rx)
}

fn run(
    cmd_rx: mpsc::Receiver<Cmd>,
    evt_tx: mpsc::Sender<Event>,
    utterances: Vec<Utterance>,
    config: DriverConfig,
) {
    kobo_core::audio::init_tls();
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");
    let local = tokio::task::LocalSet::new();
    local.block_on(&rt, driver::driver(cmd_rx, evt_tx, utterances, config));
}
