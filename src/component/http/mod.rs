use crate::component::Component;
use crate::bus::BusHandle;
use anyhow::Result;
use axum::{
    extract::{
        ws::{WebSocket, WebSocketUpgrade},
        Path, Query, State,
    },
    http::{StatusCode},
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{
    mpsc::UnboundedSender,
    RwLock,
};
use tower_http::{cors::CorsLayer, services::ServeDir};
use tracing::{debug, error, info, warn};

/// 📤 Component response builder - eliminates repetitive JSON construction
#[derive(Debug, Clone, Serialize)]
struct ComponentResponse {
    success: bool,
    action: String,
    version: u64,
    component_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    timestamp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    was_removed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    broadcast_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sent_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bind_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

impl ComponentResponse {
    /// Create success response with required context fields
    fn success(action: &str, component_id: &str, version: u64) -> Self {
        Self {
            success: true,
            action: action.to_string(),
            version,
            component_id: component_id.to_string(),
            timestamp: None,
            url: None,
            port: None,
            path: None,
            target: None,
            method: None,
            was_removed: None,
            broadcast_type: None,
            sent_count: None,
            bind_address: None,
            message: None,
            error: None,
        }
    }

    /// Create error response
    fn error(error: &str) -> Self {
        Self {
            success: false,
            action: String::new(),
            version: 0,
            component_id: String::new(),
            timestamp: None,
            url: None,
            port: None,
            path: None,
            target: None,
            method: None,
            was_removed: None,
            broadcast_type: None,
            sent_count: None,
            bind_address: None,
            message: None,
            error: Some(error.to_string()),
        }
    }

    /// Add timestamp (current UTC)
    fn with_timestamp(mut self) -> Self {
        self.timestamp = Some(chrono::Utc::now().to_rfc3339());
        self
    }

    /// Add registration info
    fn with_registration(mut self, url: &str, port: u16, path: &str, target: &str, method: &str) -> Self {
        self.url = Some(url.to_string());
        self.port = Some(port);
        self.path = Some(path.to_string());
        self.target = Some(target.to_string());
        self.method = Some(method.to_string());
        self
    }

    /// Add unregistration info
    fn with_unregistration(mut self, path: &str, was_removed: bool) -> Self {
        self.path = Some(path.to_string());
        self.was_removed = Some(was_removed);
        self
    }

    /// Add broadcast info
    fn with_broadcast(mut self, broadcast_type: &str, sent_count: usize) -> Self {
        self.broadcast_type = Some(broadcast_type.to_string());
        self.sent_count = Some(sent_count);
        self
    }

    /// Add server info
    fn with_server(mut self, port: u16, bind_address: &str) -> Self {
        self.port = Some(port);
        self.bind_address = Some(bind_address.to_string());
        self
    }

    /// Add message
    fn with_message(mut self, message: &str) -> Self {
        self.message = Some(message.to_string());
        self
    }

    /// Convert to JSON Value
    fn to_json(&self) -> Value {
        serde_json::to_value(self).unwrap_or_else(|_| json!({"error": "Failed to serialize response"}))
    }
}

/// ⚙️ HTTP component configuration
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct Config {
    /// Server host binding address
    #[serde(default = "default_bind_address")]
    pub bind_address: String,
    
    /// Server port
    #[serde(default = "default_port")]
    pub port: u16,
    
    /// Base URL for OAuth callbacks
    #[serde(default = "default_base_url")]
    pub base_url: String,
    
    /// Static file serving directory
    #[serde(default = "default_static_folder")]
    pub static_folder: String,
}

fn default_bind_address() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    8080
}

fn default_base_url() -> String {
    "http://localhost".to_string()
}

fn default_static_folder() -> String {
    "./static".to_string()
}

/// 🔗 WebSocket connection info
#[derive(Debug, Clone)]
pub struct WebSocketConnection {
    pub id: String,
    pub sender: UnboundedSender<String>,
    pub subscriptions: Vec<String>,
}

/// 📞 HTTP callback route info
#[derive(Debug, Clone, Serialize)]
pub struct CallbackRoute {
    pub path: String,
    pub target: String,
    pub method: String,
    pub http_method: String,
}

/// 📊 HTTP component statistics
#[derive(Debug, Clone)]
pub struct HttpStats {
    pub server_running: bool,
    pub config_loaded: bool,
    pub registered_callbacks: usize,
    pub websocket_connections: usize,
    pub port: Option<u16>,
}

/// 🌐 HTTP Component - Works with macro-generated infrastructure
#[crate::component(template = "dashboard.hbs", config = "Config")]
pub struct Http {
    // User fields only - macro handles all framework fields
    /// 🔌 WebSocket connections: connection_id → connection info
    websocket_connections: Arc<RwLock<HashMap<String, WebSocketConnection>>>,
    /// 📞 HTTP callback routes: path → route info  
    callbacks: Arc<RwLock<HashMap<String, CallbackRoute>>>,
    /// 🚀 HTTP server handle
    server_handle: Option<tokio::task::JoinHandle<()>>,
    /// Internal bus handle reference (derived from system)
    bus_handle: Option<BusHandle>,
}

impl Http {
    /// 🔧 Called after component creation - initialize user fields
    /// NOTE: This is called by macro-generated constructor
    pub async fn new_component(&mut self) -> Result<()> {
        // Initialize user fields with defaults
        self.websocket_connections = Arc::new(RwLock::new(HashMap::new()));
        self.callbacks = Arc::new(RwLock::new(HashMap::new()));
        self.server_handle = None;
        
        // Get bus handle from system reference
        self.bus_handle = Some(self.system.get_bus_handle().clone());
        
        // Try to load saved state
        let state_key = format!("http:{}:state", self.id());
        if let Ok(Some(state)) = self.load_state(&state_key).await {
            self.load_callbacks_from_state(&state).await?;
            debug!("📖 Loaded HTTP state for '{}'", self.id());
        }
        
        // Start HTTP server automatically
        if let Err(e) = self.start_http_server_internal().await {
            warn!("Failed to start HTTP server: {}", e);
        }
        
        info!("🌐 HTTP component '{}' initialized", self.id());
        Ok(())
    }

    /// 📝 Register HTTP callback route - routed by macro
    pub async fn register(&mut self, path: &str, target: &str, method: &str, http_method: &str) -> Result<Value> {
        info!("📝 Registering HTTP callback: {} {} → {}:{}", http_method, path, target, method);

        // Create route
        let route = CallbackRoute {
            path: path.to_string(),
            target: target.to_string(),
            method: method.to_string(),
            http_method: http_method.to_string(),
        };

        // Store in callbacks map
        {
            let mut callbacks = self.callbacks.write().await;
            callbacks.insert(path.to_string(), route);
        }

        // Increment version and save state
        self.increment_version();
        if let Err(e) = self.save_component_state().await {
            warn!("Failed to save HTTP state: {}", e);
        }

        // Build callback URL
        let callback_url = format!(
            "{}{}{}",
            self.cfg.base_url,
            if self.cfg.port != 80 && self.cfg.port != 443 {
                format!(":{}", self.cfg.port)
            } else {
                String::new()
            },
            path
        );

        Ok(ComponentResponse::success("register", self.id(), self.current_version())
            .with_registration(&callback_url, self.cfg.port, path, target, method)
            .with_timestamp()
            .to_json())
    }

    /// ❌ Unregister HTTP callback route - routed by macro
    pub async fn unregister(&mut self, path: &str) -> Result<Value> {
        info!("❌ Unregistering HTTP callback: {}", path);

        let was_removed = {
            let mut callbacks = self.callbacks.write().await;
            callbacks.remove(path).is_some()
        };

        self.increment_version();
        if let Err(e) = self.save_component_state().await {
            warn!("Failed to save HTTP state: {}", e);
        }

        Ok(ComponentResponse::success("unregister", self.id(), self.current_version())
            .with_unregistration(path, was_removed)
            .with_timestamp()
            .to_json())
    }

    /// 📡 Broadcast message to WebSocket connections - routed by macro
    pub async fn broadcast(&mut self, broadcast_type: &str, data: Value) -> Result<Value> {
        debug!("📡 Broadcasting {} to all WebSocket connections", broadcast_type);

        let sent_count = match broadcast_type {
            "render_update" | "full_render_update" => {
                self.handle_render_update_broadcast(&data).await?
            }
            "incremental_update" => {
                self.handle_incremental_update_broadcast(&data).await?
            }
            "event" => {
                self.handle_event_broadcast(&data).await?
            }
            other => {
                warn!("⚠️ Unknown broadcast type: {}", other);
                0
            }
        };

        self.increment_version();

        Ok(ComponentResponse::success("broadcast", self.id(), self.current_version())
            .with_broadcast(broadcast_type, sent_count)
            .with_timestamp()
            .to_json())
    }

    /// 📊 Get WebSocket and HTTP statistics - routed by macro
    pub async fn get(&self, key: &str) -> Result<Value> {
        match key {
            "websocket_stats" => {
                let connections = self.websocket_connections.read().await;
                let callbacks = self.callbacks.read().await;

                let connection_details: Vec<Value> = connections
                    .iter()
                    .map(|(id, conn)| {
                        json!({
                            "id": id,
                            "subscription_count": conn.subscriptions.len(),
                            "subscriptions": conn.subscriptions
                        })
                    })
                    .collect();

                let callback_details: Vec<Value> = callbacks
                    .values()
                    .map(|route| {
                        json!({
                            "path": route.path,
                            "target": route.target,
                            "method": route.method,
                            "http_method": route.http_method
                        })
                    })
                    .collect();

                Ok(json!({
                    "component_id": self.id(),
                    "websocket_connections": {
                        "count": connections.len(),
                        "details": connection_details
                    },
                    "http_callbacks": {
                        "count": callbacks.len(),
                        "details": callback_details
                    },
                    "timestamp": chrono::Utc::now().to_rfc3339()
                }))
            }
            "status" => {
                let stats = self.http_stats().await;
                Ok(json!({
                    "id": self.id(),
                    "server_running": stats.server_running,
                    "config_loaded": stats.config_loaded,
                    "registered_callbacks": stats.registered_callbacks,
                    "websocket_connections": stats.websocket_connections,
                    "port": stats.port,
                    "version": self.current_version()
                }))
            }
            _ => Ok(json!({
                "error": format!("Unknown key: {}", key),
                "available_keys": ["websocket_stats", "status"]
            }))
        }
    }

    /// 🚀 Start HTTP server - routed by macro
    pub async fn start_server(&mut self) -> Result<Value> {
        if let Err(e) = self.start_http_server_internal().await {
            Ok(ComponentResponse::error(&format!("Failed to start HTTP server: {e}")).to_json())
        } else {
            Ok(ComponentResponse::success("start_server", self.id(), self.current_version())
                .with_server(self.cfg.port, &self.cfg.bind_address)
                .with_message("HTTP server started successfully")
                .to_json())
        }
    }

    /// 🛑 Stop HTTP server - routed by macro  
    pub async fn stop_server(&mut self) -> Result<Value> {
        if let Some(handle) = self.server_handle.take() {
            handle.abort();
            info!("🛑 HTTP server stopped for '{}'", self.id());
            
            Ok(ComponentResponse::success("stop_server", self.id(), self.current_version())
                .with_message("HTTP server stopped successfully")
                .to_json())
        } else {
            Ok(ComponentResponse::success("stop_server", self.id(), self.current_version())
                .with_message("Server was not running")
                .to_json())
        }
    }

    // ================================================================================
    // 🔧 HELPER METHODS
    // ================================================================================

    /// 🚀 Start HTTP server (internal implementation)
    async fn start_http_server_internal(&mut self) -> Result<()> {
        if let Some(bus_handle) = &self.bus_handle {
            // Create HTTP server state
            let state = HttpServerState {
                _placeholder: (),
            };

            // Build the axum router
            let static_folder_clone = self.cfg.static_folder.clone();
            let app = Router::new()
                .route(
                    "/",
                    get({
                        let static_folder = static_folder_clone.clone();
                        move || async move {
                            match std::fs::read_to_string(format!("{static_folder}/index.html")) {
                                Ok(content) => Html(content).into_response(),
                                Err(_) => (StatusCode::NOT_FOUND, "index.html not found").into_response(),
                            }
                        }
                    }),
                )
                .route("/ws", get(websocket_handler))
                .route(
                    "/oauth/twitch/:instance_id",
                    get({
                        let bus_handle = bus_handle.clone();
                        move |path: axum::extract::Path<String>,
                              query: axum::extract::Query<HashMap<String, String>>| {
                            oauth_callback_handler(path, query, bus_handle)
                        }
                    }),
                )
                .nest_service("/static", ServeDir::new(&self.cfg.static_folder))
                .layer(CorsLayer::permissive())
                .with_state(state);

            // Start the server
            let listener = tokio::net::TcpListener::bind(format!("{}:{}", self.cfg.bind_address, self.cfg.port))
                .await
                .map_err(|e| anyhow::anyhow!("Failed to bind to {}:{}: {}", self.cfg.bind_address, self.cfg.port, e))?;

            info!("🚀 HTTP server listening on port {}", self.cfg.port);

            let server_handle = tokio::spawn(async move {
                if let Err(e) = axum::serve(listener, app).await {
                    error!("❌ HTTP server error: {}", e);
                }
            });

            self.server_handle = Some(server_handle);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Bus handle not available"))
        }
    }

    /// 💾 Save component state using macro-generated storage helpers
    async fn save_component_state(&self) -> Result<()> {
        let callbacks = self.callbacks.read().await;
        let callbacks_data: HashMap<String, Value> = callbacks
            .iter()
            .map(|(path, route)| {
                (path.clone(), json!({
                    "target": route.target,
                    "method": route.method,
                    "http_method": route.http_method
                }))
            })
            .collect();

        let state = json!({
            "callbacks": callbacks_data,
            "version": self.current_version(),
            "last_updated": chrono::Utc::now().to_rfc3339()
        });
        
        let state_key = format!("http:{}:state", self.id());
        self.save_state(&state_key, state).await
    }

    /// 📂 Load callbacks from saved state
    async fn load_callbacks_from_state(&mut self, state: &Value) -> Result<()> {
        if let Some(callbacks_obj) = state.get("callbacks").and_then(|v| v.as_object()) {
            let mut callbacks = self.callbacks.write().await;
            callbacks.clear();
            for (path, route_data) in callbacks_obj {
                if let (Some(target), Some(method), Some(http_method)) = (
                    route_data.get("target").and_then(|v| v.as_str()),
                    route_data.get("method").and_then(|v| v.as_str()),
                    route_data.get("http_method").and_then(|v| v.as_str()),
                ) {
                    callbacks.insert(path.clone(), CallbackRoute {
                        path: path.clone(),
                        target: target.to_string(),
                        method: method.to_string(),
                        http_method: http_method.to_string(),
                    });
                }
            }
        }
        Ok(())
    }

    /// 📊 Get HTTP component statistics  
    pub async fn http_stats(&self) -> HttpStats {
        HttpStats {
            server_running: self.server_handle.is_some(),
            config_loaded: true, // Always true since config is injected by macro
            registered_callbacks: self.callbacks.read().await.len(),
            websocket_connections: self.websocket_connections.read().await.len(),
            port: Some(self.cfg.port),
        }
    }

    /// 📡 Handle render update broadcast
    async fn handle_render_update_broadcast(&self, payload: &Value) -> Result<usize> {
        let source = payload.get("source").and_then(|v| v.as_str()).unwrap_or("unknown");
        let render_data = payload.get("render_data").cloned().unwrap_or(json!({}));
        
        let broadcast_message = json!({
            "type": "render_update",
            "source": source,
            "render_data": render_data,
            "timestamp": payload.get("timestamp").cloned().unwrap_or(json!(chrono::Utc::now().to_rfc3339()))
        });

        self.send_to_all_connections(&format!("broadcast {broadcast_message}")).await
    }

    /// 📡 Handle incremental update broadcast
    async fn handle_incremental_update_broadcast(&self, payload: &Value) -> Result<usize> {
        let source = payload.get("source").and_then(|v| v.as_str()).unwrap_or("unknown");
        let updates = payload.get("updates").cloned().unwrap_or(json!([]));
        
        let broadcast_message = json!({
            "type": "incremental_update",
            "source": source,
            "updates": updates,
            "timestamp": payload.get("timestamp").cloned().unwrap_or(json!(chrono::Utc::now().to_rfc3339()))
        });

        self.send_to_all_connections(&format!("broadcast {broadcast_message}")).await
    }

    /// 📡 Handle event broadcast
    async fn handle_event_broadcast(&self, payload: &Value) -> Result<usize> {
        let event_type = payload.get("event_type").and_then(|v| v.as_str()).unwrap_or("unknown");
        let source = payload.get("source").and_then(|v| v.as_str()).unwrap_or("unknown");
        let event_data = payload.get("event_data").cloned().unwrap_or(json!({}));
        
        let broadcast_message = json!({
            "type": "event",
            "event_type": event_type,
            "source": source,
            "data": event_data,
            "timestamp": payload.get("timestamp").cloned().unwrap_or(json!(chrono::Utc::now().to_rfc3339()))
        });

        self.send_to_all_connections(&format!("broadcast {broadcast_message}")).await
    }

    /// 📡 Send message to all WebSocket connections
    async fn send_to_all_connections(&self, message: &str) -> Result<usize> {
        let connections = self.websocket_connections.read().await;
        let mut sent_count = 0;

        for (conn_id, connection) in connections.iter() {
            if let Err(e) = connection.sender.send(message.to_string()) {
                warn!("❌ Failed to broadcast to WebSocket {}: {}", conn_id, e);
            } else {
                sent_count += 1;
            }
        }

        Ok(sent_count)
    }
}

/// 🔧 State passed to HTTP server handlers
#[derive(Clone)]
struct HttpServerState {
    // WebSocket and OAuth functionality not yet implemented in macro version
    // These fields were removed as dead code - will be re-added when implementing
    _placeholder: (),
}

/// 🔌 WebSocket connection handler
async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<HttpServerState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_websocket(socket, state))
}

/// 🔌 Handle individual WebSocket connection
async fn handle_websocket(_socket: WebSocket, _state: HttpServerState) {
    // WebSocket handling implementation would go here
    // State parameter simplified to placeholder since original fields were unused
    warn!("🚧 WebSocket handling not yet implemented in macro version");
}

/// 🔐 OAuth callback handler
async fn oauth_callback_handler(
    Path(_path): Path<String>, 
    Query(_query): Query<HashMap<String, String>>,
    _bus_handle: BusHandle,
) -> impl IntoResponse {
    // OAuth callback handling implementation would go here
    warn!("🚧 OAuth callback handling not yet implemented in macro version");
    (StatusCode::NOT_IMPLEMENTED, "OAuth callback not implemented")
}
