use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExprStats {
    nodes: usize,
    max_depth: usize,
    bound_variables: usize,
    free_variables: usize,
    metavariables: usize,
    sorts: usize,
    constants: usize,
    applications: usize,
    lambdas: usize,
    foralls: usize,
    lets: usize,
    literals: usize,
    metadata: usize,
    projections: usize,
}

#[derive(Debug, Deserialize)]
struct SourcePosition {
    line: usize,
    column: usize,
}

#[derive(Debug, Deserialize)]
struct SourceRange {
    start: SourcePosition,
    end: SourcePosition,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeclarationSnapshot {
    name: String,
    kind: String,
    module_name: Option<String>,
    doc_string: Option<String>,
    source_range: Option<SourceRange>,
    level_parameters: Vec<String>,
    r#type: String,
    has_value: bool,
    type_stats: ExprStats,
    value_stats: Option<ExprStats>,
    statement_dependencies: Vec<String>,
    proof_dependencies: Vec<String>,
    axioms: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExtractionSnapshot {
    schema_version: usize,
    imported_module: String,
    declarations: Vec<DeclarationSnapshot>,
}

fn lake_path() -> PathBuf {
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

fn extract(targets: &[String]) -> Result<ExtractionSnapshot> {
    let mut command = Command::new(lake_path());
    command.args(["exe", "sieve_extract"]);
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

fn main() -> Result<()> {
    let targets = std::env::args().skip(1).collect::<Vec<_>>();
    let snapshot = extract(&targets)?;

    println!(
        "Sieve schema {} — {}",
        snapshot.schema_version, snapshot.imported_module
    );

    for declaration in snapshot.declarations {
        let value_nodes = declaration
            .value_stats
            .as_ref()
            .map_or(0, |stats| stats.nodes);
        println!("\n{} [{}]", declaration.name, declaration.kind);
        println!(
            "  module: {}",
            declaration.module_name.as_deref().unwrap_or("<unknown>")
        );
        if let Some(range) = &declaration.source_range {
            println!(
                "  source: {}:{}–{}:{}",
                range.start.line, range.start.column, range.end.line, range.end.column
            );
        }
        if let Some(doc_string) = &declaration.doc_string {
            println!("  documentation: {}", doc_string.replace('\n', " "));
        }
        println!(
            "  universe parameters: {}",
            declaration.level_parameters.join(", ")
        );
        println!("  type: {}", declaration.r#type);
        println!(
            "  expression nodes: type={}, value={value_nodes}",
            declaration.type_stats.nodes
        );
        println!(
            "  shape: depth={}, constants={}, applications={}, binders={} (forall={}, lambda={}, let={})",
            declaration.type_stats.max_depth,
            declaration.type_stats.constants,
            declaration.type_stats.applications,
            declaration.type_stats.foralls
                + declaration.type_stats.lambdas
                + declaration.type_stats.lets,
            declaration.type_stats.foralls,
            declaration.type_stats.lambdas,
            declaration.type_stats.lets
        );
        println!(
            "  variables: bound={}, free={}, metavariables={}; other nodes: sorts={}, literals={}, metadata={}, projections={}",
            declaration.type_stats.bound_variables,
            declaration.type_stats.free_variables,
            declaration.type_stats.metavariables,
            declaration.type_stats.sorts,
            declaration.type_stats.literals,
            declaration.type_stats.metadata,
            declaration.type_stats.projections
        );
        println!("  has value: {}", declaration.has_value);
        println!(
            "  direct dependencies: statement={}, proof={}",
            declaration.statement_dependencies.len(),
            declaration.proof_dependencies.len()
        );
        println!("  transitive axioms: {}", declaration.axioms.join(", "));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_the_default_ftc_slice() {
        let snapshot = extract(&[]).expect("FTC extraction should succeed");

        assert_eq!(snapshot.schema_version, 1);
        assert_eq!(
            snapshot.imported_module,
            "Mathlib.MeasureTheory.Integral.IntervalIntegral.FundThmCalculus"
        );
        assert_eq!(snapshot.declarations.len(), 3);

        let theorem = snapshot
            .declarations
            .iter()
            .find(|declaration| declaration.name == "intervalIntegral.integral_deriv_eq_sub")
            .expect("the default slice should contain FTC-2");

        assert_eq!(theorem.kind, "theorem");
        assert!(theorem.has_value);
        assert!(theorem.type_stats.nodes > 0);
        assert!(
            theorem
                .value_stats
                .as_ref()
                .is_some_and(|stats| stats.nodes > 0)
        );
        assert!(
            theorem
                .statement_dependencies
                .iter()
                .any(|dependency| dependency == "IntervalIntegrable")
        );
        assert!(
            theorem
                .proof_dependencies
                .iter()
                .any(|dependency| dependency == "intervalIntegral.integral_eq_sub_of_hasDerivAt")
        );
        assert!(
            theorem
                .axioms
                .iter()
                .any(|axiom| axiom == "Classical.choice")
        );
    }

    #[test]
    fn extracts_a_requested_declaration() {
        let target = "intervalIntegral.integral_deriv_eq_sub'".to_owned();
        let snapshot =
            extract(std::slice::from_ref(&target)).expect("targeted FTC extraction should succeed");

        assert_eq!(snapshot.declarations.len(), 1);
        assert_eq!(snapshot.declarations[0].name, target);
    }
}
