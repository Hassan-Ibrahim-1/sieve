//! Bounded, statement-first graph payloads for the browser UI.

use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::analysis::corpus::AnalysisCorpus;
use crate::analysis::dependency::{DependencyConfig, DependencyPath};
use crate::analysis::filters::{AnalysisFilter, DependencyLayer};
use crate::analysis::proof_steps::build_proof_outline;
use crate::analysis::structure::StructuralMode;

const MIN_SIMILARITY: f64 = 0.55;
const FAMILY_SIMILARITY: f64 = 0.72;
const MAX_FAMILY_SIZE: usize = 100;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiBootstrap {
    pub schema_version: usize,
    pub corpus_fingerprint: String,
    pub declaration_count: usize,
    pub theorem_count: usize,
    pub graph_modes: Vec<&'static str>,
    pub node_size_metrics: Vec<&'static str>,
    pub default_filters: UiFilters,
    pub proof_steps_available: bool,
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
    pub mode: String,
    pub level: String,
    pub scope: Option<String>,
    pub metric: String,
    pub limit: usize,
    pub threshold: f64,
    pub depth: usize,
    pub search: Option<String>,
    pub filters: UiFilters,
}

impl Default for GraphRequest {
    fn default() -> Self {
        Self {
            mode: "similarity".into(),
            level: "corpus".into(),
            scope: None,
            metric: "recommended".into(),
            limit: 80,
            threshold: 0.65,
            depth: 2,
            search: None,
            filters: UiFilters::default(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Breadcrumb {
    pub level: String,
    pub id: Option<String>,
    pub label: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphScope {
    pub level: String,
    pub id: Option<String>,
    pub breadcrumbs: Vec<Breadcrumb>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphTotals {
    pub matching: usize,
    pub returned: usize,
    pub truncated: bool,
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
    pub family_id: Option<String>,
    pub member_count: usize,
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
    pub similarity: Option<f64>,
    pub witness_available: bool,
    pub collapsed_step_count: Option<usize>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphResponse {
    pub schema_version: usize,
    pub corpus_fingerprint: String,
    pub scope: GraphScope,
    pub totals: GraphTotals,
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProofResponse {
    pub declaration: String,
    pub complete: bool,
    pub truncation_reason: Option<String>,
    pub conclusion_step: Option<usize>,
    pub graph: GraphResponse,
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
struct Family {
    id: String,
    members: Vec<usize>,
    representative: usize,
}

#[derive(Clone, Debug)]
struct SimilarityEdge {
    left: usize,
    right: usize,
    score: f64,
}

/// Immutable index created once when `sieve serve` starts.
pub struct UiIndex {
    fingerprint: String,
    families: Vec<Family>,
    family_by_declaration: Vec<usize>,
    similarities: Vec<SimilarityEdge>,
    direct_dependents: Vec<usize>,
    reachable_dependents: Vec<usize>,
    bridge_evidence: Vec<f64>,
}

impl UiIndex {
    pub fn build(corpus: &AnalysisCorpus) -> Self {
        let fingerprint = corpus_fingerprint(corpus);
        let (direct_dependents, reachable_dependents, bridge_evidence) = dependency_metrics(corpus);
        let (mut components, similarities) = similarity_components(corpus);

        // Large components are recursively represented by deterministic chunks. Small
        // singleton groups with the same coarse shape are collected so the overview
        // remains bounded instead of becoming an indiscriminate declaration cloud.
        components = normalize_components(corpus, components);
        let mut families = components
            .into_iter()
            .map(|mut members| {
                members.sort_by(|&a, &b| {
                    corpus.declarations()[a]
                        .name
                        .cmp(&corpus.declarations()[b].name)
                });
                let representative = representative(corpus, &members, &similarities);
                let mut hash = StableHash::new();
                for &member in &members {
                    hash.feed(corpus.declarations()[member].name.as_bytes());
                }
                Family {
                    id: format!("family-{:016x}", hash.finish()),
                    members,
                    representative,
                }
            })
            .collect::<Vec<_>>();
        families.sort_by(|a, b| a.id.cmp(&b.id));
        let mut family_by_declaration = vec![0; corpus.declarations().len()];
        for (family_id, family) in families.iter().enumerate() {
            for &member in &family.members {
                family_by_declaration[member] = family_id;
            }
        }
        Self {
            fingerprint,
            families,
            family_by_declaration,
            similarities,
            direct_dependents,
            reachable_dependents,
            bridge_evidence,
        }
    }

    pub fn bootstrap(&self, corpus: &AnalysisCorpus) -> UiBootstrap {
        let default = AnalysisFilter::default();
        let summary = corpus.summary(&default);
        UiBootstrap {
            schema_version: 1,
            corpus_fingerprint: self.fingerprint.clone(),
            declaration_count: summary.declaration_count,
            theorem_count: summary.theorem_count,
            graph_modes: vec!["similarity", "influence", "connections", "proof"],
            node_size_metrics: vec![
                "recommended",
                "familySize",
                "directDependents",
                "reachableDependents",
                "bridgeEvidence",
                "proofSize",
            ],
            default_filters: UiFilters::default(),
            proof_steps_available: corpus
                .declarations()
                .iter()
                .any(|declaration| declaration.proof_steps.is_some()),
        }
    }

    pub fn graph(&self, corpus: &AnalysisCorpus, request: &GraphRequest) -> Result<GraphResponse> {
        if !matches!(
            request.mode.as_str(),
            "similarity" | "influence" | "connections"
        ) {
            bail!("unknown graph mode {}", request.mode);
        }
        let limit = request.limit.clamp(20, 160);
        match request.level.as_str() {
            "corpus" => Ok(self.corpus_graph(corpus, request, limit)),
            "family" => {
                let scope = request
                    .scope
                    .as_deref()
                    .context("family level requires scope")?;
                self.family_graph(corpus, request, scope, limit)
            }
            "neighborhood" => {
                let scope = request
                    .scope
                    .as_deref()
                    .context("neighborhood requires scope")?;
                self.neighborhood_graph(corpus, request, scope, limit)
            }
            other => bail!("unknown graph level {other}"),
        }
    }

    fn corpus_graph(
        &self,
        corpus: &AnalysisCorpus,
        request: &GraphRequest,
        limit: usize,
    ) -> GraphResponse {
        let mut candidates = self
            .families
            .iter()
            .enumerate()
            .filter_map(|(family_index, family)| {
                let visible = family
                    .members
                    .iter()
                    .copied()
                    .filter(|&id| declaration_visible(corpus, id, &request.filters))
                    .collect::<Vec<_>>();
                if visible.is_empty()
                    || !matches_search(corpus, &visible, request.search.as_deref())
                {
                    return None;
                }
                Some((family_index, visible))
            })
            .collect::<Vec<_>>();
        candidates.sort_by(|(left, lm), (right, rm)| {
            self.family_metric(*right, rm, &request.metric, &request.mode)
                .total_cmp(&self.family_metric(*left, lm, &request.metric, &request.mode))
                .then_with(|| self.families[*left].id.cmp(&self.families[*right].id))
        });
        let matching = candidates.iter().map(|(_, members)| members.len()).sum();
        let mut selected = Vec::with_capacity(limit);
        for (_, mut members) in candidates {
            members.sort_by(|&left, &right| {
                self.declaration_metric(right, &request.metric, &request.mode)
                    .total_cmp(&self.declaration_metric(left, &request.metric, &request.mode))
                    .then_with(|| {
                        corpus.declarations()[left]
                            .name
                            .cmp(&corpus.declarations()[right].name)
                    })
            });
            selected.extend(members.into_iter().take(limit - selected.len()));
            if selected.len() == limit {
                break;
            }
        }
        let active = selected.iter().copied().collect::<BTreeSet<_>>();
        let nodes = selected
            .into_iter()
            .map(|id| self.declaration_node(corpus, id))
            .collect::<Vec<_>>();
        let edges = self.declaration_edges(corpus, request, &active);
        response(
            &self.fingerprint,
            GraphScope {
                level: "corpus".into(),
                id: None,
                breadcrumbs: vec![Breadcrumb {
                    level: "corpus".into(),
                    id: None,
                    label: "Corpus".into(),
                }],
            },
            matching,
            nodes,
            edges,
        )
    }

    fn family_graph(
        &self,
        corpus: &AnalysisCorpus,
        request: &GraphRequest,
        scope: &str,
        limit: usize,
    ) -> Result<GraphResponse> {
        let family_index = self
            .families
            .iter()
            .position(|family| family.id == scope)
            .with_context(|| format!("unknown family {scope}"))?;
        let family = &self.families[family_index];
        let mut members = family
            .members
            .iter()
            .copied()
            .filter(|&id| declaration_visible(corpus, id, &request.filters))
            .filter(|&id| matches_search(corpus, &[id], request.search.as_deref()))
            .collect::<Vec<_>>();
        members.sort_by(|&a, &b| {
            self.declaration_metric(b, &request.metric, &request.mode)
                .total_cmp(&self.declaration_metric(a, &request.metric, &request.mode))
                .then_with(|| {
                    corpus.declarations()[a]
                        .name
                        .cmp(&corpus.declarations()[b].name)
                })
        });
        let matching = members.len();
        members.truncate(limit.min(MAX_FAMILY_SIZE));
        let active = members.iter().copied().collect::<BTreeSet<_>>();
        let nodes = members
            .iter()
            .map(|&id| self.declaration_node(corpus, id))
            .collect::<Vec<_>>();
        let edges = self.declaration_edges(corpus, request, &active);
        let representative = &corpus.declarations()[family.representative];
        Ok(response(
            &self.fingerprint,
            GraphScope {
                level: "family".into(),
                id: Some(scope.into()),
                breadcrumbs: vec![
                    Breadcrumb {
                        level: "corpus".into(),
                        id: None,
                        label: "Corpus".into(),
                    },
                    Breadcrumb {
                        level: "family".into(),
                        id: Some(scope.into()),
                        label: concise(&representative.r#type, 46),
                    },
                ],
            },
            matching,
            nodes,
            edges,
        ))
    }

    fn neighborhood_graph(
        &self,
        corpus: &AnalysisCorpus,
        request: &GraphRequest,
        scope: &str,
        limit: usize,
    ) -> Result<GraphResponse> {
        let name = scope.strip_prefix("decl:").unwrap_or(scope);
        let start = corpus
            .declaration_id(name)
            .with_context(|| format!("unknown declaration {name}"))?;
        let mut seen = BTreeSet::from([start]);
        let mut queue = VecDeque::from([(start, 0usize)]);
        while let Some((current, depth)) = queue.pop_front() {
            if depth >= request.depth.clamp(1, 5) || seen.len() >= limit.saturating_mul(3) {
                continue;
            }
            for next in dependency_neighbors(corpus, current)
                .into_iter()
                .chain(self.similarity_neighbors(current, request.threshold))
            {
                if declaration_visible(corpus, next, &request.filters) && seen.insert(next) {
                    queue.push_back((next, depth + 1));
                }
            }
        }
        let mut members = seen
            .into_iter()
            .filter(|&id| matches_search(corpus, &[id], request.search.as_deref()))
            .collect::<Vec<_>>();
        members.sort_by(|&a, &b| {
            (b == start)
                .cmp(&(a == start))
                .then_with(|| {
                    self.declaration_metric(b, &request.metric, &request.mode)
                        .total_cmp(&self.declaration_metric(a, &request.metric, &request.mode))
                })
                .then_with(|| {
                    corpus.declarations()[a]
                        .name
                        .cmp(&corpus.declarations()[b].name)
                })
        });
        let matching = members.len();
        members.truncate(limit);
        if !members.contains(&start) {
            members.insert(0, start);
            members.truncate(limit);
        }
        let active = members.iter().copied().collect::<BTreeSet<_>>();
        let nodes = members
            .iter()
            .map(|&id| self.declaration_node(corpus, id))
            .collect::<Vec<_>>();
        let edges = self.declaration_edges(corpus, request, &active);
        let family = &self.families[self.family_by_declaration[start]];
        let declaration = &corpus.declarations()[start];
        Ok(response(
            &self.fingerprint,
            GraphScope {
                level: "neighborhood".into(),
                id: Some(format!("decl:{}", declaration.name)),
                breadcrumbs: vec![
                    Breadcrumb {
                        level: "corpus".into(),
                        id: None,
                        label: "Corpus".into(),
                    },
                    Breadcrumb {
                        level: "family".into(),
                        id: Some(family.id.clone()),
                        label: concise(&corpus.declarations()[family.representative].r#type, 38),
                    },
                    Breadcrumb {
                        level: "neighborhood".into(),
                        id: Some(format!("decl:{}", declaration.name)),
                        label: concise(&declaration.r#type, 38),
                    },
                ],
            },
            matching,
            nodes,
            edges,
        ))
    }

    fn declaration_node(&self, corpus: &AnalysisCorpus, id: usize) -> GraphNode {
        let declaration = &corpus.declarations()[id];
        let family = &self.families[self.family_by_declaration[id]];
        let proof_size = declaration
            .value_stats
            .as_ref()
            .map(|stats| stats.nodes)
            .unwrap_or(0);
        let mut metrics = BTreeMap::from([
            ("familySize".into(), family.members.len() as f64),
            ("directDependents".into(), self.direct_dependents[id] as f64),
            (
                "reachableDependents".into(),
                self.reachable_dependents[id] as f64,
            ),
            ("bridgeEvidence".into(), self.bridge_evidence[id]),
            ("proofSize".into(), proof_size as f64),
        ]);
        metrics.insert(
            "recommended".into(),
            (1 + self.reachable_dependents[id]) as f64,
        );
        let technical = declaration.is_hidden_by_default() || !is_math_kind(&declaration.kind);
        let mut actions = vec!["neighborhood".into(), "pin".into()];
        if declaration.proof_steps.is_some() {
            actions.push("proof".into());
        }
        GraphNode {
            id: format!("decl:{}", declaration.name),
            node_kind: "declaration".into(),
            declaration_kind: Some(declaration.kind.clone()),
            statement: declaration.r#type.clone(),
            display_statement: concise(&declaration.r#type, 70),
            lean_name: Some(declaration.name.clone()),
            family_id: Some(family.id.clone()),
            member_count: 1,
            metrics,
            generated: declaration.is_generated(),
            technical,
            position: stable_position(&declaration.name),
            actions,
        }
    }

    fn family_metric(&self, family: usize, members: &[usize], metric: &str, mode: &str) -> f64 {
        match metric {
            "familySize" => members.len() as f64,
            "directDependents" => members
                .iter()
                .map(|&id| self.direct_dependents[id])
                .sum::<usize>() as f64,
            "reachableDependents" => members
                .iter()
                .map(|&id| self.reachable_dependents[id])
                .max()
                .unwrap_or(0) as f64,
            "bridgeEvidence" => members.iter().map(|&id| self.bridge_evidence[id]).sum(),
            "proofSize" => 0.0,
            _ => match mode {
                "influence" => members
                    .iter()
                    .map(|&id| self.reachable_dependents[id])
                    .max()
                    .unwrap_or(0) as f64,
                "connections" => members.iter().map(|&id| self.bridge_evidence[id]).sum(),
                _ => self.families[family].members.len() as f64,
            },
        }
    }

    fn declaration_metric(&self, id: usize, metric: &str, mode: &str) -> f64 {
        match metric {
            "familySize" => self.families[self.family_by_declaration[id]].members.len() as f64,
            "directDependents" => self.direct_dependents[id] as f64,
            "reachableDependents" => self.reachable_dependents[id] as f64,
            "bridgeEvidence" => self.bridge_evidence[id],
            _ if mode == "connections" => self.bridge_evidence[id],
            _ => self.reachable_dependents[id] as f64,
        }
    }

    fn declaration_edges(
        &self,
        corpus: &AnalysisCorpus,
        request: &GraphRequest,
        active: &BTreeSet<usize>,
    ) -> Vec<GraphEdge> {
        if request.mode == "similarity" {
            return self
                .similarities
                .iter()
                .filter(|edge| edge.score >= request.threshold.max(MIN_SIMILARITY))
                .filter(|edge| active.contains(&edge.left) && active.contains(&edge.right))
                .map(|edge| GraphEdge {
                    id: format!("similarity-{}-{}", edge.left, edge.right),
                    source: format!("decl:{}", corpus.declarations()[edge.left].name),
                    target: format!("decl:{}", corpus.declarations()[edge.right].name),
                    kind: "similarity".into(),
                    directed: false,
                    weight: edge.score,
                    aggregate_count: 1,
                    similarity: Some(edge.score),
                    witness_available: false,
                    collapsed_step_count: None,
                })
                .collect();
        }
        let mut aggregated = BTreeMap::<(usize, usize), usize>::new();
        for edge in corpus.edges().iter().filter(|edge| edge.target_in_corpus) {
            let Some(prerequisite) = corpus.declaration_id(&edge.target) else {
                continue;
            };
            if active.contains(&prerequisite) && active.contains(&edge.source) {
                *aggregated.entry((prerequisite, edge.source)).or_default() += edge.occurrences;
            }
        }
        let edges = aggregated
            .into_iter()
            .map(|((prerequisite, result), occurrences)| GraphEdge {
                id: format!("dependency-{prerequisite}-{result}"),
                source: format!("decl:{}", corpus.declarations()[prerequisite].name),
                target: format!("decl:{}", corpus.declarations()[result].name),
                kind: "dependency".into(),
                directed: true,
                weight: occurrences as f64,
                aggregate_count: occurrences,
                similarity: None,
                witness_available: request.mode == "connections",
                collapsed_step_count: None,
            })
            .collect::<Vec<_>>();
        if request.mode == "influence" {
            transitive_reduction(edges)
        } else {
            edges
        }
    }

    fn similarity_neighbors(&self, id: usize, threshold: f64) -> impl Iterator<Item = usize> + '_ {
        self.similarities
            .iter()
            .filter(move |edge| {
                edge.score >= threshold.max(MIN_SIMILARITY) && (edge.left == id || edge.right == id)
            })
            .map(move |edge| {
                if edge.left == id {
                    edge.right
                } else {
                    edge.left
                }
            })
    }

    pub fn proof(
        &self,
        corpus: &AnalysisCorpus,
        declaration_name: &str,
        detail: &str,
        path: Option<&str>,
    ) -> Result<ProofResponse> {
        let declaration = corpus
            .declaration(declaration_name)
            .with_context(|| format!("unknown declaration {declaration_name}"))?;
        let extraction = declaration.proof_steps.as_ref().with_context(|| {
            format!("proof-step extraction is unavailable for {declaration_name}")
        })?;
        let outline = build_proof_outline(extraction)?;
        let mut included = outline
            .retained_nodes
            .iter()
            .map(|node| node.raw_step_id)
            .collect::<BTreeSet<_>>();
        if detail == "raw" {
            if let Some(edge_id) = path {
                let condensed = outline
                    .condensed_edges
                    .iter()
                    .find(|edge| proof_edge_id(edge.source, edge.target) == edge_id)
                    .with_context(|| format!("unknown condensed proof path {edge_id}"))?;
                included.extend(condensed.raw_step_path.iter().copied());
            } else {
                included.extend(extraction.steps.iter().take(60).map(|step| step.id));
                if let Some(conclusion) = extraction.conclusion_step {
                    included.insert(conclusion);
                }
            }
        }
        let nodes = included
            .iter()
            .map(|&id| {
                let step = &extraction.steps[id];
                let retained = outline
                    .retained_nodes
                    .iter()
                    .any(|node| node.raw_step_id == id);
                let mut metrics = BTreeMap::from([
                    (
                        "proofSize".into(),
                        (step.named_references.len() + step.hypothesis_references.len() + 1) as f64,
                    ),
                    (
                        "recommended".into(),
                        (step.named_references.len() + 1) as f64,
                    ),
                ]);
                metrics.insert("familySize".into(), 1.0);
                GraphNode {
                    id: format!("step:{id}"),
                    node_kind: if retained {
                        "proofStep".into()
                    } else {
                        "rawProofStep".into()
                    },
                    declaration_kind: Some(step.kind.clone()),
                    statement: step.proposition.clone(),
                    display_statement: concise(&step.proposition, 92),
                    lean_name: step.named_references.first().cloned(),
                    family_id: None,
                    member_count: 1,
                    metrics,
                    generated: false,
                    technical: !retained,
                    position: stable_position(&format!("{declaration_name}:{id}")),
                    actions: vec!["inspect".into()],
                }
            })
            .collect::<Vec<_>>();
        let edges = if detail == "raw" {
            outline
                .raw_edges
                .iter()
                .filter(|edge| included.contains(&edge.source) && included.contains(&edge.target))
                .map(|edge| GraphEdge {
                    id: format!("raw-{}-{}", edge.source, edge.target),
                    source: format!("step:{}", edge.source),
                    target: format!("step:{}", edge.target),
                    kind: "dependency".into(),
                    directed: true,
                    weight: 1.0,
                    aggregate_count: 1,
                    similarity: None,
                    witness_available: false,
                    collapsed_step_count: None,
                })
                .collect()
        } else {
            outline
                .condensed_edges
                .iter()
                .map(|edge| GraphEdge {
                    id: proof_edge_id(edge.source, edge.target),
                    source: format!("step:{}", edge.source),
                    target: format!("step:{}", edge.target),
                    kind: "condensedProofPath".into(),
                    directed: true,
                    weight: 1.0,
                    aggregate_count: 1,
                    similarity: None,
                    witness_available: false,
                    collapsed_step_count: Some(edge.raw_step_path.len().saturating_sub(2)),
                })
                .collect()
        };
        let matching = nodes.len();
        Ok(ProofResponse {
            declaration: declaration_name.into(),
            complete: outline.complete,
            truncation_reason: outline.truncation_reason,
            conclusion_step: outline.conclusion_step,
            graph: response(
                &self.fingerprint,
                GraphScope {
                    level: "proof".into(),
                    id: Some(format!("decl:{declaration_name}")),
                    breadcrumbs: vec![
                        Breadcrumb {
                            level: "corpus".into(),
                            id: None,
                            label: "Corpus".into(),
                        },
                        Breadcrumb {
                            level: "proof".into(),
                            id: Some(format!("decl:{declaration_name}")),
                            label: concise(&declaration.r#type, 42),
                        },
                    ],
                },
                matching,
                nodes,
                edges,
            ),
        })
    }

    pub fn witnesses(
        &self,
        corpus: &AnalysisCorpus,
        source: &str,
        target: &str,
        limit: usize,
    ) -> Result<WitnessResponse> {
        let (source, target) = if source.starts_with("family-") && target.starts_with("family-") {
            let source_family = self
                .families
                .iter()
                .position(|family| family.id == source)
                .with_context(|| format!("unknown family {source}"))?;
            let target_family = self
                .families
                .iter()
                .position(|family| family.id == target)
                .with_context(|| format!("unknown family {target}"))?;
            corpus
                .edges()
                .iter()
                .filter(|edge| edge.target_in_corpus)
                .find_map(|edge| {
                    let prerequisite = corpus.declaration_id(&edge.target)?;
                    (self.family_by_declaration[prerequisite] == source_family
                        && self.family_by_declaration[edge.source] == target_family)
                        .then(|| {
                            (
                                corpus.declarations()[prerequisite].name.clone(),
                                corpus.declarations()[edge.source].name.clone(),
                            )
                        })
                })
                .with_context(|| format!("no dependency witness connects {source} and {target}"))?
        } else {
            (
                source.strip_prefix("decl:").unwrap_or(source).to_owned(),
                target.strip_prefix("decl:").unwrap_or(target).to_owned(),
            )
        };
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
                .then_with(|| {
                    self.reachable_dependents[*right].cmp(&self.reachable_dependents[*left])
                })
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

fn proof_edge_id(source: usize, target: usize) -> String {
    format!("proof-path-{source}-{target}")
}

fn response(
    fingerprint: &str,
    scope: GraphScope,
    matching: usize,
    nodes: Vec<GraphNode>,
    edges: Vec<GraphEdge>,
) -> GraphResponse {
    let ids = nodes
        .iter()
        .map(|node| node.id.as_str())
        .collect::<BTreeSet<_>>();
    let edges = edges
        .into_iter()
        .filter(|edge| ids.contains(edge.source.as_str()) && ids.contains(edge.target.as_str()))
        .collect::<Vec<_>>();
    GraphResponse {
        schema_version: 1,
        corpus_fingerprint: fingerprint.into(),
        scope,
        totals: GraphTotals {
            matching,
            returned: nodes.len(),
            truncated: matching > nodes.len(),
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

fn matches_search(corpus: &AnalysisCorpus, members: &[usize], search: Option<&str>) -> bool {
    let Some(search) = search.map(str::trim).filter(|value| !value.is_empty()) else {
        return true;
    };
    let query = search.to_lowercase();
    members.iter().any(|&id| {
        let declaration = &corpus.declarations()[id];
        declaration.name.to_lowercase().contains(&query)
            || declaration.r#type.to_lowercase().contains(&query)
    })
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

fn dependency_neighbors(corpus: &AnalysisCorpus, id: usize) -> Vec<usize> {
    let mut result = BTreeSet::new();
    for &edge_id in corpus.outgoing_edge_ids(id) {
        let edge = &corpus.edges()[edge_id];
        if let Some(target) = corpus.declaration_id(&edge.target) {
            result.insert(target);
        }
    }
    let name = &corpus.declarations()[id].name;
    for &edge_id in corpus.incoming_edge_ids(name) {
        result.insert(corpus.edges()[edge_id].source);
    }
    result.into_iter().collect()
}

fn dependency_metrics(corpus: &AnalysisCorpus) -> (Vec<usize>, Vec<usize>, Vec<f64>) {
    let count = corpus.declarations().len();
    let mut dependents = vec![BTreeSet::new(); count];
    let mut prerequisites = vec![BTreeSet::new(); count];
    for edge in corpus.edges().iter().filter(|edge| edge.target_in_corpus) {
        if let Some(target) = corpus.declaration_id(&edge.target) {
            dependents[target].insert(edge.source);
            prerequisites[edge.source].insert(target);
        }
    }
    let direct = dependents.iter().map(BTreeSet::len).collect::<Vec<_>>();
    let mut reachable = vec![0; count];
    for (start, reachable_count) in reachable.iter_mut().enumerate() {
        let mut seen = BTreeSet::from([start]);
        let mut queue = VecDeque::from([start]);
        while let Some(current) = queue.pop_front() {
            for &next in &dependents[current] {
                if seen.insert(next) {
                    queue.push_back(next);
                }
            }
        }
        *reachable_count = seen.len().saturating_sub(1);
    }
    let bridge = (0..count)
        .map(|id| {
            let incoming = dependents[id].len() as f64;
            let outgoing = prerequisites[id].len() as f64;
            incoming * outgoing
                + if incoming > 0.0 && outgoing > 0.0 {
                    1.0
                } else {
                    0.0
                }
        })
        .collect();
    (direct, reachable, bridge)
}

fn similarity_components(corpus: &AnalysisCorpus) -> (Vec<Vec<usize>>, Vec<SimilarityEdge>) {
    let count = corpus.declarations().len();
    let mut union = UnionFind::new(count);
    let mut buckets = BTreeMap::<(String, String, usize), Vec<usize>>::new();
    let mut exact = BTreeMap::<u64, Vec<usize>>::new();
    for (id, declaration) in corpus.declarations().iter().enumerate() {
        let root = &declaration.type_graph.nodes[declaration.type_graph.root];
        let size_bucket = usize::BITS as usize
            - declaration.type_graph.nodes.len().max(1).leading_zeros() as usize;
        buckets
            .entry((declaration.kind.clone(), root.kind.clone(), size_bucket))
            .or_default()
            .push(id);
        exact
            .entry(
                declaration
                    .type_graph
                    .root_fingerprint(StructuralMode::AlphaEquivalent),
            )
            .or_default()
            .push(id);
    }
    for group in exact.values() {
        for pair in group.windows(2) {
            union.union(pair[0], pair[1]);
        }
    }
    let mut similarities = Vec::new();
    for members in buckets.values_mut() {
        members.sort_by(|&a, &b| {
            corpus.declarations()[a]
                .name
                .cmp(&corpus.declarations()[b].name)
        });
        for left_index in 0..members.len() {
            // A deterministic sparse candidate window avoids quadratic preprocessing.
            for right_index in left_index + 1..members.len().min(left_index + 21) {
                let left = members[left_index];
                let right = members[right_index];
                let Some(comparison) = corpus.compare_declarations(
                    &corpus.declarations()[left].name,
                    &corpus.declarations()[right].name,
                    DependencyLayer::Statement,
                ) else {
                    continue;
                };
                let score = comparison.similarity.combined_score;
                if score >= MIN_SIMILARITY {
                    similarities.push(SimilarityEdge { left, right, score });
                }
                if comparison.alpha_equivalent || score >= FAMILY_SIMILARITY {
                    union.union(left, right);
                }
            }
        }
    }
    let mut groups = BTreeMap::<usize, Vec<usize>>::new();
    for id in 0..count {
        groups.entry(union.find(id)).or_default().push(id);
    }
    (groups.into_values().collect(), similarities)
}

fn normalize_components(corpus: &AnalysisCorpus, components: Vec<Vec<usize>>) -> Vec<Vec<usize>> {
    let mut result = Vec::new();
    let mut singleton_buckets = BTreeMap::<(String, String, usize), Vec<usize>>::new();
    for mut component in components {
        component.sort_by(|&a, &b| {
            corpus.declarations()[a]
                .name
                .cmp(&corpus.declarations()[b].name)
        });
        if component.len() == 1 {
            let id = component[0];
            let declaration = &corpus.declarations()[id];
            let root = &declaration.type_graph.nodes[declaration.type_graph.root];
            let size = declaration.type_graph.nodes.len() / 24;
            singleton_buckets
                .entry((declaration.kind.clone(), root.kind.clone(), size))
                .or_default()
                .push(id);
        } else {
            result.extend(component.chunks(MAX_FAMILY_SIZE).map(<[usize]>::to_vec));
        }
    }
    for mut members in singleton_buckets.into_values() {
        members.sort_by(|&a, &b| {
            corpus.declarations()[a]
                .name
                .cmp(&corpus.declarations()[b].name)
        });
        result.extend(members.chunks(MAX_FAMILY_SIZE).map(<[usize]>::to_vec));
    }
    result
}

fn representative(
    corpus: &AnalysisCorpus,
    members: &[usize],
    similarities: &[SimilarityEdge],
) -> usize {
    let member_set = members.iter().copied().collect::<BTreeSet<_>>();
    members
        .iter()
        .copied()
        .max_by(|&a, &b| {
            let score = |id| {
                similarities
                    .iter()
                    .filter(|edge| {
                        member_set.contains(&edge.left) && member_set.contains(&edge.right)
                    })
                    .filter(|edge| edge.left == id || edge.right == id)
                    .map(|edge| edge.score)
                    .sum::<f64>()
            };
            score(a)
                .total_cmp(&score(b))
                .then_with(|| {
                    corpus.declarations()[b]
                        .r#type
                        .len()
                        .cmp(&corpus.declarations()[a].r#type.len())
                })
                .then_with(|| {
                    corpus.declarations()[b]
                        .name
                        .cmp(&corpus.declarations()[a].name)
                })
        })
        .expect("family is nonempty")
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

fn transitive_reduction(edges: Vec<GraphEdge>) -> Vec<GraphEdge> {
    let adjacency = edges.iter().enumerate().fold(
        HashMap::<&str, Vec<(usize, &str)>>::new(),
        |mut map, (index, edge)| {
            map.entry(&edge.source)
                .or_default()
                .push((index, &edge.target));
            map
        },
    );
    edges
        .iter()
        .enumerate()
        .filter(|(skip, edge)| {
            let mut seen = BTreeSet::from([edge.source.as_str()]);
            let mut queue = VecDeque::from([edge.source.as_str()]);
            while let Some(current) = queue.pop_front() {
                for &(index, next) in adjacency.get(current).into_iter().flatten() {
                    if index == *skip {
                        continue;
                    }
                    if next == edge.target {
                        return false;
                    }
                    if seen.insert(next) {
                        queue.push_back(next);
                    }
                }
            }
            true
        })
        .map(|(index, _)| edges[index].clone())
        .collect()
}

struct UnionFind {
    parents: Vec<usize>,
}
impl UnionFind {
    fn new(size: usize) -> Self {
        Self {
            parents: (0..size).collect(),
        }
    }
    fn find(&mut self, value: usize) -> usize {
        if self.parents[value] != value {
            self.parents[value] = self.find(self.parents[value]);
        }
        self.parents[value]
    }
    fn union(&mut self, left: usize, right: usize) {
        let left = self.find(left);
        let right = self.find(right);
        if left != right {
            let (small, large) = if left < right {
                (left, right)
            } else {
                (right, left)
            };
            self.parents[large] = small;
        }
    }
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
        SymbolSnapshot,
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

    #[test]
    fn concise_statements_are_bounded() {
        assert_eq!(concise("  alpha   beta  ", 20), "alpha beta");
        assert_eq!(concise("abcdefghijk", 6), "abcde…");
    }

    #[test]
    fn transitive_reduction_preserves_the_short_route() {
        let edge = |id: &str, source: &str, target: &str| GraphEdge {
            id: id.into(),
            source: source.into(),
            target: target.into(),
            kind: "dependency".into(),
            directed: true,
            weight: 1.0,
            aggregate_count: 1,
            similarity: None,
            witness_available: false,
            collapsed_step_count: None,
        };
        let reduced = transitive_reduction(vec![
            edge("ab", "a", "b"),
            edge("bc", "b", "c"),
            edge("ac", "a", "c"),
        ]);
        assert_eq!(
            reduced
                .iter()
                .map(|edge| edge.id.as_str())
                .collect::<Vec<_>>(),
            vec!["ab", "bc"]
        );
    }

    #[test]
    fn stable_positions_are_repeatable() {
        let first = stable_position("a theorem");
        let second = stable_position("a theorem");
        assert_eq!((first.x, first.y), (second.x, second.y));
    }

    #[test]
    fn index_membership_representative_and_fingerprint_are_deterministic() {
        let corpus = fixture(30);
        let first = UiIndex::build(&corpus);
        let second = UiIndex::build(&corpus);
        assert_eq!(first.fingerprint, second.fingerprint);
        assert_eq!(first.family_by_declaration, second.family_by_declaration);
        assert_eq!(first.families.len(), 1);
        assert_eq!(
            first.families[0].representative,
            second.families[0].representative
        );
        assert!(
            first.families[0]
                .members
                .contains(&first.families[0].representative)
        );
    }

    #[test]
    fn oversized_families_are_subdivided_deterministically() {
        let corpus = fixture(205);
        let index = UiIndex::build(&corpus);
        assert_eq!(index.families.len(), 3);
        assert!(
            index
                .families
                .iter()
                .all(|family| family.members.len() <= MAX_FAMILY_SIZE)
        );
    }

    #[test]
    fn graph_limits_filters_and_edges_are_consistent() {
        let corpus = fixture(30);
        let index = UiIndex::build(&corpus);
        let family = index.families[0].id.clone();
        let graph = index
            .graph(
                &corpus,
                &GraphRequest {
                    level: "family".into(),
                    scope: Some(family),
                    limit: 20,
                    ..GraphRequest::default()
                },
            )
            .unwrap();
        assert_eq!(graph.totals.matching, 24);
        assert_eq!(graph.totals.returned, 20);
        assert!(graph.totals.truncated);
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
    fn corpus_graph_keeps_family_members_as_distinct_nodes() {
        let corpus = fixture(30);
        let index = UiIndex::build(&corpus);
        let graph = index.graph(&corpus, &GraphRequest::default()).unwrap();

        assert_eq!(graph.totals.matching, 24);
        assert_eq!(graph.nodes.len(), 24);
        assert!(graph.nodes.iter().all(|node| {
            node.node_kind == "declaration"
                && node.member_count == 1
                && node.family_id.as_deref() == Some(index.families[0].id.as_str())
        }));
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
    fn higher_similarity_threshold_never_adds_edges() {
        let corpus = fixture(10);
        let index = UiIndex::build(&corpus);
        let family = index.families[0].id.clone();
        let graph_at = |threshold| {
            index
                .graph(
                    &corpus,
                    &GraphRequest {
                        level: "family".into(),
                        scope: Some(family.clone()),
                        threshold,
                        ..GraphRequest::default()
                    },
                )
                .unwrap()
        };
        assert!(graph_at(0.99).edges.len() <= graph_at(MIN_SIMILARITY).edges.len());
    }
}
