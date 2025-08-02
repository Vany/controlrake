// ControlRake Client Library 🚀

class ControlRake {
    constructor() {
        this.websocket = null;
        this.pendingRequests = new Map(); // uuid -> {resolve, reject, selector}
        this.componentEventHandlers = new Map(); // component_id -> Map(event_type -> handler_function)
        this.updateStats = {
            incrementalUpdates: 0,
            fullRenders: 0,
            failedUpdates: 0,
            totalOperations: 0,
            averageUpdateTime: 0
        };
        this.debugMode = false; // Can be enabled for detailed logging
        
        // 🔄 Auto-reconnect configuration
        this.reconnect = {
            enabled: true,
            maxAttempts: 10,
            currentAttempt: 0,
            baseDelay: 1000, // Start with 1 second
            maxDelay: 30000, // Cap at 30 seconds
            backoffMultiplier: 1.5,
            url: '/ws', // Store last connection URL
            isReconnecting: false
        };
    }
    
    async connect(url = '/ws') {
        this.reconnect.url = url; // Store URL for reconnection
        this.reconnect.currentAttempt = 0; // Reset attempt counter
        
        return new Promise((resolve, reject) => {
            console.log(`🔌 Connecting to WebSocket: ws://${location.host}${url}`);
            this.websocket = new WebSocket(`ws://${location.host}${url}?auth=controlrake-test-key`);
            
            this.websocket.onopen = () => {
                console.log('✅ WebSocket connected successfully');
                this.reconnect.currentAttempt = 0; // Reset on successful connection
                this.reconnect.isReconnecting = false;
                resolve();
            };
            
            this.websocket.onerror = (error) => {
                console.error('❌ WebSocket error:', error);
                if (!this.reconnect.isReconnecting) {
                    reject(error);
                }
            };
            
            this.websocket.onmessage = (event) => {
                console.log('📨 WebSocket received:', event.data);
                this._handleMessage(event.data);
            };
            
            this.websocket.onclose = (event) => {
                console.log('🔌 WebSocket disconnected:', event.code, event.reason);
                this._handleReconnection(event);
            };
        });
    }
    
    async spawn(className, instanceId = null) {
        if (!this.websocket || this.websocket.readyState !== WebSocket.OPEN) {
            throw new Error('WebSocket not connected');
        }
        
        // Client-side validation for components that require instance IDs
        this._validateSpawnParameters(className, instanceId);
        
        // Format instance ID properly - empty string for default instances
        const formattedInstanceId = instanceId || "";
        const displayAddress = formattedInstanceId ? `${className}:${formattedInstanceId}` : `${className}:`;
        
        console.log(`🔘 Spawning component: ${displayAddress}`);
        
        const payload = {
            class: className,
            instance_id: formattedInstanceId
        };
        
        const message = `system:.spawn ${JSON.stringify(payload)}`;
        console.log(`📤 Sending spawn message: ${message}`);
        
        this.websocket.send(message);
        
        // For spawn, we don't wait for response since it's fire-and-forget
        console.log(`✅ Spawn message sent for ${displayAddress}`);
        return Promise.resolve();
    }

    async spawnWithConfig(className, instanceId = null, config = {}) {
        if (!this.websocket || this.websocket.readyState !== WebSocket.OPEN) {
            throw new Error('WebSocket not connected');
        }
        
        // Client-side validation for components that require instance IDs
        this._validateSpawnParameters(className, instanceId);
        
        // Format instance ID properly - empty string for default instances
        const formattedInstanceId = instanceId || "";
        const displayAddress = formattedInstanceId ? `${className}:${formattedInstanceId}` : `${className}:`;
        
        console.log(`🔧 Spawning component with config: ${displayAddress}`);
        console.log(`⚙️ Configuration:`, config);
        
        const payload = {
            class: className,
            instance_id: formattedInstanceId,
            config: config // Include configuration in spawn payload
        };
        
        const message = `system:.spawn ${JSON.stringify(payload)}`;
        console.log(`📤 Sending spawn with config message: ${message}`);
        
        this.websocket.send(message);
        
        // For spawn, we don't wait for response since it's fire-and-forget
        console.log(`✅ Spawn with config message sent for ${displayAddress}`);
        return Promise.resolve();
    }
    
