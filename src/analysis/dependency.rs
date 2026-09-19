use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};

use serde::{Deserialize, Serialize};

use crate::analysis::corpus::{AnalysisCorpus, DependencyEdge};
use crate::analysis::filters::{AnalysisFilter, DependencyLayer};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyConfig {
    pub filter: AnalysisFilter,
    pub weighted: bool,
    pub max_depth: usize,
    pub limit: usize,
}

impl Default for DependencyConfig {
    fn default() -> Self {
        Self {
            filter: AnalysisFilter::default(),
            weighted: false,
            max_depth: 32,
            limit: 1_000,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyRecord {
    pub source: String,
    pub target: String,
    pub layer: DependencyLayer,
    pub occurrences: usize,
    pub target_in_corpus: bool,
    pub source_module: Option<String>,
    pub source_kind: String,
    pub target_module: Option<String>,
    pub target_kind: String,
    pub target_is_class: bool,
    pub target_is_instance: bool,
    pub target_is_projection: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencySummary {
    pub declaration: String,
    pub direction: String,
    pub config: DependencyConfig,
    pub truncated: bool,
    pub occurrences_by_module: BTreeMap<String, usize>,
    pub occurrences_by_kind: BTreeMap<String, usize>,
    pub edges: Vec<DependencyRecord>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TraversalNode {
    pub name: String,
    pub depth: usize,
    pub path_weight: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TraversalResult {
    pub start: String,
    pub direction: String,
    pub config: DependencyConfig,
    pub truncated: bool,
    pub nodes: Vec<TraversalNode>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyPath {
    pub source: String,
    pub target: String,
    pub config: DependencyConfig,
    pub nodes: Vec<String>,
    pub layers: Vec<DependencyLayer>,
    pub total_weight: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CentralityEntry {
    pub name: String,
    pub in_degree: usize,
    pub out_degree: usize,
    pub weighted_in_degree: usize,
    pub weighted_out_degree: usize,
    pub page_rank: f64,
    pub betweenness: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CentralityResult {
    pub config: DependencyConfig,
    pub algorithm: String,
    pub parameters: BTreeMap<String, String>,
    pub external_symbols_included: bool,
    pub entries: Vec<CentralityEntry>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ComponentResult {
    pub config: DependencyConfig,
    pub algorithm: String,
    pub external_symbols_included: bool,
    pub components: Vec<Vec<String>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphStructureResult {
    pub config: DependencyConfig,
    pub algorithm: String,
    pub external_symbols_included: bool,
    pub articulation_points: Vec<String>,
    pub bridges: Vec<(String, String)>,
    pub communities: BTreeMap<String, usize>,
}

fn record(corpus: &AnalysisCorpus, edge: &DependencyEdge) -> DependencyRecord {
    DependencyRecord {
        source: corpus.declarations()[edge.source].name.clone(),
        target: edge.target.clone(),
        layer: edge.layer,
        occurrences: edge.occurrences,
        target_in_corpus: edge.target_in_corpus,
        source_module: edge.source_module.clone(),
        source_kind: corpus.declarations()[edge.source].kind.clone(),
        target_module: edge.target_module.clone(),
        target_kind: edge.target_kind.clone(),
        target_is_class: edge.target_is_class,
        target_is_instance: edge.target_is_instance,
        target_is_projection: edge.target_is_projection,
    }
}

fn dependency_summary(
    declaration: &str,
    direction: &str,
    config: &DependencyConfig,
    mut edges: Vec<DependencyRecord>,
) -> DependencySummary {
    let mut occurrences_by_module = BTreeMap::new();
    let mut occurrences_by_kind = BTreeMap::new();
    for edge in &edges {
        let (module, kind) = if direction == "reverse" {
            (edge.source_module.as_ref(), &edge.source_kind)
        } else {
            (edge.target_module.as_ref(), &edge.target_kind)
        };
        *occurrences_by_module
            .entry(module.cloned().unwrap_or_else(|| "<unknown>".into()))
            .or_default() += edge.occurrences;
        *occurrences_by_kind.entry(kind.clone()).or_default() += edge.occurrences;
    }
    let truncated = edges.len() > config.limit;
    edges.truncate(config.limit);
    DependencySummary {
        declaration: declaration.into(),
        direction: direction.into(),
        config: config.clone(),
        truncated,
        occurrences_by_module,
        occurrences_by_kind,
        edges,
    }
}

impl AnalysisCorpus {
    pub fn direct_dependencies(
        &self,
        name: &str,
        config: &DependencyConfig,
    ) -> Option<DependencySummary> {
        let id = self.declaration_id(name)?;
        let mut edges = self
            .outgoing_edge_ids(id)
            .iter()
            .map(|&edge| &self.edges()[edge])
            .filter(|edge| self.edge_matches(edge, &config.filter))
            .map(|edge| record(self, edge))
            .collect::<Vec<_>>();
        edges.sort_by(|a, b| a.target.cmp(&b.target).then(a.layer.cmp(&b.layer)));
        Some(dependency_summary(name, "forward", config, edges))
    }

    pub fn direct_dependents(
        &self,
        name: &str,
        config: &DependencyConfig,
    ) -> Option<DependencySummary> {
        self.symbol(name)?;
        let mut edges = self
            .incoming_edge_ids(name)
            .iter()
            .map(|&edge| &self.edges()[edge])
            .filter(|edge| self.edge_matches(edge, &config.filter))
            .map(|edge| record(self, edge))
            .collect::<Vec<_>>();
        edges.sort_by(|a, b| a.source.cmp(&b.source).then(a.layer.cmp(&b.layer)));
        Some(dependency_summary(name, "reverse", config, edges))
    }

    pub fn traverse_dependencies(
        &self,
        start: &str,
        reverse: bool,
        config: &DependencyConfig,
    ) -> Option<TraversalResult> {
        self.symbol(start)?;
        let mut queue = VecDeque::from([(start.to_owned(), 0usize, 0usize)]);
        let mut seen = BTreeSet::from([start.to_owned()]);
        let mut nodes = Vec::new();
        let mut truncated = false;
        while let Some((name, depth, weight)) = queue.pop_front() {
            if depth > 0 {
                nodes.push(TraversalNode {
                    name: name.clone(),
                    depth,
                    path_weight: weight,
                });
            }
            if nodes.len() >= config.limit {
                truncated = !queue.is_empty();
                break;
            }
            if depth >= config.max_depth {
                continue;
            }
            let edge_ids: &[usize] = if reverse {
                self.incoming_edge_ids(&name)
            } else if let Some(id) = self.declaration_id(&name) {
                self.outgoing_edge_ids(id)
            } else {
                &[]
            };
            let mut next = edge_ids
                .iter()
                .map(|&id| &self.edges()[id])
                .filter(|edge| self.edge_matches(edge, &config.filter))
                .map(|edge| {
                    let next_name = if reverse {
                        self.declarations()[edge.source].name.clone()
                    } else {
                        edge.target.clone()
                    };
                    (next_name, edge.occurrences)
                })
                .collect::<Vec<_>>();
            next.sort();
            for (next_name, edge_weight) in next {
                if seen.insert(next_name.clone()) {
                    queue.push_back((next_name, depth + 1, weight.saturating_add(edge_weight)));
                }
            }
        }
        Some(TraversalResult {
            start: start.into(),
            direction: if reverse { "reverse" } else { "forward" }.into(),
            config: config.clone(),
            truncated,
            nodes,
        })
    }

    pub fn shortest_dependency_path(
        &self,
        source: &str,
        target: &str,
        config: &DependencyConfig,
    ) -> Option<DependencyPath> {
        let source_id = self.declaration_id(source)?;
        self.symbol(target)?;
        let mut queue = VecDeque::from([(source.to_owned(), 0usize)]);
        let mut previous: HashMap<String, (String, DependencyLayer, usize)> = HashMap::new();
        let mut seen = BTreeSet::from([source.to_owned()]);
        while let Some((name, depth)) = queue.pop_front() {
            if name == target {
                break;
            }
            if depth >= config.max_depth {
                continue;
            }
            let id = if name == source {
                source_id
            } else if let Some(id) = self.declaration_id(&name) {
                id
            } else {
                continue;
            };
            let mut edges = self
                .outgoing_edge_ids(id)
                .iter()
                .map(|&id| &self.edges()[id])
                .filter(|edge| self.edge_matches(edge, &config.filter))
                .collect::<Vec<_>>();
            edges.sort_by(|a, b| a.target.cmp(&b.target).then(a.layer.cmp(&b.layer)));
            for edge in edges {
                if seen.insert(edge.target.clone()) {
                    previous.insert(
                        edge.target.clone(),
                        (name.clone(), edge.layer, edge.occurrences),
                    );
                    queue.push_back((edge.target.clone(), depth + 1));
                }
            }
        }
        if source != target && !previous.contains_key(target) {
            return None;
        }
        let mut nodes = vec![target.to_owned()];
        let mut layers = Vec::new();
        let mut total_weight = 0usize;
        let mut current = target;
        while current != source {
            let (prior, layer, weight) = previous.get(current)?;
            layers.push(*layer);
            total_weight = total_weight.saturating_add(*weight);
            nodes.push(prior.clone());
            current = prior;
        }
        nodes.reverse();
        layers.reverse();
        Some(DependencyPath {
            source: source.into(),
            target: target.into(),
            config: config.clone(),
            nodes,
            layers,
            total_weight,
        })
    }

    pub fn shared_dependencies(
        &self,
        names: &[String],
        config: &DependencyConfig,
    ) -> Option<Vec<String>> {
        let mut shared: Option<BTreeSet<String>> = None;
        for name in names {
            let summary = self.direct_dependencies(name, config)?;
            let current = summary
                .edges
                .into_iter()
                .map(|edge| edge.target)
                .collect::<BTreeSet<_>>();
            shared = Some(match shared {
                None => current,
                Some(old) => old.intersection(&current).cloned().collect(),
            });
        }
        Some(
            shared
                .unwrap_or_default()
                .into_iter()
                .take(config.limit)
                .collect(),
        )
    }

    fn graph_adjacency(&self, config: &DependencyConfig) -> Vec<Vec<(usize, usize)>> {
        let mut adjacency = vec![Vec::new(); self.declarations().len()];
        for edge in self.filtered_edges(&config.filter) {
            if let Some(target) = self.declaration_id(&edge.target) {
                adjacency[edge.source].push((target, edge.occurrences));
            }
        }
        for neighbors in &mut adjacency {
            neighbors.sort();
            neighbors.dedup_by(|a, b| {
                if a.0 == b.0 {
                    a.1 += b.1;
                    true
                } else {
                    false
                }
            });
        }
        adjacency
    }

    pub fn centrality(&self, config: &DependencyConfig) -> CentralityResult {
        let adjacency = self.graph_adjacency(config);
        let active = self
            .filtered_declaration_ids(&config.filter)
            .collect::<BTreeSet<_>>();
        let count = active.len().max(1);
        let mut ranks = vec![0.0; adjacency.len()];
        for &id in &active {
            ranks[id] = 1.0 / count as f64;
        }
        let damping = 0.85;
        let iterations = 50;
        for _ in 0..iterations {
            let mut next = vec![0.0; adjacency.len()];
            let mut dangling = 0.0;
            for &source in &active {
                let neighbors = adjacency[source]
                    .iter()
                    .filter(|(target, _)| active.contains(target))
                    .collect::<Vec<_>>();
                if neighbors.is_empty() {
                    dangling += ranks[source];
                    continue;
                }
                let total = if config.weighted {
                    neighbors.iter().map(|(_, w)| *w).sum::<usize>() as f64
                } else {
                    neighbors.len() as f64
                };
                for &&(target, weight) in &neighbors {
                    next[target] += ranks[source]
                        * if config.weighted {
                            weight as f64 / total
                        } else {
                            1.0 / total
                        };
                }
            }
            for &id in &active {
                next[id] =
                    (1.0 - damping) / count as f64 + damping * (next[id] + dangling / count as f64);
            }
            ranks = next;
        }
        let betweenness = brandes(&adjacency, &active);
        let mut entries = active
            .iter()
            .map(|&id| {
                let incoming = self
                    .incoming_edge_ids(&self.declarations()[id].name)
                    .iter()
                    .map(|&e| &self.edges()[e])
                    .filter(|e| self.edge_matches(e, &config.filter) && active.contains(&e.source))
                    .collect::<Vec<_>>();
                let outgoing = adjacency[id]
                    .iter()
                    .filter(|(target, _)| active.contains(target))
                    .collect::<Vec<_>>();
                CentralityEntry {
                    name: self.declarations()[id].name.clone(),
                    in_degree: incoming.len(),
                    out_degree: outgoing.len(),
                    weighted_in_degree: incoming.iter().map(|e| e.occurrences).sum(),
                    weighted_out_degree: outgoing.iter().map(|(_, w)| *w).sum(),
                    page_rank: ranks[id],
                    betweenness: betweenness[id],
                }
            })
            .collect::<Vec<_>>();
        entries.sort_by(|a, b| {
            b.page_rank
                .total_cmp(&a.page_rank)
                .then_with(|| a.name.cmp(&b.name))
        });
        CentralityResult {
            config: config.clone(),
            algorithm: "PageRank and unweighted Brandes betweenness".into(),
            parameters: BTreeMap::from([
                ("damping".into(), damping.to_string()),
                ("iterations".into(), iterations.to_string()),
            ]),
            external_symbols_included: false,
            entries,
        }
    }

    pub fn strongly_connected_components(&self, config: &DependencyConfig) -> ComponentResult {
        let adjacency = self.graph_adjacency(config);
        let active = self
            .filtered_declaration_ids(&config.filter)
            .collect::<BTreeSet<_>>();
        let mut state = TarjanState::new(adjacency.len());
        for &node in &active {
            if state.indices[node].is_none() {
                tarjan(node, &adjacency, &active, &mut state);
            }
        }
        let mut components = state
            .components
            .into_iter()
            .map(|component| {
                component
                    .into_iter()
                    .map(|id| self.declarations()[id].name.clone())
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        for component in &mut components {
            component.sort();
        }
        components.sort_by(|a, b| b.len().cmp(&a.len()).then_with(|| a.cmp(b)));
        ComponentResult {
            config: config.clone(),
            algorithm: "Tarjan strongly connected components".into(),
            external_symbols_included: false,
            components,
        }
    }

    pub fn connected_components(&self, config: &DependencyConfig) -> ComponentResult {
        let directed = self.graph_adjacency(config);
        let active = self
            .filtered_declaration_ids(&config.filter)
            .collect::<BTreeSet<_>>();
        let mut undirected = vec![BTreeSet::new(); directed.len()];
        for &source in &active {
            for &(target, _) in &directed[source] {
                if active.contains(&target) {
                    undirected[source].insert(target);
                    undirected[target].insert(source);
                }
            }
        }
        let mut seen = BTreeSet::new();
        let mut components = Vec::new();
        for &start in &active {
            if !seen.insert(start) {
                continue;
            }
            let mut queue = VecDeque::from([start]);
            let mut component = Vec::new();
            while let Some(node) = queue.pop_front() {
                component.push(self.declarations()[node].name.clone());
                for &next in &undirected[node] {
                    if seen.insert(next) {
                        queue.push_back(next);
                    }
                }
            }
            component.sort();
            components.push(component);
        }
        components.sort_by(|a, b| b.len().cmp(&a.len()).then_with(|| a.cmp(b)));
        ComponentResult {
            config: config.clone(),
            algorithm: "weakly connected components".into(),
            external_symbols_included: false,
            components,
        }
    }

    pub fn graph_structure(&self, config: &DependencyConfig) -> GraphStructureResult {
        let directed = self.graph_adjacency(config);
        let active = self
            .filtered_declaration_ids(&config.filter)
            .collect::<BTreeSet<_>>();
        let mut undirected = vec![BTreeSet::new(); directed.len()];
        for &source in &active {
            for &(target, _) in &directed[source] {
                if active.contains(&target) {
                    undirected[source].insert(target);
                    undirected[target].insert(source);
                }
            }
        }
        let (points, bridge_ids) = articulation_and_bridges(&undirected, &active);
        let labels = label_propagation(&undirected, &active);
        let articulation_points = points
            .into_iter()
            .map(|id| self.declarations()[id].name.clone())
            .collect();
        let mut bridges = bridge_ids
            .into_iter()
            .map(|(a, b)| {
                let mut names = [
                    self.declarations()[a].name.clone(),
                    self.declarations()[b].name.clone(),
                ];
                names.sort();
                (names[0].clone(), names[1].clone())
            })
            .collect::<Vec<_>>();
        bridges.sort();
        let unique_labels = labels
            .values()
            .copied()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .enumerate()
            .map(|(community, label)| (label, community))
            .collect::<BTreeMap<_, _>>();
        let communities = active
            .into_iter()
            .map(|id| {
                (
                    self.declarations()[id].name.clone(),
                    unique_labels[&labels[&id]],
                )
            })
            .collect();
        GraphStructureResult {
            config: config.clone(),
            algorithm: "undirected articulation/bridge search and deterministic label propagation"
                .into(),
            external_symbols_included: false,
            articulation_points,
            bridges,
            communities,
        }
    }
}

fn brandes(adjacency: &[Vec<(usize, usize)>], active: &BTreeSet<usize>) -> Vec<f64> {
    let mut centrality = vec![0.0; adjacency.len()];
    for &source in active {
        let mut predecessors = vec![Vec::new(); adjacency.len()];
        let mut sigma = vec![0.0; adjacency.len()];
        sigma[source] = 1.0;
        let mut distance = vec![usize::MAX; adjacency.len()];
        distance[source] = 0;
        let mut queue = VecDeque::from([source]);
        let mut stack = Vec::new();
        while let Some(v) = queue.pop_front() {
            stack.push(v);
            for &(w, _) in &adjacency[v] {
                if !active.contains(&w) {
                    continue;
                }
                if distance[w] == usize::MAX {
                    distance[w] = distance[v] + 1;
                    queue.push_back(w);
                }
                if distance[w] == distance[v] + 1 {
                    sigma[w] += sigma[v];
                    predecessors[w].push(v);
                }
            }
        }
        let mut delta = vec![0.0; adjacency.len()];
        while let Some(w) = stack.pop() {
            for &v in &predecessors[w] {
                if sigma[w] > 0.0 {
                    delta[v] += (sigma[v] / sigma[w]) * (1.0 + delta[w]);
                }
            }
            if w != source {
                centrality[w] += delta[w];
            }
        }
    }
    centrality
}

struct TarjanState {
    next: usize,
    indices: Vec<Option<usize>>,
    low: Vec<usize>,
    stack: Vec<usize>,
    on_stack: Vec<bool>,
    components: Vec<Vec<usize>>,
}
impl TarjanState {
    fn new(size: usize) -> Self {
        Self {
            next: 0,
            indices: vec![None; size],
            low: vec![0; size],
            stack: vec![],
            on_stack: vec![false; size],
            components: vec![],
        }
    }
}
fn tarjan(
    node: usize,
    adjacency: &[Vec<(usize, usize)>],
    active: &BTreeSet<usize>,
    state: &mut TarjanState,
) {
    let index = state.next;
    state.next += 1;
    state.indices[node] = Some(index);
    state.low[node] = index;
    state.stack.push(node);
    state.on_stack[node] = true;
    for &(next, _) in &adjacency[node] {
        if !active.contains(&next) {
            continue;
        }
        if state.indices[next].is_none() {
            tarjan(next, adjacency, active, state);
            state.low[node] = state.low[node].min(state.low[next]);
        } else if state.on_stack[next] {
            state.low[node] = state.low[node].min(state.indices[next].unwrap());
        }
    }
    if state.low[node] == index {
        let mut component = Vec::new();
        loop {
            let member = state.stack.pop().unwrap();
            state.on_stack[member] = false;
            component.push(member);
            if member == node {
                break;
            }
        }
        state.components.push(component);
    }
}

fn articulation_and_bridges(
    adjacency: &[BTreeSet<usize>],
    active: &BTreeSet<usize>,
) -> (BTreeSet<usize>, Vec<(usize, usize)>) {
    struct State {
        time: usize,
        disc: Vec<usize>,
        low: Vec<usize>,
        parent: Vec<Option<usize>>,
        points: BTreeSet<usize>,
        bridges: Vec<(usize, usize)>,
    }
    fn visit(u: usize, adjacency: &[BTreeSet<usize>], state: &mut State) {
        state.time += 1;
        state.disc[u] = state.time;
        state.low[u] = state.time;
        let mut children = 0;
        for &v in &adjacency[u] {
            if state.disc[v] == 0 {
                children += 1;
                state.parent[v] = Some(u);
                visit(v, adjacency, state);
                state.low[u] = state.low[u].min(state.low[v]);
                if state.parent[u].is_none() && children > 1
                    || state.parent[u].is_some() && state.low[v] >= state.disc[u]
                {
                    state.points.insert(u);
                }
                if state.low[v] > state.disc[u] {
                    state.bridges.push((u, v));
                }
            } else if state.parent[u] != Some(v) {
                state.low[u] = state.low[u].min(state.disc[v]);
            }
        }
    }
    let mut state = State {
        time: 0,
        disc: vec![0; adjacency.len()],
        low: vec![0; adjacency.len()],
        parent: vec![None; adjacency.len()],
        points: BTreeSet::new(),
        bridges: vec![],
    };
    for &node in active {
        if state.disc[node] == 0 {
            visit(node, adjacency, &mut state);
        }
    }
    (state.points, state.bridges)
}

pub(crate) fn label_propagation(
    adjacency: &[BTreeSet<usize>],
    active: &BTreeSet<usize>,
) -> BTreeMap<usize, usize> {
    let mut labels = active
        .iter()
        .map(|&id| (id, id))
        .collect::<BTreeMap<_, _>>();
    for _ in 0..50 {
        let mut changed = false;
        let previous = labels.clone();
        for &node in active {
            let mut counts = BTreeMap::new();
            for neighbor in &adjacency[node] {
                *counts.entry(previous[neighbor]).or_insert(0usize) += 1;
            }
            if let Some((&label, _)) = counts
                .iter()
                .max_by(|(la, ca), (lb, cb)| ca.cmp(cb).then_with(|| lb.cmp(la)))
                && previous[&node] != label
            {
                labels.insert(node, label);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    labels
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_propagation_is_deterministic() {
        let graph = vec![
            BTreeSet::from([1]),
            BTreeSet::from([0, 2]),
            BTreeSet::from([1]),
            BTreeSet::new(),
        ];
        let active = BTreeSet::from([0, 1, 2, 3]);
        assert_eq!(
            label_propagation(&graph, &active),
            label_propagation(&graph, &active)
        );
    }
}
