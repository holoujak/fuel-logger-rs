use std::path::Path;

use anyhow::{Context, Result};
use tokio::process::Command;
use tracing::{info, warn};

/// Capture a single PNG frame from an RTSP camera using ffmpeg.
///
/// Files are stored as `{snapshot_dir}/{station_id}/{timestamp}.png`,
/// matching the original Python `RtspLogger` layout.
///
/// Returns the relative path (e.g. `"1/2026-03-21-14-30-00.png"`) on success.
pub async fn capture_snapshot(
    rtsp_url: &str,
    snapshot_dir: &str,
    station_id: u32,
    created_at: chrono::DateTime<chrono_tz::Tz>,
) -> Result<String> {
    let station_dir = Path::new(snapshot_dir).join(station_id.to_string());
    tokio::fs::create_dir_all(&station_dir)
        .await
        .context("Failed to create snapshot directory")?;

    let timestamp = created_at.format("%Y-%m-%d-%H-%M-%S");
    let relative_path = format!("{station_id}/{timestamp}.png");
    let filepath = Path::new(snapshot_dir).join(&relative_path);

    info!(
        "Capturing snapshot from {rtsp_url} → {}",
        filepath.display()
    );

    let output = Command::new("ffmpeg")
        .args([
            "-rtsp_transport",
            "tcp",
            "-i",
            rtsp_url,
            "-frames:v",
            "1",
            "-y",
        ])
        .arg(&filepath)
        .output()
        .await
        .context("Failed to execute ffmpeg")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("ffmpeg failed (exit {}): {stderr}", output.status);
    }

    info!("Snapshot saved: {relative_path}");
    Ok(relative_path)
}

/// Spawn snapshot capture in background — log errors but don't block the caller.
/// Returns a JoinHandle whose result is the relative path on success.
pub fn capture_snapshot_background(
    rtsp_url: String,
    snapshot_dir: String,
    station_id: u32,
    created_at: chrono::DateTime<chrono_tz::Tz>,
) -> tokio::task::JoinHandle<Option<String>> {
    tokio::spawn(async move {
        match capture_snapshot(&rtsp_url, &snapshot_dir, station_id, created_at).await {
            Ok(path) => Some(path),
            Err(e) => {
                warn!("Snapshot capture failed for station {station_id}: {e:#}");
                None
            }
        }
    })
}
