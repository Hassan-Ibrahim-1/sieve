//! Flat, statement-first graph payloads for the browser UI.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::analysis::corpus::AnalysisCorpus;
use crate::analysis::dependency::{DependencyConfig, DependencyPath};
use crate::analysis::filters::{AnalysisFilter, DependencyLayer};
use crate::analysis::proof_steps::{ProofOutlineEvidence, build_proof_outline};
use crate::analysis::structure::StructuralMode;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiBootstrap {
    pub schema_version: usize,
    pub corpus_fingerprint: String,
    pub declaration_count: usize,
    pub theorem_count: usize,
    pub proof_declarations: Vec<ProofDeclaration>,
    pub default_filters: UiFilters,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProofDeclaration {
    pub name: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProofOutlineResponse {
    pub declaration: String,
    pub statement: String,
    pub outline: ProofOutlineEvidence,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiFilters {
    pub include_theorems: bool,
    pub include_definitions: bool,
    pub include_technical: bool,
}

impl Default for UiFilters {
    fn default() -> Self {
        Self {
            include_theorems: true,
            include_definitions: true,
            include_technical: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct GraphRequest {
    pub filters: UiFilters,
}

impl Default for GraphRequest {
    fn default() -> Self {
        Self {
            filters: UiFilters::default(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphTotals {
    pub declarations: usize,
    pub groups: usize,
    pub connections: usize,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphPosition {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphNode {
    pub id: String,
    pub node_kind: String,
    pub declaration_kind: Option<String>,
    pub statement: String,
    pub display_statement: String,
    pub lean_name: Option<String>,
    pub group_id: Option<String>,
    pub member_count: usize,
    pub member_ids: Vec<String>,
    pub metrics: BTreeMap<String, f64>,
    pub generated: bool,
    pub technical: bool,
    pub position: GraphPosition,
    pub actions: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub kind: String,
    pub directed: bool,
    pub weight: f64,
    pub aggregate_count: usize,
    pub statement_count: usize,
    pub proof_count: usize,
    pub witness_available: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphResponse {
    pub schema_version: usize,
    pub corpus_fingerprint: String,
    pub totals: GraphTotals,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WitnessResponse {
    pub source: String,
    pub target: String,
    pub paths: Vec<DependencyPath>,
}

#[derive(Clone, Debug, Serialize)]
pub struct SearchResponse {
    pub results: Vec<GraphNode>,
}

#[derive(Clone, Debug)]
struct EquivalenceGroup {
    id: String,
    members: Vec<usize>,
    representative: usize,
}

/// Immutable index created once when `sieve serve` starts.
pub struct UiIndex {
    fingerprint: String,
    groups: Vec<EquivalenceGroup>,
    group_by_declaration: Vec<usize>,
    direct_dependents: Vec<usize>,
}

impl UiIndex {
    pub fn build(corpus: &AnalysisCorpus) -> Self {
        let fingerprint = corpus_fingerprint(corpus);
        let direct_dependents = direct_dependents(corpus);
        let components = equivalence_groups(corpus);
        let mut groups = components
            .into_iter()
            .map(|mut members| {
                members.sort_by(|&a, &b| {
                    corpus.declarations()[a]
                        .name
                        .cmp(&corpus.declarations()[b].name)
                });
                let representative = representative(corpus, &members);
                let mut hash = StableHash::new();
                for &member in &members {
                    hash.feed(corpus.declarations()[member].name.as_bytes());
                }
                EquivalenceGroup {
                    id: format!("group-{:016x}", hash.finish()),
                    members,
                    representative,
                }
            })
            .collect::<Vec<_>>();
        groups.sort_by(|a, b| a.id.cmp(&b.id));
        let mut group_by_declaration = vec![0; corpus.declarations().len()];
        for (group_id, group) in groups.iter().enumerate() {
            for &member in &group.members {
                group_by_declaration[member] = group_id;
            }
        }
        Self {
            fingerprint,
            groups,
            group_by_declaration,
            direct_dependents,
        }
    }

    pub fn bootstrap(&self, corpus: &AnalysisCorpus) -> UiBootstrap {
        let default = AnalysisFilter::default();
        let summary = corpus.summary(&default);
        let mut proof_declarations = corpus
            .declarations()
            .iter()
            .filter(|declaration| {
                declaration.kind == "theorem"
                    && !declaration.is_hidden_by_default()
                    && declaration.proof_steps.is_some()
            })
            .map(|declaration| ProofDeclaration {
                name: declaration.name.clone(),
            })
            .collect::<Vec<_>>();
        proof_declarations.sort_by(|left, right| left.name.cmp(&right.name));
        UiBootstrap {
            schema_version: 2,
            corpus_fingerprint: self.fingerprint.clone(),
            declaration_count: summary.declaration_count,
            theorem_count: summary.theorem_count,
            proof_declarations,
            default_filters: UiFilters::default(),
        }
    }

    pub fn proof_outline(
        &self,
        corpus: &AnalysisCorpus,
        name: &str,
    ) -> Result<ProofOutlineResponse> {
        let declaration = corpus
            .declaration(name)
            .with_context(|| format!("unknown declaration {name}"))?;
        let extraction = declaration
            .proof_steps
            .as_ref()
            .with_context(|| format!("proof-step extraction is absent for {name}"))?;
        Ok(ProofOutlineResponse {
            declaration: declaration.name.clone(),
            statement: declaration.r#type.clone(),
            outline: build_proof_outline(extraction)?,
        })
    }

    pub fn graph(&self, corpus: &AnalysisCorpus, request: &GraphRequest) -> Result<GraphResponse> {
        Ok(self.corpus_graph(corpus, request))
    }

    fn corpus_graph(&self, corpus: &AnalysisCorpus, request: &GraphRequest) -> GraphResponse {
        let mut visible_by_group = BTreeMap::<usize, Vec<usize>>::new();
        for id in 0..corpus.declarations().len() {
            if declaration_visible(corpus, id, &request.filters) {
                visible_by_group
                    .entry(self.group_by_declaration[id])
                    .or_default()
                    .push(id);
            }
        }
        for members in visible_by_group.values_mut() {
            members.sort_by(|&left, &right| {
                corpus.declarations()[left]
                    .name
                    .cmp(&corpus.declarations()[right].name)
            });
        }

        let active = visible_by_group
            .values()
            .flatten()
            .copied()
            .collect::<BTreeSet<_>>();
        let multi_member_groups = visible_by_group
            .iter()
            .filter(|(_, members)| members.len() > 1)
            .map(|(&group, _)| group)
            .collect::<BTreeSet<_>>();
        let mut endpoint_by_declaration = BTreeMap::<usize, String>::new();
        let mut nodes = Vec::with_capacity(active.len() + multi_member_groups.len());

        for &group_index in &multi_member_groups {
            let members = &visible_by_group[&group_index];
            nodes.push(self.group_node(corpus, group_index, members));
            for &member in members {
                endpoint_by_declaration.insert(member, self.groups[group_index].id.clone());
            }
        }
        for &id in &active {
            let group_index = self.group_by_declaration[id];
            let group_id = multi_member_groups
                .contains(&group_index)
                .then(|| self.groups[group_index].id.clone());
            endpoint_by_declaration
                .entry(id)
                .or_insert_with(|| format!("decl:{}", corpus.declarations()[id].name));
            let mut node = self.declaration_node(corpus, id);
            node.group_id = group_id;
            nodes.push(node);
        }
        let edges = self.grouped_declaration_edges(corpus, &active, &endpoint_by_declaration);
        response(&self.fingerprint, nodes, edges)
    }

    fn group_node(
        &self,
        corpus: &AnalysisCorpus,
        group_index: usize,
        members: &[usize],
    ) -> GraphNode {
        let group = &self.groups[group_index];
        let representative = members
            .iter()
            .copied()
            .find(|member| *member == group.representative)
            .unwrap_or(members[0]);
        let declaration = &corpus.declarations()[representative];
        let direct_dependents = self.group_direct_dependents(corpus, group_index);
        GraphNode {
            id: group.id.clone(),
            node_kind: "group".into(),
            declaration_kind: Some(declaration.kind.clone()),
            statement: declaration.r#type.clone(),
            display_statement: concise(&declaration.r#type, 70),
            lean_name: None,
            group_id: None,
            member_count: members.len(),
            member_ids: members
                .iter()
                .map(|&id| format!("decl:{}", corpus.declarations()[id].name))
                .collect(),
            metrics: BTreeMap::from([
                ("groupSize".into(), members.len() as f64),
                ("directDependents".into(), direct_dependents as f64),
                ("recommended".into(), direct_dependents as f64),
            ]),
            generated: members
                .iter()
                .all(|&id| corpus.declarations()[id].is_generated()),
            technical: members
                .iter()
                .all(|&id| corpus.declarations()[id].is_hidden_by_default()),
            position: stable_position(&group.id),
            actions: vec!["inspect".into()],
        }
    }

    fn group_direct_dependents(&self, corpus: &AnalysisCorpus, group_index: usize) -> usize {
        let mut dependents = BTreeSet::new();
        for &member in &self.groups[group_index].members {
            let name = &corpus.declarations()[member].name;
            for &edge_id in corpus.incoming_edge_ids(name) {
                let source = corpus.edges()[edge_id].source;
                if self.group_by_declaration[source] != group_index {
                    dependents.insert(source);
                }
            }
        }
        dependents.len()
    }

    fn grouped_declaration_edges(
        &self,
        corpus: &AnalysisCorpus,
        active: &BTreeSet<usize>,
        endpoint_by_declaration: &BTreeMap<usize, String>,
    ) -> Vec<GraphEdge> {
        let mut aggregated = BTreeMap::<(String, String), (usize, usize, usize)>::new();
        for edge in corpus.edges().iter().filter(|edge| edge.target_in_corpus) {
            let Some(prerequisite) = corpus.declaration_id(&edge.target) else {
                continue;
            };
            if !active.contains(&prerequisite) || !active.contains(&edge.source) {
                continue;
            }
            let source = endpoint_by_declaration[&prerequisite].clone();
            let target = endpoint_by_declaration[&edge.source].clone();
            if source == target {
                continue;
            }
            let counts = aggregated.entry((source, target)).or_default();
            counts.0 += edge.occurrences;
            match edge.layer {
                DependencyLayer::Statement => counts.1 += edge.occurrences,
                DependencyLayer::Proof => counts.2 += edge.occurrences,
            }
        }
        aggregated
            .into_iter()
            .map(
                |((source, target), (aggregate_count, statement_count, proof_count))| {
                    let mut hash = StableHash::new();
                    hash.feed(source.as_bytes());
                    hash.feed(target.as_bytes());
                    GraphEdge {
                        id: format!("dependency-{:016x}", hash.finish()),
                        source,
                        target,
                        kind: "dependency".into(),
                        directed: true,
                        weight: aggregate_count as f64,
                        aggregate_count,
                        statement_count,
                        proof_count,
                        witness_available: true,
                    }
                },
            )
            .collect()
    }

    fn declaration_node(&self, corpus: &AnalysisCorpus, id: usize) -> GraphNode {
        let declaration = &corpus.declarations()[id];
        let group = &self.groups[self.group_by_declaration[id]];
        let metrics = BTreeMap::from([
            ("groupSize".into(), group.members.len() as f64),
            ("directDependents".into(), self.direct_dependents[id] as f64),
            ("recommended".into(), self.direct_dependents[id] as f64),
        ]);
        let technical = declaration.is_hidden_by_default() || !is_math_kind(&declaration.kind);
        GraphNode {
            id: format!("decl:{}", declaration.name),
            node_kind: "declaration".into(),
            declaration_kind: Some(declaration.kind.clone()),
            statement: declaration.r#type.clone(),
            display_statement: concise(&declaration.r#type, 70),
            lean_name: Some(declaration.name.clone()),
            group_id: Some(group.id.clone()),
            member_count: 1,
            member_ids: vec![],
            metrics,
            generated: declaration.is_generated(),
            technical,
            position: stable_position(&declaration.name),
            actions: vec!["inspect".into()],
        }
    }

    pub fn witnesses(
        &self,
        corpus: &AnalysisCorpus,
        source: &str,
        target: &str,
        limit: usize,
    ) -> Result<WitnessResponse> {
        let matches_endpoint = |endpoint: &str, declaration: usize| {
            if endpoint.starts_with("group-") {
                self.groups[self.group_by_declaration[declaration]].id == endpoint
            } else {
                corpus.declarations()[declaration].name
                    == endpoint.strip_prefix("decl:").unwrap_or(endpoint)
            }
        };
        let (source, target) = corpus
            .edges()
            .iter()
            .filter(|edge| edge.target_in_corpus)
            .find_map(|edge| {
                let prerequisite = corpus.declaration_id(&edge.target)?;
                (matches_endpoint(source, prerequisite) && matches_endpoint(target, edge.source))
                    .then(|| {
                        (
                            corpus.declarations()[prerequisite].name.clone(),
                            corpus.declarations()[edge.source].name.clone(),
                        )
                    })
            })
            .with_context(|| format!("no dependency witness connects {source} and {target}"))?;
        let config = DependencyConfig {
            filter: AnalysisFilter::all(),
            weighted: false,
            max_depth: 32,
            limit: 500,
        };
        let mut paths = Vec::new();
        if let Some(path) = corpus.shortest_dependency_path(&target, &source, &config) {
            // Dependency paths run from results to prerequisites; expose graph direction.
            paths.push(path);
        }
        if paths.len() < limit.clamp(1, 10)
            && let Some(path) = corpus.shortest_dependency_path(&source, &target, &config)
        {
            paths.push(path);
        }
        if paths.is_empty() {
            bail!("no dependency witness connects {source} and {target}");
        }
        Ok(WitnessResponse {
            source,
            target,
            paths,
        })
    }

    pub fn search(&self, corpus: &AnalysisCorpus, query: &str, limit: usize) -> SearchResponse {
        let query = query.trim().to_lowercase();
        if query.is_empty() {
            return SearchResponse { results: vec![] };
        }
        let mut ids = corpus
            .declarations()
            .iter()
            .enumerate()
            .filter(|(_, declaration)| {
                !declaration.is_hidden_by_default()
                    && (declaration.name.to_lowercase().contains(&query)
                        || declaration.r#type.to_lowercase().contains(&query))
            })
            .map(|(id, declaration)| {
                let name_match = declaration.name.to_lowercase().contains(&query);
                (id, name_match)
            })
            .collect::<Vec<_>>();
        ids.sort_by(|(left, left_name), (right, right_name)| {
            right_name
                .cmp(left_name)
                .then_with(|| self.direct_dependents[*right].cmp(&self.direct_dependents[*left]))
                .then_with(|| {
                    corpus.declarations()[*left]
                        .name
                        .cmp(&corpus.declarations()[*right].name)
                })
        });
        SearchResponse {
            results: ids
                .into_iter()
                .take(limit.clamp(1, 50))
                .map(|(id, _)| self.declaration_node(corpus, id))
                .collect(),
        }
    }
}

fn response(fingerprint: &str, nodes: Vec<GraphNode>, edges: Vec<GraphEdge>) -> GraphResponse {
    let declarations = nodes
        .iter()
        .filter(|node| node.node_kind == "declaration")
        .count();
    let groups = nodes
        .iter()
        .filter(|node| node.node_kind == "group")
        .count();
    let ids = nodes
        .iter()
        .map(|node| node.id.as_str())
        .collect::<BTreeSet<_>>();
    let edges = edges
        .into_iter()
        .filter(|edge| ids.contains(edge.source.as_str()) && ids.contains(edge.target.as_str()))
        .collect::<Vec<_>>();
    let connections = edges.len();
    GraphResponse {
        schema_version: 2,
        corpus_fingerprint: fingerprint.into(),
        totals: GraphTotals {
            declarations,
            groups,
            connections,
        },
        nodes,
        edges,
    }
}

fn declaration_visible(corpus: &AnalysisCorpus, id: usize, filters: &UiFilters) -> bool {
    let declaration = &corpus.declarations()[id];
    if !filters.include_technical && declaration.is_hidden_by_default() {
        return false;
    }
    if declaration.kind == "theorem" {
        filters.include_theorems
    } else if is_math_kind(&declaration.kind) {
        filters.include_definitions
    } else {
        filters.include_technical
    }
}

fn is_math_kind(kind: &str) -> bool {
    matches!(kind, "theorem" | "definition" | "def" | "opaque" | "abbrev")
}

fn concise(statement: &str, maximum: usize) -> String {
    let normalized = statement.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.chars().count() <= maximum {
        return normalized;
    }
    let mut value = normalized
        .chars()
        .take(maximum.saturating_sub(1))
        .collect::<String>();
    value.push('…');
    value
}

fn stable_position(value: &str) -> GraphPosition {
    let mut hash = StableHash::new();
    hash.feed(value.as_bytes());
    let raw = hash.finish();
    let angle = (raw % 100_000) as f64 / 100_000.0 * std::f64::consts::TAU;
    let radius = 20.0 + ((raw >> 17) % 10_000) as f64 / 80.0;
    GraphPosition {
        x: angle.cos() * radius,
        y: angle.sin() * radius,
    }
}

fn direct_dependents(corpus: &AnalysisCorpus) -> Vec<usize> {
    let count = corpus.declarations().len();
    let mut dependents = vec![BTreeSet::new(); count];
    for edge in corpus.edges().iter().filter(|edge| edge.target_in_corpus) {
        if let Some(target) = corpus.declaration_id(&edge.target) {
            dependents[target].insert(edge.source);
        }
    }
    dependents.iter().map(BTreeSet::len).collect()
}

fn equivalence_groups(corpus: &AnalysisCorpus) -> Vec<Vec<usize>> {
    let mut groups = BTreeMap::<(String, u64), Vec<usize>>::new();
    for (id, declaration) in corpus.declarations().iter().enumerate() {
        groups
            .entry((
                declaration.kind.clone(),
                declaration
                    .type_graph
                    .root_fingerprint(StructuralMode::AlphaEquivalent),
            ))
            .or_default()
            .push(id);
    }
    groups.into_values().collect()
}

fn representative(corpus: &AnalysisCorpus, members: &[usize]) -> usize {
    members
        .iter()
        .copied()
        .min_by(|&a, &b| {
            corpus.declarations()[a]
                .r#type
                .len()
                .cmp(&corpus.declarations()[b].r#type.len())
                .then_with(|| {
                    corpus.declarations()[a]
                        .name
                        .cmp(&corpus.declarations()[b].name)
                })
        })
        .expect("equivalence group is nonempty")
}

fn corpus_fingerprint(corpus: &AnalysisCorpus) -> String {
    let mut hash = StableHash::new();
    hash.feed(&corpus.snapshot().schema_version.to_le_bytes());
    hash.feed(corpus.snapshot().lean_version.as_bytes());
    for module in &corpus.snapshot().imported_modules {
        hash.feed(module.as_bytes());
    }
    for declaration in corpus.declarations() {
        hash.feed(declaration.name.as_bytes());
        hash.feed(
            &declaration
                .type_graph
                .root_fingerprint(StructuralMode::Exact)
                .to_le_bytes(),
        );
    }
    format!("{:016x}", hash.finish())
}

struct StableHash(u64);
impl StableHash {
    fn new() -> Self {
        Self(0xcbf29ce484222325)
    }
    fn feed(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(0x100000001b3);
        }
        self.0 ^= 0xff;
        self.0 = self.0.wrapping_mul(0x100000001b3);
    }
    fn finish(self) -> u64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        DeclarationSnapshot, ExprStats, ExpressionGraph, ExpressionNode, ExtractionSnapshot,
        ProofStep, ProofStepExtraction, SymbolSnapshot,
    };

    fn declaration(index: usize, generated: bool) -> DeclarationSnapshot {
        let statement = if index == 0 {
            "P".to_owned()
        } else {
            format!("A deliberately longer rendering of P number {index}")
        };
        DeclarationSnapshot {
            name: format!("Fixture.t{index:03}"),
            kind: "theorem".into(),
            module_name: Some("Fixture".into()),
            generation_kind: generated.then(|| "equationTheorem".into()),
            is_internal: false,
            is_private: false,
            is_unsafe: false,
            is_partial: false,
            doc_string: None,
            source_range: None,
            level_parameters: vec![],
            r#type: statement,
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
                    name: Some("Fixture.P".into()),
                    index: None,
                    value: None,
                    binder_info: None,
                    universe_levels: vec![],
                }],
            },
            value_graph: None,
            statement_dependencies: vec!["Fixture.P".into()],
            proof_dependencies: vec![],
            axioms: vec![],
            proof_steps: None,
        }
    }

    fn fixture(count: usize) -> AnalysisCorpus {
        let declarations = (0..count)
            .map(|index| declaration(index, index % 5 == 4))
            .collect::<Vec<_>>();
        let mut symbols = declarations
            .iter()
            .map(|declaration| SymbolSnapshot {
                name: declaration.name.clone(),
                kind: declaration.kind.clone(),
                module_name: declaration.module_name.clone(),
                generation_kind: declaration.generation_kind.clone(),
                is_internal: false,
                is_private: false,
                is_unsafe: false,
                is_partial: false,
                is_class: false,
                is_instance: false,
                is_projection: false,
            })
            .collect::<Vec<_>>();
        symbols.push(SymbolSnapshot {
            name: "Fixture.P".into(),
            kind: "axiom".into(),
            module_name: Some("Fixture".into()),
            generation_kind: None,
            is_internal: false,
            is_private: false,
            is_unsafe: false,
            is_partial: false,
            is_class: false,
            is_instance: false,
            is_projection: false,
        });
        AnalysisCorpus::new(ExtractionSnapshot {
            schema_version: 6,
            lean_version: "fixture".into(),
            imported_modules: vec!["Fixture".into()],
            declarations,
            symbols,
        })
        .unwrap()
    }

    fn fixture_with_proof() -> AnalysisCorpus {
        let mut snapshot = fixture(1).snapshot().clone();
        let declaration = &mut snapshot.declarations[0];
        declaration.has_value = true;
        declaration.value_stats = Some(declaration.type_stats.clone());
        declaration.value_graph = Some(declaration.type_graph.clone());
        declaration.proof_dependencies = declaration.statement_dependencies.clone();
        declaration.proof_steps = Some(ProofStepExtraction {
            complete: true,
            truncation_reason: None,
            visited_terms: 1,
            conclusion_step: Some(0),
            steps: vec![ProofStep {
                id: 0,
                kind: "conclusion".into(),
                proposition: declaration.r#type.clone(),
                proposition_graph: declaration.type_graph.clone(),
                context: vec![],
                scope: vec![],
                proof_term_path: vec![],
                prerequisite_steps: vec![],
                hypothesis_references: vec![],
                named_references: vec![],
            }],
            named_results: vec![],
        });
        AnalysisCorpus::new(snapshot).unwrap()
    }

    #[test]
    fn concise_statements_are_bounded() {
        assert_eq!(concise("  alpha   beta  ", 20), "alpha beta");
        assert_eq!(concise("abcdefghijk", 6), "abcde…");
    }

    #[test]
    fn stable_positions_are_repeatable() {
        let first = stable_position("a theorem");
        let second = stable_position("a theorem");
        assert_eq!((first.x, first.y), (second.x, second.y));
    }

    #[test]
    fn bootstrap_lists_proofs_and_outline_uses_condensed_evidence() {
        let corpus = fixture_with_proof();
        let index = UiIndex::build(&corpus);
        let bootstrap = index.bootstrap(&corpus);
        assert_eq!(bootstrap.proof_declarations.len(), 1);
        assert_eq!(bootstrap.proof_declarations[0].name, "Fixture.t000");

        let response = index.proof_outline(&corpus, "Fixture.t000").unwrap();
        assert_eq!(response.declaration, "Fixture.t000");
        assert_eq!(response.outline.conclusion_step, Some(0));
        assert_eq!(response.outline.retained_nodes.len(), 1);
    }

    #[test]
    fn index_membership_representative_and_fingerprint_are_deterministic() {
        let corpus = fixture(30);
        let first = UiIndex::build(&corpus);
        let second = UiIndex::build(&corpus);
        assert_eq!(first.fingerprint, second.fingerprint);
        assert_eq!(first.group_by_declaration, second.group_by_declaration);
        assert_eq!(first.groups.len(), 1);
        assert_eq!(
            first.groups[0].representative,
            second.groups[0].representative
        );
        assert!(
            first.groups[0]
                .members
                .contains(&first.groups[0].representative)
        );
    }

    #[test]
    fn equivalent_declarations_remain_in_one_group() {
        let corpus = fixture(205);
        let index = UiIndex::build(&corpus);
        assert_eq!(index.groups.len(), 1);
        assert_eq!(index.groups[0].members.len(), 205);
    }

    #[test]
    fn equivalence_groups_do_not_mix_declaration_kinds() {
        let mut snapshot = fixture(2).snapshot().clone();
        snapshot.declarations[1].kind = "definition".into();
        snapshot.symbols[1].kind = "definition".into();
        let corpus = AnalysisCorpus::new(snapshot).unwrap();
        let index = UiIndex::build(&corpus);
        assert_eq!(index.groups.len(), 2);
    }

    #[test]
    fn dependencies_of_equivalent_members_are_routed_through_the_group() {
        let mut snapshot = fixture(3).snapshot().clone();
        snapshot.declarations[2].r#type = "Fixture.t000".into();
        snapshot.declarations[2].type_graph.nodes[0].name = Some("Fixture.t000".into());
        snapshot.declarations[2].statement_dependencies = vec!["Fixture.t000".into()];
        let corpus = AnalysisCorpus::new(snapshot).unwrap();
        let index = UiIndex::build(&corpus);
        let graph = index.graph(&corpus, &GraphRequest::default()).unwrap();
        let group = graph
            .nodes
            .iter()
            .find(|node| node.node_kind == "group")
            .unwrap();
        let edge = graph
            .edges
            .iter()
            .find(|edge| edge.target == "decl:Fixture.t002")
            .unwrap();
        assert_eq!(edge.source, group.id);
        assert_eq!(edge.statement_count, 1);
        assert_eq!(group.metrics["directDependents"], 1.0);
    }

    #[test]
    fn flat_graph_does_not_apply_a_manual_limit() {
        let corpus = fixture(30);
        let index = UiIndex::build(&corpus);
        let graph = index.graph(&corpus, &GraphRequest::default()).unwrap();
        assert_eq!(graph.totals.declarations, 24);
        assert_eq!(graph.totals.groups, 1);
        assert_eq!(graph.nodes.len(), 25);
        let ids = graph
            .nodes
            .iter()
            .map(|node| node.id.as_str())
            .collect::<BTreeSet<_>>();
        assert!(graph.edges.iter().all(|edge| {
            ids.contains(edge.source.as_str()) && ids.contains(edge.target.as_str())
        }));
    }

    #[test]
    fn corpus_graph_keeps_group_members_as_distinct_nodes() {
        let corpus = fixture(30);
        let index = UiIndex::build(&corpus);
        let graph = index.graph(&corpus, &GraphRequest::default()).unwrap();

        assert_eq!(graph.totals.declarations, 24);
        assert_eq!(graph.nodes.len(), 25);
        assert!(
            graph
                .nodes
                .iter()
                .filter(|node| node.node_kind == "declaration")
                .all(|node| {
                    node.node_kind == "declaration"
                        && node.member_count == 1
                        && node.group_id.as_deref() == Some(index.groups[0].id.as_str())
                })
        );
        let group = graph
            .nodes
            .iter()
            .find(|node| node.node_kind == "group")
            .unwrap();
        assert_eq!(group.member_count, 24);
        assert_eq!(group.member_ids.len(), 24);
        assert_eq!(
            graph
                .nodes
                .iter()
                .map(|node| node.id.as_str())
                .collect::<BTreeSet<_>>()
                .len(),
            graph.nodes.len()
        );
    }

    #[test]
    fn alpha_equivalent_declarations_form_one_group() {
        let corpus = fixture(10);
        let index = UiIndex::build(&corpus);
        let graph = index.graph(&corpus, &GraphRequest::default()).unwrap();
        assert_eq!(graph.totals.groups, 1);
    }
}
