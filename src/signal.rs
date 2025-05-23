use tokio::{select, signal};
use tracing::debug;

use crate::common::TestrpcError;

pub async fn wait_exit_signals() -> Result<(), TestrpcError> {
    let mut terminate = signal::unix::signal(signal::unix::SignalKind::terminate())
        .map_err(|e| TestrpcError::TerminationError(e.to_string()))?;
    let mut interrupt = signal::unix::signal(signal::unix::SignalKind::interrupt())
        .map_err(|e| TestrpcError::TerminationError(e.to_string()))?;
    let mut quit = signal::unix::signal(signal::unix::SignalKind::quit())
        .map_err(|e| TestrpcError::TerminationError(e.to_string()))?;

    select! {
        _ = terminate.recv() => {
            debug!("Received terminate signal");
        }
        _ = interrupt.recv() => {
            debug!("Received interrupt signal");
        }
        _ = quit.recv() => {
            debug!("Received quit signal");
        }
    }

    Ok(())
}