    async render(selector, address) {
        if (!this.websocket || this.websocket.readyState !== WebSocket.OPEN) {
            throw new Error('WebSocket not connected');
        }
        
        // Parse and validate the address format
        const parsed = this._parseAddress(address);
        
        console.log(`🎨 Rendering component: ${parsed.displayAddress} into ${selector}`);
        if (parsed.isDefault) {
            console.log(`📌 Using default instance for ${parsed.className}`);
        }
        
        const requestId = this._generateId();
        const payload = {
            id: requestId
        };
        
        return new Promise((resolve, reject) => {
            // Store request with selector for content insertion
            this.pendingRequests.set(requestId, {resolve, reject, selector});
            
            const message = `${address}.render ${JSON.stringify(payload)}`;
            console.log(`📤 Sending render message: ${message}`);
            
            this.websocket.send(message);
            
            // Set timeout for request
            setTimeout(() => {
                if (this.pendingRequests.has(requestId)) {
                    this.pendingRequests.delete(requestId);
                    reject(new Error(`Render request timed out for ${address}`));
                }
            }, 5000);
        });
    }

    _handleMessage(messageText) {
        try {
            console.log(`🔍 Processing message: ${messageText}`);
            
            // Handle broadcast messages (render updates from server)
            if (messageText.startsWith('broadcast ')) {
                const payloadText = messageText.substring('broadcast '.length);
                const payload = JSON.parse(payloadText);
                
                // 🆕 ENHANCED: Handle both incremental and full render updates
                if (payload.type === "incremental_update") {
                    console.log(`⚡ Received incremental update for ${payload.address}`);
                    
                    if (payload.updates && Array.isArray(payload.updates)) {
                        const startTime = performance.now();
                        console.log(`🎯 Applying ${payload.updates.length} incremental updates`);
                        
                        let successCount = 0;
                        payload.updates.forEach((update, index) => {
                            try {
                                this._applyDomUpdate(update);
                                successCount++;
                                if (this.debugMode) {
                                    console.log(`✅ Applied update ${index + 1}/${payload.updates.length}: ${update.operation}`);
                                }
                            } catch (error) {
                                console.error(`❌ Failed to apply update ${index + 1}:`, error, update);
                                this.updateStats.failedUpdates++;
                            }
                        });
                        
                        const endTime = performance.now();
                        const updateTime = endTime - startTime;
                        
                        // Update statistics
                        this.updateStats.incrementalUpdates++;
                        this.updateStats.totalOperations += payload.updates.length;
                        this._updateAverageTime(updateTime);
                        
                        console.log(`🎉 Incremental update complete: ${successCount}/${payload.updates.length} successful in ${updateTime.toFixed(2)}ms`);
                        
                        if (this.debugMode) {
                            console.log('📊 Update stats:', this.updateStats);
                        }
                    } else {
                        console.warn('⚠️ Incremental update missing updates array:', payload);
                        this.updateStats.failedUpdates++;
                    }
                    
                } else if (payload.type === "render_update" || payload.type === "full_render_update") {
                    console.log(`🔄 Received full render update for ${payload.address}`);
                    
                    if (payload.content && payload.selector) {
                        const startTime = performance.now();
                        this._insertContent(payload.content, payload.selector);
                        const endTime = performance.now();
                        const renderTime = endTime - startTime;
                        
                        // Update statistics
                        this.updateStats.fullRenders++;
                        this._updateAverageTime(renderTime);
                        
                        console.log(`✅ Updated ${payload.selector} with new content in ${renderTime.toFixed(2)}ms`);
                        
                        if (this.debugMode) {
                            console.log('📊 Update stats:', this.updateStats);
                        }
                    } else {
                        console.warn('⚠️ Full render update missing content or selector:', payload);
                        this.updateStats.failedUpdates++;
                    }
                    
                } else if (payload.type === "event") {
                    console.log(`📡 Received event: ${payload.event_type} from ${payload.source}`);
                    this._handleComponentEvent(payload);
                    
                } else {
                    console.log(`📭 Unknown broadcast type: ${payload.type}`);
                }
                return;
            }
            
            // Parse regular response: "target {json_payload}"
            const spaceIndex = messageText.indexOf(' ');
            if (spaceIndex === -1) {
                console.warn('⚠️ Message missing payload:', messageText);
                return;
            }
            
            const target = messageText.substring(0, spaceIndex);
            const payloadText = messageText.substring(spaceIndex + 1);
            const payload = JSON.parse(payloadText);
            
            console.log(`📨 Parsed message - target: ${target}, payload:`, payload);
            
            // 🆕 ENHANCED: Handle incremental responses in addition to full render responses
            if (payload.reply && this.pendingRequests.has(payload.reply)) {
                console.log(`📬 Found pending request for reply: ${payload.reply}`);
                
                const {resolve, reject, selector} = this.pendingRequests.get(payload.reply);
                
                if (payload.finish) {
                    this.pendingRequests.delete(payload.reply);
                    
                    // Handle different response types
                    if (payload.updates && Array.isArray(payload.updates) && selector) {
                        // Incremental update response
                        console.log(`⚡ Applying ${payload.updates.length} updates to ${selector}`);
                        payload.updates.forEach(update => {
                            try {
                                this._applyDomUpdate(update);
                            } catch (error) {
                                console.error('❌ Failed to apply update:', error, update);
                            }
                        });
                        
                    } else if (payload.content && selector) {
                        // Full render response
                        console.log(`🎨 Inserting full content into ${selector}`);
                        this._insertContent(payload.content, selector);
                    }
                    
                    if (payload.error) {
                        console.error(`❌ Request failed: ${payload.error}`);
                        reject(new Error(payload.error));
                    } else {
                        console.log(`✅ Request completed successfully`);
                        resolve(payload);
                    }
                } else {
                    console.log(`📄 Partial response (not finished yet)`);
                }
            } else {
                console.log(`📭 No matching pending request for: ${payload.reply || 'no-reply'}`);
            }
            
        } catch (error) {
            console.error('❌ Failed to handle message:', error, 'Raw message:', messageText);
        }
    }

