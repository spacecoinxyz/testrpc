use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::task;
use tokio::time::Duration;

use crate::common::{RoundResults, TestflowError};
use crate::config::{self, Adapter};
use crate::{ctx, hotshot};

pub async fn load_endpoints(cfg: config::Config) -> Result<Vec<String>, TestflowError> {
    match cfg.adapter {
        Adapter::Hotshot => hotshot::load_endpoints(cfg.args.clone()).await,
        _ => Err(TestflowError::UnsupportedAdapter(cfg.adapter.to_string())),
    }
}

/// Run the test flow with the given configuration.
/// This function will run the test flow until we reach cfg.iterations or if the context is stopped.
/// Upon completion, we wait for all the open threads to complete. and the function will return a vector of RoundResults.
pub async fn run(
    ctx: Arc<ctx::Context>,
    cfg: config::Config,
    rpc_urls: Vec<String>,
) -> Result<Vec<RoundResults>, TestflowError> {
    let mut i: u32 = 0;
    let mut quit = ctx.recv();
    let results = Arc::new(RwLock::new(Vec::new()));
    loop {
        let rounds = cfg.rounds.clone();
        let mut r: usize = 0;
        for round in rounds {
            let round_templates = cfg.round_templates.clone();
            let rpc_urls = rpc_urls.clone();
            let results = Arc::clone(&results);
            i += 1;
            r += 1;
            let iteration = i;
            let round_num = r;
            let adapter = cfg.adapter.clone();
            tokio::select! {
                _ = task::spawn(async move {
                    match process_round(adapter, round, iteration, rpc_urls, round_templates).await {
                        Ok(result) => {
                            tracing::debug!("Iteration {} round {} completed", iteration, round_num);
                            let mut results = results.write().unwrap();
                            results.push(result);
                        }
                        Err(e) => {
                            tracing::warn!("Iteration {} round {} failed: {}", iteration, round_num, e);
                        }
                    }
                }) => {}
                _ = quit.recv() => {
                    tracing::debug!("Iteration {} round {} timed out as ctx was stopped", iteration, round_num);
                    break;
                }
            }
            tokio::select! {
                _ = quit.recv() => {
                    tracing::debug!("ctx stopped during iteration {} round {}", iteration, round_num);
                    break;
                }
                _ = tokio::time::sleep(Duration::from_secs(cfg.interval)) => {}
            }
            if let Some(iterations) = cfg.iterations {
                if i >= iterations as u32 {
                    break;
                }
            }
        }
        if let Some(iterations) = cfg.iterations {
            if i >= iterations as u32 {
                break;
            }
        }
    }
    let results = Arc::try_unwrap(results).unwrap().into_inner().unwrap();
    Ok(results)
}

/// Process a single round, sending transactions to the RPC servers concurrently
async fn process_round(
    adapter: Adapter,
    round: config::Round,
    iteration: u32,
    rpc_urls: Vec<String>,
    round_templates: HashMap<String, config::RoundTemplate>,
) -> Result<RoundResults, TestflowError> {
    match adapter {
        Adapter::Hotshot => {
            hotshot::process_round(round, iteration, rpc_urls, round_templates).await
        }
        _ => Err(TestflowError::UnsupportedAdapter(adapter.to_string())),
    }
}
