use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::str::FromStr;

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use crate::analysis::comparison::SimilarityComponents;
use crate::analysis::corpus::AnalysisCorpus;
use crate::analysis::dependency::DependencyConfig;
use crate::analysis::dependency::label_propagation;
use crate::analysis::filters::{AnalysisFilter, DependencyLayer, LayerSelection};
use crate::analysis::proof_steps::{ProofOutlineEvidence, build_proof_outline};
use crate::model::SourceRange;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LensKind {
    Influence,
    Bridge,
    Neighbors,
    Proof,
    Trust,
    All,
}

impl FromStr for LensKind {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "influence" => Ok(Self::Influence),
            "bridge" => Ok(Self::Bridge),
            "neighbors" | "neighbours" => Ok(Self::Neighbors),
            "proof" => Ok(Self::Proof),
            "trust" => Ok(Self::Trust),
            "all" => Ok(Self::All),
            _ => bail!("unknown lens {value}"),
        }
    }
}

impl LensKind {
    pub fn expand_for_discovery(self, discovery: bool) -> Vec<Self> {
        match (self, discovery) {
            (Self::All, true) => vec![Self::Influence, Self::Bridge, Self::Neighbors],
            (Self::All, false) => vec![
                Self::Influence,
                Self::Bridge,
                Self::Neighbors,
                Self::Proof,
                Self::Trust,
            ],
            (lens, _) => vec![lens],
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LensAnalysisConfig {
    pub filter: AnalysisFilter,
    pub layer_explicit: bool,
    pub max_depth: usize,
    pub limit: usize,
    pub representative_path_limit: usize,
}

impl Default for LensAnalysisConfig {
    fn default() -> Self {
        Self {
            filter: AnalysisFilter::default(),
            layer_explicit: false,
            max_depth: 32,
            limit: 100,
            representative_path_limit: 3,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyWitnessPath {
    pub nodes: Vec<String>,
    pub layers: Vec<DependencyLayer>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphRegion {
    pub id: String,
    pub layer: DependencyLayer,
    pub members: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InfluenceEvidence {
    pub name: String,
    pub statement: String,
    pub reason: String,
    pub layer: DependencyLayer,
    pub direct_dependent_count: usize,
    pub direct_dependents: Vec<String>,
    pub reachable_dependent_count: usize,
    pub reachable_dependents: Vec<String>,
    pub graph_region_coverage: usize,
    pub graph_regions: Vec<String>,
    pub representative_paths: Vec<DependencyWitnessPath>,
    pub traversal_truncated: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectedRegionPair {
    pub incoming_region: String,
    pub outgoing_region: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeEvidence {
    pub name: String,
    pub statement: String,
    pub reason: String,
    pub layer: DependencyLayer,
    pub articulation_point: bool,
    pub distinct_directed_region_pair_count: usize,
    pub directed_region_pairs: Vec<DirectedRegionPair>,
    pub betweenness: f64,
    pub incoming_theorems: Vec<String>,
    pub outgoing_theorems: Vec<String>,
    pub witness_paths: Vec<DependencyWitnessPath>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NeighborMatchEvidence {
    pub name: String,
    pub statement: String,
    pub exact_equal: bool,
    pub alpha_equivalent: bool,
    pub similarity: SimilarityComponents,
    pub shared_dependencies: Vec<String>,
    pub focal_only_dependencies: Vec<String>,
    pub neighbor_only_dependencies: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NeighborEvidence {
    pub name: String,
    pub statement: String,
    pub reason: String,
    pub layer: DependencyLayer,
    pub neighbors: Vec<NeighborMatchEvidence>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProofExtractionStatus {
    pub available: bool,
    pub complete: Option<bool>,
    pub truncation_reason: Option<String>,
    pub visited_terms: Option<usize>,
    pub step_count: Option<usize>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustEvidence {
    pub axioms: Vec<String>,
    pub is_internal: bool,
    pub is_private: bool,
    pub is_generated: bool,
    pub is_unsafe: bool,
    pub is_partial: bool,
    pub has_value: bool,
    pub source_available: bool,
    pub source_range: Option<SourceRange>,
    pub documentation_available: bool,
    pub documentation: Option<String>,
    pub proof_extraction: ProofExtractionStatus,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryReport {
    pub requested_lenses: Vec<LensKind>,
    pub config: LensAnalysisConfig,
    pub ranking_rules: Vec<LensRankingRule>,
    pub graph_region_algorithm: String,
    pub graph_regions: Vec<GraphRegion>,
    pub influence: Vec<InfluenceEvidence>,
    pub bridges: Vec<BridgeEvidence>,
    pub neighbors: Vec<NeighborEvidence>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TheoremLensReport {
    pub theorem: String,
    pub statement: String,
    pub requested_lenses: Vec<LensKind>,
    pub config: LensAnalysisConfig,
    pub graph_region_algorithm: String,
    pub graph_regions: Vec<GraphRegion>,
    pub influence: Vec<InfluenceEvidence>,
    pub bridges: Vec<BridgeEvidence>,
    pub neighbors: Vec<NeighborEvidence>,
    pub proof_outline: Option<ProofOutlineEvidence>,
    pub trust: Option<TrustEvidence>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LensRankingRule {
    pub lens: LensKind,
    pub ordering: String,
}

struct LayerGraph {
    adjacency: Vec<Vec<usize>>,
    reverse: Vec<Vec<usize>>,
    /// Declarations eligible to appear as reported candidates/endpoints.
    active: BTreeSet<usize>,
    /// Declarations allowed to connect evidence paths, including hidden
    /// generated and infrastructure declarations.
    path_active: BTreeSet<usize>,
}

impl AnalysisCorpus {
    pub fn discover_lenses(
        &self,
        requested_lenses: &[LensKind],
        config: &LensAnalysisConfig,
    ) -> DiscoveryReport {
        let requested_lenses = normalize_lenses(requested_lenses, true);
        let region_layers = graph_layers(&requested_lenses, config);
        let (graph_regions, region_maps) = self.regions_for_layers(&region_layers, config);
        let mut influence = Vec::new();
        let mut bridges = Vec::new();
        let mut neighbors = Vec::new();

        if requested_lenses.contains(&LensKind::Influence) {
            for layer in layers_for(LensKind::Influence, config) {
                influence.extend(self.influence_evidence(layer, config, &region_maps, false));
            }
            influence.sort_by(|a, b| {
                b.reachable_dependent_count
                    .cmp(&a.reachable_dependent_count)
                    .then_with(|| b.direct_dependent_count.cmp(&a.direct_dependent_count))
                    .then_with(|| a.name.cmp(&b.name))
                    .then_with(|| a.layer.cmp(&b.layer))
            });
            influence.truncate(config.limit);
        }
        if requested_lenses.contains(&LensKind::Bridge) {
            for layer in layers_for(LensKind::Bridge, config) {
                bridges.extend(self.bridge_evidence(layer, config, &region_maps, false));
            }
            bridges.sort_by(|a, b| {
                b.articulation_point
                    .cmp(&a.articulation_point)
                    .then_with(|| {
                        b.distinct_directed_region_pair_count
                            .cmp(&a.distinct_directed_region_pair_count)
                    })
                    .then_with(|| b.betweenness.total_cmp(&a.betweenness))
                    .then_with(|| a.name.cmp(&b.name))
                    .then_with(|| a.layer.cmp(&b.layer))
            });
            bridges.truncate(config.limit);
        }
        if requested_lenses.contains(&LensKind::Neighbors) {
            for layer in layers_for(LensKind::Neighbors, config) {
                neighbors.extend(self.neighbor_evidence(layer, config, None));
            }
            neighbors.sort_by(|a, b| {
                best_neighbor_score(b)
                    .total_cmp(&best_neighbor_score(a))
                    .then_with(|| a.name.cmp(&b.name))
                    .then_with(|| a.layer.cmp(&b.layer))
            });
            neighbors.truncate(config.limit);
        }

        DiscoveryReport {
            requested_lenses: requested_lenses.clone(),
            config: config.clone(),
            ranking_rules: discovery_ranking_rules(&requested_lenses),
            graph_region_algorithm: graph_region_algorithm().into(),
            graph_regions,
            influence,
            bridges,
            neighbors,
        }
    }

    pub fn inspect_lenses(
        &self,
        theorem: &str,
        requested_lenses: &[LensKind],
        config: &LensAnalysisConfig,
    ) -> Result<TheoremLensReport> {
        let requested_lenses = normalize_lenses(requested_lenses, false);
        let declaration = self
            .declaration(theorem)
            .ok_or_else(|| anyhow::anyhow!("unknown theorem {theorem}"))?;
        if declaration.kind != "theorem" {
            bail!("{theorem} is a {}, not a theorem", declaration.kind);
        }
        let region_layers = graph_layers(&requested_lenses, config);
        let (graph_regions, region_maps) = self.regions_for_layers(&region_layers, config);
        let mut influence = Vec::new();
        let mut bridges = Vec::new();
        let mut neighbors = Vec::new();
        if requested_lenses.contains(&LensKind::Influence) {
            for layer in layers_for(LensKind::Influence, config) {
                influence.extend(
                    self.influence_evidence(layer, config, &region_maps, true)
                        .into_iter()
                        .filter(|evidence| evidence.name == theorem),
                );
            }
        }
        if requested_lenses.contains(&LensKind::Bridge) {
            for layer in layers_for(LensKind::Bridge, config) {
                bridges.extend(
                    self.bridge_evidence(layer, config, &region_maps, true)
                        .into_iter()
                        .filter(|evidence| evidence.name == theorem),
                );
            }
        }
        if requested_lenses.contains(&LensKind::Neighbors) {
            for layer in layers_for(LensKind::Neighbors, config) {
                neighbors.extend(self.neighbor_evidence(layer, config, Some(theorem)));
            }
        }
        let proof_outline = if requested_lenses.contains(&LensKind::Proof) {
            let extraction = declaration
                .proof_steps
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("proof-step extraction is absent for {theorem}"))?;
            Some(build_proof_outline(extraction)?)
        } else {
            None
        };
        let trust = requested_lenses
            .contains(&LensKind::Trust)
            .then(|| trust_evidence(declaration));
        Ok(TheoremLensReport {
            theorem: theorem.into(),
            statement: declaration.r#type.clone(),
            requested_lenses: requested_lenses.clone(),
            config: config.clone(),
            graph_region_algorithm: graph_region_algorithm().into(),
            graph_regions,
            influence,
            bridges,
            neighbors,
            proof_outline,
            trust,
        })
    }

    fn layer_graph(&self, layer: DependencyLayer, config: &LensAnalysisConfig) -> LayerGraph {
        let filter = exact_layer_filter(&config.filter, layer);
        let active = self
            .filtered_declaration_ids(&filter)
            .collect::<BTreeSet<_>>();
        let path_filter = path_layer_filter(&config.filter, layer);
        let path_active = self
            .filtered_declaration_ids(&path_filter)
            .collect::<BTreeSet<_>>();
        let mut adjacency = vec![Vec::new(); self.declarations().len()];
        let mut reverse = vec![Vec::new(); self.declarations().len()];
        for edge in self.filtered_edges(&path_filter) {
            let Some(target) = self.declaration_id(&edge.target) else {
                continue;
            };
            if path_active.contains(&edge.source) && path_active.contains(&target) {
                adjacency[edge.source].push(target);
                reverse[target].push(edge.source);
            }
        }
        for edges in adjacency.iter_mut().chain(reverse.iter_mut()) {
            edges.sort_by_key(|id| &self.declarations()[*id].name);
            edges.dedup();
        }
        LayerGraph {
            adjacency,
            reverse,
            active,
            path_active,
        }
    }

    fn regions_for_layers(
        &self,
        layers: &[DependencyLayer],
        config: &LensAnalysisConfig,
    ) -> (
        Vec<GraphRegion>,
        BTreeMap<(DependencyLayer, String), String>,
    ) {
        let mut regions = Vec::new();
        let mut lookup = BTreeMap::new();
        for &layer in layers {
            let graph = self.layer_graph(layer, config);
            let mut undirected = vec![BTreeSet::new(); self.declarations().len()];
            for &source in &graph.path_active {
                for &target in &graph.adjacency[source] {
                    undirected[source].insert(target);
                    undirected[target].insert(source);
                }
            }
            let labels = label_propagation(&undirected, &graph.path_active);
            let mut groups = BTreeMap::<usize, Vec<String>>::new();
            for (&id, &community) in &labels {
                groups
                    .entry(community)
                    .or_default()
                    .push(self.declarations()[id].name.clone());
            }
            let mut groups = groups.into_values().collect::<Vec<_>>();
            for members in &mut groups {
                members.sort();
            }
            groups.sort();
            for (index, members) in groups.into_iter().enumerate() {
                let id = format!("{}-region-{:04}", layer_name(layer), index + 1);
                for member in &members {
                    lookup.insert((layer, member.clone()), id.clone());
                }
                let visible_members = members
                    .into_iter()
                    .filter(|member| {
                        self.declaration_id(member)
                            .is_some_and(|member_id| graph.active.contains(&member_id))
                    })
                    .collect::<Vec<_>>();
                if !visible_members.is_empty() {
                    regions.push(GraphRegion {
                        id,
                        layer,
                        members: visible_members,
                    });
                }
            }
        }
        (regions, lookup)
    }

    fn influence_evidence(
        &self,
        layer: DependencyLayer,
        config: &LensAnalysisConfig,
        regions: &BTreeMap<(DependencyLayer, String), String>,
        include_all: bool,
    ) -> Vec<InfluenceEvidence> {
        let graph = self.layer_graph(layer, config);
        theorem_ids(self, &graph.active)
            .into_iter()
            .filter_map(|target| {
                self.graph(target, layer)?;
                let direct = graph.reverse[target]
                    .iter()
                    .copied()
                    .filter(|id| {
                        graph.active.contains(id) && self.declarations()[*id].kind == "theorem"
                    })
                    .collect::<Vec<_>>();
                let mut queue = VecDeque::from([(target, 0usize)]);
                let mut seen = BTreeSet::from([target]);
                let mut toward_target = BTreeMap::new();
                let mut depth = BTreeMap::from([(target, 0usize)]);
                let mut truncated = false;
                while let Some((current, current_depth)) = queue.pop_front() {
                    if current_depth >= config.max_depth {
                        if graph.reverse[current].iter().any(|next| !seen.contains(next)) {
                            truncated = true;
                        }
                        continue;
                    }
                    for &next in &graph.reverse[current] {
                        if seen.insert(next) {
                            toward_target.insert(next, current);
                            depth.insert(next, current_depth + 1);
                            queue.push_back((next, current_depth + 1));
                        }
                    }
                }
                let reachable = seen
                    .iter()
                    .copied()
                    .filter(|id| {
                        *id != target
                            && graph.active.contains(id)
                            && self.declarations()[*id].kind == "theorem"
                    })
                    .collect::<Vec<_>>();
                if !include_all && reachable.is_empty() {
                    return None;
                }
                let direct_names = sorted_names(self, direct.iter().copied());
                let reachable_names = sorted_names(self, reachable.iter().copied());
                let graph_regions = reachable
                    .iter()
                    .filter_map(|id| {
                        regions.get(&(layer, self.declarations()[*id].name.clone()))
                    })
                    .cloned()
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect::<Vec<_>>();
                let mut path_starts = reachable.clone();
                path_starts.sort_by(|a, b| {
                    depth[b]
                        .cmp(&depth[a])
                        .then_with(|| self.declarations()[*a].name.cmp(&self.declarations()[*b].name))
                });
                let representative_paths = path_starts
                    .into_iter()
                    .take(config.representative_path_limit)
                    .map(|start| {
                        let mut nodes = vec![self.declarations()[start].name.clone()];
                        let mut current = start;
                        while current != target {
                            current = toward_target[&current];
                            nodes.push(self.declarations()[current].name.clone());
                        }
                        DependencyWitnessPath {
                            layers: vec![layer; nodes.len().saturating_sub(1)],
                            nodes,
                        }
                    })
                    .collect();
                let name = self.declarations()[target].name.clone();
                let reachable_count = reachable_names.len();
                let direct_count = direct_names.len();
                Some(InfluenceEvidence {
                    statement: self.declarations()[target].r#type.clone(),
                    reason: format!(
                        "reachable from {reachable_count} theorem(s), including {direct_count} direct dependent(s), across {} graph region(s)",
                        graph_regions.len()
                    ),
                    name,
                    layer,
                    direct_dependent_count: direct_count,
                    direct_dependents: direct_names,
                    reachable_dependent_count: reachable_count,
                    reachable_dependents: reachable_names,
                    graph_region_coverage: graph_regions.len(),
                    graph_regions,
                    representative_paths,
                    traversal_truncated: truncated,
                })
            })
            .collect()
    }

    fn bridge_evidence(
        &self,
        layer: DependencyLayer,
        config: &LensAnalysisConfig,
        regions: &BTreeMap<(DependencyLayer, String), String>,
        include_all: bool,
    ) -> Vec<BridgeEvidence> {
        let graph = self.layer_graph(layer, config);
        let dependency_config = DependencyConfig {
            filter: path_layer_filter(&config.filter, layer),
            weighted: false,
            max_depth: config.max_depth,
            limit: usize::MAX,
        };
        let structure = self.graph_structure(&dependency_config);
        let articulation = structure
            .articulation_points
            .into_iter()
            .collect::<BTreeSet<_>>();
        let betweenness = self
            .centrality(&dependency_config)
            .entries
            .into_iter()
            .map(|entry| (entry.name, entry.betweenness))
            .collect::<BTreeMap<_, _>>();
        theorem_ids(self, &graph.active)
            .into_iter()
            .filter_map(|id| {
                self.graph(id, layer)?;
                let incoming = graph.reverse[id]
                    .iter()
                    .copied()
                    .filter(|other| {
                        graph.active.contains(other)
                            && self.declarations()[*other].kind == "theorem"
                    })
                    .collect::<Vec<_>>();
                let outgoing = graph.adjacency[id]
                    .iter()
                    .copied()
                    .filter(|other| {
                        graph.active.contains(other)
                            && self.declarations()[*other].kind == "theorem"
                    })
                    .collect::<Vec<_>>();
                let mut pairs = BTreeSet::new();
                let mut witnesses = Vec::new();
                for &source in &incoming {
                    for &target in &outgoing {
                        let source_name = &self.declarations()[source].name;
                        let target_name = &self.declarations()[target].name;
                        let (Some(incoming_region), Some(outgoing_region)) = (
                            regions.get(&(layer, source_name.clone())),
                            regions.get(&(layer, target_name.clone())),
                        ) else {
                            continue;
                        };
                        if incoming_region == outgoing_region {
                            continue;
                        }
                        if pairs.insert((incoming_region.clone(), outgoing_region.clone())) {
                            witnesses.push(DependencyWitnessPath {
                                nodes: vec![
                                    source_name.clone(),
                                    self.declarations()[id].name.clone(),
                                    target_name.clone(),
                                ],
                                layers: vec![layer, layer],
                            });
                        }
                    }
                }
                if !include_all && pairs.is_empty() {
                    return None;
                }
                let directed_region_pairs = pairs
                    .into_iter()
                    .map(|(incoming_region, outgoing_region)| DirectedRegionPair {
                        incoming_region,
                        outgoing_region,
                    })
                    .collect::<Vec<_>>();
                witnesses.truncate(config.representative_path_limit);
                let name = self.declarations()[id].name.clone();
                let is_articulation = articulation.contains(&name);
                let pair_count = directed_region_pairs.len();
                Some(BridgeEvidence {
                    statement: self.declarations()[id].r#type.clone(),
                    reason: if pair_count == 0 {
                        "no direct dependency witness connects distinct graph regions".into()
                    } else {
                        format!(
                            "lies on direct dependency witnesses for {pair_count} distinct directed graph-region pair(s)"
                        )
                    },
                    name: name.clone(),
                    layer,
                    articulation_point: is_articulation,
                    distinct_directed_region_pair_count: pair_count,
                    directed_region_pairs,
                    betweenness: betweenness.get(&name).copied().unwrap_or(0.0),
                    incoming_theorems: sorted_names(self, incoming),
                    outgoing_theorems: sorted_names(self, outgoing),
                    witness_paths: witnesses,
                })
            })
            .collect()
    }

    fn neighbor_evidence(
        &self,
        layer: DependencyLayer,
        config: &LensAnalysisConfig,
        only: Option<&str>,
    ) -> Vec<NeighborEvidence> {
        let filter = exact_layer_filter(&config.filter, layer);
        let candidates = self
            .filtered_declaration_ids(&filter)
            .filter(|id| {
                self.declarations()[*id].kind == "theorem" && self.graph(*id, layer).is_some()
            })
            .collect::<Vec<_>>();
        candidates
            .iter()
            .copied()
            .filter(|id| only.is_none_or(|name| self.declarations()[*id].name == name))
            .filter_map(|id| {
                let name = &self.declarations()[id].name;
                let mut matches = candidates
                    .iter()
                    .copied()
                    .filter(|other| *other != id)
                    .filter_map(|other| {
                        let comparison = self.compare_declarations(
                            name,
                            &self.declarations()[other].name,
                            layer,
                        )?;
                        Some(NeighborMatchEvidence {
                            name: comparison.right,
                            statement: self.declarations()[other].r#type.clone(),
                            exact_equal: comparison.exact_equal,
                            alpha_equivalent: comparison.alpha_equivalent,
                            similarity: comparison.similarity,
                            shared_dependencies: comparison.shared_dependencies,
                            focal_only_dependencies: comparison.left_only_dependencies,
                            neighbor_only_dependencies: comparison.right_only_dependencies,
                        })
                    })
                    .collect::<Vec<_>>();
                matches.sort_by(|a, b| {
                    b.similarity
                        .combined_score
                        .total_cmp(&a.similarity.combined_score)
                        .then_with(|| a.name.cmp(&b.name))
                });
                matches.truncate(if only.is_some() { config.limit } else { 3 });
                let best = matches.first()?;
                Some(NeighborEvidence {
                    name: name.clone(),
                    statement: self.declarations()[id].r#type.clone(),
                    reason: neighbor_reason(best),
                    layer,
                    neighbors: matches,
                })
            })
            .collect()
    }
}

fn trust_evidence(declaration: &crate::model::DeclarationSnapshot) -> TrustEvidence {
    let proof_extraction = declaration.proof_steps.as_ref();
    TrustEvidence {
        axioms: declaration.axioms.clone(),
        is_internal: declaration.is_internal,
        is_private: declaration.is_private,
        is_generated: declaration.is_internal || declaration.is_private,
        is_unsafe: declaration.is_unsafe,
        is_partial: declaration.is_partial,
        has_value: declaration.has_value,
        source_available: declaration.source_range.is_some(),
        source_range: declaration.source_range.clone(),
        documentation_available: declaration.doc_string.is_some(),
        documentation: declaration.doc_string.clone(),
        proof_extraction: ProofExtractionStatus {
            available: proof_extraction.is_some(),
            complete: proof_extraction.map(|proof| proof.complete),
            truncation_reason: proof_extraction.and_then(|proof| proof.truncation_reason.clone()),
            visited_terms: proof_extraction.map(|proof| proof.visited_terms),
            step_count: proof_extraction.map(|proof| proof.steps.len()),
        },
    }
}

fn exact_layer_filter(filter: &AnalysisFilter, layer: DependencyLayer) -> AnalysisFilter {
    let mut filter = filter.clone();
    filter.layer = match layer {
        DependencyLayer::Statement => LayerSelection::Statement,
        DependencyLayer::Proof => LayerSelection::Proof,
    };
    filter
}

fn path_layer_filter(filter: &AnalysisFilter, layer: DependencyLayer) -> AnalysisFilter {
    let mut filter = exact_layer_filter(filter, layer);
    filter.include_generated = true;
    filter.include_infrastructure = true;
    filter
}

fn layers_for(kind: LensKind, config: &LensAnalysisConfig) -> Vec<DependencyLayer> {
    if config.layer_explicit {
        match config.filter.layer {
            LayerSelection::Statement => vec![DependencyLayer::Statement],
            LayerSelection::Proof => vec![DependencyLayer::Proof],
            LayerSelection::Both => vec![DependencyLayer::Statement, DependencyLayer::Proof],
        }
    } else {
        match kind {
            LensKind::Influence | LensKind::Bridge => vec![DependencyLayer::Proof],
            LensKind::Neighbors => vec![DependencyLayer::Statement],
            LensKind::Proof | LensKind::Trust | LensKind::All => vec![],
        }
    }
}

fn graph_layers(
    requested_lenses: &[LensKind],
    config: &LensAnalysisConfig,
) -> Vec<DependencyLayer> {
    requested_lenses
        .iter()
        .filter(|lens| matches!(lens, LensKind::Influence | LensKind::Bridge))
        .flat_map(|lens| layers_for(*lens, config))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn normalize_lenses(requested: &[LensKind], discovery: bool) -> Vec<LensKind> {
    requested
        .iter()
        .flat_map(|lens| lens.expand_for_discovery(discovery))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn theorem_ids(corpus: &AnalysisCorpus, active: &BTreeSet<usize>) -> Vec<usize> {
    let mut ids = active
        .iter()
        .copied()
        .filter(|id| corpus.declarations()[*id].kind == "theorem")
        .collect::<Vec<_>>();
    ids.sort_by_key(|id| &corpus.declarations()[*id].name);
    ids
}

fn sorted_names(corpus: &AnalysisCorpus, ids: impl IntoIterator<Item = usize>) -> Vec<String> {
    let mut names = ids
        .into_iter()
        .map(|id| corpus.declarations()[id].name.clone())
        .collect::<Vec<_>>();
    names.sort();
    names.dedup();
    names
}

fn layer_name(layer: DependencyLayer) -> &'static str {
    match layer {
        DependencyLayer::Statement => "statement",
        DependencyLayer::Proof => "proof",
    }
}

fn best_neighbor_score(evidence: &NeighborEvidence) -> f64 {
    evidence
        .neighbors
        .first()
        .map(|neighbor| neighbor.similarity.combined_score)
        .unwrap_or(0.0)
}

fn neighbor_reason(neighbor: &NeighborMatchEvidence) -> String {
    let structural_claim = if neighbor.exact_equal {
        "exact expression equality"
    } else if neighbor.alpha_equivalent {
        "alpha-equivalent expression structure"
    } else {
        "no exact or alpha-equivalence claim"
    };
    format!(
        "closest reported statement is {} with combined similarity {:.3}; {structural_claim}",
        neighbor.name, neighbor.similarity.combined_score
    )
}

fn discovery_ranking_rules(requested: &[LensKind]) -> Vec<LensRankingRule> {
    requested
        .iter()
        .filter_map(|lens| {
            let ordering = match lens {
                LensKind::Influence => "transitive reach descending, direct dependents descending, name ascending",
                LensKind::Bridge => "articulation status descending, distinct directed region pairs descending, betweenness descending, name ascending",
                LensKind::Neighbors => "existing combined similarity descending, name ascending",
                LensKind::Proof | LensKind::Trust | LensKind::All => return None,
            };
            Some(LensRankingRule {
                lens: *lens,
                ordering: ordering.into(),
            })
        })
        .collect()
}

fn graph_region_algorithm() -> &'static str {
    "deterministic synchronous label propagation; region ids are assigned by sorted complete member lists"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        DeclarationSnapshot, ExprStats, ExpressionGraph, ExpressionNode, ExtractionSnapshot,
        SymbolSnapshot,
    };

    fn graph(dependencies: &[&str]) -> (ExpressionGraph, ExprStats) {
        if dependencies.is_empty() {
            return (
                ExpressionGraph {
                    root: 0,
                    nodes: vec![ExpressionNode {
                        id: 0,
                        kind: "sort".into(),
                        children: vec![],
                        name: None,
                        index: None,
                        value: Some("0".into()),
                        binder_info: None,
                        universe_levels: vec![],
                    }],
                },
                stats(1, 1, 0, 0, 1),
            );
        }
        let mut nodes = dependencies
            .iter()
            .enumerate()
            .map(|(id, dependency)| ExpressionNode {
                id,
                kind: "constant".into(),
                children: vec![],
                name: Some((*dependency).into()),
                index: None,
                value: None,
                binder_info: None,
                universe_levels: vec![],
            })
            .collect::<Vec<_>>();
        let mut root = 0;
        for constant in 1..dependencies.len() {
            let id = nodes.len();
            nodes.push(ExpressionNode {
                id,
                kind: "application".into(),
                children: vec![root, constant],
                name: None,
                index: None,
                value: None,
                binder_info: None,
                universe_levels: vec![],
            });
            root = id;
        }
        (
            ExpressionGraph { root, nodes },
            stats(
                dependencies.len() * 2 - 1,
                dependencies.len(),
                dependencies.len(),
                dependencies.len().saturating_sub(1),
                0,
            ),
        )
    }

    fn stats(
        nodes: usize,
        max_depth: usize,
        constants: usize,
        applications: usize,
        sorts: usize,
    ) -> ExprStats {
        ExprStats {
            nodes,
            max_depth,
            bound_variables: 0,
            free_variables: 0,
            metavariables: 0,
            sorts,
            constants,
            applications,
            lambdas: 0,
            foralls: 0,
            lets: 0,
            literals: 0,
            metadata: 0,
            projections: 0,
        }
    }

    fn declaration(name: &str, proof_dependencies: &[&str]) -> DeclarationSnapshot {
        let (type_graph, type_stats) = graph(&[]);
        let (value_graph, value_stats) = graph(proof_dependencies);
        DeclarationSnapshot {
            name: name.into(),
            kind: "theorem".into(),
            module_name: Some("Fixture".into()),
            is_internal: false,
            is_private: false,
            is_unsafe: false,
            is_partial: false,
            doc_string: None,
            source_range: None,
            level_parameters: vec![],
            r#type: format!("Statement {name}"),
            has_value: true,
            type_stats,
            value_stats: Some(value_stats),
            type_graph,
            value_graph: Some(value_graph),
            statement_dependencies: vec![],
            proof_dependencies: proof_dependencies
                .iter()
                .map(|name| (*name).into())
                .collect(),
            axioms: vec![],
            proof_steps: None,
        }
    }

    fn symbol(name: &str) -> SymbolSnapshot {
        SymbolSnapshot {
            name: name.into(),
            kind: "theorem".into(),
            module_name: Some("Fixture".into()),
            is_internal: false,
            is_private: false,
            is_unsafe: false,
            is_partial: false,
            is_class: false,
            is_instance: false,
            is_projection: false,
        }
    }

    fn fixture() -> AnalysisCorpus {
        let declarations = vec![
            declaration("A1", &["A2", "B"]),
            declaration("A2", &["A1", "A3"]),
            declaration("A3", &["A1", "A2"]),
            declaration("B", &["C1"]),
            declaration("C1", &["C2", "C3"]),
            declaration("C2", &["C1", "C3"]),
            declaration("C3", &["C1", "C2"]),
            declaration("Disconnected", &[]),
        ];
        let symbols = declarations
            .iter()
            .map(|declaration| symbol(&declaration.name))
            .collect();
        AnalysisCorpus::new(ExtractionSnapshot {
            schema_version: 4,
            lean_version: "fixture".into(),
            imported_module: "Fixture".into(),
            declarations,
            symbols,
        })
        .unwrap()
    }

    fn hidden_path_fixture() -> AnalysisCorpus {
        let mut hidden = declaration("Hidden", &["Foundation"]);
        hidden.is_internal = true;
        let declarations = vec![
            declaration("VisibleLater", &["Hidden"]),
            hidden,
            declaration("Foundation", &[]),
        ];
        let mut symbols = declarations
            .iter()
            .map(|declaration| symbol(&declaration.name))
            .collect::<Vec<_>>();
        symbols
            .iter_mut()
            .find(|symbol| symbol.name == "Hidden")
            .unwrap()
            .is_internal = true;
        AnalysisCorpus::new(ExtractionSnapshot {
            schema_version: 4,
            lean_version: "fixture".into(),
            imported_module: "Fixture".into(),
            declarations,
            symbols,
        })
        .unwrap()
    }

    #[test]
    fn influence_is_cycle_safe_and_ties_are_stable() {
        let report = fixture().discover_lenses(
            &[LensKind::Influence],
            &LensAnalysisConfig {
                limit: 20,
                ..LensAnalysisConfig::default()
            },
        );
        assert!(!report.influence.is_empty());
        assert!(
            report
                .influence
                .iter()
                .all(|entry| entry.reachable_dependents.len() <= 7)
        );
        let first = serde_json::to_string(&report).unwrap();
        let decoded: DiscoveryReport = serde_json::from_str(&first).unwrap();
        assert_eq!(decoded.influence.len(), report.influence.len());
        let second = serde_json::to_string(&fixture().discover_lenses(
            &[LensKind::Influence],
            &LensAnalysisConfig {
                limit: 20,
                ..LensAnalysisConfig::default()
            },
        ))
        .unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn bridge_witnesses_are_real_directed_dependency_paths() {
        let corpus = fixture();
        let report = corpus.discover_lenses(
            &[LensKind::Bridge],
            &LensAnalysisConfig {
                limit: 20,
                ..LensAnalysisConfig::default()
            },
        );
        let bridge = report
            .bridges
            .iter()
            .find(|evidence| evidence.name == "B")
            .expect("B connects the two dense graph regions");
        assert!(bridge.distinct_directed_region_pair_count > 0);
        for witness in &bridge.witness_paths {
            assert_eq!(witness.nodes.len(), 3);
            for edge in witness.nodes.windows(2) {
                let source = corpus.declaration_id(&edge[0]).unwrap();
                assert!(corpus.outgoing_edge_ids(source).iter().any(|&id| {
                    let dependency = &corpus.edges()[id];
                    dependency.layer == DependencyLayer::Proof && dependency.target == edge[1]
                }));
            }
        }
    }

    #[test]
    fn neighbors_expose_every_similarity_component_without_equivalence_claims() {
        let report = fixture()
            .inspect_lenses(
                "A1",
                &[LensKind::Neighbors],
                &LensAnalysisConfig {
                    limit: 2,
                    ..LensAnalysisConfig::default()
                },
            )
            .unwrap();
        let neighbors = &report.neighbors[0].neighbors;
        assert_eq!(neighbors.len(), 2);
        for neighbor in neighbors {
            assert!((0.0..=1.0).contains(&neighbor.similarity.dependency_jaccard));
            assert!((0.0..=1.0).contains(&neighbor.similarity.kind_histogram_cosine));
            assert!((0.0..=1.0).contains(&neighbor.similarity.size_ratio));
            assert!((0.0..=1.0).contains(&neighbor.similarity.depth_ratio));
            assert!(neighbor.exact_equal || neighbor.alpha_equivalent);
        }
    }

    #[test]
    fn explicit_both_keeps_statement_and_proof_evidence_separate() {
        let report = fixture()
            .inspect_lenses(
                "A1",
                &[LensKind::Influence],
                &LensAnalysisConfig {
                    layer_explicit: true,
                    filter: AnalysisFilter {
                        layer: LayerSelection::Both,
                        ..AnalysisFilter::default()
                    },
                    ..LensAnalysisConfig::default()
                },
            )
            .unwrap();
        assert_eq!(report.influence.len(), 2);
        assert_ne!(report.influence[0].layer, report.influence[1].layer);
    }

    #[test]
    fn discovery_limit_is_applied_after_stable_ranking() {
        let report = fixture().discover_lenses(
            &[LensKind::Influence, LensKind::Neighbors],
            &LensAnalysisConfig {
                limit: 1,
                ..LensAnalysisConfig::default()
            },
        );
        assert_eq!(report.influence.len(), 1);
        assert_eq!(report.neighbors.len(), 1);
        assert_eq!(report.neighbors[0].name, "A1");
    }

    #[test]
    fn hidden_generated_nodes_connect_paths_without_becoming_reported_endpoints() {
        let report = hidden_path_fixture()
            .inspect_lenses(
                "Foundation",
                &[LensKind::Influence],
                &LensAnalysisConfig::default(),
            )
            .unwrap();
        let influence = &report.influence[0];
        assert_eq!(influence.direct_dependent_count, 0);
        assert_eq!(influence.reachable_dependents, ["VisibleLater"]);
        assert_eq!(
            influence.representative_paths[0].nodes,
            ["VisibleLater", "Hidden", "Foundation"]
        );
        let included = hidden_path_fixture()
            .inspect_lenses(
                "Foundation",
                &[LensKind::Influence],
                &LensAnalysisConfig {
                    filter: AnalysisFilter {
                        include_generated: true,
                        ..AnalysisFilter::default()
                    },
                    ..LensAnalysisConfig::default()
                },
            )
            .unwrap();
        assert!(
            included.influence[0]
                .reachable_dependents
                .contains(&"Hidden".into())
        );
    }

    #[test]
    fn all_expands_in_public_reports_and_unknown_theorems_are_errors() {
        let corpus = fixture();
        let report = corpus.discover_lenses(&[LensKind::All], &LensAnalysisConfig::default());
        assert_eq!(
            report.requested_lenses,
            [LensKind::Influence, LensKind::Bridge, LensKind::Neighbors]
        );
        assert!(
            corpus
                .inspect_lenses(
                    "Missing",
                    &[LensKind::Trust],
                    &LensAnalysisConfig::default()
                )
                .is_err()
        );
    }
}
