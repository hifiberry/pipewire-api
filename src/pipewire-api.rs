use anyhow::Result;
use pw_api::{AppState, NodeState};
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

    /// Disable automatic link management
    #[arg(long)]
    no_auto_link: bool,

    /// Do not start the API server, only apply initial rules and exit
    #[arg(long)]
    no_api: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create global application state (not tied to any specific node)
    let app_state = Arc::new(AppState::new());

    // Load PipeWire object cache on startup
    if let Err(e) = app_state.refresh_object_cache() {
        tracing::warn!("Failed to load object cache on startup: {}", e);
    }

    // Load and apply volume rules on startup
    let volume_rules = pw_api::config::load_all_volume_rules();
    if !volume_rules.is_empty() {
        tracing::info!("Applying {} volume rule(s)", volume_rules.len());
        if let Err(e) = pw_api::volume::apply_volume_rules(volume_rules) {
            tracing::error!("Failed to apply volume rules: {}", e);
        }
    }

    // Load link rules unless disabled
    if !args.no_auto_link {
        // Load rules from config files (user config takes precedence over system config)
        let mut all_rules = pw_api::config::load_all_link_rules();
        
        // If no rules were loaded from config files, use hardcoded defaults
        if all_rules.is_empty() {
            tracing::info!("No config files found, using hardcoded default rules");
            let default_rules = pw_api::default_link_rules::get_default_rules();
            tracing::info!("Loaded {} hardcoded default rule(s)", default_rules.len());
            all_rules.extend(default_rules);
        }
        
        tracing::info!("Total {} link rule(s) configured", all_rules.len());
        app_state.set_link_rules(all_rules);

        // Apply startup rules
        pw_api::link_scheduler::apply_startup_rules(app_state.clone()).await;

        // If --no-api is set, exit now after applying rules
        if args.no_api {
            tracing::info!("Initial rules applied, exiting (--no-api mode)");
            return Ok(());
        }

        // Start the link scheduler for periodic relinking
        let _scheduler_handle = pw_api::link_scheduler::start_link_scheduler(app_state.clone());
    } else if args.no_api {
        // --no-api without link rules, just exit
        tracing::info!("Volume rules applied, exiting (--no-api mode)");
        return Ok(());
    }

    // Create node-specific state for modules that manage specific nodes
    // speakereq uses pattern matching to find speakereq2x2, speakereq4x4, etc.
    let speakereq_state = Arc::new(NodeState::with_pattern(
        "speakereq".to_string(),
        r"speakereq[0-9]+x[0-9]+".to_string()
    ));
    let riaa_state = Arc::new(NodeState::new("riaa".to_string()));
    
    // Create router with global api and module-specific endpoints
    let app = pw_api::api::create_router(app_state)
        .merge(pw_api::speakereq::create_router(speakereq_state))
        .merge(pw_api::riaa::create_router(riaa_state));

    // Bind to localhost or all interfaces
    let host = if args.localhost { "127.0.0.1" } else { "0.0.0.0" };
    let addr = format!("{}:{}", host, args.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Server listening on http://{}", addr);
    
    axum::serve(listener, app).await?;

    Ok(())
}
