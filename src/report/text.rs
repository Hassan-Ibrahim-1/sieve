use std::fmt::Write;

use crate::analysis::comparison::DeclarationComparison;
use crate::analysis::dependency::{
    CentralityResult, DependencyPath, DependencySummary, GraphStructureResult, TraversalResult,
};
use crate::analysis::lenses::{
    BridgeEvidence, DiscoveryReport, InfluenceEvidence, NeighborEvidence, TheoremLensReport,
};
use crate::analysis::metrics::{CorpusSummary, DeclarationMetrics, RankedDeclaration};
use crate::analysis::proof_steps::ProofStepsReport;
use crate::analysis::structure::{RepeatedSubexpression, StructuralMatch, layer_selection_name};
use crate::model::DeclarationSnapshot;

pub fn summary(summary: &CorpusSummary, schema: usize, lean: &str, module: &str) -> String {
    let mut out = String::new();
    writeln!(out, "Sieve schema {schema} — Lean {lean} — {module}").unwrap();
    writeln!(out, "filters: generated={}, infrastructure={}, internal_only={}, source_backed_only={}, layer={}", summary.filter.include_generated, summary.filter.include_infrastructure, summary.filter.internal_only, summary.filter.source_backed_only, layer_selection_name(summary.filter.layer)).unwrap();
    writeln!(
        out,
        "corpus: {} declarations, {} theorems, {} generated/private",
        summary.declaration_count, summary.theorem_count, summary.generated_count
    )
    .unwrap();
    writeln!(out, "expressions: statement {} occurrences/{} unique nodes; value {} occurrences/{} unique nodes", summary.statement_occurrences, summary.statement_unique_nodes, summary.value_occurrences, summary.value_unique_nodes).unwrap();
    writeln!(
        out,
        "dependencies: {} edges, {} occurrences, {} internal edges",
        summary.dependency_edges, summary.dependency_occurrences, summary.internal_dependency_edges
    )
    .unwrap();
    if let (Some(mean), Some(median)) = (
        summary.statement_size_distribution.mean,
        summary.statement_size_distribution.median,
    ) {
        writeln!(
            out,
            "statement size: min {:.0}, mean {:.1}, median {:.1}, max {:.0}",
            summary.statement_size_distribution.minimum.unwrap(),
            mean,
            median,
            summary.statement_size_distribution.maximum.unwrap()
        )
        .unwrap();
    }
    writeln!(out, "declaration kinds:").unwrap();
    for (kind, count) in &summary.by_kind {
        writeln!(out, "  {count:>5}  {kind}").unwrap();
    }
    out
}

