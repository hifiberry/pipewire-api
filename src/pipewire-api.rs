use anyhow::Result;
use pw_api::{PipeWireClient, AppState, create_router};
use std::sync::Arc;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "pipewire-api")]
#[command(about = "REST API server for SpeakerEQ 2x2 PipeWire plugin", long_about = None)]
struct Args {
    /// Port to listen on
    #[arg(short, long, default_value_t = 2716)]
    port: u16,

    /// Bind to localhost only (127.0.0.1) instead of all interfaces (0.0.0.0)
    #[arg(long)]
    localhost: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Verify node exists
    let client = PipeWireClient::new()?;
    let (info, _node) = client.find_and_bind_node("speakereq2x2", 2)?;
    
    tracing::info!("Found speakereq2x2 node (id: {})", info.id);

    // Create shared state (just the node name, we'll reconnect per request)
    let state = Arc::new(AppState::new(info.name));

    // Create router
    let app = create_router(state);

    // Bind to localhost or all interfaces
    let host = if args.localhost { "127.0.0.1" } else { "0.0.0.0" };
    let addr = format!("{}:{}", host, args.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Server listening on http://{}", addr);
    
    axum::serve(listener, app).await?;

    Ok(())
}
