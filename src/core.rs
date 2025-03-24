use anyhow::Result;
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
    ui_event_channel: Arc<Mutex<mpsc::Receiver<UIEvent>>>,
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
    pub fn new(ui_event_channel: mpsc::Receiver<UIEvent>, options: &Arc<RwLock<Options>>) -> Self {
        Self {
            ui_event_channel: Arc::new(Mutex::new(ui_event_channel)),
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

    async fn handle_event(&self, event: UIEvent) -> Result<()> {
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
        }

        Ok(())
    }

    async fn start_event_loop(&self) {
        let mut ui_event_channel = self.ui_event_channel.lock().await;
        loop {
            select! {
                Some(event) =  ui_event_channel.recv() => {
                    if let Err(e) = self.handle_event(event).await {
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
