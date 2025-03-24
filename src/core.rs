use anyhow::Result;
use eframe::egui::{Context, ViewportCommand};
use rosc::{encoder, OscMessage, OscPacket, OscType};
use sha2::Digest;
use std::{sync::Arc, time::Duration};
use tokio::{
    net::UdpSocket,
    select,
    sync::{mpsc, Mutex, RwLock},
};

use crate::{options::Options, ui::UIEvent};

struct BackgroundState {
    socket: Option<UdpSocket>,
}

#[derive(Clone)]
pub struct BackgroundTask {
    ui_event_receiver: Arc<Mutex<mpsc::Receiver<UIEvent>>>,
    ui_context: Arc<RwLock<Option<Context>>>,
    options: Arc<RwLock<Options>>,
    state: Arc<RwLock<BackgroundState>>,
}

fn get_parameter_name(index: usize) -> String {
    format!("SHELL_PROTECTOR_saved_key{}", index)
}

fn get_osc_address_name(index: usize) -> String {
    format!("/avatar/parameters/{}", get_parameter_name(index))
}

fn get_osc_message(index: usize, password: &[u8], hash: &[u8]) -> Result<OscMessage> {
    let password = password.get(index).unwrap_or(&0);
    let hash = hash[index];

    let value = password ^ hash;
    let value = 1.0 - (value as f32) / 128.0;
    const DECIMAL_PRECISION: f32 = 10000.0;
    let value = -((value * DECIMAL_PRECISION).round() / DECIMAL_PRECISION);

    Ok(OscMessage {
        addr: get_osc_address_name(index),
        args: vec![OscType::Float(value)],
    })
}

impl BackgroundTask {
    pub fn new(
        ui_event_receiver: mpsc::Receiver<UIEvent>,
        ui_context: Arc<RwLock<Option<Context>>>,
        options: &Arc<RwLock<Options>>,
    ) -> Self {
        Self {
            ui_event_receiver: Arc::new(Mutex::new(ui_event_receiver)),
            ui_context: ui_context.clone(),
            options: options.clone(),
            state: Arc::new(RwLock::new(BackgroundState { socket: None })),
        }
    }

    async fn send(&self) {
        loop {
            if let Some(socket) = self.state.read().await.socket.as_ref() {
                let password = self.options.read().await.password.clone();
                let password_length = self.options.read().await.password_length;
                let hash = sha2::Sha256::digest(password.as_bytes());
                for i in 0..password_length {
                    if let Ok(message) = get_osc_message(i, password.as_bytes(), hash.as_ref()) {
                        if let Ok(message) = encoder::encode(&OscPacket::Message(message)) {
                            if let Err(e) = socket.send(&message).await {
                                eprintln!("Error sending message: {}", e);
                            }
                        }
                    }
                }
            } else {
                break;
            }

            tokio::time::sleep(Duration::from_millis(
                self.options.read().await.refresh_rate,
            ))
            .await;
        }
    }

    async fn handle_ui_event(&self, event: UIEvent) -> Result<()> {
        match event {
            UIEvent::OptionsChanged(options) => {
                options.save()?;
                *self.options.write().await = options;
            }
            UIEvent::Start => {
                let socket = UdpSocket::bind("0.0.0.0:0").await?;
                socket
                    .connect(format!("127.0.0.1:{}", self.options.read().await.port))
                    .await?;
                self.state.write().await.socket = Some(socket);
                let me = self.clone();
                tokio::spawn(async move {
                    me.send().await;
                });
            }
            UIEvent::Stop => {
                self.state.write().await.socket.take();
            }
            UIEvent::Hide => {
                if let Some(context) = self.ui_context.read().await.as_ref() {
                    context.send_viewport_cmd(ViewportCommand::Visible(false));
                    context.request_repaint();
                }
            }
            UIEvent::Show => {
                if let Some(context) = self.ui_context.read().await.as_ref() {
                    context.send_viewport_cmd(ViewportCommand::Visible(true));
                    context.request_repaint();
                }
            }
            UIEvent::Quit => {
                std::process::exit(0);
            }
        }

        Ok(())
    }

    async fn start_event_loop(&self) {
        let mut ui_event_receiver = self.ui_event_receiver.lock().await;
        loop {
            select! {
                Some(event) =  ui_event_receiver.recv() => {
                    if let Err(e) = self.handle_ui_event(event).await {
                        eprintln!("Error handling event: {}", e);
                    }
                }
                else => { }
            }
        }
    }

    pub async fn run(self) -> Result<()> {
        self.start_event_loop().await;
        Ok(())
    }
}
