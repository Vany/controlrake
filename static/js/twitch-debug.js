// 🎮 ControlRake Twitch Component Debug Helper
// Add this to browser console to debug event flow

function debugTwitchEvents() {
    console.log('🔍 ControlRake Twitch Debug Helper');
    console.log('=====================================');
    
    // Check if ControlRake system is available
    if (!window.system) {
        console.error('❌ window.system not available');
        return;
    }
    
    console.log('✅ ControlRake system available');
    console.log('📊 Connection status:', window.system.getConnectionStatus());
    
    // List registered event handlers
    const handlers = window.system.getEventHandlers();
    console.log('📋 Registered event handlers:', handlers);
    
    if (handlers['twitch:main']) {
        console.log('✅ Twitch component event handlers registered');
        console.log('   Events:', handlers['twitch:main']);
    } else {
        console.warn('⚠️ No Twitch component event handlers found');
        console.log('💡 Try: window.system.registerEventHandler("twitch:main", "connection_status", console.log)');
    }
    
    // Check WebSocket connection
    if (window.system.websocket) {
        console.log('✅ WebSocket available');
        console.log('   State:', window.system.websocket.readyState);
        console.log('   URL:', window.system.websocket.url);
    } else {
        console.warn('❌ WebSocket not connected');
    }
    
    // Manual event test
    window.testTwitchEvent = function(eventType = 'connection_status', data = {connected: true, channel: 'test'}) {
        console.log(`🧪 Testing event: ${eventType}`);
        const payload = {
            type: 'event',
            event_type: eventType,
            source: 'twitch:main',
            data: data,
            timestamp: new Date().toISOString()
        };
        
        if (window.system._handleComponentEvent) {
            window.system._handleComponentEvent(payload);
            console.log('✅ Test event sent');
        } else {
            console.error('❌ _handleComponentEvent not available');
        }
    };
    
    console.log('🎯 Debug functions available:');
    console.log('   - testTwitchEvent() - test event handling');
    console.log('   - debugTwitchEvents() - this function');
    
    // Enable debug mode
    window.system.setDebugMode(true);
    console.log('✅ Debug mode enabled');
}

// Auto-run debug helper
debugTwitchEvents();

// Helper to manually trigger component connection
function connectTwitchManually() {
    if (window.system && window.system.websocket) {
        console.log('🎮 Manually triggering Twitch connection...');
        window.system.websocket.send('twitch:main connect');
    } else {
        console.error('❌ WebSocket not available');
    }
}

// Helper to check component status
function getTwitchStatus() {
    if (window.system && window.system.websocket) {
        console.log('📊 Requesting Twitch component status...');
        window.system.websocket.send('twitch:main get_status');
    } else {
        console.error('❌ WebSocket not available');
    }
}

console.log('🎮 Twitch debug helpers loaded');
console.log('   Use: connectTwitchManually(), getTwitchStatus(), testTwitchEvent()');
