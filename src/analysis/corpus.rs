use std::collections::{BTreeMap, HashMap};
use std::sync::OnceLock;

use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

use crate::analysis::filters::{AnalysisFilter, DependencyLayer};
use crate::model::{DeclarationSnapshot, ExpressionGraph, ExtractionSnapshot, SymbolSnapshot};
use crate::validation::validate_snapshot;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyEdge {
    pub source: usize,
    pub target: String,
    pub layer: DependencyLayer,
    pub occurrences: usize,
    pub target_in_corpus: bool,
    pub source_module: Option<String>,
    pub target_module: Option<String>,
    pub target_kind: String,
    pub target_is_class: bool,
    pub target_is_instance: bool,
    pub target_is_projection: bool,
}

pub struct AnalysisCorpus {
    snapshot: ExtractionSnapshot,
    declaration_index: HashMap<String, usize>,
    symbol_index: HashMap<String, usize>,
    dependency_edges: Vec<DependencyEdge>,
    outgoing_edges: Vec<Vec<usize>>,
    incoming_edges: HashMap<String, Vec<usize>>,
    multiplicities: HashMap<(usize, DependencyLayer), Vec<usize>>,
    pub(crate) structural_fingerprints: OnceLock<HashMap<(usize, DependencyLayer, bool), Vec<u64>>>,
}

impl AnalysisCorpus {
    pub fn new(snapshot: ExtractionSnapshot) -> Result<Self> {
        validate_snapshot(&snapshot)?;
        let declaration_index = snapshot
            .declarations
            .iter()
            .enumerate()
            .map(|(index, declaration)| (declaration.name.clone(), index))
            .collect::<HashMap<_, _>>();
        let symbol_index = snapshot
            .symbols
            .iter()
            .enumerate()
            .map(|(index, symbol)| (symbol.name.clone(), index))
            .collect::<HashMap<_, _>>();
        let mut dependency_edges = Vec::new();
        let mut outgoing_edges = vec![Vec::new(); snapshot.declarations.len()];
        let mut incoming_edges: HashMap<String, Vec<usize>> = HashMap::new();
        let mut multiplicities = HashMap::new();

        for (source, declaration) in snapshot.declarations.iter().enumerate() {
            let layers = [
                (DependencyLayer::Statement, Some(&declaration.type_graph)),
                (DependencyLayer::Proof, declaration.value_graph.as_ref()),
            ];
            for (layer, graph) in layers {
                let Some(graph) = graph else { continue };
                multiplicities.insert((source, layer), graph.occurrence_multiplicities());
                for (target, occurrences) in graph.constant_occurrences() {
                    let symbol = &snapshot.symbols[symbol_index[&target]];
                    let edge_id = dependency_edges.len();
                    dependency_edges.push(DependencyEdge {
                        source,
                        target: target.clone(),
                        layer,
                        occurrences,
                        target_in_corpus: declaration_index.contains_key(&target),
                        source_module: declaration.module_name.clone(),
                        target_module: symbol.module_name.clone(),
                        target_kind: symbol.kind.clone(),
                        target_is_class: symbol.is_class,
                        target_is_instance: symbol.is_instance,
                        target_is_projection: symbol.is_projection,
                    });
                    outgoing_edges[source].push(edge_id);
                    incoming_edges.entry(target).or_default().push(edge_id);
                }
            }
        }
        ensure!(
            dependency_edges.iter().all(|edge| edge.occurrences > 0),
            "zero-weight dependency edge"
        );
        Ok(Self {
            snapshot,
            declaration_index,
            symbol_index,
            dependency_edges,
            outgoing_edges,
            incoming_edges,
            multiplicities,
            structural_fingerprints: OnceLock::new(),
        })
    }

