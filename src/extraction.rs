use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::model::ExtractionSnapshot;

pub fn lake_path() -> PathBuf {
    if let Some(path) = std::env::var_os("SIEVE_LAKE") {
        return path.into();
    }
    if let Some(home) = std::env::var_os("HOME") {
        let elan_lake = PathBuf::from(home).join(".elan/bin/lake");
        if elan_lake.is_file() {
            return elan_lake;
        }
    }
    "lake".into()
}

pub fn extract(targets: &[String]) -> Result<ExtractionSnapshot> {
    extract_internal(targets, false)
}

pub fn extract_with_proof_steps(targets: &[String]) -> Result<ExtractionSnapshot> {
    extract_internal(targets, true)
}

fn extract_internal(targets: &[String], include_proof_steps: bool) -> Result<ExtractionSnapshot> {
    let mut command = Command::new(lake_path());
    command.args(["exe", "sieve_extract"]);
    if include_proof_steps {
        command.arg("__sieve_proof_steps__");
    }
    command.args(targets);
    let output = command
        .output()
        .context("failed to start the Lean extractor through Lake")?;
    if !output.status.success() {
        bail!(
            "Lean extractor failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    serde_json::from_slice(&output.stdout).context("Lean extractor emitted invalid JSON")
}
