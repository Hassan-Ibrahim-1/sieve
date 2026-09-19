use std::collections::BTreeSet;

use anyhow::{Context, Result, bail, ensure};

use crate::analysis::proof_steps::validate_proof_steps;
use crate::model::{DeclarationSnapshot, ExpressionGraph, ExtractionSnapshot};

impl ExpressionGraph {
    pub fn validate(&self) -> Result<()> {
        ensure!(!self.nodes.is_empty(), "expression graph is empty");
        ensure!(
            self.root < self.nodes.len(),
            "expression root is out of bounds"
        );
        for (expected_id, node) in self.nodes.iter().enumerate() {
            ensure!(
                node.id == expected_id,
                "expression node {} is stored at index {expected_id}",
                node.id
            );
            for &child in &node.children {
                ensure!(child < node.id, "expression graph is not in postorder");
            }
        }
        Ok(())
    }

    /// Number of root-to-node paths, restoring occurrences collapsed by hash-consing.
    pub fn occurrence_multiplicities(&self) -> Vec<usize> {
        let mut multiplicities = vec![0; self.nodes.len()];
        multiplicities[self.root] = 1;
        for id in (0..self.nodes.len()).rev() {
            let multiplicity = multiplicities[id];
            for &child in &self.nodes[id].children {
                multiplicities[child] += multiplicity;
            }
        }
        multiplicities
    }

    pub fn occurrence_node_count(&self) -> usize {
        self.occurrence_multiplicities().into_iter().sum()
    }

    pub fn constant_occurrences(&self) -> std::collections::BTreeMap<String, usize> {
        let multiplicities = self.occurrence_multiplicities();
        let mut occurrences = std::collections::BTreeMap::new();
        for node in &self.nodes {
            if node.kind == "constant"
                && let Some(name) = &node.name
            {
                *occurrences.entry(name.clone()).or_default() += multiplicities[node.id];
            }
        }
        occurrences
    }
}

impl DeclarationSnapshot {
    pub fn validate(&self) -> Result<()> {
        self.type_graph
            .validate()
            .with_context(|| format!("invalid type graph for {}", self.name))?;
        ensure!(
            self.type_graph.occurrence_node_count() == self.type_stats.nodes,
            "type graph occurrence count disagrees with type statistics for {}",
            self.name
        );
        let graph_dependencies = self
            .type_graph
            .constant_occurrences()
            .into_keys()
            .collect::<BTreeSet<_>>();
        let extracted_dependencies = self
            .statement_dependencies
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        ensure!(
            graph_dependencies == extracted_dependencies,
            "statement dependency set disagrees with the expression graph for {}",
            self.name
        );

        match (&self.value_graph, &self.value_stats) {
            (Some(graph), Some(stats)) => {
                graph
                    .validate()
                    .with_context(|| format!("invalid value graph for {}", self.name))?;
                ensure!(
                    graph.occurrence_node_count() == stats.nodes,
                    "value graph occurrence count disagrees with value statistics for {}",
                    self.name
                );
                let graph_dependencies = graph
                    .constant_occurrences()
                    .into_keys()
                    .collect::<BTreeSet<_>>();
                let extracted_dependencies = self
                    .proof_dependencies
                    .iter()
                    .cloned()
                    .collect::<BTreeSet<_>>();
                ensure!(
                    graph_dependencies == extracted_dependencies,
                    "value dependency set disagrees with the expression graph for {}",
                    self.name
                );
            }
            (None, None) => {}
            _ => bail!(
                "value graph and value statistics disagree for {}",
                self.name
            ),
        }
        ensure!(
            self.has_value == self.value_graph.is_some(),
            "hasValue disagrees with the value graph for {}",
            self.name
        );
        if let Some(proof_steps) = &self.proof_steps {
            ensure!(
                self.has_value,
                "proof steps recorded for valueless declaration {}",
                self.name
            );
            validate_proof_steps(proof_steps)
                .with_context(|| format!("invalid proof steps for {}", self.name))?;
        }
        Ok(())
    }
}

pub fn validate_snapshot(snapshot: &ExtractionSnapshot) -> Result<()> {
    ensure!(
        snapshot.schema_version == 6,
        "unsupported extraction schema"
    );
    ensure!(
        !snapshot.imported_modules.is_empty(),
        "snapshot has no imported modules"
    );
    let mut declarations = BTreeSet::new();
    let symbols = snapshot
        .symbols
        .iter()
        .map(|symbol| symbol.name.as_str())
        .collect::<BTreeSet<_>>();
    ensure!(symbols.len() == snapshot.symbols.len(), "duplicate symbols");
    for declaration in &snapshot.declarations {
        declaration.validate()?;
        ensure!(
            declarations.insert(declaration.name.as_str()),
            "duplicate declaration {}",
            declaration.name
        );
        ensure!(
            symbols.contains(declaration.name.as_str()),
            "missing symbol metadata for {}",
            declaration.name
        );
        for dependency in declaration
            .statement_dependencies
            .iter()
            .chain(&declaration.proof_dependencies)
        {
            ensure!(
                symbols.contains(dependency.as_str()),
                "dependency target {dependency} is absent from the symbol table"
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::model::{ExpressionGraph, ExpressionNode};

    #[test]
    fn reconstructs_dag_multiplicity() {
        let graph = ExpressionGraph {
            root: 2,
            nodes: vec![
                ExpressionNode {
                    id: 0,
                    kind: "constant".into(),
                    children: vec![],
                    name: Some("x".into()),
                    index: None,
                    value: None,
                    binder_info: None,
                    universe_levels: vec![],
                },
                ExpressionNode {
                    id: 1,
                    kind: "application".into(),
                    children: vec![0, 0],
                    name: None,
                    index: None,
                    value: None,
                    binder_info: None,
                    universe_levels: vec![],
                },
                ExpressionNode {
                    id: 2,
                    kind: "application".into(),
                    children: vec![1, 0],
                    name: None,
                    index: None,
                    value: None,
                    binder_info: None,
                    universe_levels: vec![],
                },
            ],
        };
        assert_eq!(graph.occurrence_multiplicities(), vec![3, 1, 1]);
        assert_eq!(graph.occurrence_node_count(), 5);
        assert_eq!(graph.constant_occurrences()["x"], 3);
    }
}
