use std::collections::{BTreeMap, BTreeSet, HashMap};

use serde::{Deserialize, Serialize};

use crate::analysis::corpus::AnalysisCorpus;
use crate::analysis::filters::{AnalysisFilter, DependencyLayer, LayerSelection};
use crate::model::{ExpressionGraph, ExpressionNode};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum StructuralMode {
    Exact,
    AlphaEquivalent,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StructuralConfig {
    pub filter: AnalysisFilter,
    pub mode: StructuralMode,
    pub minimum_size: usize,
    pub minimum_support: usize,
    pub limit: usize,
}

impl Default for StructuralConfig {
    fn default() -> Self {
        Self {
            filter: AnalysisFilter::default(),
            mode: StructuralMode::AlphaEquivalent,
            minimum_size: 4,
            minimum_support: 2,
            limit: 100,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StructureLocation {
    pub declaration: String,
    pub layer: DependencyLayer,
    pub node: usize,
    pub occurrences: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StructuralMatch {
    pub config: StructuralConfig,
    pub fingerprint: String,
    pub declarations: Vec<String>,
    pub locations: Vec<StructureLocation>,
    pub expression_size: usize,
    pub expression_depth: usize,
    pub total_occurrences: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepeatedSubexpression {
    pub config: StructuralConfig,
    pub fingerprint: String,
    pub unique_declarations: usize,
    pub total_occurrences: usize,
    pub expression_size: usize,
    pub expression_depth: usize,
    pub locations: Vec<StructureLocation>,
}

const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

fn feed(hash: &mut u64, bytes: &[u8]) {
    for byte in bytes {
        *hash ^= u64::from(*byte);
        *hash = hash.wrapping_mul(FNV_PRIME);
    }
    *hash ^= 0xff;
    *hash = hash.wrapping_mul(FNV_PRIME);
}
fn feed_option(hash: &mut u64, value: Option<&str>) {
    match value {
        Some(value) => {
            feed(hash, &[1]);
            feed(hash, value.as_bytes());
        }
        None => feed(hash, &[0]),
    }
}

impl ExpressionGraph {
    /// Stable FNV-1a structural fingerprints. A match is only a candidate until verified.
    pub fn structural_fingerprints(&self, mode: StructuralMode) -> Vec<u64> {
        let mut fingerprints: Vec<u64> = Vec::with_capacity(self.nodes.len());
        for node in &self.nodes {
            let mut hash = FNV_OFFSET;
            feed(&mut hash, node.kind.as_bytes());
            let ignore_name = mode == StructuralMode::AlphaEquivalent
                && matches!(node.kind.as_str(), "lambda" | "forall" | "let");
            if ignore_name {
                feed(&mut hash, &[0]);
            } else {
                feed_option(&mut hash, node.name.as_deref());
            }
            match node.index {
                Some(index) => {
                    feed(&mut hash, &[1]);
                    feed(&mut hash, &index.to_le_bytes());
                }
                None => feed(&mut hash, &[0]),
            }
            feed_option(&mut hash, node.value.as_deref());
            feed_option(&mut hash, node.binder_info.as_deref());
            for level in &node.universe_levels {
                feed(&mut hash, level.as_bytes());
            }
            for &child in &node.children {
                feed(&mut hash, &fingerprints[child].to_le_bytes());
            }
            fingerprints.push(hash);
        }
        fingerprints
    }

    pub fn root_fingerprint(&self, mode: StructuralMode) -> u64 {
        self.structural_fingerprints(mode)[self.root]
    }

    pub fn render_tree(&self, max_depth: usize) -> String {
        fn render(
            graph: &ExpressionGraph,
            id: usize,
            depth: usize,
            max_depth: usize,
            out: &mut String,
        ) {
            let node = &graph.nodes[id];
            out.push_str(&"  ".repeat(depth));
            out.push_str(&format!("#{} {}\n", node.id, node.label()));
            if depth == max_depth {
                if !node.children.is_empty() {
                    out.push_str(&"  ".repeat(depth + 1));
                    out.push_str("…\n");
                }
                return;
            }
            for &child in &node.children {
                render(graph, child, depth + 1, max_depth, out);
            }
        }
        let mut output = String::new();
        render(self, self.root, 0, max_depth, &mut output);
        output
    }
}

fn node_fields_equal(left: &ExpressionNode, right: &ExpressionNode, mode: StructuralMode) -> bool {
    let ignore_name = mode == StructuralMode::AlphaEquivalent
        && matches!(left.kind.as_str(), "lambda" | "forall" | "let");
    left.kind == right.kind
        && (ignore_name || left.name == right.name)
        && left.index == right.index
        && left.value == right.value
        && left.binder_info == right.binder_info
        && left.universe_levels == right.universe_levels
        && left.children.len() == right.children.len()
}

pub fn structurally_equal(
    left: (&ExpressionGraph, usize),
    right: (&ExpressionGraph, usize),
    mode: StructuralMode,
) -> bool {
    fn compare(
        left: (&ExpressionGraph, usize),
        right: (&ExpressionGraph, usize),
        mode: StructuralMode,
        memo: &mut HashMap<(usize, usize), bool>,
    ) -> bool {
        if let Some(&result) = memo.get(&(left.1, right.1)) {
            return result;
        }
        let left_node = &left.0.nodes[left.1];
        let right_node = &right.0.nodes[right.1];
        let result = node_fields_equal(left_node, right_node, mode)
            && left_node
                .children
                .iter()
                .zip(&right_node.children)
                .all(|(&l, &r)| compare((left.0, l), (right.0, r), mode, memo));
        memo.insert((left.1, right.1), result);
        result
    }
    compare(left, right, mode, &mut HashMap::new())
}

fn subtree_shapes(graph: &ExpressionGraph) -> (Vec<usize>, Vec<usize>) {
    let mut sizes = vec![0; graph.nodes.len()];
    let mut depths = vec![0; graph.nodes.len()];
    for node in &graph.nodes {
        sizes[node.id] = 1 + node
            .children
            .iter()
            .map(|&child| sizes[child])
            .sum::<usize>();
        depths[node.id] = 1 + node
            .children
            .iter()
            .map(|&child| depths[child])
            .max()
            .unwrap_or(0);
    }
    (sizes, depths)
}

impl AnalysisCorpus {
    pub fn cached_structural_fingerprints(
        &self,
        declaration: usize,
        layer: DependencyLayer,
        mode: StructuralMode,
    ) -> Option<&[u64]> {
        let cache = self.structural_fingerprints.get_or_init(|| {
            let mut cache = HashMap::new();
            for declaration in 0..self.declarations().len() {
                for layer in [DependencyLayer::Statement, DependencyLayer::Proof] {
                    let Some(graph) = self.graph(declaration, layer) else {
                        continue;
                    };
                    cache.insert(
                        (declaration, layer, false),
                        graph.structural_fingerprints(StructuralMode::Exact),
                    );
                    cache.insert(
                        (declaration, layer, true),
                        graph.structural_fingerprints(StructuralMode::AlphaEquivalent),
                    );
                }
            }
            cache
        });
        cache
            .get(&(declaration, layer, mode == StructuralMode::AlphaEquivalent))
            .map(Vec::as_slice)
    }

    pub fn find_duplicates(&self, config: &StructuralConfig) -> Vec<StructuralMatch> {
        let mut candidates: BTreeMap<(DependencyLayer, u64), Vec<(usize, usize)>> = BTreeMap::new();
        for declaration in self.filtered_declaration_ids(&config.filter) {
            for layer in [DependencyLayer::Statement, DependencyLayer::Proof] {
                if !config.filter.layer.includes(layer) {
                    continue;
                }
                let Some(graph) = self.graph(declaration, layer) else {
                    continue;
                };
                let fingerprints = self
                    .cached_structural_fingerprints(declaration, layer, config.mode)
                    .expect("every graph has cached fingerprints");
                candidates
                    .entry((layer, fingerprints[graph.root]))
                    .or_default()
                    .push((declaration, graph.root));
            }
        }
        let mut results = Vec::new();
        for ((layer, fingerprint), candidates) in candidates
            .into_iter()
            .filter(|(_, group)| group.len() >= config.minimum_support)
        {
            for equivalent in verified_groups(self, layer, candidates, config.mode) {
                if equivalent.len() < config.minimum_support {
                    continue;
                }
                let first = equivalent[0];
                let graph = self.graph(first.0, layer).unwrap();
                let (sizes, depths) = subtree_shapes(graph);
                let mut declarations = equivalent
                    .iter()
                    .map(|(id, _)| self.declarations()[*id].name.clone())
                    .collect::<Vec<_>>();
                declarations.sort();
                let locations = equivalent
                    .iter()
                    .map(|(id, node)| StructureLocation {
                        declaration: self.declarations()[*id].name.clone(),
                        layer,
                        node: *node,
                        occurrences: 1,
                    })
                    .collect();
                results.push(StructuralMatch {
                    config: config.clone(),
                    fingerprint: format!("{fingerprint:016x}"),
                    declarations,
                    locations,
                    expression_size: sizes[first.1],
                    expression_depth: depths[first.1],
                    total_occurrences: equivalent.len(),
                });
            }
        }
        results.sort_by(|a, b| {
            b.expression_size
                .cmp(&a.expression_size)
                .then_with(|| a.declarations.cmp(&b.declarations))
        });
        results.truncate(config.limit);
        results
    }

    pub fn repeated_subexpressions(&self, config: &StructuralConfig) -> Vec<RepeatedSubexpression> {
        let mut candidates: BTreeMap<(DependencyLayer, u64), Vec<(usize, usize)>> = BTreeMap::new();
        let mut shapes: HashMap<(usize, DependencyLayer), (Vec<usize>, Vec<usize>)> =
            HashMap::new();
        for declaration in self.filtered_declaration_ids(&config.filter) {
            for layer in [DependencyLayer::Statement, DependencyLayer::Proof] {
                if !config.filter.layer.includes(layer) {
                    continue;
                }
                let Some(graph) = self.graph(declaration, layer) else {
                    continue;
                };
                let (sizes, depths) = subtree_shapes(graph);
                let fingerprints = self
                    .cached_structural_fingerprints(declaration, layer, config.mode)
                    .expect("every graph has cached fingerprints");
                for node in 0..graph.nodes.len() {
                    if sizes[node] >= config.minimum_size {
                        candidates
                            .entry((layer, fingerprints[node]))
                            .or_default()
                            .push((declaration, node));
                    }
                }
                shapes.insert((declaration, layer), (sizes, depths));
            }
        }
        let mut results = Vec::new();
        for ((layer, fingerprint), candidates) in candidates {
            let declaration_support = candidates
                .iter()
                .map(|(id, _)| *id)
                .collect::<BTreeSet<_>>()
                .len();
            if declaration_support < config.minimum_support {
                continue;
            }
            for equivalent in verified_groups(self, layer, candidates, config.mode) {
                let unique_declarations = equivalent
                    .iter()
                    .map(|(id, _)| *id)
                    .collect::<BTreeSet<_>>()
                    .len();
                if unique_declarations < config.minimum_support {
                    continue;
                }
                let first = equivalent[0];
                let (sizes, depths) = &shapes[&(first.0, layer)];
                let mut locations = equivalent
                    .into_iter()
                    .map(|(declaration, node)| StructureLocation {
                        declaration: self.declarations()[declaration].name.clone(),
                        layer,
                        node,
                        occurrences: self.multiplicities(declaration, layer).unwrap()[node],
                    })
                    .collect::<Vec<_>>();
                locations
                    .sort_by(|a, b| a.declaration.cmp(&b.declaration).then(a.node.cmp(&b.node)));
                let total_occurrences = locations.iter().map(|location| location.occurrences).sum();
                results.push(RepeatedSubexpression {
                    config: config.clone(),
                    fingerprint: format!("{fingerprint:016x}"),
                    unique_declarations,
                    total_occurrences,
                    expression_size: sizes[first.1],
                    expression_depth: depths[first.1],
                    locations,
                });
            }
        }
        results.sort_by(|a, b| {
            b.expression_size
                .cmp(&a.expression_size)
                .then_with(|| b.unique_declarations.cmp(&a.unique_declarations))
                .then_with(|| a.fingerprint.cmp(&b.fingerprint))
        });
        results.truncate(config.limit);
        results
    }
}

fn verified_groups(
    corpus: &AnalysisCorpus,
    layer: DependencyLayer,
    candidates: Vec<(usize, usize)>,
    mode: StructuralMode,
) -> Vec<Vec<(usize, usize)>> {
    let mut groups: Vec<Vec<(usize, usize)>> = Vec::new();
    'candidate: for candidate in candidates {
        for group in &mut groups {
            let representative = group[0];
            if structurally_equal(
                (corpus.graph(candidate.0, layer).unwrap(), candidate.1),
                (
                    corpus.graph(representative.0, layer).unwrap(),
                    representative.1,
                ),
                mode,
            ) {
                group.push(candidate);
                continue 'candidate;
            }
        }
        groups.push(vec![candidate]);
    }
    groups
}

pub fn layer_selection_name(layer: LayerSelection) -> &'static str {
    match layer {
        LayerSelection::Statement => "statement",
        LayerSelection::Proof => "proof",
        LayerSelection::Both => "both",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn binder(name: &str) -> ExpressionGraph {
        ExpressionGraph {
            root: 1,
            nodes: vec![
                ExpressionNode {
                    id: 0,
                    kind: "sort".into(),
                    children: vec![],
                    name: None,
                    index: None,
                    value: Some("0".into()),
                    binder_info: None,
                    universe_levels: vec![],
                },
                ExpressionNode {
                    id: 1,
                    kind: "forall".into(),
                    children: vec![0, 0],
                    name: Some(name.into()),
                    index: None,
                    value: None,
                    binder_info: Some("explicit".into()),
                    universe_levels: vec![],
                },
            ],
        }
    }

    #[test]
    fn alpha_comparison_ignores_only_binder_names() {
        let left = binder("x");
        let right = binder("y");
        assert!(!structurally_equal(
            (&left, left.root),
            (&right, right.root),
            StructuralMode::Exact
        ));
        assert!(structurally_equal(
            (&left, left.root),
            (&right, right.root),
            StructuralMode::AlphaEquivalent
        ));
        assert_ne!(
            left.root_fingerprint(StructuralMode::Exact),
            right.root_fingerprint(StructuralMode::Exact)
        );
        assert_eq!(
            left.root_fingerprint(StructuralMode::AlphaEquivalent),
            right.root_fingerprint(StructuralMode::AlphaEquivalent)
        );
    }
}