    /// 🎯 Apply incremental DOM update operation efficiently
    ///
    /// This method performs surgical DOM modifications without destroying
    /// client state like form focus, scroll position, or CSS transitions.
    /// Much faster than innerHTML replacement for small changes.
    _applyDomUpdate(update) {
        if (!update || !update.operation) {
            console.warn('⚠️ Invalid update object:', update);
            return;
        }

        console.log(`🔧 Applying ${update.operation} update:`, update);

        // Find target element(s)
        const selector = update.selector || update.parent;
        if (!selector) {
            console.error('❌ Update missing selector/parent:', update);
            return;
        }

        const elements = document.querySelectorAll(selector);
        if (elements.length === 0) {
            console.warn(`⚠️ No elements found for selector: ${selector}`);
            return;
        }

        console.log(`🎯 Found ${elements.length} element(s) for selector: ${selector}`);

        // Apply operation to all matching elements
        let successCount = 0;
        elements.forEach((element, index) => {
            try {
                switch (update.operation) {
                    case 'set_text':
                        if (update.value !== undefined) {
                            element.textContent = update.value;
                            console.log(`✅ Set text on element ${index + 1}: "${update.value}"`);
                            successCount++;
                        } else {
                            console.error('❌ set_text missing value:', update);
                        }
                        break;

                    case 'set_attribute':
                        if (update.attribute && update.value !== undefined) {
                            element.setAttribute(update.attribute, update.value);
                            console.log(`✅ Set attribute ${update.attribute}="${update.value}" on element ${index + 1}`);
                            successCount++;
                        } else {
                            console.error('❌ set_attribute missing attribute/value:', update);
                        }
                        break;

                    case 'set_style':
                        if (update.property && update.value !== undefined) {
                            element.style[update.property] = update.value;
                            console.log(`✅ Set style ${update.property}="${update.value}" on element ${index + 1}`);
                            successCount++;
                        } else {
                            console.error('❌ set_style missing property/value:', update);
                        }
                        break;

                    case 'toggle_class':
                        if (update.class) {
                            const wasActive = element.classList.contains(update.class);
                            element.classList.toggle(update.class);
                            const isActive = element.classList.contains(update.class);
                            console.log(`✅ Toggled class "${update.class}" on element ${index + 1}: ${wasActive} → ${isActive}`);
                            successCount++;
                        } else {
                            console.error('❌ toggle_class missing class:', update);
                        }
                        break;

                    case 'insert_element':
                        if (update.html) {
                            const position = update.position || 'append';
                            
                            switch (position) {
                                case 'prepend':
                                    element.insertAdjacentHTML('afterbegin', update.html);
                                    break;
                                case 'append':
                                    element.insertAdjacentHTML('beforeend', update.html);
                                    break;
                                case 'before':
                                    element.insertAdjacentHTML('beforebegin', update.html);
                                    break;
                                case 'after':
                                    element.insertAdjacentHTML('afterend', update.html);
                                    break;
                                default:
                                    console.error(`❌ Unknown insert position: ${position}`);
                                    return;
                            }
                            
                            console.log(`✅ Inserted HTML at ${position} of element ${index + 1}`);
                            successCount++;
                        } else {
                            console.error('❌ insert_element missing html:', update);
                        }
                        break;

                    case 'remove_element':
                        // For remove_element, the selector points to the element to remove
                        element.remove();
                        console.log(`✅ Removed element ${index + 1}`);
                        successCount++;
                        break;

                    default:
                        console.error(`❌ Unknown update operation: ${update.operation}`);
                        break;
                }
            } catch (error) {
                console.error(`❌ Failed to apply ${update.operation} to element ${index + 1}:`, error);
            }
        });

        if (successCount > 0) {
            console.log(`🎉 Successfully applied ${update.operation} to ${successCount}/${elements.length} elements`);
        } else {
            console.warn(`⚠️ No successful applications for ${update.operation}`);
        }
    }

