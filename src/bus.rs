use crate::{message::{Message, MessageId, RawMessage}};
use anyhow::Result;
use dashmap::DashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::sync::oneshot;


// 🧵 Thread-local storage for tracking caller context within async tasks
thread_local! {
    static CALLER_CONTEXT: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

/// 🎯 Set caller context for current async task
/// Call this at the beginning of component message handlers
pub fn set_caller_context(caller: String) {
    CALLER_CONTEXT.with(|ctx| {
        *ctx.borrow_mut() = Some(caller);
    });
}

/// 🧹 Clear caller context (typically in Drop implementations)
pub fn clear_caller_context() {
    CALLER_CONTEXT.with(|ctx| {
        *ctx.borrow_mut() = None;
    });
}

/// ⏱️ Default timeout constants for bus operations
const DEFAULT_CALL_TIMEOUT: Duration = Duration::from_secs(3);

// Temporary import for component registration fix
use crate::component::Component;

/// 📞 Pending call information for request-response tracking
struct PendingCall {
    sender: oneshot::Sender<serde_json::Value>,
    started_at: std::time::Instant,
}

impl std::fmt::Debug for PendingCall {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PendingCall")
            .field("started_at", &self.started_at)
            .field("sender", &"<oneshot::Sender>")
            .finish()
    }
}

/// 📞 Global response handler for managing pending calls across all bus handles
#[derive(Clone, Debug)]
struct ResponseHandler {
    pending_calls: Arc<DashMap<String, PendingCall>>,
}

impl ResponseHandler {
    fn new() -> Self {
        Self {
            pending_calls: Arc::new(DashMap::new()),
        }
    }

    /// Add a pending call
    fn add_pending_call(&self, id: String, sender: oneshot::Sender<serde_json::Value>) {
        let pending_call = PendingCall {
            sender,
            started_at: std::time::Instant::now(),
        };
        self.pending_calls.insert(id, pending_call);
    }

    /// Handle a response message
    fn handle_response(&self, message: &Message) -> bool {
        if let Some(reply_id) = &message.reply {
            if let Some((_, pending_call)) = self.pending_calls.remove(reply_id.as_str()) {
                let elapsed = pending_call.started_at.elapsed();
                tracing::trace!(
                    "📨 Delivering response for call ID {} (took {:?})",
                    reply_id.as_str(),
                    elapsed
                );

                let _ = pending_call.sender.send(message.payload.clone());
                return true;
            }
        }
        false
    }

    /// Get pending call count
    fn pending_count(&self) -> usize {
        self.pending_calls.len()
    }

    /// Cleanup expired calls
    fn cleanup_expired(&self, max_age: Duration) {
        let now = std::time::Instant::now();
        let mut expired_keys = Vec::new();

        for entry in self.pending_calls.iter() {
            if now.duration_since(entry.value().started_at) > max_age {
                expired_keys.push(entry.key().clone());
            }
        }

        for key in expired_keys {
            if let Some((_, pending_call)) = self.pending_calls.remove(&key) {
                let _ = pending_call
                    .sender
                    .send(serde_json::json!({"error": "Call expired before response received"}));
                tracing::warn!("🧹 Cleaned up expired call: {}", key);
            }
        }
    }
}

/// 🚌 Message Bus with routing and subscription management
///
/// The bus routes messages based on address patterns:
/// - Instance methods (`class:id.method`) → instance channel
/// - Class methods (`class:.method`) → class channel  
/// - Plain addresses (`class:id`) → instance channel
/// - Handles subscriptions with exact and prefix matching
/// - Manages request-response patterns for async calls
pub struct Bus {
    /// Instance channels: "class:id" → `UnboundedSender<Message>`
    instances: Arc<DashMap<String, UnboundedSender<Message>>>,

    /// Class channels: "class" → `UnboundedSender<Message>`
    classes: Arc<DashMap<String, UnboundedSender<Message>>>,

    /// Message queue for routing
    message_queue: UnboundedSender<RawMessage>,

    /// Response handler for async calls
    response_handler: ResponseHandler,

    /// Routing task handle for cleanup
    routing_handle: Option<tokio::task::JoinHandle<()>>,
}

