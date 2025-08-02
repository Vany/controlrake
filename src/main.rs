
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize the ControlRake system
    let (_bus, _registry) = controlrake::init().await?;

    // Set up graceful shutdown handler
    tokio::spawn(async {
        tokio::signal::ctrl_c().await.ok();
        
        tracing::info!("🛑 Shutdown signal received, initiating graceful shutdown...");
        
        // Step 1: Stop the system component gracefully
        match controlrake::shutdown_system().await {
            Ok(_) => tracing::info!("✅ System component shutdown initiated successfully"),
            Err(e) => tracing::error!("❌ System component shutdown failed: {}", e),
        }
        
        // Step 2: Stop the bus gracefully
        match controlrake::shutdown_bus().await {
            Ok(_) => tracing::info!("✅ Message bus shut down successfully"),
            Err(e) => tracing::error!("❌ Message bus shutdown failed: {}", e),
        }
        
        // Step 3: Release process lock and exit
        tracing::info!("🔓 Releasing process lock...");
        controlrake::component::system::process_lock::release_process_lock();
        
        tracing::info!("🏁 Graceful shutdown complete");
        std::process::exit(0);
    });

    // Keep the service running
    tracing::info!("🎯 ControlRake running - press Ctrl+C to stop");
    
    // Wait indefinitely (shutdown is handled by the signal handler)
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
    }
}