    _insertContent(htmlContent, selector) {
        console.log(`🎨 Inserting content into selector: ${selector}`);
        
        const container = document.querySelector(selector);
        if (!container) {
            console.error(`❌ Container not found for selector: ${selector}`);
            return;
        }
        
        // Clear existing content and insert new HTML
        container.innerHTML = htmlContent;
        console.log(`✅ Content inserted successfully`);
        
        // Execute any scripts in the content
        const scripts = container.querySelectorAll('script');
        console.log(`🔧 Executing ${scripts.length} scripts`);
        
        scripts.forEach((script, index) => {
            try {
                const newScript = document.createElement('script');
                newScript.textContent = script.textContent;
                document.head.appendChild(newScript);
                console.log(`✅ Script ${index + 1} executed`);
                
                // Remove the script after execution to keep DOM clean
                setTimeout(() => {
                    document.head.removeChild(newScript);
                }, 100);
            } catch (error) {
                console.error(`❌ Failed to execute script ${index + 1}:`, error);
            }
        });
    }

    /// 🔄 Handle WebSocket reconnection with exponential backoff
    _handleReconnection(event) {
        // Don't reconnect if explicitly disabled or if already reconnecting
        if (!this.reconnect.enabled || this.reconnect.isReconnecting) {
            console.log('🛑 Auto-reconnect disabled or already in progress');
            return;
        }

        // Don't reconnect if it was a clean close (code 1000)
        if (event.code === 1000) {
            console.log('🚪 Clean disconnection, not reconnecting');
            return;
        }

        // Check if we've exceeded max attempts
        if (this.reconnect.currentAttempt >= this.reconnect.maxAttempts) {
            console.error(`❌ Max reconnection attempts (${this.reconnect.maxAttempts}) exceeded`);
            this._emitReconnectionEvent('failed', {
                attempts: this.reconnect.currentAttempt,
                reason: 'max_attempts_exceeded'
            });
            return;
        }

        this.reconnect.isReconnecting = true;
        this.reconnect.currentAttempt++;

        // Calculate delay with exponential backoff
        const baseDelay = this.reconnect.baseDelay;
        const attemptDelay = Math.min(
            baseDelay * Math.pow(this.reconnect.backoffMultiplier, this.reconnect.currentAttempt - 1),
            this.reconnect.maxDelay
        );

        // Add some jitter to prevent thundering herd
        const jitter = Math.random() * 0.3 * attemptDelay;
        const finalDelay = attemptDelay + jitter;

        console.log(`🔄 Reconnecting in ${(finalDelay / 1000).toFixed(1)}s (attempt ${this.reconnect.currentAttempt}/${this.reconnect.maxAttempts})`);
        
        this._emitReconnectionEvent('attempting', {
            attempt: this.reconnect.currentAttempt,
            maxAttempts: this.reconnect.maxAttempts,
            delay: finalDelay
        });

        setTimeout(() => {
            this._attemptReconnection();
        }, finalDelay);
    }