pub fn declaration(
    snapshot: &DeclarationSnapshot,
    metrics: &DeclarationMetrics,
    tree: Option<&str>,
) -> String {
    let mut out = String::new();
    writeln!(out, "{} [{}]", snapshot.name, snapshot.kind).unwrap();
    writeln!(
        out,
        "  module: {}",
        snapshot.module_name.as_deref().unwrap_or("<unknown>")
    )
    .unwrap();
    if let Some(range) = &snapshot.source_range {
        writeln!(
            out,
            "  source: {}:{}–{}:{}",
            range.start.line, range.start.column, range.end.line, range.end.column
        )
        .unwrap();
    }
    if let Some(doc) = &snapshot.doc_string {
        writeln!(out, "  documentation: {}", doc.replace('\n', " ")).unwrap();
    }
    writeln!(
        out,
        "  flags: internal={}, private={}, unsafe={}, partial={}",
        snapshot.is_internal, snapshot.is_private, snapshot.is_unsafe, snapshot.is_partial
    )
    .unwrap();
    writeln!(out, "  type: {}", snapshot.r#type).unwrap();
    write_expression(&mut out, "statement", &metrics.statement);
    if let Some(value) = &metrics.value {
        write_expression(
            &mut out,
            metrics.value_label.as_deref().unwrap_or("value"),
            value,
        );
    } else {
        writeln!(out, "  value: absent").unwrap();
    }
    writeln!(
        out,
        "  transitive axioms ({}): {}",
        snapshot.axioms.len(),
        snapshot.axioms.join(", ")
    )
    .unwrap();
    if let Some(tree) = tree {
        writeln!(out, "  value tree:\n{tree}").unwrap();
    }
    out
}

pub fn proof_steps(report: &ProofStepsReport<'_>) -> String {
    let mut out = String::new();
    writeln!(out, "proof steps for {}", report.declaration).unwrap();
    writeln!(
        out,
        "  extraction: complete={}, visited_terms={}, steps={}, conclusion={}",
        report.complete,
        report.visited_terms,
        report.steps.len(),
        report
            .conclusion_step
            .map(|id| format!("#{id}"))
            .unwrap_or_else(|| "<none>".into())
    )
    .unwrap();
    if let Some(reason) = report.truncation_reason {
        writeln!(out, "  incomplete: {reason}").unwrap();
    }
    for step in report.steps {
        let path = step
            .proof_term_path
            .iter()
            .map(usize::to_string)
            .collect::<Vec<_>>()
            .join(".");
        writeln!(out, "\n#{:<4} [{}] term-path={path}", step.id, step.kind).unwrap();
        writeln!(out, "  establishes: {}", step.proposition).unwrap();
        if !step.prerequisite_steps.is_empty() {
            writeln!(
                out,
                "  direct prerequisite steps: {}",
                step.prerequisite_steps
                    .iter()
                    .map(|id| format!("#{id}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
            .unwrap();
        }
        if !step.hypothesis_references.is_empty() {
            writeln!(
                out,
                "  hypotheses used: {}",
                step.hypothesis_references.join(", ")
            )
            .unwrap();
        }
        if !step.named_references.is_empty() {
            writeln!(out, "  named results: {}", step.named_references.join(", ")).unwrap();
        }
        writeln!(out, "  context:").unwrap();
        for entry in &step.context {
            writeln!(
                out,
                "    {}  {} [{}]: {}",
                entry.id, entry.user_name, entry.kind, entry.r#type
            )
            .unwrap();
        }
    }
    if let Some(selected) = &report.selected {
        writeln!(out, "\nselected step #{} evidence", selected.step).unwrap();
        writeln!(
            out,
            "  direct prerequisites: {}",
            format_step_ids(&selected.direct_prerequisites)
        )
        .unwrap();
        writeln!(
            out,
            "  transitive prerequisites: {}",
            format_step_ids(&selected.transitive_prerequisites)
        )
        .unwrap();
        writeln!(
            out,
            "  direct dependents: {}",
            format_step_ids(&selected.direct_dependents)
        )
        .unwrap();
        writeln!(
            out,
            "  paths to conclusion{}:",
            if selected.paths_truncated {
                " (truncated)"
            } else {
                ""
            }
        )
        .unwrap();
        for path in &selected.paths_to_conclusion {
            writeln!(out, "    {}", format_step_ids(path)).unwrap();
        }
    }
    writeln!(out, "\nnamed result statements:").unwrap();
    for result in report.named_results {
        writeln!(
            out,
            "  {} [{}]: {}",
            result.name, result.kind, result.r#type
        )
        .unwrap();
    }
    out
}

fn format_step_ids(ids: &[usize]) -> String {
    if ids.is_empty() {
        return "<none>".into();
    }
    ids.iter()
        .map(|id| format!("#{id}"))
        .collect::<Vec<_>>()
        .join(" -> ")
}

fn write_expression(
    out: &mut String,
    label: &str,
    metrics: &crate::analysis::metrics::ExpressionMetrics,
) {
    writeln!(
        out,
        "  {label}: {} occurrences, {} unique nodes, {:.2}× DAG compression, depth {}",
        metrics.total_occurrences,
        metrics.unique_nodes,
        metrics.dag_compression_ratio,
        metrics.maximum_depth
    )
    .unwrap();
    writeln!(out, "    constants: {} occurrences / {} distinct; dependencies: {} internal + {} external across {} modules", metrics.total_constant_occurrences, metrics.distinct_constants, metrics.internal_dependencies, metrics.external_dependencies, metrics.modules_referenced).unwrap();
    let frequent = metrics
        .constant_frequencies
        .iter()
        .take(10)
        .map(|entry| format!("{}×{}", entry.name, entry.occurrences))
        .collect::<Vec<_>>();
    if !frequent.is_empty() {
        writeln!(out, "    most frequent constants: {}", frequent.join(", ")).unwrap();
    }
}

pub fn dependencies(summary: &DependencySummary) -> String {
    let mut out = format!(
        "{} dependencies for {} (layer={:?}, weighted={}, truncated={}):\n",
        summary.direction,
        summary.declaration,
        summary.config.filter.layer,
        summary.config.weighted,
        summary.truncated
    );
    if !summary.occurrences_by_module.is_empty() {
        let modules = summary
            .occurrences_by_module
            .iter()
            .map(|(module, count)| format!("{module}×{count}"))
            .collect::<Vec<_>>();
        writeln!(out, "  occurrences by module: {}", modules.join(", ")).unwrap();
    }
    for edge in &summary.edges {
        writeln!(
            out,
            "  {:>6}  {:?}  {} -> {} [{}{}]",
            edge.occurrences,
            edge.layer,
            edge.source,
            edge.target,
            edge.target_kind,
            if edge.target_in_corpus {
                ", internal"
            } else {
                ", external"
            }
        )
        .unwrap();
    }
    out
}

pub fn traversal(result: &TraversalResult) -> String {
    let mut out = format!(
        "{} traversal from {} (depth≤{}, limit={}, truncated={}):\n",
        result.direction,
        result.start,
        result.config.max_depth,
        result.config.limit,
        result.truncated
    );
    for node in &result.nodes {
        writeln!(
            out,
            "  depth {:>2}  weight {:>6}  {}",
            node.depth, node.path_weight, node.name
        )
        .unwrap();
    }
    out
}

pub fn path(path: &DependencyPath) -> String {
    if path.nodes.is_empty() {
        return "no dependency path found\n".into();
    }
    format!(
        "shortest path ({} edges, total occurrence weight {}):\n  {}\n",
        path.layers.len(),
        path.total_weight,
        path.nodes.join(" -> ")
    )
}

pub fn ranking(entries: &[RankedDeclaration]) -> String {
    let mut out = String::new();
    for entry in entries {
        writeln!(
            out,
            "{:>3}. {:>12.3}  {}",
            entry.rank, entry.value, entry.name
        )
        .unwrap();
    }
    out
}

pub fn comparison(result: &DeclarationComparison) -> String {
    let mut out = format!(
        "{} comparison: {} vs {}\nexact={}, alpha_equivalent={}, combined_similarity={:.3}\n",
        format!("{:?}", result.layer).to_lowercase(),
        result.left,
        result.right,
        result.exact_equal,
        result.alpha_equivalent,
        result.similarity.combined_score
    );
    for line in &result.explanation {
        writeln!(out, "  {line}").unwrap();
    }
    writeln!(
        out,
        "  shared dependencies ({}): {}",
        result.shared_dependencies.len(),
        result.shared_dependencies.join(", ")
    )
    .unwrap();
    out
}

pub fn discovery(report: &DiscoveryReport) -> String {
    let mut out = String::new();
    if !report.influence.is_empty() {
        writeln!(out, "influence lens").unwrap();
        for evidence in &report.influence {
            write_influence(&mut out, evidence);
        }
    }
    if !report.bridges.is_empty() {
        writeln!(out, "bridge lens").unwrap();
        for evidence in &report.bridges {
            write_bridge(&mut out, evidence);
        }
    }
    if !report.neighbors.is_empty() {
        writeln!(out, "statement-neighbor lens").unwrap();
        for evidence in &report.neighbors {
            write_neighbors(&mut out, evidence);
        }
    }
    out
}

pub fn theorem_lenses(report: &TheoremLensReport) -> String {
    let mut out = String::new();
    writeln!(out, "{}", report.theorem).unwrap();
    writeln!(out, "  statement: {}", report.statement).unwrap();
    for evidence in &report.influence {
        write_influence(&mut out, evidence);
    }
    for evidence in &report.bridges {
        write_bridge(&mut out, evidence);
    }
    for evidence in &report.neighbors {
        write_neighbors(&mut out, evidence);
    }
    if let Some(proof) = &report.proof_outline {
        writeln!(out, "  proof outline").unwrap();
        writeln!(
            out,
            "    extraction: complete={}, raw_steps={}, retained_steps={}, condensed_edges={}",
            proof.complete,
            proof.raw_steps.len(),
            proof.retained_nodes.len(),
            proof.condensed_edges.len()
        )
        .unwrap();
        if let Some(reason) = &proof.truncation_reason {
            writeln!(out, "    incomplete: {reason}").unwrap();
        }
        for node in &proof.retained_nodes {
            writeln!(
                out,
                "    #{} [{}] {}",
                node.raw_step_id, node.kind, node.proposition
            )
            .unwrap();
            if !node.named_references.is_empty() {
                writeln!(
                    out,
                    "      named results: {}",
                    node.named_references.join(", ")
                )
                .unwrap();
            }
        }
        for edge in &proof.condensed_edges {
            writeln!(
                out,
                "    #{} -> #{} via {}",
                edge.source,
                edge.target,
                format_raw_path(&edge.raw_step_path)
            )
            .unwrap();
        }
    }
    if let Some(trust) = &report.trust {
        writeln!(out, "  trust evidence").unwrap();
        writeln!(
            out,
            "    axioms ({}): {}",
            trust.axioms.len(),
            if trust.axioms.is_empty() {
                "<none>".into()
            } else {
                trust.axioms.join(", ")
            }
        )
        .unwrap();
        writeln!(
            out,
            "    flags: generated={}, internal={}, private={}, unsafe={}, partial={}",
            trust.is_generated,
            trust.is_internal,
            trust.is_private,
            trust.is_unsafe,
            trust.is_partial
        )
        .unwrap();
        writeln!(
            out,
            "    source={}, documentation={}",
            availability(trust.source_available),
            availability(trust.documentation_available)
        )
        .unwrap();
        writeln!(
            out,
            "    proof extraction: available={}, complete={}",
            trust.proof_extraction.available,
            trust
                .proof_extraction
                .complete
                .map(|value| value.to_string())
                .unwrap_or_else(|| "not extracted".into())
        )
        .unwrap();
    }
    out
}

fn write_influence(out: &mut String, evidence: &InfluenceEvidence) {
    writeln!(out, "\n{}", evidence.name).unwrap();
    writeln!(out, "  statement: {}", evidence.statement).unwrap();
    writeln!(
        out,
        "  reason [{}]: {}",
        format!("{:?}", evidence.layer).to_lowercase(),
        evidence.reason
    )
    .unwrap();
    writeln!(
        out,
        "  direct dependents ({}): {}",
        evidence.direct_dependent_count,
        display_names(&evidence.direct_dependents)
    )
    .unwrap();
    writeln!(
        out,
        "  reachable dependents ({}): {}",
        evidence.reachable_dependent_count,
        display_names(&evidence.reachable_dependents)
    )
    .unwrap();
    writeln!(
        out,
        "  graph regions ({}): {}",
        evidence.graph_region_coverage,
        display_names(&evidence.graph_regions)
    )
    .unwrap();
    for path in &evidence.representative_paths {
        writeln!(out, "  witness: {}", path.nodes.join(" -> ")).unwrap();
    }
}

fn write_bridge(out: &mut String, evidence: &BridgeEvidence) {
    writeln!(out, "\n{}", evidence.name).unwrap();
    writeln!(out, "  statement: {}", evidence.statement).unwrap();
    writeln!(
        out,
        "  reason [{}]: {}",
        format!("{:?}", evidence.layer).to_lowercase(),
        evidence.reason
    )
    .unwrap();
    writeln!(
        out,
        "  articulation={}, directed_region_pairs={}, betweenness={:.3}",
        evidence.articulation_point,
        evidence.distinct_directed_region_pair_count,
        evidence.betweenness
    )
    .unwrap();
    for path in &evidence.witness_paths {
        writeln!(out, "  witness: {}", path.nodes.join(" -> ")).unwrap();
    }
}

fn write_neighbors(out: &mut String, evidence: &NeighborEvidence) {
    writeln!(out, "\n{}", evidence.name).unwrap();
    writeln!(out, "  statement: {}", evidence.statement).unwrap();
    writeln!(
        out,
        "  reason [{}]: {}",
        format!("{:?}", evidence.layer).to_lowercase(),
        evidence.reason
    )
    .unwrap();
    for neighbor in &evidence.neighbors {
        writeln!(out, "  neighbor: {}", neighbor.name).unwrap();
        writeln!(out, "    statement: {}", neighbor.statement).unwrap();
        writeln!(out, "    similarity: combined={:.3}, dependencies={:.3}, kinds={:.3}, size={:.3}, depth={:.3}", neighbor.similarity.combined_score, neighbor.similarity.dependency_jaccard, neighbor.similarity.kind_histogram_cosine, neighbor.similarity.size_ratio, neighbor.similarity.depth_ratio).unwrap();
        writeln!(
            out,
            "    exact={}, alpha_equivalent={}",
            neighbor.exact_equal, neighbor.alpha_equivalent
        )
        .unwrap();
        writeln!(
            out,
            "    shared dependencies: {}",
            display_names(&neighbor.shared_dependencies)
        )
        .unwrap();
        writeln!(
            out,
            "    focal-only dependencies: {}",
            display_names(&neighbor.focal_only_dependencies)
        )
        .unwrap();
        writeln!(
            out,
            "    neighbor-only dependencies: {}",
            display_names(&neighbor.neighbor_only_dependencies)
        )
        .unwrap();
    }
}

fn display_names(names: &[String]) -> String {
    if names.is_empty() {
        "<none>".into()
    } else {
        const TEXT_NAME_LIMIT: usize = 12;
        let mut rendered = names
            .iter()
            .take(TEXT_NAME_LIMIT)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        if names.len() > TEXT_NAME_LIMIT {
            write!(rendered, " (+{} more)", names.len() - TEXT_NAME_LIMIT).unwrap();
        }
        rendered
    }
}

fn availability(value: bool) -> &'static str {
    if value { "available" } else { "absent" }
}

fn format_raw_path(path: &[usize]) -> String {
    path.iter()
        .map(|id| format!("#{id}"))
        .collect::<Vec<_>>()
        .join(" -> ")
}

pub fn duplicates(results: &[StructuralMatch]) -> String {
    let mut out = String::new();
    for result in results {
        writeln!(
            out,
            "size {:>6}, depth {:>4}, fingerprint {}: {}",
            result.expression_size,
            result.expression_depth,
            result.fingerprint,
            result.declarations.join(", ")
        )
        .unwrap();
    }
    out
}

pub fn repeated(results: &[RepeatedSubexpression]) -> String {
    let mut out = String::new();
    for result in results {
        writeln!(
            out,
            "size {:>6}, depth {:>4}, support {:>4}, occurrences {:>8}, fingerprint {}",
            result.expression_size,
            result.expression_depth,
            result.unique_declarations,
            result.total_occurrences,
            result.fingerprint
        )
        .unwrap();
        for location in &result.locations {
            writeln!(
                out,
                "  {:?} {} #{} ×{}",
                location.layer, location.declaration, location.node, location.occurrences
            )
            .unwrap();
        }
    }
    out
}

pub fn centrality(result: &CentralityResult, limit: usize) -> String {
    let mut out = format!(
        "{} (layer={:?}, weighted={})\n",
        result.algorithm, result.config.filter.layer, result.config.weighted
    );
    for entry in result.entries.iter().take(limit) {
        writeln!(
            out,
            "  PR {:.6}  between {:>9.2}  in/out {:>3}/{:<3}  {}",
            entry.page_rank, entry.betweenness, entry.in_degree, entry.out_degree, entry.name
        )
        .unwrap();
    }
    out
}

pub fn graph_structure(result: &GraphStructureResult) -> String {
    let communities = result
        .communities
        .values()
        .copied()
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    format!(
        "graph structure: {} communities, {} articulation points, {} bridges\n",
        communities,
        result.articulation_points.len(),
        result.bridges.len()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::filters::{AnalysisFilter, DependencyLayer};
    use crate::analysis::lenses::{DependencyWitnessPath, LensAnalysisConfig, LensKind};

    #[test]
    fn discovery_text_snapshot_is_statement_first_and_deterministic() {
        let report = DiscoveryReport {
            requested_lenses: vec![LensKind::Influence],
            config: LensAnalysisConfig {
                filter: AnalysisFilter::default(),
                layer_explicit: false,
                max_depth: 32,
                limit: 1,
                representative_path_limit: 3,
            },
            ranking_rules: vec![],
            graph_region_algorithm: "fixture".into(),
            graph_regions: vec![],
            influence: vec![InfluenceEvidence {
                name: "Foundation".into(),
                statement: "P".into(),
                reason: "reachable from 1 theorem(s), including 1 direct dependent(s), across 1 graph region(s)".into(),
                layer: DependencyLayer::Proof,
                direct_dependent_count: 1,
                direct_dependents: vec!["Later".into()],
                reachable_dependent_count: 1,
                reachable_dependents: vec!["Later".into()],
                graph_region_coverage: 1,
                graph_regions: vec!["proof-region-0001".into()],
                representative_paths: vec![DependencyWitnessPath {
                    nodes: vec!["Later".into(), "Foundation".into()],
                    layers: vec![DependencyLayer::Proof],
                }],
                traversal_truncated: false,
            }],
            bridges: vec![],
            neighbors: vec![],
        };
        assert_eq!(
            discovery(&report),
            "influence lens\n\nFoundation\n  statement: P\n  reason [proof]: reachable from 1 theorem(s), including 1 direct dependent(s), across 1 graph region(s)\n  direct dependents (1): Later\n  reachable dependents (1): Later\n  graph regions (1): proof-region-0001\n  witness: Later -> Foundation\n"
        );
    }
}
