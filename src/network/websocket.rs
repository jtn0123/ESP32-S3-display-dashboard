use anyhow::Result;
use esp_idf_svc::ws::FrameType;
use embedded_svc::ws::asynch::server::{Acceptor, AcceptorError};
use esp_idf_svc::http::server::{EspHttpServer, Method};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::time::{Duration, Instant};

const MAX_CONNECTIONS: usize = 8;
const PING_INTERVAL: Duration = Duration::from_secs(30);

pub struct WebSocketServer {
    connections: Arc<Mutex<HashMap<u32, WsConnection>>>,
    next_id: Arc<Mutex<u32>>,
}

struct WsConnection {
    id: u32,
    last_ping: Instant,
    sender: Arc<Mutex<Box<dyn embedded_svc::ws::Sender + Send>>>,
}

impl WebSocketServer {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(Mutex::new(0)),
        }
    }

    pub fn register_handlers(&self, server: &mut EspHttpServer<'static>) -> Result<()> {
        let connections = self.connections.clone();
        let next_id = self.next_id.clone();

        // WebSocket endpoint
        server.ws_handler("/ws", move |ws| {
            // Get next connection ID
            let conn_id = match next_id.lock() {
                Ok(mut id) => {
                    let current = *id;
                    *id += 1;
                    current
                }
                Err(e) => {
                    log::error!("WebSocket: next_id lock poisoned: {}", e);
                    return Err(AcceptorError::Other);
                }
            };

            // Check connection limit
            {
                match connections.lock() {
                    Ok(conns) => {
                        if conns.len() >= MAX_CONNECTIONS {
                            log::warn!("WebSocket connection limit reached");
                            return Err(AcceptorError::OutOfMemory);
                        }
                    }
                    Err(e) => {
                        log::error!("WebSocket: connections lock poisoned: {}", e);
                        return Err(AcceptorError::Other);
                    }
                }
            }

            log::info!("WebSocket client {} connected", conn_id);

            // Accept the connection
            let (sender, receiver) = ws.split();
            let sender = Arc::new(Mutex::new(sender));

            // Store connection
            {
                if let Ok(mut conns) = connections.lock() {
                    conns.insert(conn_id, WsConnection {
                        id: conn_id,
                        last_ping: Instant::now(),
                        sender: sender.clone(),
                    });
                } else {
                    log::error!("WebSocket: failed to lock connections for insert");
                }
            }

            // Send initial data
            if let Ok(metrics) = crate::metrics::get_current_metrics() {
                if let Ok(json) = serde_json::to_string(&metrics) {
                    if let Ok(sender_guard) = sender.lock() {
                        let _ = sender_guard.send(FrameType::Text(false), json.as_bytes());
                    }
                }
            }

            // Handle incoming messages
            let connections_clone = connections.clone();
            std::thread::spawn(move || {
                loop {
                    match receiver.recv() {
                        Ok((frame_type, data)) => {
                            match frame_type {
                                FrameType::Text(_) | FrameType::Binary(_) => {
                                    // Handle commands from client
                                    if let Ok(text) = std::str::from_utf8(&data) {
                                        log::debug!("WS {} received: {}", conn_id, text);
                                        // Parse and handle commands here
                                    }
                                }
                                FrameType::Ping => {
                                    // Respond with pong
                                    if let Ok(sender_guard) = sender.lock() {
                                        let _ = sender_guard.send(FrameType::Pong, &[]);
                                    }
                                }
                                FrameType::Close => {
                                    log::info!("WebSocket client {} disconnected", conn_id);
                                    break;
                                }
                                _ => {}
                            }
                        }
                        Err(e) => {
                            log::error!("WebSocket receive error: {:?}", e);
                            break;
                        }
                    }
                }

                // Remove connection
                if let Ok(mut conns) = connections_clone.lock() {
                    conns.remove(&conn_id);
                }
            });

            Ok(())
        })?;

        // Start ping task
        self.start_ping_task();

        Ok(())
    }

    pub fn broadcast_metrics(&self, metrics: &crate::metrics::MetricsData) -> Result<()> {
        let json = serde_json::to_string(metrics)?;
        self.broadcast(FrameType::Text(false), json.as_bytes())
    }

    pub fn broadcast(&self, frame_type: FrameType, data: &[u8]) -> Result<()> {
        let mut dead_connections = Vec::new();

        {
            let connections = match self.connections.lock() {
                Ok(c) => c,
                Err(e) => {
                    log::error!("WebSocket: connections lock poisoned during broadcast: {}", e);
                    return Ok(());
                }
            };
            for (id, conn) in connections.iter() {
                if let Ok(mut sender) = conn.sender.try_lock() {
                    if let Err(e) = sender.send(frame_type, data) {
                        log::warn!("Failed to send to WebSocket {}: {:?}", id, e);
                        dead_connections.push(*id);
                    }
                }
            }
        }

        // Remove dead connections
        if !dead_connections.is_empty() {
            if let Ok(mut connections) = self.connections.lock() {
                for id in dead_connections {
                    connections.remove(&id);
                    log::info!("Removed dead WebSocket connection {}", id);
                }
            }
        }

        Ok(())
    }

    fn start_ping_task(&self) {
        let connections = self.connections.clone();

        std::thread::spawn(move || {
            loop {
                std::thread::sleep(PING_INTERVAL);

                let mut dead_connections = Vec::new();

                {
                    let conns = match connections.lock() {
                        Ok(c) => c,
                        Err(e) => {
                            log::error!("WebSocket: connections lock poisoned during ping: {}", e);
                            continue;
                        }
                    };
                    for (id, conn) in conns.iter() {
                        if conn.last_ping.elapsed() > PING_INTERVAL * 2 {
                            dead_connections.push(*id);
                        } else if let Ok(mut sender) = conn.sender.try_lock() {
                            if sender.send(FrameType::Ping, &[]).is_err() {
                                dead_connections.push(*id);
                            }
                        }
                    }
                }

                // Remove dead connections
                if !dead_connections.is_empty() {
                    if let Ok(mut conns) = connections.lock() {
                        for id in dead_connections {
                            conns.remove(&id);
                            log::info!("Removed inactive WebSocket connection {}", id);
                        }
                    }
                }
            }
        });
    }

    pub fn connection_count(&self) -> usize {
        self.connections.lock().map(|c| c.len()).unwrap_or(0)
    }
}

// Global WebSocket server instance (safe)
use std::sync::OnceLock;
static WS_SERVER: OnceLock<Arc<WebSocketServer>> = OnceLock::new();

pub fn init() -> Arc<WebSocketServer> {
    WS_SERVER.get_or_init(|| Arc::new(WebSocketServer::new())).clone()
}

pub fn broadcast_metrics_update(metrics: &crate::metrics::MetricsData) {
    if let Some(server) = WS_SERVER.get() {
        let _ = server.broadcast_metrics(metrics);
    }
}