    /// 🔄 Attempt to reconnect to WebSocket
    async _attemptReconnection() {
        try {
            console.log(`🔌 Attempting reconnection #${this.reconnect.currentAttempt}...`);
            
            // Create new WebSocket connection
            this.websocket = new WebSocket(`ws://${location.host}${this.reconnect.url}?auth=controlrake-test-key`);
            
            this.websocket.onopen = () => {
                console.log('✅ Reconnection successful!');
                this.reconnect.currentAttempt = 0;
                this.reconnect.isReconnecting = false;
                
                this._emitReconnectionEvent('success', {
                    attempts: this.reconnect.currentAttempt
                });
            };
            
            this.websocket.onerror = (error) => {
                console.error(`❌ Reconnection attempt #${this.reconnect.currentAttempt} failed:`, error);
                this.reconnect.isReconnecting = false;
                
                // Try again after a short delay
                setTimeout(() => {
                    this._handleReconnection({code: 1006, reason: 'Connection failed'});
                }, 1000);
            };
            
            this.websocket.onmessage = (event) => {
                console.log('📨 WebSocket received:', event.data);
                this._handleMessage(event.data);
            };
            
            this.websocket.onclose = (event) => {
                console.log('🔌 WebSocket disconnected during reconnection:', event.code, event.reason);
                this.reconnect.isReconnecting = false;
                this._handleReconnection(event);
            };
            
        } catch (error) {
            console.error(`❌ Failed to create WebSocket during reconnection attempt #${this.reconnect.currentAttempt}:`, error);
            this.reconnect.isReconnecting = false;
            
            // Try again
            setTimeout(() => {
                this._handleReconnection({code: 1006, reason: 'Connection creation failed'});
            }, 1000);
        }
    }

    /// 📡 Emit reconnection events for UI updates
    _emitReconnectionEvent(type, data) {
        const event = new CustomEvent('controlrake-reconnection', {
            detail: {
                type: type, // 'attempting', 'success', 'failed'
                data: data,
                timestamp: new Date().toISOString()
            }
        });
        
        window.dispatchEvent(event);
        
        if (this.debugMode) {
            console.log(`📡 Emitted reconnection event: ${type}`, data);
        }
    }

    /// ⚙️ Configure auto-reconnection behavior
    configureReconnection(options = {}) {
        this.reconnect = {
            ...this.reconnect,
            ...options
        };
        
        console.log('⚙️ Reconnection configuration updated:', this.reconnect);
        return this.reconnect;
    }

    /// 🛑 Disable auto-reconnection
    disableReconnection() {
        this.reconnect.enabled = false;
        console.log('🛑 Auto-reconnection disabled');
    }

    /// ✅ Enable auto-reconnection
    enableReconnection() {
        this.reconnect.enabled = true;
        console.log('✅ Auto-reconnection enabled');
    }

    /// 🔌 Manually trigger reconnection
    async forceReconnect() {
        console.log('🔄 Force reconnecting...');
        
        if (this.websocket) {
            this.websocket.close(1000, 'Manual reconnection');
        }
        
        this.reconnect.currentAttempt = 0;
        this.reconnect.isReconnecting = false;
        
        return this.connect(this.reconnect.url);
    }

    /// 📊 Get current connection status
    getConnectionStatus() {
        const status = {
            connected: this.websocket?.readyState === WebSocket.OPEN,
            connecting: this.websocket?.readyState === WebSocket.CONNECTING,
            reconnecting: this.reconnect.isReconnecting,
            attempts: this.reconnect.currentAttempt,
            maxAttempts: this.reconnect.maxAttempts,
            autoReconnectEnabled: this.reconnect.enabled
        };
        
        if (this.debugMode) {
            console.log('📊 Connection status:', status);
        }
        
        return status;
    }

