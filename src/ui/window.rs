use anyhow::Result;
use eframe::{
    egui::{self, Context, Layout, Theme, ViewportBuilder, ViewportCommand},
    App, Frame, NativeOptions,
};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

use crate::options::Options;

use super::windows;

#[derive(Clone, Debug)]
pub enum WindowEvent {
    OptionsChanged(Options),
}

#[derive(Clone, Debug)]
pub struct WindowHandle {
    pid: u32,
    context: Arc<RwLock<Option<Context>>>,
}

impl WindowHandle {
    pub async fn show(&self) -> Result<()> {
        windows::set_visible(self.pid, true)?;
        if let Some(context) = self.context.read().await.as_ref() {
            context.send_viewport_cmd(ViewportCommand::Visible(true));
        }
        Ok(())
    }

    pub fn blocking_hide(&self) -> Result<()> {
        windows::set_visible(self.pid, false)?;
        if let Some(context) = self.context.blocking_read().as_ref() {
            context.send_viewport_cmd(ViewportCommand::Visible(false));
        }
        Ok(())
    }
}

pub struct Window {
    options: Arc<RwLock<Options>>,
    event_sender: mpsc::Sender<WindowEvent>,
    handle: WindowHandle,
    initialized: bool,
}

impl Window {
    pub fn create_event_channel() -> (mpsc::Sender<WindowEvent>, mpsc::Receiver<WindowEvent>) {
        mpsc::channel(1)
    }

    pub fn new(
        options: &Arc<RwLock<Options>>,
        handle: WindowHandle,
        event_sender: mpsc::Sender<WindowEvent>,
    ) -> Self {
        Self {
            options: options.clone(),
            event_sender,
            handle,
            initialized: false,
        }
    }

    pub fn create_handle() -> Result<WindowHandle> {
        Ok(WindowHandle {
            pid: std::process::id(),
            context: Arc::new(RwLock::new(None)),
        })
    }

    pub fn show(self) -> Result<()> {
        eframe::run_native(
            "ShellProtectorOSC",
            NativeOptions {
                centered: true,
                viewport: ViewportBuilder::default()
                    .with_inner_size([300.0, 290.0])
                    .with_resizable(false)
                    .with_decorations(false)
                    .with_transparent(true)
                    .with_maximize_button(false),
                ..Default::default()
            },
            Box::new(|ctx| {
                // Set theme
                ctx.egui_ctx.set_theme(Theme::Dark);

                // Store context
                self.handle
                    .context
                    .blocking_write()
                    .replace(ctx.egui_ctx.clone());

                Ok(Box::new(self))
            }),
        )
        .map_err(|e| anyhow::anyhow!("Error running eframe: {}", e))
    }
}

impl App for Window {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn update(&mut self, ctx: &Context, _: &mut Frame) {
        if !self.initialized {
            // Try to hide window
            if self.options.blocking_read().start_tray {
                if let Err(_) = self.handle.blocking_hide() {
                    return;
                }
            }

            self.initialized = true;
            ctx.send_viewport_cmd(ViewportCommand::Transparent(false));
            ctx.send_viewport_cmd(ViewportCommand::Decorations(true));
            return;
        }

        if ctx.input(|i| i.viewport().minimized == Some(true) || i.viewport().close_requested()) {
            ctx.send_viewport_cmd(ViewportCommand::CancelClose);
            _ = self.handle.blocking_hide();
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.with_layout(Layout::top_down_justified(egui::Align::LEFT), |ui| {
                ui.add_space(4.0);
                ui.vertical_centered(|ui| {
                    ui.label("Shell Protector OSC");
                });
                ui.add_space(8.0);

                let mut options = self.options.blocking_read().clone();
                ui.group(|ui| {
                    if options.started {
                        ui.disable();
                    }
                    ui.label("Password");
                    ui.horizontal(|ui| {
                        ui.set_width(120.0);
                        ui.text_edit_singleline(&mut options.password);

                        egui::ComboBox::from_label("Length")
                            .width(60.0)
                            .selected_text(options.password_length.to_string())
                            .show_ui(ui, |ui| {
                                for option in [4, 8, 12, 16] {
                                    ui.selectable_value(
                                        &mut options.password_length,
                                        option,
                                        option.to_string(),
                                    );
                                }
                            });
                    });

                    ui.separator();
                    let mut port = options.port.to_string();
                    ui.label("Port");
                    ui.text_edit_singleline(&mut port);
                    if let Ok(port) = port.parse::<u16>() {
                        options.port = port;
                    }

                    ui.separator();
                    let mut refresh_rate = options.refresh_rate.to_string();
                    ui.label("Refresh Rate (ms)");
                    ui.text_edit_singleline(&mut refresh_rate);
                    if let Ok(refresh_rate) = refresh_rate.parse::<u64>() {
                        options.refresh_rate = refresh_rate;
                    }

                    ui.separator();
                    ui.checkbox(&mut options.start_tray, "Start In Tray");
                });

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.vertical_centered(|ui| {
                        if ui
                            .button(if options.started { "Stop" } else { "Start" })
                            .clicked()
                        {
                            options.started = !options.started;
                        }
                    });
                });

                if self.options.blocking_read().clone() != options {
                    if let Err(_) = self
                        .event_sender
                        .blocking_send(WindowEvent::OptionsChanged(options))
                    {
                        eprintln!("Failed to send options changed event");
                    }
                }
            });
        });
    }
}
