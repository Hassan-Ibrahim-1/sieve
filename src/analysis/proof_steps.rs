use std::collections::{BTreeMap, BTreeSet, VecDeque};

use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};

use crate::model::{ProofStep, ProofStepExtraction};

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProofStepEdge {
    /// Prerequisite step. Edges always point from prerequisite to the claim using it.
    pub source: usize,
    pub target: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProofOutlineNode {
    pub raw_step_id: usize,
    pub kind: String,
    pub proposition: String,
    pub context: Vec<crate::model::ProofContextEntry>,
    pub hypothesis_references: Vec<String>,
    pub named_references: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProofOutlineEdge {
    pub source: usize,
    pub target: usize,
    /// Number of raw paths represented by this edge.
    pub path_count: usize,
    /// Longest represented path, including both retained endpoints.
    pub maximum_raw_path_length: usize,
    /// One inclusive raw-step path, retained as inspectable evidence.
    pub raw_step_path: Vec<usize>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProofOutlineEvidence {
    pub algorithm: String,
    pub retention_rules: Vec<String>,
    pub complete: bool,
    pub truncation_reason: Option<String>,
    pub visited_terms: usize,
    pub conclusion_step: Option<usize>,
    pub raw_steps: Vec<ProofStep>,
    pub raw_edges: Vec<ProofStepEdge>,
    /// Nodes matching the semantic retention rules before overview limiting.
    pub candidate_node_count: usize,
    pub retained_nodes: Vec<ProofOutlineNode>,
    pub condensed_edges: Vec<ProofOutlineEdge>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StepInspection {
    pub step: usize,
    pub direct_prerequisites: Vec<usize>,
    pub transitive_prerequisites: Vec<usize>,
    pub direct_dependents: Vec<usize>,
    pub paths_to_conclusion: Vec<Vec<usize>>,
    pub paths_truncated: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProofStepsReport<'a> {
    pub declaration: &'a str,
    pub complete: bool,
    pub truncation_reason: Option<&'a str>,
    pub visited_terms: usize,
    pub conclusion_step: Option<usize>,
    pub steps: &'a [ProofStep],
    pub named_results: &'a [crate::model::NamedResultSnapshot],
    pub edges: Vec<ProofStepEdge>,
    pub selected: Option<StepInspection>,
}

pub struct ProofStepGraph<'a> {
    extraction: &'a ProofStepExtraction,
    dependents: Vec<Vec<usize>>,
}

impl<'a> ProofStepGraph<'a> {
    pub fn new(extraction: &'a ProofStepExtraction) -> Result<Self> {
        validate_proof_steps(extraction)?;
        let mut dependents = vec![Vec::new(); extraction.steps.len()];
        for step in &extraction.steps {
            for &prerequisite in &step.prerequisite_steps {
                dependents[prerequisite].push(step.id);
            }
        }
        Ok(Self {
            extraction,
            dependents,
        })
    }

    pub fn edges(&self) -> Vec<ProofStepEdge> {
        self.extraction
            .steps
            .iter()
            .flat_map(|step| {
                step.prerequisite_steps
                    .iter()
                    .map(move |&source| ProofStepEdge {
                        source,
                        target: step.id,
                    })
            })
            .collect()
    }

    pub fn direct_prerequisites(&self, step: usize) -> Option<&[usize]> {
        self.extraction
            .steps
            .get(step)
            .map(|step| step.prerequisite_steps.as_slice())
    }

    pub fn direct_dependents(&self, step: usize) -> Option<&[usize]> {
        self.dependents.get(step).map(Vec::as_slice)
    }

    pub fn transitive_prerequisites(&self, step: usize) -> Option<Vec<usize>> {
        self.extraction.steps.get(step)?;
        let mut seen = BTreeSet::new();
        let mut queue = VecDeque::from([step]);
        while let Some(current) = queue.pop_front() {
            for &prerequisite in &self.extraction.steps[current].prerequisite_steps {
                if seen.insert(prerequisite) {
                    queue.push_back(prerequisite);
                }
            }
        }
        Some(seen.into_iter().collect())
    }

    pub fn paths_to_conclusion(
        &self,
        step: usize,
        maximum_paths: usize,
    ) -> Option<(Vec<Vec<usize>>, bool)> {
        self.extraction.steps.get(step)?;
        let conclusion = self.extraction.conclusion_step?;
        let mut paths = Vec::new();
        let mut path = vec![step];
        let mut truncated = false;
        self.collect_paths(
            step,
            conclusion,
            maximum_paths,
            &mut path,
            &mut paths,
            &mut truncated,
        );
        Some((paths, truncated))
    }

    fn collect_paths(
        &self,
        current: usize,
        conclusion: usize,
        maximum_paths: usize,
        path: &mut Vec<usize>,
        paths: &mut Vec<Vec<usize>>,
        truncated: &mut bool,
    ) {
        if paths.len() >= maximum_paths {
            *truncated = true;
            return;
        }
        if current == conclusion {
            paths.push(path.clone());
            return;
        }
        for &dependent in &self.dependents[current] {
            path.push(dependent);
            self.collect_paths(dependent, conclusion, maximum_paths, path, paths, truncated);
            path.pop();
            if *truncated {
                return;
            }
        }
    }

    pub fn inspect(&self, step: usize, maximum_paths: usize) -> Option<StepInspection> {
        let direct_prerequisites = self.direct_prerequisites(step)?.to_vec();
        let transitive_prerequisites = self.transitive_prerequisites(step)?;
        let direct_dependents = self.direct_dependents(step)?.to_vec();
        let (paths_to_conclusion, paths_truncated) = self
            .paths_to_conclusion(step, maximum_paths)
            .unwrap_or_default();
        Some(StepInspection {
            step,
            direct_prerequisites,
            transitive_prerequisites,
            direct_dependents,
            paths_to_conclusion,
            paths_truncated,
        })
    }

    pub fn report(
        &self,
        declaration: &'a str,
        selected_step: Option<usize>,
    ) -> Result<ProofStepsReport<'a>> {
        let selected = selected_step
            .map(|step| {
                self.inspect(step, 100)
                    .with_context(|| format!("unknown proof step {step}"))
            })
            .transpose()?;
        Ok(ProofStepsReport {
            declaration,
            complete: self.extraction.complete,
            truncation_reason: self.extraction.truncation_reason.as_deref(),
            visited_terms: self.extraction.visited_terms,
            conclusion_step: self.extraction.conclusion_step,
            steps: &self.extraction.steps,
            named_results: &self.extraction.named_results,
            edges: self.edges(),
            selected,
        })
    }
}

/// Conservatively removes anonymous plumbing while retaining the conclusion,
/// local facts, branch conclusions, and non-plumbing named applications that
/// can reach the conclusion. Each replacement edge carries its complete raw
/// step path, and the full uncontracted extraction remains in the report.
pub fn build_proof_outline(extraction: &ProofStepExtraction) -> Result<ProofOutlineEvidence> {
    let graph = ProofStepGraph::new(extraction)?;
    let raw_edges = graph.edges();
    let Some(conclusion) = extraction.conclusion_step else {
        return Ok(ProofOutlineEvidence {
            algorithm: "conservative proof-step path contraction".into(),
            retention_rules: proof_outline_retention_rules(),
            complete: extraction.complete,
            truncation_reason: extraction.truncation_reason.clone(),
            visited_terms: extraction.visited_terms,
            conclusion_step: None,
            raw_steps: extraction.steps.clone(),
            raw_edges,
            candidate_node_count: 0,
            retained_nodes: vec![],
            condensed_edges: vec![],
        });
    };

    let ancestors = graph
        .transitive_prerequisites(conclusion)
        .unwrap_or_default()
        .into_iter()
        .chain(std::iter::once(conclusion))
        .collect::<BTreeSet<_>>();
    let candidates = extraction
        .steps
        .iter()
        .filter(|step| {
            ancestors.contains(&step.id)
                && (step.id == conclusion
                    || matches!(step.kind.as_str(), "localFact" | "branchConclusion")
                    || (step.kind == "namedApplication"
                        && step
                            .named_references
                            .iter()
                            .any(|name| meaningful_named_reference(name))))
        })
        .map(|step| step.id)
        .collect::<BTreeSet<_>>();
    let candidate_node_count = candidates.len();
    let retained = select_outline_nodes(extraction, &candidates, conclusion);

    let retained_nodes = retained
        .iter()
        .map(|&id| {
            let step = &extraction.steps[id];
            ProofOutlineNode {
                raw_step_id: id,
                kind: step.kind.clone(),
                proposition: step.proposition.clone(),
                context: step.context.clone(),
                hypothesis_references: step.hypothesis_references.clone(),
                named_references: step.named_references.clone(),
            }
        })
        .collect::<Vec<_>>();

    let mut condensed_edges = Vec::new();
    for &source in &retained {
        // The raw proof graph is a DAG whose step ids are topological order.
        // Aggregate paths as they cross omitted nodes instead of enumerating
        // every path. Enumerating paths is exponential for branching proofs
        // such as exists_le_sylow, even though the rendered graph ultimately
        // merges all paths with the same retained endpoints.
        let mut paths = BTreeMap::<usize, (usize, usize, Vec<usize>)>::new();
        for &next in graph.direct_dependents(source).unwrap_or_default() {
            if ancestors.contains(&next) {
                paths.insert(next, (1, 2, vec![source, next]));
            }
        }
        while let Some((&current, _)) = paths.first_key_value() {
            let (path_count, maximum_raw_path_length, sample_path) =
                paths.remove(&current).expect("path summary exists");
            if retained.contains(&current) {
                condensed_edges.push(ProofOutlineEdge {
                    source,
                    target: current,
                    path_count,
                    maximum_raw_path_length,
                    raw_step_path: sample_path,
                });
                continue;
            }
            for &next in graph.direct_dependents(current).unwrap_or_default() {
                if ancestors.contains(&next) {
                    let entry = paths.entry(next).or_insert_with(|| {
                        let mut path = sample_path.clone();
                        path.push(next);
                        (0, 0, path)
                    });
                    entry.0 = entry.0.saturating_add(path_count);
                    entry.1 = entry.1.max(maximum_raw_path_length.saturating_add(1));
                }
            }
        }
    }

    Ok(ProofOutlineEvidence {
        algorithm: "conservative proof-step path contraction".into(),
        retention_rules: proof_outline_retention_rules(),
        complete: extraction.complete,
        truncation_reason: extraction.truncation_reason.clone(),
        visited_terms: extraction.visited_terms,
        conclusion_step: Some(conclusion),
        raw_steps: extraction.steps.clone(),
        raw_edges,
        candidate_node_count,
        retained_nodes,
        condensed_edges,
    })
}

fn proof_outline_retention_rules() -> Vec<String> {
    vec![
        "retain the conclusion and every local-fact or branch-conclusion ancestor".into(),
        "retain named applications except generated instances, coercion adapters, and core equality/congruence plumbing".into(),
        "for large proofs, limit named-application landmarks to a 240-node overview, prioritizing rare references and sampling the full proof order".into(),
        "replace omitted paths with condensed edges carrying inclusive raw-step paths".into(),
    ]
}

const MAX_OUTLINE_NODES: usize = 240;

fn select_outline_nodes(
    extraction: &ProofStepExtraction,
    candidates: &BTreeSet<usize>,
    conclusion: usize,
) -> BTreeSet<usize> {
    if candidates.len() <= MAX_OUTLINE_NODES {
        return candidates.clone();
    }

    let mandatory = candidates
        .iter()
        .copied()
        .filter(|&id| {
            id != conclusion
                && matches!(
                    extraction.steps[id].kind.as_str(),
                    "localFact" | "branchConclusion"
                )
        })
        .collect::<Vec<_>>();
    let mut retained = BTreeSet::from([conclusion]);
    if mandatory.len() >= MAX_OUTLINE_NODES - 1 {
        for index in 0..MAX_OUTLINE_NODES - 1 {
            retained.insert(mandatory[index * mandatory.len() / (MAX_OUTLINE_NODES - 1)]);
        }
        return retained;
    }
    retained.extend(mandatory);

    let mut reference_frequency = BTreeMap::<&str, usize>::new();
    for &id in candidates {
        if retained.contains(&id) {
            continue;
        }
        for name in &extraction.steps[id].named_references {
            *reference_frequency.entry(name).or_default() += 1;
        }
    }
    let mut optional = candidates
        .iter()
        .copied()
        .filter(|id| !retained.contains(id))
        .collect::<Vec<_>>();
    optional.sort_by_key(|&id| {
        let rarity = extraction.steps[id]
            .named_references
            .iter()
            .filter_map(|name| reference_frequency.get(name.as_str()))
            .copied()
            .min()
            .unwrap_or(usize::MAX);
        (rarity, id)
    });

    let budget = MAX_OUTLINE_NODES - retained.len();
    let priority_budget = budget * 3 / 4;
    retained.extend(optional.iter().take(priority_budget).copied());

    let mut remainder = optional
        .into_iter()
        .filter(|id| !retained.contains(id))
        .collect::<Vec<_>>();
    remainder.sort_unstable();
    let remaining_budget = MAX_OUTLINE_NODES - retained.len();
    for index in 0..remaining_budget {
        let sample = index * remainder.len() / remaining_budget;
        retained.insert(remainder[sample]);
    }
    retained
}

fn meaningful_named_reference(name: &str) -> bool {
    const CORE_PLUMBING: &[&str] = &[
        "id",
        "rfl",
        "Eq.refl",
        "congrArg",
        "Eq.mpr",
        "Eq.mp",
        "Eq.symm",
        "Iff.mpr",
        "Iff.mp",
        "Exists.intro",
        "Exists.elim",
        "And.intro",
        "And.left",
        "And.right",
        "Or.elim",
        "Subtype.property",
    ];
    !name.starts_with("inst")
        && !name.contains(".to")
        && !name.contains(".match_")
        && !CORE_PLUMBING.contains(&name)
}

fn is_prefix(left: &[String], right: &[String]) -> bool {
    left.len() <= right.len() && left.iter().zip(right).all(|(left, right)| left == right)
}

pub fn validate_proof_steps(extraction: &ProofStepExtraction) -> Result<()> {
    ensure!(
        extraction.complete == extraction.truncation_reason.is_none(),
        "proof-step completeness and truncation reason disagree"
    );
    let named_results = extraction
        .named_results
        .iter()
        .map(|result| result.name.as_str())
        .collect::<BTreeSet<_>>();
    ensure!(
        named_results.len() == extraction.named_results.len(),
        "duplicate named proof result"
    );
    for result in &extraction.named_results {
        result
            .type_graph
            .validate()
            .with_context(|| format!("invalid statement graph for {}", result.name))?;
    }
    let mut paths = BTreeSet::new();
    for (expected_id, step) in extraction.steps.iter().enumerate() {
        ensure!(step.id == expected_id, "proof step ids are not contiguous");
        ensure!(
            paths.insert(step.proof_term_path.as_slice()),
            "multiple proof steps occupy path {:?}",
            step.proof_term_path
        );
        step.proposition_graph
            .validate()
            .with_context(|| format!("invalid proposition graph for proof step {}", step.id))?;
        let context_ids = step
            .context
            .iter()
            .map(|entry| entry.id.as_str())
            .collect::<BTreeSet<_>>();
        ensure!(
            context_ids.len() == step.context.len(),
            "duplicate context entry in proof step {}",
            step.id
        );
        ensure!(
            step.scope
                == step
                    .context
                    .iter()
                    .map(|entry| entry.id.clone())
                    .collect::<Vec<_>>(),
            "scope disagrees with context for proof step {}",
            step.id
        );
        for entry in &step.context {
            entry.type_graph.validate().with_context(|| {
                format!(
                    "invalid context type {} in proof step {}",
                    entry.id, step.id
                )
            })?;
        }
        let mut prerequisites = BTreeSet::new();
        for &prerequisite in &step.prerequisite_steps {
            ensure!(
                prerequisite < step.id,
                "proof step {} has a non-earlier prerequisite {}",
                step.id,
                prerequisite
            );
            ensure!(
                prerequisites.insert(prerequisite),
                "duplicate prerequisite {} in proof step {}",
                prerequisite,
                step.id
            );
            let prerequisite_scope = &extraction.steps[prerequisite].scope;
            ensure!(
                is_prefix(prerequisite_scope, &step.scope)
                    || is_prefix(&step.scope, prerequisite_scope),
                "incomparable scopes for proof edge {} -> {}",
                prerequisite,
                step.id
            );
        }
        for hypothesis in &step.hypothesis_references {
            ensure!(
                step.context
                    .iter()
                    .any(|entry| entry.id == *hypothesis && entry.kind == "assumption"),
                "proof step {} references unavailable hypothesis {}",
                step.id,
                hypothesis
            );
        }
        for named in &step.named_references {
            ensure!(
                named_results.contains(named.as_str()),
                "proof step {} references missing named result {}",
                step.id,
                named
            );
        }
    }
    if extraction.complete && !extraction.steps.is_empty() {
        let conclusion = extraction
            .conclusion_step
            .context("complete proof-step extraction has no conclusion")?;
        ensure!(
            conclusion < extraction.steps.len(),
            "conclusion is out of bounds"
        );
        ensure!(
            extraction.steps[conclusion].kind == "conclusion",
            "conclusion step has the wrong kind"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ExpressionGraph, ExpressionNode, ProofStep};

    fn graph() -> ExpressionGraph {
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
        }
    }

    fn step(id: usize, prerequisites: Vec<usize>) -> ProofStep {
        ProofStep {
            id,
            kind: if id == 2 {
                "conclusion".into()
            } else {
                "namedApplication".into()
            },
            proposition: format!("P{id}"),
            proposition_graph: graph(),
            context: vec![],
            scope: vec![],
            proof_term_path: vec![id],
            prerequisite_steps: prerequisites,
            hypothesis_references: vec![],
            named_references: vec![],
        }
    }

    fn named_step(id: usize, kind: &str, prerequisites: Vec<usize>, named: &[&str]) -> ProofStep {
        ProofStep {
            id,
            kind: kind.into(),
            proposition: format!("P{id}"),
            proposition_graph: graph(),
            context: vec![],
            scope: vec![],
            proof_term_path: vec![id],
            prerequisite_steps: prerequisites,
            hypothesis_references: vec![],
            named_references: named.iter().map(|name| (*name).into()).collect(),
        }
    }

    #[test]
    fn edges_point_from_prerequisites_to_claims() {
        let extraction = ProofStepExtraction {
            complete: true,
            truncation_reason: None,
            visited_terms: 3,
            conclusion_step: Some(2),
            steps: vec![step(0, vec![]), step(1, vec![0]), step(2, vec![0, 1])],
            named_results: vec![],
        };
        let graph = ProofStepGraph::new(&extraction).unwrap();
        assert_eq!(graph.direct_dependents(0).unwrap(), [1, 2]);
        assert_eq!(graph.transitive_prerequisites(2).unwrap(), [0, 1]);
        assert_eq!(
            graph.paths_to_conclusion(0, 10).unwrap().0,
            vec![vec![0, 1, 2], vec![0, 2]]
        );
    }

    #[test]
    fn condensed_edges_expand_to_valid_raw_edges() {
        let extraction = ProofStepExtraction {
            complete: true,
            truncation_reason: None,
            visited_terms: 5,
            conclusion_step: Some(4),
            steps: vec![
                named_step(0, "namedApplication", vec![], &["Foundation"]),
                named_step(1, "anonymousApplication", vec![0], &[]),
                named_step(2, "localFact", vec![1], &[]),
                named_step(3, "branchConclusion", vec![2], &[]),
                named_step(4, "conclusion", vec![3], &["Finish"]),
            ],
            named_results: vec![
                crate::model::NamedResultSnapshot {
                    name: "Foundation".into(),
                    kind: "theorem".into(),
                    module_name: Some("Fixture".into()),
                    r#type: "P".into(),
                    type_graph: graph(),
                },
                crate::model::NamedResultSnapshot {
                    name: "Finish".into(),
                    kind: "theorem".into(),
                    module_name: Some("Fixture".into()),
                    r#type: "P".into(),
                    type_graph: graph(),
                },
            ],
        };
        let outline = build_proof_outline(&extraction).unwrap();
        assert_eq!(
            outline
                .retained_nodes
                .iter()
                .map(|node| node.raw_step_id)
                .collect::<Vec<_>>(),
            vec![0, 2, 3, 4]
        );
        let raw = outline
            .raw_edges
            .iter()
            .map(|edge| (edge.source, edge.target))
            .collect::<BTreeSet<_>>();
        for edge in &outline.condensed_edges {
            assert_eq!(edge.raw_step_path.first(), Some(&edge.source));
            assert_eq!(edge.raw_step_path.last(), Some(&edge.target));
            assert!(
                edge.raw_step_path
                    .windows(2)
                    .all(|pair| raw.contains(&(pair[0], pair[1])))
            );
        }
        let conclusion = outline.conclusion_step.unwrap();
        for node in &outline.retained_nodes {
            let mut seen = BTreeSet::from([node.raw_step_id]);
            let mut queue = VecDeque::from([node.raw_step_id]);
            while let Some(current) = queue.pop_front() {
                for edge in outline
                    .condensed_edges
                    .iter()
                    .filter(|edge| edge.source == current)
                {
                    if seen.insert(edge.target) {
                        queue.push_back(edge.target);
                    }
                }
            }
            assert!(seen.contains(&conclusion));
        }
    }

    #[test]
    fn condensed_edges_count_branching_paths_without_enumerating_them() {
        let extraction = ProofStepExtraction {
            complete: true,
            truncation_reason: None,
            visited_terms: 5,
            conclusion_step: Some(4),
            steps: vec![
                named_step(0, "localFact", vec![], &[]),
                named_step(1, "anonymousApplication", vec![0], &[]),
                named_step(2, "anonymousApplication", vec![0], &[]),
                named_step(3, "anonymousApplication", vec![1, 2], &[]),
                named_step(4, "conclusion", vec![3], &[]),
            ],
            named_results: vec![],
        };

        let outline = build_proof_outline(&extraction).unwrap();
        let edge = outline
            .condensed_edges
            .iter()
            .find(|edge| edge.source == 0 && edge.target == 4)
            .expect("branching paths are represented by one edge");
        assert_eq!(edge.path_count, 2);
        assert_eq!(edge.maximum_raw_path_length, 4);
        assert_eq!(edge.raw_step_path.first(), Some(&0));
        assert_eq!(edge.raw_step_path.last(), Some(&4));
    }

    #[test]
    fn large_outlines_are_limited_to_semantic_landmarks() {
        let mut steps = (0..300)
            .map(|id| {
                named_step(
                    id,
                    "namedApplication",
                    id.checked_sub(1).into_iter().collect(),
                    &["Foundation"],
                )
            })
            .collect::<Vec<_>>();
        steps.push(named_step(300, "conclusion", vec![299], &[]));
        let extraction = ProofStepExtraction {
            complete: true,
            truncation_reason: None,
            visited_terms: 301,
            conclusion_step: Some(300),
            steps,
            named_results: vec![crate::model::NamedResultSnapshot {
                name: "Foundation".into(),
                kind: "theorem".into(),
                module_name: Some("Fixture".into()),
                r#type: "P".into(),
                type_graph: graph(),
            }],
        };

        let outline = build_proof_outline(&extraction).unwrap();
        assert_eq!(outline.candidate_node_count, 301);
        assert_eq!(outline.retained_nodes.len(), MAX_OUTLINE_NODES);
        assert!(
            outline
                .retained_nodes
                .iter()
                .any(|node| node.raw_step_id == 300)
        );
    }

    #[test]
    fn incomplete_outline_preserves_extraction_status() {
        let extraction = ProofStepExtraction {
            complete: false,
            truncation_reason: Some("fixture limit reached".into()),
            visited_terms: 10,
            conclusion_step: None,
            steps: vec![],
            named_results: vec![],
        };
        let outline = build_proof_outline(&extraction).unwrap();
        assert!(!outline.complete);
        assert_eq!(
            outline.truncation_reason.as_deref(),
            Some("fixture limit reached")
        );
        assert!(outline.retained_nodes.is_empty());
    }

    #[test]
    fn rejects_forward_and_missing_step_references() {
        let extraction = ProofStepExtraction {
            complete: true,
            truncation_reason: None,
            visited_terms: 2,
            conclusion_step: Some(1),
            steps: vec![step(0, vec![1]), step(1, vec![])],
            named_results: vec![],
        };
        assert!(validate_proof_steps(&extraction).is_err());
    }
}