    /// 📡 Handle component events from server broadcasts
    _handleComponentEvent(payload) {
        const componentId = payload.source; // e.g., "twitch:main"
        const eventType = payload.event_type; // e.g., "connection_status"
        const eventData = payload.data;

        console.log(`🎯 Processing ${eventType} event for ${componentId}:`, eventData);

        // Check if component has registered handlers
        if (this.componentEventHandlers.has(componentId)) {
            const handlers = this.componentEventHandlers.get(componentId);
            
            if (handlers.has(eventType)) {
                try {
                    const handler = handlers.get(eventType);
                    console.log(`⚡ Calling registered handler for ${componentId}.${eventType}`);
                    handler(eventData, payload);
                } catch (error) {
                    console.error(`❌ Error in event handler for ${componentId}.${eventType}:`, error);
                }
            } else {
                console.log(`📝 No handler registered for ${componentId}.${eventType}`);
            }
        } else {
            console.log(`📭 No event handlers registered for component: ${componentId}`);
        }

        // Emit global event for any listeners
        const globalEvent = new CustomEvent('controlrake-component-event', {
            detail: {
                component: componentId,
                eventType: eventType,
                data: eventData,
                timestamp: payload.timestamp
            }
        });
        window.dispatchEvent(globalEvent);
    }

    /// 📝 Register event handler for a specific component and event type
    registerEventHandler(componentId, eventType, handler) {
        if (!this.componentEventHandlers.has(componentId)) {
            this.componentEventHandlers.set(componentId, new Map());
        }
        
        this.componentEventHandlers.get(componentId).set(eventType, handler);
        console.log(`✅ Registered event handler: ${componentId}.${eventType}`);
    }

    /// 🗑️ Unregister event handler
    unregisterEventHandler(componentId, eventType) {
        if (this.componentEventHandlers.has(componentId)) {
            const handlers = this.componentEventHandlers.get(componentId);
            if (handlers.has(eventType)) {
                handlers.delete(eventType);
                console.log(`🗑️ Unregistered event handler: ${componentId}.${eventType}`);
                
                // Clean up empty component entry
                if (handlers.size === 0) {
                    this.componentEventHandlers.delete(componentId);
                    console.log(`🧹 Cleaned up empty handlers for: ${componentId}`);
                }
            }
        }
    }

    /// 📋 List all registered event handlers
    getEventHandlers() {
        const handlers = {};
        for (const [componentId, eventMap] of this.componentEventHandlers) {
            handlers[componentId] = Array.from(eventMap.keys());
        }
        console.log('📋 Registered event handlers:', handlers);
        return handlers;
    }

