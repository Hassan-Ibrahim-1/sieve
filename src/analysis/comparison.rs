use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::analysis::corpus::AnalysisCorpus;
use crate::analysis::filters::DependencyLayer;
use crate::analysis::structure::{StructuralMode, structurally_equal};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SimilarityComponents {
    pub dependency_jaccard: f64,
    pub kind_histogram_cosine: f64,
    pub size_ratio: f64,
    pub depth_ratio: f64,
    pub combined_score: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeclarationComparison {
    pub left: String,
    pub right: String,
    pub layer: DependencyLayer,
    pub exact_equal: bool,
    pub alpha_equivalent: bool,
    pub similarity: SimilarityComponents,
    pub shared_dependencies: Vec<String>,
    pub left_only_dependencies: Vec<String>,
    pub right_only_dependencies: Vec<String>,
    pub explanation: Vec<String>,
}

impl AnalysisCorpus {
    pub fn compare_declarations(
        &self,
        left: &str,
        right: &str,
        layer: DependencyLayer,
    ) -> Option<DeclarationComparison> {
        let left_id = self.declaration_id(left)?;
        let right_id = self.declaration_id(right)?;
        let left_graph = self.graph(left_id, layer)?;
        let right_graph = self.graph(right_id, layer)?;
        let exact_equal = structurally_equal(
            (left_graph, left_graph.root),
            (right_graph, right_graph.root),
            StructuralMode::Exact,
        );
        let alpha_equivalent = exact_equal
            || structurally_equal(
                (left_graph, left_graph.root),
                (right_graph, right_graph.root),
                StructuralMode::AlphaEquivalent,
            );
        let dependencies = |id: usize| {
            self.outgoing_edge_ids(id)
                .iter()
                .map(|&edge| &self.edges()[edge])
                .filter(|edge| edge.layer == layer)
                .map(|edge| edge.target.clone())
                .collect::<BTreeSet<_>>()
        };
        let left_dependencies = dependencies(left_id);
        let right_dependencies = dependencies(right_id);
        let shared_dependencies = left_dependencies
            .intersection(&right_dependencies)
            .cloned()
            .collect::<Vec<_>>();
        let left_only_dependencies = left_dependencies
            .difference(&right_dependencies)
            .cloned()
            .collect::<Vec<_>>();
        let right_only_dependencies = right_dependencies
            .difference(&left_dependencies)
            .cloned()
            .collect::<Vec<_>>();
        let union = left_dependencies.union(&right_dependencies).count();
        let dependency_jaccard = if union == 0 {
            1.0
        } else {
            shared_dependencies.len() as f64 / union as f64
        };
        let kind_counts = |graph: &crate::model::ExpressionGraph| {
            let mut counts = std::collections::BTreeMap::<String, f64>::new();
            for node in &graph.nodes {
                *counts.entry(node.kind.clone()).or_default() += 1.0;
            }
            counts
        };
        let left_kinds = kind_counts(left_graph);
        let right_kinds = kind_counts(right_graph);
        let kinds = left_kinds
            .keys()
            .chain(right_kinds.keys())
            .cloned()
            .collect::<BTreeSet<_>>();
        let dot = kinds
            .iter()
            .map(|kind| {
                left_kinds.get(kind).unwrap_or(&0.0) * right_kinds.get(kind).unwrap_or(&0.0)
            })
            .sum::<f64>();
        let left_norm = left_kinds.values().map(|v| v * v).sum::<f64>().sqrt();
        let right_norm = right_kinds.values().map(|v| v * v).sum::<f64>().sqrt();
        let kind_histogram_cosine = if left_norm == 0.0 || right_norm == 0.0 {
            1.0
        } else {
            dot / (left_norm * right_norm)
        };
        let ratio = |a: usize, b: usize| {
            if a.max(b) == 0 {
                1.0
            } else {
                a.min(b) as f64 / a.max(b) as f64
            }
        };
        let sizes = (left_graph.nodes.len(), right_graph.nodes.len());
        let depth = |graph: &crate::model::ExpressionGraph| {
            let mut depths = vec![0; graph.nodes.len()];
            for node in &graph.nodes {
                depths[node.id] = 1 + node
                    .children
                    .iter()
                    .map(|&child| depths[child])
                    .max()
                    .unwrap_or(0);
            }
            depths[graph.root]
        };
        let size_ratio = ratio(sizes.0, sizes.1);
        let depth_ratio = ratio(depth(left_graph), depth(right_graph));
        let combined_score =
            (dependency_jaccard + kind_histogram_cosine + size_ratio + depth_ratio) / 4.0;
        let mut explanation = vec![
            format!("dependency overlap: {:.1}%", dependency_jaccard * 100.0),
            format!(
                "expression-kind similarity: {:.1}%",
                kind_histogram_cosine * 100.0
            ),
            format!("size ratio: {:.1}%", size_ratio * 100.0),
            format!("depth ratio: {:.1}%", depth_ratio * 100.0),
        ];
        if exact_equal {
            explanation.insert(0, "complete expression DAGs are exactly equal".into());
        } else if alpha_equivalent {
            explanation.insert(
                0,
                "expressions are equal after ignoring binder names".into(),
            );
        } else {
            explanation.insert(0, "expressions are not structural duplicates".into());
        }
        Some(DeclarationComparison {
            left: left.into(),
            right: right.into(),
            layer,
            exact_equal,
            alpha_equivalent,
            similarity: SimilarityComponents {
                dependency_jaccard,
                kind_histogram_cosine,
                size_ratio,
                depth_ratio,
                combined_score,
            },
            shared_dependencies,
            left_only_dependencies,
            right_only_dependencies,
            explanation,
        })
    }
}