impl std::fmt::Debug for Bus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Bus")
            .field("instances", &self.instances.len())
            .field("classes", &self.classes.len())
            .field("pending_calls", &self.response_handler.pending_count())
            .field("routing_handle", &self.routing_handle.is_some())
            .field("message_queue", &"<UnboundedSender>")
            .finish()
    }
}

/// 📡 Bus handle for component interaction (cloneable)
///
/// Provides methods for sending messages, registering channels,
/// and managing subscriptions. Shared between components.
#[derive(Clone, Debug)]
pub struct BusHandle {
    instances: Arc<DashMap<String, UnboundedSender<Message>>>,
    classes: Arc<DashMap<String, UnboundedSender<Message>>>,
    message_queue: UnboundedSender<RawMessage>,
    /// Shared response handler for async calls
    response_handler: ResponseHandler,
}

impl Bus {
    /// 🏗️ Create new message bus with routing task
    pub async fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        let instances = Arc::new(DashMap::new());
        let classes = Arc::new(DashMap::new());
        let response_handler = ResponseHandler::new();

        // Spawn routing task with response handling
        let routing_handle = Some(Self::spawn_routing_task(
            rx,
            instances.clone(),
            classes.clone(),
            response_handler.clone(),
        ));

        Self {
            instances,
            classes,
            message_queue: tx,
            response_handler,
            routing_handle,
        }
    }

    /// 🎮 Get bus handle for component interaction
    pub fn handle(&self) -> BusHandle {
        BusHandle {
            instances: self.instances.clone(),
            classes: self.classes.clone(),
            message_queue: self.message_queue.clone(),
            response_handler: self.response_handler.clone(),
        }
    }

    /// ⚙️ Spawn message routing task with integrated response handling
    fn spawn_routing_task(
        mut rx: UnboundedReceiver<RawMessage>,
        instances: Arc<DashMap<String, UnboundedSender<Message>>>,
        classes: Arc<DashMap<String, UnboundedSender<Message>>>,
        response_handler: ResponseHandler,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            tracing::info!("🚌 Message bus routing task started with response handling");

            while let Some(raw_message) = rx.recv().await {
                // Check for internal shutdown signal
                if raw_message.raw.starts_with("__internal__:.shutdown") {
                    tracing::info!("🛑 Bus shutdown signal received - terminating routing task");
                    break;
                }

                if let Err(e) =
                    Self::route_message(&raw_message, &instances, &classes, &response_handler).await
                {
                    tracing::error!("❌ Failed to route message: {}", e);
                }
            }

            tracing::info!("🛑 Message bus routing task stopped gracefully");
        })
    }

    /// 🎯 Route message to appropriate channels with response handling
    async fn route_message(
        raw_message: &RawMessage,
        instances: &DashMap<String, UnboundedSender<Message>>,
        classes: &DashMap<String, UnboundedSender<Message>>,
        response_handler: &ResponseHandler,
    ) -> Result<()> {
        // Parse message
        let message = match raw_message.parse() {
            Ok(msg) => msg,
            Err(e) => {
                tracing::warn!("🔍 Failed to parse message '{}': {}", raw_message.raw, e);
                return Ok(()); // Don't fail routing for parse errors
            }
        };

        let target = &message.target;
        tracing::trace!("📨 Routing message to: {}", target);

        // Check if this is a response to a pending call
        if message.reply.is_some() && response_handler.handle_response(&message) {
            tracing::trace!("✅ Response handled for pending call");
            return Ok(()); // Response was handled, don't route further
        }

        // Route based on address type
        let routed = if target.is_class_method() {
            // Class method: route to class channel
            Self::route_to_class(&message, classes).await
        } else if target.is_instance_method() || target.is_instance_address() {
            // Instance method or plain instance: route to instance channel
            Self::route_to_instance(&message, instances).await
        } else {
            tracing::warn!("🤷 Unknown address pattern: {}", target);
            false
        };

        if !routed {
            tracing::warn!("📭 No route found for address: {}", target);
        }

        Ok(())
    }

    /// 🏭 Route message to class channel
    async fn route_to_class(
        message: &Message,
        classes: &DashMap<String, UnboundedSender<Message>>,
    ) -> bool {
        let class_key = message.target.class_key();

        if let Some(entry) = classes.get(&class_key) {
            let sender = entry.value();

            match sender.send(message.clone()) {
                Ok(_) => {
                    tracing::trace!("✅ Routed to class: {}", class_key);
                    true
                }
                Err(_) => {
                    tracing::warn!("💀 Class channel '{}' closed, removing", class_key);
                    drop(entry); // Release the reference
                    classes.remove(&class_key);
                    false
                }
            }
        } else {
            false
        }
    }

    /// 🎯 Route message to instance channel with default instance support
    async fn route_to_instance(
        message: &Message,
        instances: &DashMap<String, UnboundedSender<Message>>,
    ) -> bool {
        // Use the new method that handles default instances with "class:" format
        let instance_key = message.target.instance_key_with_default();
        
        if let Some(entry) = instances.get(&instance_key) {
            let sender = entry.value();

            match sender.send(message.clone()) {
                Ok(_) => {
                    tracing::trace!("✅ Routed to instance: {}", instance_key);
                    true
                }
                Err(_) => {
                    tracing::warn!("💀 Instance channel '{}' closed, removing", instance_key);
                    drop(entry); // Release the reference
                    instances.remove(&instance_key);
                    false
                }
            }
        } else {
            // Add better error logging for missing default instances
            if message.target.is_default_instance() {
                tracing::warn!("📭 No default instance registered for class '{}'", message.target.class);
            } else {
                tracing::warn!("📭 No instance found: {}", instance_key);
            }
            false
        }
    }

    /// 🛑 Gracefully stop the bus and wait for routing task completion
    ///
    /// This method performs clean shutdown:
    /// - Closes the message queue to stop accepting new messages
    /// - Waits for the routing task to finish processing remaining messages
    /// - Cleans up all channel registrations
    /// - Returns when shutdown is complete
    ///
    /// # Returns
    /// Result indicating success or failure of shutdown process
    ///
    /// # Example
    /// ```rust
    /// # use controlrake::Bus;
    /// # tokio_test::block_on(async {
    /// let mut bus = Bus::new().await;
    /// // ... use bus ...
    /// bus.stop().await?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// # }).unwrap();
    /// ```
    pub async fn stop(&mut self) -> Result<()> {
        tracing::info!("🛑 Bus graceful shutdown initiated");

        // Close the message queue to prevent new messages
        // This will cause the routing task to eventually exit when it processes all remaining messages
        drop(self.message_queue.clone());
        tracing::debug!("📭 Message queue closed - no new messages will be accepted");

        // Wait for the routing task to complete
        if let Some(handle) = self.routing_handle.take() {
            match handle.await {
                Ok(_) => {
                    tracing::info!("✅ Bus routing task completed gracefully");
                }
                Err(e) if e.is_cancelled() => {
                    tracing::warn!("🔄 Bus routing task was cancelled");
                }
                Err(e) => {
                    tracing::error!("❌ Bus routing task failed: {}", e);
                    return Err(anyhow::anyhow!("Bus routing task failed during shutdown: {}", e));
                }
            }
        } else {
            tracing::warn!("🤷 No routing task handle found during shutdown");
        }

        // Clean up all registered channels
        let instance_count = self.instances.len();
        let class_count = self.classes.len();
        let pending_count = self.response_handler.pending_count();

        self.instances.clear();
        self.classes.clear();
        
        // Cancel any remaining pending calls
        if pending_count > 0 {
            tracing::warn!("🧹 Cleaning up {} pending calls during shutdown", pending_count);
            self.response_handler.cleanup_expired(std::time::Duration::from_secs(0)); // Force cleanup all
        }

        tracing::info!(
            "🏁 Bus shutdown complete - cleaned up {} instances, {} classes, {} pending calls",
            instance_count,
            class_count,
            pending_count
        );

        Ok(())
    }
}

