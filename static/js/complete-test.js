// 🎮 Complete Event Flow Test
// Run this in browser console to test the entire chain

async function testCompleteEventFlow() {
    console.log('🎯 Starting Complete Twitch Event Flow Test');
    console.log('===========================================');
    
    let step = 1;
    
    try {
        // Step 1: Connect WebSocket
        console.log(`${step++}. 🔌 Connecting WebSocket...`);
        await system.connect('/ws');
        console.log('✅ WebSocket connected');
        
        // Step 2: Set up event monitoring
        console.log(`${step++}. 📡 Setting up event monitoring...`);
        let eventCount = 0;
        system.registerEventHandler('twitch:main', 'connection_status', (data) => {
            eventCount++;
            console.log(`📊 Event ${eventCount}: Connection status =`, data);
        });
        
        system.registerEventHandler('twitch:main', 'chat_message', (data) => {
            eventCount++;
            console.log(`💬 Event ${eventCount}: Chat message =`, data);
        });
        
        system.registerEventHandler('twitch:main', 'error_state', (data) => {
            eventCount++;
            console.log(`🚨 Event ${eventCount}: Error state =`, data);
        });
        console.log('✅ Event monitoring active');
        
        // Step 3: Spawn component
        console.log(`${step++}. 🚀 Spawning Twitch component...`);
        await system.spawn('twitch', 'main');
        console.log('✅ Component spawned (should auto-connect)');
        
        // Wait for auto-connection events
        console.log(`${step++}. ⏳ Waiting for auto-connection events...`);
        await new Promise(resolve => setTimeout(resolve, 2000));
        
        // Step 4: Render component
        console.log(`${step++}. 🎨 Rendering component...`);
        await system.render('#component-container', 'twitch:main');
        console.log('✅ Component rendered');
        
        // Step 5: Manual connection if needed
        console.log(`${step++}. 🔗 Checking connection status...`);
        system.websocket.send('twitch:main get_status');
        
        await new Promise(resolve => setTimeout(resolve, 1000));
        
        console.log('');
        console.log('🎉 Test Complete! Results:');
        console.log(`📊 Total events received: ${eventCount}`);
        console.log('');
        
        if (eventCount > 0) {
            console.log('✅ SUCCESS: Events are flowing from server to UI!');
            console.log('🎯 Your real-time subscription system is working');
        } else {
            console.log('⚠️  No events received yet. Possible issues:');
            console.log('   1. Check server logs for event generation');
            console.log('   2. Verify Twitch config in config.toml');
            console.log('   3. Try manual connection: connectTwitchManually()');
        }
        
        console.log('');
        console.log('🔧 Next steps:');
        console.log('   - Watch the rendered component UI for real-time updates');
        console.log('   - Check connection status changes');
        console.log('   - Look for chat messages if connected to active channel');
        
    } catch (error) {
        console.error('❌ Test failed:', error);
        console.log('💡 Try individual steps manually');
    }
}

// Auto-run test when script loads
testCompleteEventFlow();
