use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Context, Result, bail, ensure};

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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExtractionConfig {
    pub project: PathBuf,
    pub modules: Vec<String>,
}

impl ExtractionConfig {
    pub fn new(project: impl Into<PathBuf>, modules: Vec<String>) -> Result<Self> {
        ensure!(
            !modules.is_empty(),
            "at least one --import module is required"
        );
        ensure!(
            modules.iter().all(|module| !module.trim().is_empty()),
            "imported module names cannot be empty"
        );
        let mut unique_modules = Vec::with_capacity(modules.len());
        for module in modules {
            if !unique_modules.contains(&module) {
                unique_modules.push(module);
            }
        }
        Ok(Self {
            project: project.into(),
            modules: unique_modules,
        })
    }
}

pub fn extract(config: &ExtractionConfig, targets: &[String]) -> Result<ExtractionSnapshot> {
    extract_internal(config, targets, false)
}

pub fn extract_with_proof_steps(
    config: &ExtractionConfig,
    targets: &[String],
) -> Result<ExtractionSnapshot> {
    extract_internal(config, targets, true)
}

static TEMP_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

struct TemporaryExtractor(PathBuf);

impl TemporaryExtractor {
    fn create() -> Result<Self> {
        for _ in 0..100 {
            let sequence = TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "sieve-extractor-{}-{sequence}.lean",
                std::process::id()
            ));
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(mut file) => {
                    let temporary = Self(path);
                    file.write_all(include_str!("../Sieve/Extract.lean").as_bytes())
                        .with_context(|| {
                            format!(
                                "failed to write temporary Lean extractor {}",
                                temporary.path().display()
                            )
                        })?;
                    return Ok(temporary);
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(error).with_context(|| {
                        format!(
                            "failed to create temporary Lean extractor {}",
                            path.display()
                        )
                    });
                }
            }
        }
        bail!("failed to allocate a unique temporary Lean extractor path")
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TemporaryExtractor {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn extract_internal(
    config: &ExtractionConfig,
    targets: &[String],
    include_proof_steps: bool,
) -> Result<ExtractionSnapshot> {
    ensure!(
        config.project.is_dir(),
        "Lean project directory does not exist: {}",
        config.project.display()
    );
    let extractor = TemporaryExtractor::create()?;
    let mut command = Command::new(lake_path());
    command
        .current_dir(&config.project)
        .args(["env", "lean", "--run"])
        .arg(extractor.path());
    if include_proof_steps {
        command.arg("--proof-steps");
    }
    for module in &config.modules {
        command.args(["--module", module]);
    }
    if !targets.is_empty() {
        command.arg("--");
        command.args(targets);
    }
    let output = command.output().with_context(|| {
        format!(
            "failed to start the Lean extractor in project {}",
            config.project.display()
        )
    })?;
    if !output.status.success() {
        bail!(
            "Lean extractor failed in project {}:\n{}",
            config.project.display(),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    serde_json::from_slice(&output.stdout).context("Lean extractor emitted invalid JSON")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extraction_config_requires_modules_and_deduplicates_them() {
        assert!(ExtractionConfig::new(".", vec![]).is_err());
        let config = ExtractionConfig::new(
            "/tmp/example",
            vec!["Example.Basic".into(), "Example.Basic".into()],
        )
        .unwrap();
        assert_eq!(config.modules, vec!["Example.Basic"]);
    }
}