impl Drop for Bus {
    fn drop(&mut self) {
        if let Some(handle) = self.routing_handle.take() {
            handle.abort();
            tracing::info!("🛑 Bus routing task aborted");
        }
    }
}

impl BusHandle {

    /// 🔍 Extract caller address from current tracing span context
    ///
    /// Uses thread-local storage pattern to track caller context within the same
    /// async task execution context. This provides automatic caller detection
    /// for components that need it.
    fn extract_caller_from_span() -> Option<String> {
        // 🎯 IMPLEMENTED: Use thread-local storage for caller context tracking
        CALLER_CONTEXT.with(|caller| caller.borrow().clone())
    }


    /// 📞 Make an async call and wait for response
    ///
    /// This method provides a clean async interface for request-response communication:
    /// - Automatically generates unique message ID
    /// - Sets the `from` field if caller address is configured
    /// - Waits for response with matching `reply` field
    /// - Returns when `finish: true` is received
    /// - Handles timeouts and cleanup
    ///
    /// # Arguments
    /// * `target` - Target address (e.g., "system:.get", "http:.register")
    /// * `payload` - Message payload as JSON value
    ///
    /// # Returns
    /// The response payload as JSON value, or an error.
    ///
    /// # Example
    /// ```rust
    /// # use controlrake::Bus;
    /// # use serde_json::json;
    /// # tokio_test::block_on(async {
    /// let bus = Bus::new().await;
    /// let bus_handle = bus.handle();
    /// // Simple storage get  
    /// // let result = bus_handle.call("system:.get", json!("auth_token")).await?;
    ///
    /// // HTTP registration
    /// // let response = bus_handle.call("http:.register", json!({
    /// //     "path": "/api/status",
    /// //     "method": "GET"
    /// // })).await?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// # }).unwrap();
    /// ```
    pub async fn call(
        &self,
        target: &str,
        payload: serde_json::Value,
    ) -> Result<serde_json::Value> {
        self.call_with_timeout(target, payload, DEFAULT_CALL_TIMEOUT)
            .await
    }