    _generateId() {
        return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, function(c) {
            const r = Math.random() * 16 | 0;
            const v = c == 'x' ? r : (r & 0x3 | 0x8);
            return v.toString(16);
        });
    }

    /// 🔍 Client-side validation for component spawn parameters
    ///
    /// Ensures components that require instance IDs (like button) get them,
    /// while allowing components that support defaults to use empty instances.
    _validateSpawnParameters(className, instanceId) {
        // Components that don't support default instances - require explicit instanceId
        const requiresInstanceId = ['button'];
        
        if (requiresInstanceId.includes(className) && (!instanceId || instanceId.trim() === '')) {
            throw new Error(`Component '${className}' requires an explicit instance ID. Default instances are not supported for this component type.`);
        }
        
        // Components that support default instances - allow null/empty instanceId
        const supportsDefault = ['clock', 'http', 'twitch'];
        
        if (supportsDefault.includes(className)) {
            // Validation passed - either has instanceId or using default
            console.log(`✅ Validation passed for ${className}: ${instanceId ? `using instance '${instanceId}'` : 'using default instance'}`);
        } else if (!requiresInstanceId.includes(className)) {
            // Warn about unknown component types
            console.warn(`⚠️ Unknown component type '${className}' - proceeding without validation`);
        }
    }

    /// 🏷️ Parse and normalize component address for default instance support
    ///
    /// Handles address formats:
    /// - "class:" → default instance
    /// - "class:instance" → named instance  
    /// - "class:.method" → default instance with method
    /// - "class:instance.method" → named instance with method
    _parseAddress(address) {
        if (!address || typeof address !== 'string') {
            throw new Error('Invalid address: must be a non-empty string');
        }

        // Split by first dot to separate method if present
        const dotIndex = address.indexOf('.');
        const hasMethod = dotIndex !== -1;
        
        const baseAddress = hasMethod ? address.substring(0, dotIndex) : address;
        const method = hasMethod ? address.substring(dotIndex + 1) : null;

        // Split base address by colon
        const colonIndex = baseAddress.indexOf(':');
        if (colonIndex === -1) {
            throw new Error(`Invalid address format: '${address}'. Expected 'class:' or 'class:instance' format`);
        }

        const className = baseAddress.substring(0, colonIndex);
        const instancePart = baseAddress.substring(colonIndex + 1);

        // Empty string after colon means default instance
        const isDefault = instancePart === '';
        const instanceId = isDefault ? '' : instancePart;

        return {
            className,
            instanceId,
            isDefault,
            method,
            fullAddress: address,
            baseAddress,
            displayAddress: isDefault ? `${className}:` : `${className}:${instanceId}`
        };
    }

    /// 🔧 Create properly formatted address string
    ///
    /// Converts className and instanceId into proper address format
    _formatAddress(className, instanceId = null, method = null) {
        const formattedInstanceId = instanceId || '';
        const baseAddress = formattedInstanceId ? `${className}:${formattedInstanceId}` : `${className}:`;
        return method ? `${baseAddress}.${method}` : baseAddress;
    }

    /// 📊 Update running average of operation times
    _updateAverageTime(newTime) {
        const totalOps = this.updateStats.incrementalUpdates + this.updateStats.fullRenders;
        if (totalOps === 1) {
            this.updateStats.averageUpdateTime = newTime;
        } else {
            // Rolling average calculation
            this.updateStats.averageUpdateTime = 
                (this.updateStats.averageUpdateTime * (totalOps - 1) + newTime) / totalOps;
        }
    }

    /// 🐛 Enable/disable debug mode for detailed logging
    setDebugMode(enabled) {
        this.debugMode = enabled;
        console.log(`🔧 Debug mode ${enabled ? 'enabled' : 'disabled'}`);
        
        if (enabled) {
            console.log('📊 Current update statistics:', this.updateStats);
        }
    }

    /// 📈 Get performance statistics
    getUpdateStats() {
        const stats = {
            ...this.updateStats,
            efficiency: this.updateStats.totalOperations > 0 ? 
                (this.updateStats.totalOperations - this.updateStats.failedUpdates) / this.updateStats.totalOperations * 100 : 0,
            averageTimeFormatted: `${this.updateStats.averageUpdateTime.toFixed(2)}ms`
        };
        
        console.log('📊 ControlRake Performance Statistics:');
        console.log(`⚡ Incremental updates: ${stats.incrementalUpdates}`);
        console.log(`🔄 Full renders: ${stats.fullRenders}`);
        console.log(`❌ Failed updates: ${stats.failedUpdates}`);
        console.log(`🎯 Total operations: ${stats.totalOperations}`);
        console.log(`📈 Success rate: ${stats.efficiency.toFixed(1)}%`);
        console.log(`⏱️ Average update time: ${stats.averageTimeFormatted}`);
        
        return stats;
    }

    /// 🔄 Reset performance statistics
    resetStats() {
        this.updateStats = {
            incrementalUpdates: 0,
            fullRenders: 0,
            failedUpdates: 0,
            totalOperations: 0,
            averageUpdateTime: 0
        };
        console.log('📊 Performance statistics reset');
    }

    /// 📤 Send arbitrary message to component (for API calls)
    async send(target, payload = {}) {
        if (!this.websocket || this.websocket.readyState !== WebSocket.OPEN) {
            throw new Error('WebSocket not connected');
        }
        
        // Parse and validate the target address format
        const parsed = this._parseAddress(target);
        
        console.log(`📤 Sending message to: ${parsed.displayAddress}`);
        if (parsed.isDefault) {
            console.log(`📌 Using default instance for ${parsed.className}`);
        }
        console.log(`📦 Payload:`, payload);
        
        const requestId = this._generateId();
        const messagePayload = {
            id: requestId,
            ...payload
        };
        
        return new Promise((resolve, reject) => {
            // Store request for response tracking
            this.pendingRequests.set(requestId, {resolve, reject, selector: null});
            
            const message = `${target} ${JSON.stringify(messagePayload)}`;
            console.log(`📡 Sending raw message: ${message}`);
            
            this.websocket.send(message);
            
            // Set timeout for request
            setTimeout(() => {
                if (this.pendingRequests.has(requestId)) {
                    this.pendingRequests.delete(requestId);
                    reject(new Error(`Request timed out for ${target}`));
                }
            }, 10000); // 10 second timeout for API calls
        });
    }

    /// 🔌 Get WebSocket reference for advanced usage
    get socket() {
        return this.websocket;
    }
}

// Global instance
window.system = new ControlRake();

console.log('🚀 ControlRake client library loaded');
