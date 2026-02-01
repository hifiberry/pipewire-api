use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use tracing::{debug, error, info, warn};

use crate::api_server::AppState;
use crate::link_manager_cli;
use crate::linker::LogLevel;

/// Log a message at the specified level
macro_rules! log_at_level {
    ($level:expr, $($arg:tt)*) => {
        match $level {
            LogLevel::Debug => debug!($($arg)*),
            LogLevel::Info => info!($($arg)*),
            LogLevel::Warn => warn!($($arg)*),
            LogLevel::Error => error!($($arg)*),
        }
    };
}

/// Start the link scheduler task that monitors and relinks based on rules
pub fn start_link_scheduler(state: Arc<AppState>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        // Check every second for rules that need to be applied
        let mut ticker = interval(Duration::from_secs(1));
        let mut last_check: std::collections::HashMap<usize, std::time::Instant> =
            std::collections::HashMap::new();

        info!("Link scheduler started");

        loop {
            ticker.tick().await;

            let rules = state.get_link_rules();
            if rules.is_empty() {
                continue;
            }

            for (idx, rule) in rules.iter().enumerate() {
                // Skip if relink_every is 0 and we've already processed this rule
                if rule.relink_every == 0 && last_check.contains_key(&idx) {
                    continue;
                }

                // Check if it's time to apply this rule
                let should_apply = if let Some(last) = last_check.get(&idx) {
                    last.elapsed() >= Duration::from_secs(rule.relink_every)
                } else {
                    // First time seeing this rule, apply if link_at_startup is true
                    rule.link_at_startup
                };

                if should_apply {
                    debug!(
                        "Applying link rule '{}' (idx: {}, relink_every: {}s)",
                        rule.name, idx, rule.relink_every
                    );

                    // Apply the rule
                    match apply_rule_safe(rule).await {
                        Ok(results) => {
                            let success_count = results.iter().filter(|r| r.success).count();
                            let failed_count = results.iter().filter(|r| !r.success).count();
                            let total = results.len();

                            // Log successful links at info_level
                            if success_count > 0 {
                                log_at_level!(
                                    &rule.info_level,
                                    "Link rule '{}' applied: {}/{} links successful",
                                    rule.name, success_count, total
                                );
                            }

                            let error_msg = if failed_count > 0 {
                                let errors: Vec<String> = results.iter()
                                    .filter(|r| !r.success)
                                    .map(|r| r.message.clone())
                                    .collect();
                                Some(errors.join("; "))
                            } else {
                                None
                            };

                            // Update rule status
                            state.update_rule_status(idx, success_count, failed_count, error_msg.clone());

                            // Log failures at the rule's configured error_level
                            if failed_count > 0 {
                                if let Some(ref err_msg) = error_msg {
                                    log_at_level!(
                                        &rule.error_level,
                                        "Link rule '{}' failed: {}",
                                        rule.name,
                                        err_msg
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            log_at_level!(
                                &rule.error_level,
                                "Failed to apply link rule '{}': {}",
                                rule.name,
                                e
                            );
                            // Update status with error
                            state.update_rule_status(idx, 0, 0, Some(e.to_string()));
                        }
                    }

                    // Update last check time
                    last_check.insert(idx, std::time::Instant::now());
                }
            }
        }
    })
}

/// Apply a rule safely, handling any PipeWire connection issues
async fn apply_rule_safe(
    rule: &crate::linker::LinkRule,
) -> anyhow::Result<Vec<link_manager_cli::LinkRuleResult>> {
    // Run the blocking operations in a blocking task
    let rule = rule.clone();
    let result = tokio::task::spawn_blocking(move || {
        link_manager_cli::apply_link_rule(&rule)
    })
    .await?;
    
    // Convert Result<Vec<LinkRuleResult>, String> to anyhow::Result
    result.map_err(|e| anyhow::anyhow!(e))
}

/// Apply startup rules immediately
pub async fn apply_startup_rules(state: Arc<AppState>) {
    let rules = state.get_link_rules();
    
    info!("Applying {} startup link rules", rules.len());

    for (idx, rule) in rules.iter().enumerate() {
        if !rule.link_at_startup {
            debug!("Skipping rule '{}' (link_at_startup=false)", rule.name);
            continue;
        }

        debug!("Applying startup rule '{}'", rule.name);
        match apply_rule_safe(rule).await {
            Ok(results) => {
                let success_count = results.iter().filter(|r| r.success).count();
                let failed_count = results.iter().filter(|r| !r.success).count();
                let total = results.len();

                if total > 0 {
                    info!(
                        "Startup rule '{}' applied: {}/{} links successful",
                        rule.name, success_count, total
                    );
                }

                let error_msg = if failed_count > 0 {
                    let errors: Vec<String> = results.iter()
                        .filter(|r| !r.success)
                        .map(|r| r.message.clone())
                        .collect();
                    Some(errors.join("; "))
                } else {
                    None
                };

                // Update rule status
                state.update_rule_status(idx, success_count, failed_count, error_msg.clone());

                // Log results using appropriate log levels
                for result in results {
                    if result.success {
                        log_at_level!(
                            &rule.info_level,
                            "  ✓ {}",
                            result.message
                        );
                    } else {
                        log_at_level!(
                            &rule.error_level,
                            "  ✗ {}",
                            result.message
                        );
                    }
                }
                
                // Also log a summary if there were failures
                if failed_count > 0 {
                    if let Some(ref err_msg) = error_msg {
                        log_at_level!(
                            &rule.error_level,
                            "Startup rule '{}' had {} failure(s): {}",
                            rule.name,
                            failed_count,
                            err_msg
                        );
                    }
                }
            }
            Err(e) => {
                log_at_level!(
                    &rule.error_level,
                    "Failed to apply startup rule '{}': {}",
                    rule.name,
                    e
                );
                // Update status with error
                state.update_rule_status(idx, 0, 0, Some(e.to_string()));
            }
        }
    }
}