    /// 📞 Make an async call with custom timeout
    ///
    /// Same as `call()` but allows specifying a custom timeout duration.
    ///
    /// # Arguments
    /// * `target` - Target address
    /// * `payload` - Message payload
    /// * `timeout` - Maximum wait time for response
    pub async fn call_with_timeout(
        &self,
        target: &str,
        payload: serde_json::Value,
        timeout: Duration,
    ) -> Result<serde_json::Value> {
        let operation_start = std::time::Instant::now();
        let request_id = MessageId::new();

        tracing::debug!(
            "📞 Starting call to '{}' with ID {} (timeout: {:?})",
            target,
            request_id.as_str(),
            timeout
        );

        // Create response channel
        let (sender, receiver) = oneshot::channel();

        // Register pending call with response handler
        self.response_handler
            .add_pending_call(request_id.as_str().to_string(), sender);

        // Parse target address
        let target_address = match target.parse() {
            Ok(addr) => addr,
            Err(e) => {
                // Cleanup on error
                self.response_handler
                    .pending_calls
                    .remove(request_id.as_str());
                return Err(anyhow::anyhow!(
                    "Invalid target address '{}': {}",
                    target,
                    e
                ));
            }
        };

        // Create message with automatic metadata
        let mut message = Message::new(target_address, payload).with_id(request_id.clone());

        tracing::debug!("🔧 Message before source: {:?}", message);

        // Extract caller address from tracing span automatically
        if let Some(caller_address) = Self::extract_caller_from_span() {
            tracing::debug!("🔧 Auto-detected caller from tracing span: {}", caller_address);
            if let Ok(source_addr) = caller_address.parse() {
                message = message.with_source(source_addr);
                tracing::debug!("✅ Source address set from tracing span");
            } else {
                tracing::warn!("❌ Failed to parse auto-detected caller address: {}", caller_address);
            }
        } else {
            tracing::debug!("❌ No caller context found in tracing span");
            tracing::debug!("💡 Using fallback source address for test/external calls");
            // Fallback source for test calls - use a generic test caller address
            if let Ok(fallback_addr) = "test:caller".parse() {
                message = message.with_source(fallback_addr);
                tracing::debug!("✅ Using fallback source address: test:caller");
            }
        }

        tracing::debug!(
            "🔧 Final message: source={:?}, id={:?}",
            message.source,
            message.id
        );

        // Send the message
        let send_start = std::time::Instant::now();
        if let Err(e) = self.send(message).await {
            // Cleanup on send failure
            self.response_handler
                .pending_calls
                .remove(request_id.as_str());
            return Err(anyhow::anyhow!("Failed to send call message: {}", e));
        }

        tracing::trace!("📤 Call message sent in {:?}", send_start.elapsed());

        // Wait for response with timeout
        let wait_start = std::time::Instant::now();
        match tokio::time::timeout(timeout, receiver).await {
            Ok(Ok(response)) => {
                let total_duration = operation_start.elapsed();
                tracing::debug!(
                    "✅ Call completed successfully in {:?} (wait: {:?})",
                    total_duration,
                    wait_start.elapsed()
                );
                Ok(response)
            }
            Ok(Err(_)) => {
                // Channel closed (shouldn't happen normally)
                self.response_handler
                    .pending_calls
                    .remove(request_id.as_str());
                Err(anyhow::anyhow!("Call cancelled - response channel closed"))
            }
            Err(_) => {
                // Timeout - cleanup and return detailed error
                let was_pending = self
                    .response_handler
                    .pending_calls
                    .remove(request_id.as_str())
                    .is_some();
                let total_duration = operation_start.elapsed();

                tracing::error!(
                    "⏰ Call timeout after {:?} for target '{}' (ID: {}, pending: {})",
                    total_duration,
                    target,
                    request_id.as_str(),
                    was_pending
                );

                Err(anyhow::anyhow!(
                    "Call timeout after {:?} for target '{}'",
                    timeout,
                    target
                ))
            }
        }
    }

