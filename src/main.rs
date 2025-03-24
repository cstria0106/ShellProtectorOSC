#![windows_subsystem = "windows"]

mod core;
mod options;
mod ui;

use anyhow::Result;
use core::BackgroundTask;
use options::Options;
use std::sync::Arc;
use tokio::{
    runtime::{self},
    sync::RwLock,
};
use ui::UI;

fn main() -> Result<()> {
    let rt = runtime::Builder::new_multi_thread().enable_all().build()?;
    let options = Arc::new(RwLock::new(Options::load_or_default()?));

    let (ui, ui_event_receiver, ui_context) = UI::new(&options);
    let background_task = BackgroundTask::new(ui_event_receiver, ui_context, &options);

    ui.run(|| {
        rt.spawn(background_task.run());
    })
    .expect("Error running UI");

    Ok(())
}