    pub fn snapshot(&self) -> &ExtractionSnapshot {
        &self.snapshot
    }
    pub fn declarations(&self) -> &[DeclarationSnapshot] {
        &self.snapshot.declarations
    }
    pub fn symbols(&self) -> &[SymbolSnapshot] {
        &self.snapshot.symbols
    }
    pub fn edges(&self) -> &[DependencyEdge] {
        &self.dependency_edges
    }
    pub fn declaration_id(&self, name: &str) -> Option<usize> {
        self.declaration_index.get(name).copied()
    }
    pub fn declaration(&self, name: &str) -> Option<&DeclarationSnapshot> {
        self.declaration_id(name)
            .map(|id| &self.snapshot.declarations[id])
    }
    pub fn symbol(&self, name: &str) -> Option<&SymbolSnapshot> {
        self.symbol_index
            .get(name)
            .map(|&id| &self.snapshot.symbols[id])
    }
    pub fn graph(&self, declaration: usize, layer: DependencyLayer) -> Option<&ExpressionGraph> {
        let declaration = &self.snapshot.declarations[declaration];
        match layer {
            DependencyLayer::Statement => Some(&declaration.type_graph),
            DependencyLayer::Proof => declaration.value_graph.as_ref(),
        }
    }
    pub fn multiplicities(&self, declaration: usize, layer: DependencyLayer) -> Option<&[usize]> {
        self.multiplicities
            .get(&(declaration, layer))
            .map(Vec::as_slice)
    }
    pub fn outgoing_edge_ids(&self, declaration: usize) -> &[usize] {
        &self.outgoing_edges[declaration]
    }
    pub fn incoming_edge_ids(&self, symbol: &str) -> &[usize] {
        self.incoming_edges
            .get(symbol)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }
    pub fn filtered_declaration_ids<'a>(
        &'a self,
        filter: &'a AnalysisFilter,
    ) -> impl Iterator<Item = usize> + 'a {
        self.snapshot
            .declarations
            .iter()
            .enumerate()
            .filter(move |(_, declaration)| filter.matches_declaration(declaration))
            .map(|(id, _)| id)
    }
    pub fn edge_matches(&self, edge: &DependencyEdge, filter: &AnalysisFilter) -> bool {
        filter.layer.includes(edge.layer)
            && filter.matches_declaration(&self.snapshot.declarations[edge.source])
            && filter.matches_symbol(
                &self.snapshot.symbols[self.symbol_index[&edge.target]],
                edge.target_in_corpus,
            )
    }
    pub fn filtered_edges<'a>(
        &'a self,
        filter: &'a AnalysisFilter,
    ) -> impl Iterator<Item = &'a DependencyEdge> + 'a {
        self.dependency_edges
            .iter()
            .filter(move |edge| self.edge_matches(edge, filter))
    }
    pub fn internal_dependency_edge_count(&self) -> usize {
        self.dependency_edges
            .iter()
            .filter(|edge| edge.target_in_corpus)
            .count()
    }
    pub fn module_dependency_occurrences(
        &self,
        filter: &AnalysisFilter,
    ) -> BTreeMap<(String, String, DependencyLayer), usize> {
        let mut modules = BTreeMap::new();
        for edge in self.filtered_edges(filter) {
            *modules
                .entry((
                    edge.source_module
                        .clone()
                        .unwrap_or_else(|| "<unknown>".into()),
                    edge.target_module
                        .clone()
                        .unwrap_or_else(|| "<unknown>".into()),
                    edge.layer,
                ))
                .or_default() += edge.occurrences;
        }
        modules
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::dependency::DependencyConfig;
    use crate::model::{ExprStats, ExpressionNode, SymbolSnapshot};

    fn declaration(name: &str, dependency: &str, generated: bool) -> DeclarationSnapshot {
        DeclarationSnapshot {
            name: name.into(),
            kind: "theorem".into(),
            module_name: Some("Fixture".into()),
            is_internal: generated,
            is_private: false,
            is_unsafe: false,
            is_partial: false,
            doc_string: None,
            source_range: None,
            level_parameters: vec![],
            r#type: dependency.into(),
            has_value: false,
            type_stats: ExprStats {
                nodes: 1,
                max_depth: 1,
                bound_variables: 0,
                free_variables: 0,
                metavariables: 0,
                sorts: 0,
                constants: 1,
                applications: 0,
                lambdas: 0,
                foralls: 0,
                lets: 0,
                literals: 0,
                metadata: 0,
                projections: 0,
            },
            value_stats: None,
            type_graph: ExpressionGraph {
                root: 0,
                nodes: vec![ExpressionNode {
                    id: 0,
                    kind: "constant".into(),
                    children: vec![],
                    name: Some(dependency.into()),
                    index: None,
                    value: None,
                    binder_info: None,
                    universe_levels: vec![],
                }],
            },
            value_graph: None,
            statement_dependencies: vec![dependency.into()],
            proof_dependencies: vec![],
            axioms: vec![],
        }
    }

    fn symbol(name: &str, generated: bool) -> SymbolSnapshot {
        SymbolSnapshot {
            name: name.into(),
            kind: "theorem".into(),
            module_name: Some("Fixture".into()),
            is_internal: generated,
            is_private: false,
            is_unsafe: false,
            is_partial: false,
            is_class: false,
            is_instance: false,
            is_projection: false,
        }
    }

    fn fixture() -> AnalysisCorpus {
        AnalysisCorpus::new(ExtractionSnapshot {
            schema_version: 3,
            lean_version: "fixture".into(),
            imported_module: "Fixture".into(),
            declarations: vec![
                declaration("A", "B", false),
                declaration("B", "A", false),
                declaration("C", "B", true),
            ],
            symbols: vec![symbol("A", false), symbol("B", false), symbol("C", true)],
        })
        .unwrap()
    }

    #[test]
    fn filtering_does_not_mutate_unfiltered_results() {
        let corpus = fixture();
        assert_eq!(
            corpus
                .filtered_declaration_ids(&AnalysisFilter::all())
                .count(),
            3
        );
        assert_eq!(
            corpus
                .filtered_declaration_ids(&AnalysisFilter::default())
                .count(),
            2
        );
        assert_eq!(
            corpus
                .filtered_declaration_ids(&AnalysisFilter::all())
                .count(),
            3
        );
    }

    #[test]
    fn traversal_is_cycle_safe_and_reverse_edges_are_indexed() {
        let corpus = fixture();
        let config = DependencyConfig {
            filter: AnalysisFilter::all(),
            max_depth: 20,
            limit: 20,
            ..DependencyConfig::default()
        };
        let forward = corpus.traverse_dependencies("A", false, &config).unwrap();
        assert_eq!(
            forward
                .nodes
                .iter()
                .map(|node| &node.name)
                .collect::<Vec<_>>(),
            vec!["B"]
        );
        let reverse = corpus.direct_dependents("B", &config).unwrap();
        assert_eq!(
            reverse
                .edges
                .iter()
                .map(|edge| &edge.source)
                .collect::<Vec<_>>(),
            vec!["A", "C"]
        );
    }
}
