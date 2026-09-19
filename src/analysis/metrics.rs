use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::analysis::corpus::AnalysisCorpus;
use crate::analysis::filters::{AnalysisFilter, DependencyLayer};
use crate::model::{DeclarationSnapshot, ExprStats, ExpressionGraph};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpressionMetrics {
    pub total_occurrences: usize,
    pub unique_nodes: usize,
    pub dag_compression_ratio: f64,
    pub maximum_depth: usize,
    pub kind_occurrences: BTreeMap<String, usize>,
    pub kind_proportions: BTreeMap<String, f64>,
    pub distinct_constants: usize,
    pub total_constant_occurrences: usize,
    pub direct_dependencies: usize,
    pub internal_dependencies: usize,
    pub external_dependencies: usize,
    pub modules_referenced: usize,
    pub binder_counts: BTreeMap<String, usize>,
    pub constant_frequencies: Vec<ConstantFrequency>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConstantFrequency {
    pub name: String,
    pub occurrences: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeclarationMetrics {
    pub name: String,
    pub kind: String,
    pub module: Option<String>,
    pub generated: bool,
    pub source_backed: bool,
    pub has_documentation: bool,
    pub axiom_count: usize,
    pub statement: ExpressionMetrics,
    pub value_label: Option<String>,
    pub value: Option<ExpressionMetrics>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionSummary {
    pub count: usize,
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
    pub mean: Option<f64>,
    pub median: Option<f64>,
    pub standard_deviation: Option<f64>,
    pub quantiles: BTreeMap<String, f64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistogramBucket {
    pub lower_inclusive: Option<f64>,
    pub upper_exclusive: Option<f64>,
    pub count: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RankedDeclaration {
    pub rank: usize,
    pub name: String,
    pub metric: String,
    pub value: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CorpusSummary {
    pub filter: AnalysisFilter,
    pub declaration_count: usize,
    pub theorem_count: usize,
    pub generated_count: usize,
    pub symbol_count: usize,
    pub statement_occurrences: usize,
    pub value_occurrences: usize,
    pub statement_unique_nodes: usize,
    pub value_unique_nodes: usize,
    pub dependency_edges: usize,
    pub dependency_occurrences: usize,
    pub internal_dependency_edges: usize,
    pub by_kind: BTreeMap<String, usize>,
    pub by_module: BTreeMap<String, usize>,
    pub statement_size_distribution: DistributionSummary,
    pub value_size_distribution: DistributionSummary,
    pub histogram_boundaries: Vec<f64>,
    pub statement_size_histogram: Vec<HistogramBucket>,
    pub value_size_histogram: Vec<HistogramBucket>,
    pub breakdown_by_kind: BTreeMap<String, MetricBreakdown>,
    pub breakdown_by_module: BTreeMap<String, MetricBreakdown>,
    pub breakdown_by_visibility: BTreeMap<String, MetricBreakdown>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetricBreakdown {
    pub declaration_count: usize,
    pub statement_size: DistributionSummary,
    pub value_size: DistributionSummary,
}

fn metric_breakdown(declarations: &[&DeclarationSnapshot]) -> MetricBreakdown {
    let statement = declarations
        .iter()
        .map(|declaration| declaration.type_stats.nodes as f64)
        .collect::<Vec<_>>();
    let value = declarations
        .iter()
        .filter_map(|declaration| {
            declaration
                .value_stats
                .as_ref()
                .map(|stats| stats.nodes as f64)
        })
        .collect::<Vec<_>>();
    MetricBreakdown {
        declaration_count: declarations.len(),
        statement_size: summarize_distribution(&statement),
        value_size: summarize_distribution(&value),
    }
}

fn expression_metrics(
    corpus: &AnalysisCorpus,
    declaration_id: usize,
    layer: DependencyLayer,
    graph: &ExpressionGraph,
    stats: &ExprStats,
) -> ExpressionMetrics {
    let multiplicities = corpus
        .multiplicities(declaration_id, layer)
        .expect("corpus caches every graph");
    let mut kind_occurrences = BTreeMap::new();
    let mut binder_counts = BTreeMap::new();
    let mut constants = BTreeSet::new();
    let mut modules = BTreeSet::new();
    let mut internal = 0;
    let mut external = 0;
    for node in &graph.nodes {
        *kind_occurrences.entry(node.kind.clone()).or_default() += multiplicities[node.id];
        if matches!(node.kind.as_str(), "lambda" | "forall") {
            *binder_counts
                .entry(node.binder_info.clone().unwrap_or_else(|| "unknown".into()))
                .or_default() += multiplicities[node.id];
        } else if node.kind == "let" {
            *binder_counts.entry("let".into()).or_default() += multiplicities[node.id];
        }
        if node.kind == "constant"
            && let Some(name) = &node.name
        {
            constants.insert(name.clone());
        }
    }
    for name in &constants {
        if corpus.declaration(name).is_some() {
            internal += 1;
        } else {
            external += 1;
        }
        if let Some(module) = corpus
            .symbol(name)
            .and_then(|symbol| symbol.module_name.as_ref())
        {
            modules.insert(module.clone());
        }
    }
    let kind_proportions = kind_occurrences
        .iter()
        .map(|(kind, &count)| (kind.clone(), count as f64 / stats.nodes as f64))
        .collect();
    let mut constant_frequencies = graph
        .constant_occurrences()
        .into_iter()
        .map(|(name, occurrences)| ConstantFrequency { name, occurrences })
        .collect::<Vec<_>>();
    constant_frequencies.sort_by(|a, b| {
        b.occurrences
            .cmp(&a.occurrences)
            .then_with(|| a.name.cmp(&b.name))
    });
    ExpressionMetrics {
        total_occurrences: stats.nodes,
        unique_nodes: graph.nodes.len(),
        dag_compression_ratio: if graph.nodes.is_empty() {
            1.0
        } else {
            stats.nodes as f64 / graph.nodes.len() as f64
        },
        maximum_depth: stats.max_depth,
        kind_occurrences,
        kind_proportions,
        distinct_constants: constants.len(),
        total_constant_occurrences: stats.constants,
        direct_dependencies: constants.len(),
        internal_dependencies: internal,
        external_dependencies: external,
        modules_referenced: modules.len(),
        binder_counts,
        constant_frequencies,
    }
}

impl AnalysisCorpus {
    pub fn declaration_metrics_by_id(&self, id: usize) -> DeclarationMetrics {
        let declaration = &self.declarations()[id];
        DeclarationMetrics {
            name: declaration.name.clone(),
            kind: declaration.kind.clone(),
            module: declaration.module_name.clone(),
            generated: declaration.is_internal || declaration.is_private,
            source_backed: declaration.source_range.is_some(),
            has_documentation: declaration.doc_string.is_some(),
            axiom_count: declaration.axioms.len(),
            statement: expression_metrics(
                self,
                id,
                DependencyLayer::Statement,
                &declaration.type_graph,
                &declaration.type_stats,
            ),
            value_label: declaration.value_graph.as_ref().map(|_| {
                if declaration.kind == "theorem" {
                    "proof"
                } else {
                    "implementation"
                }
                .into()
            }),
            value: declaration
                .value_graph
                .as_ref()
                .zip(declaration.value_stats.as_ref())
                .map(|(graph, stats)| {
                    expression_metrics(self, id, DependencyLayer::Proof, graph, stats)
                }),
        }
    }

    pub fn declaration_metrics(&self, name: &str) -> Option<DeclarationMetrics> {
        self.declaration_id(name)
            .map(|id| self.declaration_metrics_by_id(id))
    }

    pub fn summary(&self, filter: &AnalysisFilter) -> CorpusSummary {
        self.summary_with_histogram(
            filter,
            &[10.0, 50.0, 100.0, 250.0, 500.0, 1_000.0, 2_500.0, 5_000.0],
        )
    }

    pub fn summary_with_histogram(
        &self,
        filter: &AnalysisFilter,
        histogram_boundaries: &[f64],
    ) -> CorpusSummary {
        let ids = self.filtered_declaration_ids(filter).collect::<Vec<_>>();
        let declarations = ids
            .iter()
            .map(|&id| &self.declarations()[id])
            .collect::<Vec<_>>();
        let edges = self.filtered_edges(filter).collect::<Vec<_>>();
        let mut by_kind = BTreeMap::new();
        let mut by_module = BTreeMap::new();
        let mut kind_groups: BTreeMap<String, Vec<&DeclarationSnapshot>> = BTreeMap::new();
        let mut module_groups: BTreeMap<String, Vec<&DeclarationSnapshot>> = BTreeMap::new();
        let mut visibility_groups: BTreeMap<String, Vec<&DeclarationSnapshot>> = BTreeMap::new();
        for declaration in &declarations {
            *by_kind.entry(declaration.kind.clone()).or_default() += 1;
            kind_groups
                .entry(declaration.kind.clone())
                .or_default()
                .push(declaration);
            let module = declaration
                .module_name
                .clone()
                .unwrap_or_else(|| "<unknown>".into());
            *by_module.entry(module.clone()).or_default() += 1;
            module_groups.entry(module).or_default().push(declaration);
            visibility_groups
                .entry(declaration_visibility(declaration).into())
                .or_default()
                .push(declaration);
        }
        let statement_sizes = declarations
            .iter()
            .map(|declaration| declaration.type_stats.nodes as f64)
            .collect::<Vec<_>>();
        let value_sizes = declarations
            .iter()
            .filter_map(|declaration| {
                declaration
                    .value_stats
                    .as_ref()
                    .map(|stats| stats.nodes as f64)
            })
            .collect::<Vec<_>>();
        CorpusSummary {
            filter: filter.clone(),
            declaration_count: declarations.len(),
            theorem_count: declarations.iter().filter(|d| d.kind == "theorem").count(),
            generated_count: declarations
                .iter()
                .filter(|d| d.is_internal || d.is_private)
                .count(),
            symbol_count: self
                .symbols()
                .iter()
                .filter(|symbol| {
                    filter.matches_symbol(symbol, self.declaration(&symbol.name).is_some())
                })
                .count(),
            statement_occurrences: declarations.iter().map(|d| d.type_stats.nodes).sum(),
            value_occurrences: declarations
                .iter()
                .filter_map(|d| d.value_stats.as_ref())
                .map(|s| s.nodes)
                .sum(),
            statement_unique_nodes: declarations.iter().map(|d| d.type_graph.nodes.len()).sum(),
            value_unique_nodes: declarations
                .iter()
                .filter_map(|d| d.value_graph.as_ref())
                .map(|g| g.nodes.len())
                .sum(),
            dependency_edges: edges.len(),
            dependency_occurrences: edges.iter().map(|e| e.occurrences).sum(),
            internal_dependency_edges: edges.iter().filter(|e| e.target_in_corpus).count(),
            by_kind,
            by_module,
            statement_size_distribution: summarize_distribution(&statement_sizes),
            value_size_distribution: summarize_distribution(&value_sizes),
            histogram_boundaries: histogram_boundaries.to_vec(),
            statement_size_histogram: histogram(&statement_sizes, histogram_boundaries),
            value_size_histogram: histogram(&value_sizes, histogram_boundaries),
            breakdown_by_kind: kind_groups
                .into_iter()
                .map(|(name, declarations)| (name, metric_breakdown(&declarations)))
                .collect(),
            breakdown_by_module: module_groups
                .into_iter()
                .map(|(name, declarations)| (name, metric_breakdown(&declarations)))
                .collect(),
            breakdown_by_visibility: visibility_groups
                .into_iter()
                .map(|(name, declarations)| (name, metric_breakdown(&declarations)))
                .collect(),
        }
    }

    pub fn rank(
        &self,
        metric: &str,
        filter: &AnalysisFilter,
        limit: usize,
    ) -> Option<Vec<RankedDeclaration>> {
        let mut values = self
            .filtered_declaration_ids(filter)
            .map(|id| {
                let metrics = self.declaration_metrics_by_id(id);
                let value = match metric {
                    "statement-occurrences" => metrics.statement.total_occurrences as f64,
                    "statement-unique-nodes" => metrics.statement.unique_nodes as f64,
                    "statement-depth" => metrics.statement.maximum_depth as f64,
                    "statement-dependencies" => metrics.statement.direct_dependencies as f64,
                    "value-occurrences" | "proof-occurrences" => metrics
                        .value
                        .as_ref()
                        .map_or(0.0, |m| m.total_occurrences as f64),
                    "value-unique-nodes" | "proof-unique-nodes" => metrics
                        .value
                        .as_ref()
                        .map_or(0.0, |m| m.unique_nodes as f64),
                    "value-depth" | "proof-depth" => metrics
                        .value
                        .as_ref()
                        .map_or(0.0, |m| m.maximum_depth as f64),
                    "value-dependencies" | "proof-dependencies" => metrics
                        .value
                        .as_ref()
                        .map_or(0.0, |m| m.direct_dependencies as f64),
                    "axioms" => metrics.axiom_count as f64,
                    _ => return None,
                };
                Some((metrics.name, value))
            })
            .collect::<Option<Vec<_>>>()?;
        values.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        Some(
            values
                .into_iter()
                .take(limit)
                .enumerate()
                .map(|(index, (name, value))| RankedDeclaration {
                    rank: index + 1,
                    name,
                    metric: metric.into(),
                    value,
                })
                .collect(),
        )
    }
}

pub fn summarize_distribution(values: &[f64]) -> DistributionSummary {
    if values.is_empty() {
        return DistributionSummary::default();
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let count = sorted.len();
    let mean = sorted.iter().sum::<f64>() / count as f64;
    let variance = sorted
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / count as f64;
    let quantiles = [
        ("p50", 0.50),
        ("p75", 0.75),
        ("p90", 0.90),
        ("p95", 0.95),
        ("p99", 0.99),
    ]
    .into_iter()
    .map(|(name, q)| (name.into(), quantile(&sorted, q).unwrap()))
    .collect();
    DistributionSummary {
        count,
        minimum: sorted.first().copied(),
        maximum: sorted.last().copied(),
        mean: Some(mean),
        median: quantile(&sorted, 0.5),
        standard_deviation: Some(variance.sqrt()),
        quantiles,
    }
}

pub fn quantile(sorted_values: &[f64], q: f64) -> Option<f64> {
    if sorted_values.is_empty() {
        return None;
    }
    let position = q.clamp(0.0, 1.0) * (sorted_values.len() - 1) as f64;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    let fraction = position - lower as f64;
    Some(sorted_values[lower] * (1.0 - fraction) + sorted_values[upper] * fraction)
}

pub fn histogram(values: &[f64], boundaries: &[f64]) -> Vec<HistogramBucket> {
    let mut sorted_boundaries = boundaries.to_vec();
    sorted_boundaries.sort_by(f64::total_cmp);
    sorted_boundaries.dedup_by(|a, b| a.total_cmp(b).is_eq());
    let mut buckets = Vec::new();
    let mut lower = None;
    for &upper in &sorted_boundaries {
        buckets.push(HistogramBucket {
            lower_inclusive: lower,
            upper_exclusive: Some(upper),
            count: values
                .iter()
                .filter(|&&v| lower.is_none_or(|lower| v >= lower) && v < upper)
                .count(),
        });
        lower = Some(upper);
    }
    buckets.push(HistogramBucket {
        lower_inclusive: lower,
        upper_exclusive: None,
        count: values
            .iter()
            .filter(|&&v| lower.is_none_or(|lower| v >= lower))
            .count(),
    });
    buckets
}

pub fn declaration_visibility(declaration: &DeclarationSnapshot) -> &'static str {
    if declaration.is_internal {
        "internal"
    } else if declaration.is_private {
        "private"
    } else {
        "public"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statistics_cover_empty_odd_and_even_inputs() {
        assert_eq!(summarize_distribution(&[]).count, 0);
        assert_eq!(summarize_distribution(&[1.0, 9.0, 3.0]).median, Some(3.0));
        assert_eq!(
            summarize_distribution(&[1.0, 3.0, 5.0, 7.0]).median,
            Some(4.0)
        );
        assert_eq!(quantile(&[0.0, 10.0], 0.75), Some(7.5));
    }

    #[test]
    fn histogram_accounts_for_every_value() {
        let buckets = histogram(&[0.0, 1.0, 2.0, 10.0], &[1.0, 5.0]);
        assert_eq!(buckets.iter().map(|b| b.count).sum::<usize>(), 4);
        assert_eq!(
            buckets.iter().map(|b| b.count).collect::<Vec<_>>(),
            vec![1, 2, 1]
        );
    }
}
