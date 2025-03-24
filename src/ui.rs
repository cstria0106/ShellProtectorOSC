use anyhow::Result;
use eframe::{
    egui::{self, Context, Layout, Theme, ViewportBuilder, ViewportCommand, Window},
    App, Frame, NativeOptions,
};
use std::{sync::Arc, thread};
use tokio::sync::{mpsc, RwLock};
use tray_icon::{
    menu::{Menu, MenuEvent, MenuId, MenuItemBuilder},
    Icon, TrayIconBuilder, TrayIconEvent,
};

use crate::options::Options;

#[derive(Clone, Debug)]
pub enum UIEvent {
    Start,
    Stop,
    OptionsChanged(Options),
    Hide,
    Show,
    Quit,
}

struct UIState {
    started: bool,
}

pub struct UI {
    options: Arc<RwLock<Options>>,
    state: UIState,
    ui_event_sender: mpsc::Sender<UIEvent>,
    shared_ctx: Arc<RwLock<Option<Context>>>,
}

impl UI {
    pub fn new(
        options: &Arc<RwLock<Options>>,
    ) -> (Self, mpsc::Receiver<UIEvent>, Arc<RwLock<Option<Context>>>) {
        let (ui_event_sender, ui_event_receiver) = mpsc::channel(1);
        let shared_ctx = Arc::new(RwLock::new(None));
        (
            Self {
                options: options.clone(),
                state: UIState { started: false },
                ui_event_sender,
                shared_ctx: shared_ctx.clone(),
            },
            ui_event_receiver,
            shared_ctx,
        )
    }

    fn load_icon(buffer: &[u8]) -> Result<Icon> {
        let image = image::load_from_memory(buffer)?.into_rgba8();
        let (width, height) = image.dimensions();
        let icon = Icon::from_rgba(image.into_raw(), width, height)?;
        Ok(icon)
    }

    pub fn run(self, on_created: impl FnOnce()) -> Result<()> {
        let visible = !self.options.blocking_read().start_tray;
        let menu = Box::new(Menu::new());
        menu.append(
            &MenuItemBuilder::new()
                .id(MenuId::new("open"))
                .text("Open")
                .enabled(true)
                .build(),
        )?;
        menu.append(
            &MenuItemBuilder::new()
                .id(MenuId::new("quit"))
                .text("Quit")
                .enabled(true)
                .build(),
        )?;
        let _tray_icon = TrayIconBuilder::new()
            .with_tooltip("ShellProtectorOSC")
            .with_icon(Self::load_icon(include_bytes!("../assets/icon32.png"))?)
            .with_menu(menu);

        let ui_event_sender_for_tray = self.ui_event_sender.clone();
        thread::spawn(move || loop {
            crossbeam_channel::select! {
                recv(TrayIconEvent::receiver()) -> event => {
                    match event {
                        Ok(event) => {
                            match event {
                                TrayIconEvent::DoubleClick { .. } => {
                                    _ = ui_event_sender_for_tray.blocking_send(UIEvent::Show);
                                }
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                },

                recv(MenuEvent::receiver()) -> event => {
                    if let Ok(event) = event {
                        match event.id().0.as_str() {
                            "open" => {
                                _ = ui_event_sender_for_tray.blocking_send(UIEvent::Show);
                            }
                            "quit" => {
                                _ = ui_event_sender_for_tray.blocking_send(UIEvent::Quit);
                            }
                            _ => {}
                        }
                    }
                }
            }
        });

        eframe::run_native(
            "ShellProtectorOSC",
            NativeOptions {
                centered: true,
                viewport: ViewportBuilder::default()
                    .with_inner_size([300.0, 240.0])
                    .with_resizable(false)
                    .with_visible(visible),
                ..Default::default()
            },
            Box::new(|ctx| {
                ctx.egui_ctx.set_theme(Theme::Dark);
                self.shared_ctx
                    .blocking_write()
                    .replace(ctx.egui_ctx.clone());

                on_created();
                Ok(Box::new(self))
            }),
        )
        .map_err(|e| anyhow::anyhow!("Error running eframe: {}", e))
    }
}

impl App for UI {
    fn update(&mut self, ctx: &Context, _: &mut Frame) {
        if ctx.input(|i| i.viewport().close_requested()) {
            return;
            ctx.send_viewport_cmd(ViewportCommand::CancelClose);
            if let Err(e) = self.ui_event_sender.blocking_send(UIEvent::Hide) {
                eprintln!("Error sending event: {}", e);
            }
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
                    if self.state.started {
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
                });

                if self.options.blocking_read().clone() != options {
                    self.ui_event_sender
                        .blocking_send(UIEvent::OptionsChanged(options))
                        .unwrap();
                }

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.vertical_centered(|ui| {
                        if ui
                            .button(if self.state.started { "Stop" } else { "Start" })
                            .clicked()
                        {
                            self.state.started = !self.state.started;
                            if let Err(e) =
                                self.ui_event_sender.blocking_send(if self.state.started {
                                    UIEvent::Start
                                } else {
                                    UIEvent::Stop
                                })
                            {
                                eprintln!("Error sending event: {}", e);
                            }
                        }
                    });
                });
            });
        });
    }
}