    /// 📤 Send raw message string
    pub async fn send_raw(&self, raw: &str) -> Result<()> {
        // Basic validation: empty string
        if raw.is_empty() {
            anyhow::bail!("Raw message cannot be empty");
        }

        // Basic validation: must contain space separator
        if !raw.contains(' ') {
            anyhow::bail!("Raw message must contain space separator between target and payload");
        }

        // Basic validation: check target address format
        let space_pos = raw.find(' ').unwrap(); // Safe due to check above
        let target_str = &raw[..space_pos];
        
        // Validate target address format
        if let Err(e) = target_str.parse::<crate::address::Address>() {
            anyhow::bail!("Invalid target address '{}': {}", target_str, e);
        }

        let raw_message = RawMessage::new(raw.to_string());

        match self.message_queue.send(raw_message) {
            Ok(_) => {
                tracing::trace!("📤 Queued raw message: {}", raw);
                Ok(())
            }
            Err(_) => {
                anyhow::bail!("Message bus is closed")
            }
        }
    }

    /// 📨 Send structured message
    pub async fn send(&self, message: Message) -> Result<()> {
        let raw = message.to_raw();
        self.send_raw(&raw).await
    }

    /// 🔌 Register instance channel
    pub fn register_instance(&self, class: &str, id: &str, sender: UnboundedSender<Message>) {
        let key = format!("{class}:{id}");
        self.instances.insert(key.clone(), sender);
        tracing::debug!("🔌 Registered instance channel: {}", key);
    }

    /// 🔌 Register default instance channel (empty instance ID)
    pub fn register_default_instance(&self, class: &str, sender: UnboundedSender<Message>) {
        let key = format!("{class}:");  // Default instance uses "class:" format
        self.instances.insert(key.clone(), sender);
        tracing::debug!("🔌 Registered default instance channel: {}", key);
    }

    /// 🏭 Register class channel
    pub fn register_class(&self, class: &str, sender: UnboundedSender<Message>) {
        self.classes.insert(class.to_string(), sender);
        tracing::debug!("🏭 Registered class channel: {}", class);
    }

    /// 📊 Get bus statistics (for debugging)
    #[must_use]
    pub fn stats(&self) -> BusStats {
        let mut default_instances = 0;
        let mut named_instances = 0;
        
        for key in self.instances.iter() {
            if key.key().ends_with(':') {
                default_instances += 1;
            } else {
                named_instances += 1;
            }
        }
        
        BusStats {
            instance_count: self.instances.len(),
            class_count: self.classes.len(),
            pending_calls: self.response_handler.pending_count(),
            default_instances,
            named_instances,
        }
    }

    /// 🧹 Cleanup expired pending calls (for maintenance)
    ///
    /// This method removes pending calls that have been waiting too long,
    /// preventing memory leaks from abandoned calls.
    pub fn cleanup_expired_calls(&self, max_age: Duration) {
        self.response_handler.cleanup_expired(max_age);
    }

    /// 🎯 Create a new BusHandle with a fixed caller address
    /// 
    /// This allows components to set their calling identity once and have it
    /// automatically included in all subsequent calls.
    pub fn with_caller(&self, caller_address: &str) -> BusHandleWithCaller {
        BusHandleWithCaller {
            inner: self.clone(),
            caller_address: caller_address.to_string(),
        }
    }

