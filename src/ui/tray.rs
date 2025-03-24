use anyhow::Result;
use std::io::{BufReader, Read};
use std::thread;
use tokio::sync::mpsc;
use tray_icon::{
    menu::{Menu, MenuEvent, MenuId, MenuItemBuilder},
    Icon, TrayIconBuilder, TrayIconEvent,
};

use super::icon::get_icon;

#[derive(Clone, Debug)]
pub enum TrayEvent {
    ShowWindow,
    Quit,
}

pub struct Tray {
    event_sender: mpsc::Sender<TrayEvent>,
    _tray_icon: tray_icon::TrayIcon,
}

impl Tray {
    pub fn create_event_channel() -> (mpsc::Sender<TrayEvent>, mpsc::Receiver<TrayEvent>) {
        mpsc::channel(1)
    }

    pub fn new(event_sender: mpsc::Sender<TrayEvent>) -> Result<Self> {
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

        let (width, height, buffer) = get_icon()?;

        let tray_icon = TrayIconBuilder::new()
            .with_tooltip("ShellProtectorOSC")
            .with_icon(Icon::from_rgba(buffer, width, height)?)
            .with_menu(menu)
            .build()?;

        let tray = Self {
            event_sender: event_sender.clone(),
            _tray_icon: tray_icon,
        };

        tray.spawn_event_handler();
        Ok(tray)
    }

    fn spawn_event_handler(&self) {
        let sender = self.event_sender.clone();
        thread::spawn(move || loop {
            crossbeam_channel::select! {
                recv(TrayIconEvent::receiver()) -> event => {
                    match event {
                        Ok(event) => {
                            match event {
                                TrayIconEvent::DoubleClick { .. } => {
                                    _ = sender.blocking_send(TrayEvent::ShowWindow);
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
                                _ = sender.blocking_send(TrayEvent::ShowWindow);
                            }
                            "quit" => {
                                _ = sender.blocking_send(TrayEvent::Quit);
                            }
                            _ => {}
                        }
                    }
                }
            }
        });
    }
}
