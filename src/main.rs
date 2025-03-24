#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod core;
mod options;
mod ui;

use anyhow::Result;
use core::CoreTask;
use options::Options;
use std::sync::Arc;
use tokio::{
    runtime::{self},
    sync::RwLock,
};
use ui::{Tray, Window};

fn main() -> Result<()> {
    let runtime = runtime::Builder::new_multi_thread().enable_all().build()?;
    let options = Arc::new(RwLock::new(Options::load_or_default()?));

    // Create window
    let window_handle = Window::create_handle()?;
    let (window_event_sender, window_event_receiver) = Window::create_event_channel();
    let window = Window::new(&options, window_handle.clone(), window_event_sender.clone());

    // Create tray
    let (tray_event_sender, tray_event_receiver) = Tray::create_event_channel();
    let _tray = Tray::new(tray_event_sender.clone())?;

    // Run core task in background
    let core_task = CoreTask::new(
        window_event_receiver,
        window_handle,
        tray_event_receiver,
        &options,
    );
    runtime.spawn(core_task.run());

    // Run window in main thread
    window.show().expect("Error showing window");

    Ok(())
}