    /// 🧪 Create test bus handle for unit tests
    /// 
    /// **Note**: For testing purposes only
    pub fn new_test() -> Self {
        use dashmap::DashMap;
        use std::sync::Arc;
        use tokio::sync::mpsc;

        let (tx, _) = mpsc::unbounded_channel();
        
        Self {
            instances: Arc::new(DashMap::new()),
            classes: Arc::new(DashMap::new()),
            message_queue: tx,
            response_handler: ResponseHandler::new(),
        }
    }

    /// 📦 Register component instance for dynamic spawning support
    ///
    /// **Purpose**: Register a component instance for message routing
    /// **Usage**: `bus.register_component(address, component_box).await?`
    /// **Arguments**: component address (e.g., "button:my-btn") and component instance
    ///
    /// **Note**: This is a temporary method to support the dynamic spawning fix.
    /// Components will eventually be managed by SystemComponent directly.
    pub async fn register_component(&self, address: String, _component: Box<dyn Component>) -> Result<()> {
        tracing::debug!("📦 Registering component for address: {}", address);
        
        // Parse the address to extract class and instance_id
        let parts: Vec<&str> = address.split(':').collect();
        if parts.len() != 2 {
            anyhow::bail!("Invalid component address format '{}'. Expected 'class:instance_id'", address);
        }
        
        let class = parts[0];
        let instance_id = parts[1];
        
        // Create a channel for the component
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        
        // Register the channel with the bus
        self.register_component_instance(class, instance_id, tx);
        
        // For now, just drop the component and receiver
        // TODO: This needs to be properly integrated with SystemComponent
        tracing::warn!("⚠️ Component registration is incomplete - dropping component instance");
        tracing::warn!("⚠️ Full integration with SystemComponent registry is required");
        
        // Drop the receiver to prevent channel buildup
        tokio::spawn(async move {
            while rx.recv().await.is_some() {
                // Consume messages but don't process them
                // This prevents channel from backing up
            }
        });
        
        tracing::info!("📦 Component address '{}' registered with bus routing", address);
        Ok(())
    }

    // ================================================================================
    // 🚀 MACRO SYSTEM REQUIRED METHODS - Task 4.2
    // ================================================================================

    /// 🎯 Register component instance (required by macro system)
    ///
    /// **Purpose**: Register component instances with the bus for routing
    /// **Usage**: `bus.register_component_instance("button", "my-btn", sender)?;`
    /// **Arguments**: class, instance_id, message sender for the instance
    ///
    /// **Required by macro system** for component lifecycle management.
    /// **Note**: This is an alias for register_instance for macro system compatibility
    pub fn register_component_instance(&self, class: &str, instance_id: &str, sender: UnboundedSender<Message>) {
        if instance_id.is_empty() {
            // Default instance registration
            self.register_default_instance(class, sender);
        } else {
            // Named instance registration
            self.register_instance(class, instance_id, sender);
        }
    }

    /// 🔍 Get component instance channel (required by macro system)
    ///
    /// **Purpose**: Retrieve component instance message sender for direct communication
    /// **Usage**: `let sender = bus.get_component_instance("button", "my-btn")?;`
    /// **Returns**: Message sender for the component instance or None if not found
    ///
    /// **Required by macro system** for direct component communication.
    pub fn get_component_instance(&self, class: &str, instance_id: &str) -> Option<UnboundedSender<Message>> {
        let key = if instance_id.is_empty() {
            format!("{class}:")  // Default instance: "button:"
        } else {
            format!("{class}:{instance_id}")  // Named instance: "button:my-btn"
        };
        
        self.instances.get(&key).map(|entry| entry.value().clone())
    }

    /// 🔍 Check if component instance exists in bus (required by macro system)
    ///
    /// **Purpose**: Check if component instance is registered for message routing
    /// **Usage**: `if bus.has_component_instance("button", "my-btn") { ... }`
    /// **Returns**: true if instance channel exists, false otherwise
    ///
    /// **Required by macro system** for component discovery and validation.
    pub fn has_component_instance(&self, class: &str, instance_id: &str) -> bool {
        let key = if instance_id.is_empty() {
            format!("{class}:")
        } else {
            format!("{class}:{instance_id}")
        };
        
        self.instances.contains_key(&key)
    }

    /// 🗑️ Unregister component instance (required by macro system)
    ///
    /// **Purpose**: Remove component instance from bus routing when component is destroyed
    /// **Usage**: `bus.unregister_component_instance("button", "my-btn");`
    /// **Returns**: true if instance was removed, false if it didn't exist
    ///
    /// **Required by macro system** for component cleanup.
    pub fn unregister_component_instance(&self, class: &str, instance_id: &str) -> bool {
        let key = if instance_id.is_empty() {
            format!("{class}:")
        } else {
            format!("{class}:{instance_id}")
        };
        
        let was_removed = self.instances.remove(&key).is_some();
        if was_removed {
            tracing::debug!("🗑️ Unregistered component instance: {}", key);
        }
        was_removed
    }

    /// 📋 List all registered component instances (required by macro system)
    ///
    /// **Purpose**: Get list of all component instances registered for routing
    /// **Usage**: `let instances = bus.list_component_instances();`
    /// **Returns**: Vector of (class, instance_id) tuples
    ///
    /// **Required by macro system** for component discovery and debugging.
    pub fn list_component_instances(&self) -> Vec<(String, String)> {
        self.instances
            .iter()
            .map(|entry| {
                let key = entry.key();
                if key.ends_with(':') {
                    // Default instance: "button:" -> ("button", "")
                    (key.trim_end_matches(':').to_string(), String::new())
                } else if let Some(colon_pos) = key.find(':') {
                    // Named instance: "button:my-btn" -> ("button", "my-btn")
                    (key[..colon_pos].to_string(), key[colon_pos + 1..].to_string())
                } else {
                    // Invalid format - should not happen
                    ("unknown".to_string(), key.clone())
                }
            })
            .collect()
    }

    // ================================================================================
    // END MACRO SYSTEM REQUIRED METHODS
    // ================================================================================
}

/// 📊 Bus statistics for monitoring
#[derive(Debug, Clone)]
pub struct BusStats {
    pub instance_count: usize,
    pub class_count: usize,
    pub pending_calls: usize,
    pub default_instances: usize,
    pub named_instances: usize,
}

/// 🎯 BusHandle with fixed caller address for component identity
/// 
/// This wrapper automatically sets the caller address for all operations,
/// eliminating the need to manually specify the "from" field in every call.
#[derive(Debug, Clone)]
pub struct BusHandleWithCaller {
    inner: BusHandle,
    caller_address: String,
}

impl BusHandleWithCaller {
    /// 📞 Make an async call with automatic caller addressing
    pub async fn call(
        &self,
        target: &str,
        payload: serde_json::Value,
    ) -> Result<serde_json::Value> {
        self.call_with_timeout(target, payload, DEFAULT_CALL_TIMEOUT).await
    }

    /// 📞 Make an async call with custom timeout and automatic caller addressing
    pub async fn call_with_timeout(
        &self,
        target: &str,
        payload: serde_json::Value,
        timeout: Duration,
    ) -> Result<serde_json::Value> {
        // Set caller context for this async task
        set_caller_context(self.caller_address.clone());
        
        // Use the inner BusHandle's call_with_timeout
        let result = self.inner.call_with_timeout(target, payload, timeout).await;
        
        // Clear caller context when done
        clear_caller_context();
        
        result
    }

    /// 🔌 Register default instance channel (empty instance ID)
    pub fn register_default_instance(&self, class: &str, sender: UnboundedSender<Message>) {
        let key = format!("{class}:");  // Default instance uses "class:" format
        self.inner.instances.insert(key.clone(), sender);
        tracing::debug!("🔌 Registered default instance channel: {}", key);
    }

    /// 📤 Send a message with automatic caller addressing
    pub async fn send(&self, target: &str, payload: serde_json::Value) -> Result<()> {
        let target_addr = target.parse().map_err(|e| anyhow::anyhow!("Invalid target address: {}", e))?;
        let source_addr = self.caller_address.parse().map_err(|e| anyhow::anyhow!("Invalid caller address: {}", e))?;
        
        let message = crate::message::Message::new(target_addr, payload)
            .with_source(source_addr);
        
        self.inner.send(message).await
    }

    /// 🔗 Get the inner BusHandle for compatibility with existing APIs
    pub fn inner(&self) -> &BusHandle {
        &self.inner
    }

    /// 📊 Get bus statistics (delegates to inner handle)
    pub fn stats(&self) -> BusStats {
        self.inner.stats()
    }
}
