use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result, bail, ensure};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExprStats {
    nodes: usize,
    max_depth: usize,
    bound_variables: usize,
    free_variables: usize,
    metavariables: usize,
    sorts: usize,
    constants: usize,
    applications: usize,
    lambdas: usize,
    foralls: usize,
    lets: usize,
    literals: usize,
    metadata: usize,
    projections: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExpressionNode {
    id: usize,
    kind: String,
    children: Vec<usize>,
    name: Option<String>,
    index: Option<usize>,
    value: Option<String>,
    binder_info: Option<String>,
    universe_levels: Vec<String>,
}

impl ExpressionNode {
    fn label(&self) -> String {
        let mut details = Vec::new();
        if let Some(name) = &self.name {
            details.push(name.clone());
        }
        if let Some(index) = self.index {
            details.push(format!("#{index}"));
        }
        if let Some(binder_info) = &self.binder_info {
            details.push(binder_info.clone());
        }
        if let Some(value) = &self.value {
            details.push(value.clone());
        }
        if !self.universe_levels.is_empty() {
            details.push(format!("levels={}", self.universe_levels.join(",")));
        }

        if details.is_empty() {
            self.kind.clone()
        } else {
            format!("{} {}", self.kind, details.join(" "))
        }
    }
}

#[derive(Debug, Deserialize)]
struct ExpressionGraph {
    root: usize,
    nodes: Vec<ExpressionNode>,
}

impl ExpressionGraph {
    fn validate(&self) -> Result<()> {
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

    /// Number of paths from the root to each hash-consed node. This restores
    /// occurrence counts that are intentionally collapsed in the DAG.
    fn occurrence_multiplicities(&self) -> Vec<usize> {
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

    fn occurrence_node_count(&self) -> usize {
        self.occurrence_multiplicities().into_iter().sum()
    }

    fn constant_occurrences(&self) -> BTreeMap<String, usize> {
        let multiplicities = self.occurrence_multiplicities();
        let mut occurrences = BTreeMap::new();

        for node in &self.nodes {
            if node.kind == "constant"
                && let Some(name) = &node.name
            {
                *occurrences.entry(name.clone()).or_default() += multiplicities[node.id];
            }
        }

        occurrences
    }

    /// Computes structural fingerprints in postorder. Binder names are
    /// omitted in alpha mode because bound variables use de Bruijn indices.
    /// Hash matches are candidates for comparison, not proof of equality.
    fn structural_fingerprints(&self, alpha: bool) -> Vec<u64> {
        let mut fingerprints: Vec<u64> = Vec::with_capacity(self.nodes.len());

        for node in &self.nodes {
            let mut hasher = DefaultHasher::new();
            node.kind.hash(&mut hasher);
            let is_binder = matches!(node.kind.as_str(), "lambda" | "forall" | "let");
            if !alpha || !is_binder {
                node.name.hash(&mut hasher);
            }
            node.index.hash(&mut hasher);
            node.value.hash(&mut hasher);
            node.binder_info.hash(&mut hasher);
            node.universe_levels.hash(&mut hasher);
            for &child in &node.children {
                fingerprints[child].hash(&mut hasher);
            }
            fingerprints.push(hasher.finish());
        }

        fingerprints
    }

    fn root_fingerprint(&self, alpha: bool) -> u64 {
        self.structural_fingerprints(alpha)[self.root]
    }

    fn render_tree(&self, max_depth: usize) -> String {
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

#[derive(Debug, Deserialize)]
struct SourcePosition {
    line: usize,
    column: usize,
}

#[derive(Debug, Deserialize)]
struct SourceRange {
    start: SourcePosition,
    end: SourcePosition,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeclarationSnapshot {
    name: String,
    kind: String,
    module_name: Option<String>,
    is_internal: bool,
    is_private: bool,
    is_unsafe: bool,
    is_partial: bool,
    doc_string: Option<String>,
    source_range: Option<SourceRange>,
    level_parameters: Vec<String>,
    r#type: String,
    has_value: bool,
    type_stats: ExprStats,
    value_stats: Option<ExprStats>,
    type_graph: ExpressionGraph,
    value_graph: Option<ExpressionGraph>,
    statement_dependencies: Vec<String>,
    proof_dependencies: Vec<String>,
    axioms: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SymbolSnapshot {
    name: String,
    kind: String,
    module_name: Option<String>,
    is_internal: bool,
    is_private: bool,
    is_unsafe: bool,
    is_partial: bool,
    is_class: bool,
    is_instance: bool,
    is_projection: bool,
}

impl DeclarationSnapshot {
    fn validate(&self) -> Result<()> {
        self.type_graph
            .validate()
            .with_context(|| format!("invalid type graph for {}", self.name))?;
        ensure!(
            self.type_graph.occurrence_node_count() == self.type_stats.nodes,
            "type graph occurrence count disagrees with type statistics for {}",
            self.name
        );

        let graph_statement_dependencies = self
            .type_graph
            .constant_occurrences()
            .into_keys()
            .collect::<BTreeSet<_>>();
        let extracted_statement_dependencies = self
            .statement_dependencies
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        ensure!(
            graph_statement_dependencies == extracted_statement_dependencies,
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

        Ok(())
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExtractionSnapshot {
    schema_version: usize,
    lean_version: String,
    imported_module: String,
    declarations: Vec<DeclarationSnapshot>,
    symbols: Vec<SymbolSnapshot>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum DependencyLayer {
    Statement,
    Value,
}

#[derive(Debug)]
struct DependencyEdge {
    source: usize,
    target: String,
    layer: DependencyLayer,
    occurrences: usize,
}

struct AnalysisData {
    snapshot: ExtractionSnapshot,
    declaration_index: HashMap<String, usize>,
    symbol_index: HashMap<String, usize>,
    dependency_edges: Vec<DependencyEdge>,
    incoming_edges: HashMap<String, Vec<usize>>,
}

impl AnalysisData {
    fn new(snapshot: ExtractionSnapshot) -> Result<Self> {
        ensure!(
            snapshot.schema_version == 3,
            "unsupported extraction schema"
        );

        let mut declaration_index = HashMap::new();
        let mut symbol_index = HashMap::new();
        let mut dependency_edges = Vec::new();
        let mut incoming_edges: HashMap<String, Vec<usize>> = HashMap::new();

        for (index, symbol) in snapshot.symbols.iter().enumerate() {
            ensure!(
                symbol_index.insert(symbol.name.clone(), index).is_none(),
                "duplicate symbol {}",
                symbol.name
            );
        }

        for (source, declaration) in snapshot.declarations.iter().enumerate() {
            declaration.validate()?;
            ensure!(
                declaration_index
                    .insert(declaration.name.clone(), source)
                    .is_none(),
                "duplicate declaration {}",
                declaration.name
            );

            let layers = [
                (DependencyLayer::Statement, Some(&declaration.type_graph)),
                (DependencyLayer::Value, declaration.value_graph.as_ref()),
            ];

            for (layer, graph) in layers {
                let Some(graph) = graph else { continue };
                for (target, occurrences) in graph.constant_occurrences() {
                    ensure!(
                        symbol_index.contains_key(&target),
                        "dependency target {target} is absent from the symbol table"
                    );
                    let edge_id = dependency_edges.len();
                    dependency_edges.push(DependencyEdge {
                        source,
                        target: target.clone(),
                        layer,
                        occurrences,
                    });
                    incoming_edges.entry(target).or_default().push(edge_id);
                }
            }
        }

        Ok(Self {
            snapshot,
            declaration_index,
            symbol_index,
            dependency_edges,
            incoming_edges,
        })
    }

    fn declaration(&self, name: &str) -> Option<&DeclarationSnapshot> {
        self.declaration_index
            .get(name)
            .map(|&index| &self.snapshot.declarations[index])
    }

    fn internal_dependency_edge_count(&self) -> usize {
        self.dependency_edges
            .iter()
            .filter(|edge| self.declaration_index.contains_key(&edge.target))
            .count()
    }

    fn module_dependency_occurrences(&self) -> BTreeMap<(String, String, DependencyLayer), usize> {
        let mut module_edges = BTreeMap::new();

        for edge in &self.dependency_edges {
            let source_module = self.snapshot.declarations[edge.source]
                .module_name
                .as_deref()
                .unwrap_or("<unknown>");
            let target_symbol = &self.snapshot.symbols[self.symbol_index[&edge.target]];
            let target_module = target_symbol.module_name.as_deref().unwrap_or("<unknown>");
            *module_edges
                .entry((
                    source_module.to_owned(),
                    target_module.to_owned(),
                    edge.layer,
                ))
                .or_default() += edge.occurrences;
        }

        module_edges
    }

    fn alpha_statement_fingerprint_groups(&self) -> HashMap<u64, Vec<usize>> {
        let mut groups: HashMap<u64, Vec<usize>> = HashMap::new();
        for (declaration_id, declaration) in self.snapshot.declarations.iter().enumerate() {
            groups
                .entry(declaration.type_graph.root_fingerprint(true))
                .or_default()
                .push(declaration_id);
        }
        groups
    }

    fn print_summary(&self) {
        let declarations = &self.snapshot.declarations;
        let symbols = &self.snapshot.symbols;
        let theorem_count = declarations
            .iter()
            .filter(|declaration| declaration.kind == "theorem")
            .count();
        let generated_count = declarations
            .iter()
            .filter(|declaration| declaration.is_internal || declaration.is_private)
            .count();
        let type_occurrences = declarations
            .iter()
            .map(|declaration| declaration.type_stats.nodes)
            .sum::<usize>();
        let value_occurrences = declarations
            .iter()
            .filter_map(|declaration| declaration.value_stats.as_ref())
            .map(|stats| stats.nodes)
            .sum::<usize>();
        let type_unique_nodes = declarations
            .iter()
            .map(|declaration| declaration.type_graph.nodes.len())
            .sum::<usize>();
        let value_unique_nodes = declarations
            .iter()
            .filter_map(|declaration| declaration.value_graph.as_ref())
            .map(|graph| graph.nodes.len())
            .sum::<usize>();
        let statement_dependency_edges = self
            .dependency_edges
            .iter()
            .filter(|edge| edge.layer == DependencyLayer::Statement)
            .count();
        let value_dependency_edges = self.dependency_edges.len() - statement_dependency_edges;
        let dependency_occurrences = self
            .dependency_edges
            .iter()
            .map(|edge| edge.occurrences)
            .sum::<usize>();
        let declarations_with_dependencies = self
            .dependency_edges
            .iter()
            .map(|edge| edge.source)
            .collect::<BTreeSet<_>>()
            .len();
        let module_dependency_edges = self.module_dependency_occurrences().len();
        let class_count = symbols.iter().filter(|symbol| symbol.is_class).count();
        let instance_count = symbols.iter().filter(|symbol| symbol.is_instance).count();
        let projection_count = symbols.iter().filter(|symbol| symbol.is_projection).count();
        let unsafe_or_partial_count = symbols
            .iter()
            .filter(|symbol| symbol.is_unsafe || symbol.is_partial)
            .count();
        let internal_or_private_symbol_count = symbols
            .iter()
            .filter(|symbol| symbol.is_internal || symbol.is_private)
            .count();
        let theorem_symbols = symbols
            .iter()
            .filter(|symbol| symbol.kind == "theorem")
            .count();
        let referenced_modules = symbols
            .iter()
            .filter_map(|symbol| symbol.module_name.as_deref())
            .collect::<BTreeSet<_>>()
            .len();
        let repeated_statement_shapes = self
            .alpha_statement_fingerprint_groups()
            .values()
            .filter(|group| group.len() > 1)
            .count();

        println!(
            "Sieve schema {} — Lean {} — {}",
            self.snapshot.schema_version, self.snapshot.lean_version, self.snapshot.imported_module
        );
        println!(
            "corpus: {} declarations, {theorem_count} theorems, {generated_count} internal/private",
            declarations.len()
        );
        println!(
            "symbol table: {} symbols from {referenced_modules} modules; {theorem_symbols} theorems, \
             {class_count} classes, {instance_count} instances, {projection_count} projections, \
             {internal_or_private_symbol_count} internal/private, {unsafe_or_partial_count} unsafe/partial",
            self.symbol_index.len()
        );
        println!(
            "expressions: statement {type_occurrences} occurrences/{type_unique_nodes} unique nodes; \
             value {value_occurrences} occurrences/{value_unique_nodes} unique nodes"
        );
        println!(
            "structural comparison: {repeated_statement_shapes} repeated alpha-normalized statement fingerprint groups"
        );
        println!(
            "dependencies: {} typed edges ({statement_dependency_edges} statement, \
             {value_dependency_edges} value), {dependency_occurrences} occurrences",
            self.dependency_edges.len(),
        );
        println!(
            "dependency coverage: {declarations_with_dependencies} source declarations, {} edges \
             within this corpus, {} referenced declarations, {module_dependency_edges} module edges",
            self.internal_dependency_edge_count(),
            self.incoming_edges.len()
        );

        let mut largest_proofs = declarations
            .iter()
            .filter_map(|declaration| {
                declaration
                    .value_stats
                    .as_ref()
                    .map(|stats| (stats.nodes, declaration.name.as_str()))
            })
            .collect::<Vec<_>>();
        largest_proofs.sort_unstable_by(|left, right| right.cmp(left));

        println!("largest elaborated values:");
        for (nodes, name) in largest_proofs.into_iter().take(10) {
            println!("  {nodes:>8}  {name}");
        }
    }

    fn print_declaration(&self, declaration: &DeclarationSnapshot, tree_depth: Option<usize>) {
        let value_nodes = declaration
            .value_stats
            .as_ref()
            .map_or(0, |stats| stats.nodes);
        let value_unique_nodes = declaration
            .value_graph
            .as_ref()
            .map_or(0, |graph| graph.nodes.len());

        println!("\n{} [{}]", declaration.name, declaration.kind);
        println!(
            "  module: {}",
            declaration.module_name.as_deref().unwrap_or("<unknown>")
        );
        if let Some(range) = &declaration.source_range {
            println!(
                "  source: {}:{}–{}:{}",
                range.start.line, range.start.column, range.end.line, range.end.column
            );
        }
        if let Some(doc_string) = &declaration.doc_string {
            println!("  documentation: {}", doc_string.replace('\n', " "));
        }
        println!(
            "  flags: internal={}, private={}, unsafe={}, partial={}",
            declaration.is_internal,
            declaration.is_private,
            declaration.is_unsafe,
            declaration.is_partial
        );
        println!(
            "  universe parameters: {}",
            declaration.level_parameters.join(", ")
        );
        println!("  type: {}", declaration.r#type);
        println!(
            "  expression occurrences: type={}, value={value_nodes}",
            declaration.type_stats.nodes
        );
        println!(
            "  unique graph nodes: type={}, value={value_unique_nodes}",
            declaration.type_graph.nodes.len()
        );
        println!(
            "  type shape: depth={}, constants={}, applications={}, binders={} (forall={}, lambda={}, let={})",
            declaration.type_stats.max_depth,
            declaration.type_stats.constants,
            declaration.type_stats.applications,
            declaration.type_stats.foralls
                + declaration.type_stats.lambdas
                + declaration.type_stats.lets,
            declaration.type_stats.foralls,
            declaration.type_stats.lambdas,
            declaration.type_stats.lets
        );
        println!(
            "  type variables: bound={}, free={}, metavariables={}; other nodes: sorts={}, literals={}, metadata={}, projections={}",
            declaration.type_stats.bound_variables,
            declaration.type_stats.free_variables,
            declaration.type_stats.metavariables,
            declaration.type_stats.sorts,
            declaration.type_stats.literals,
            declaration.type_stats.metadata,
            declaration.type_stats.projections
        );
        println!("  has value: {}", declaration.has_value);
        println!(
            "  direct dependencies: statement={}, value={}",
            declaration.statement_dependencies.len(),
            declaration.proof_dependencies.len()
        );
        println!("  transitive axioms: {}", declaration.axioms.join(", "));

        if let (Some(depth), Some(graph)) = (tree_depth, &declaration.value_graph) {
            println!("  value tree through depth {depth}:");
            print!("{}", graph.render_tree(depth));
        }
    }
}

fn lake_path() -> PathBuf {
    if let Some(path) = std::env::var_os("SIEVE_LAKE") {
        return path.into();
    }

    if let Some(home) = std::env::var_os("HOME") {
        let elan_lake = PathBuf::from(home).join(".elan/bin/lake");
        if elan_lake.is_file() {
            return elan_lake;
        }
    }

    "lake".into()
}

fn extract(targets: &[String]) -> Result<ExtractionSnapshot> {
    let mut command = Command::new(lake_path());
    command.args(["exe", "sieve_extract"]);
    command.args(targets);

    let output = command
        .output()
        .context("failed to start the Lean extractor through Lake")?;

    if !output.status.success() {
        bail!(
            "Lean extractor failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    serde_json::from_slice(&output.stdout).context("Lean extractor emitted invalid JSON")
}

fn parse_args() -> Result<(Option<usize>, Vec<String>)> {
    let mut tree_depth = None;
    let mut targets = Vec::new();
    let mut args = std::env::args().skip(1);

    while let Some(argument) = args.next() {
        if argument == "--tree-depth" {
            let depth = args.next().context("--tree-depth requires a number")?;
            tree_depth = Some(depth.parse().context("invalid tree depth")?);
        } else {
            targets.push(argument);
        }
    }

    Ok((tree_depth, targets))
}

fn main() -> Result<()> {
    let (tree_depth, targets) = parse_args()?;
    let analysis = AnalysisData::new(extract(&targets)?)?;
    analysis.print_summary();

    if !targets.is_empty() {
        for target in &targets {
            let declaration = analysis
                .declaration(target)
                .with_context(|| format!("extracted declaration {target} is missing"))?;
            analysis.print_declaration(declaration, tree_depth);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "slow full-Mathlib integration test"]
    fn extracts_an_analysis_ready_ftc_module() {
        let analysis = AnalysisData::new(extract(&[]).expect("FTC extraction should succeed"))
            .expect("FTC snapshot should satisfy its invariants");

        assert_eq!(analysis.snapshot.schema_version, 3);
        assert_eq!(
            analysis.snapshot.imported_module,
            "Mathlib.MeasureTheory.Integral.IntervalIntegral.FundThmCalculus"
        );
        assert!(analysis.snapshot.declarations.len() >= 70);
        assert!(analysis.snapshot.symbols.len() > analysis.snapshot.declarations.len());
        assert!(analysis.internal_dependency_edge_count() > 0);

        let theorem = analysis
            .declaration("intervalIntegral.integral_deriv_eq_sub")
            .expect("the FTC module should contain FTC-2");

        assert_eq!(theorem.kind, "theorem");
        assert!(theorem.has_value);
        assert!(theorem.type_stats.nodes > 0);
        assert!(
            theorem
                .value_stats
                .as_ref()
                .is_some_and(|stats| stats.nodes > 0)
        );
        assert!(theorem.type_graph.nodes.len() <= theorem.type_stats.nodes);
        assert!(
            theorem
                .statement_dependencies
                .iter()
                .any(|dependency| dependency == "IntervalIntegrable")
        );
        assert!(
            theorem
                .proof_dependencies
                .iter()
                .any(|dependency| dependency == "intervalIntegral.integral_eq_sub_of_hasDerivAt")
        );
        assert!(
            theorem
                .axioms
                .iter()
                .any(|axiom| axiom == "Classical.choice")
        );
    }

    #[test]
    fn extracts_and_navigates_a_requested_declaration() {
        let target = "intervalIntegral.integral_deriv_eq_sub'".to_owned();
        let analysis = AnalysisData::new(
            extract(std::slice::from_ref(&target)).expect("targeted FTC extraction should succeed"),
        )
        .expect("targeted snapshot should satisfy its invariants");

        assert_eq!(analysis.snapshot.declarations.len(), 1);
        let declaration = analysis
            .declaration(&target)
            .expect("target declaration should be indexed");
        let value_graph = declaration
            .value_graph
            .as_ref()
            .expect("the theorem should have a proof graph");
        assert!(
            value_graph
                .constant_occurrences()
                .contains_key("intervalIntegral.integral_deriv_eq_sub")
        );
        assert!(!value_graph.render_tree(2).is_empty());
        assert_ne!(value_graph.root_fingerprint(false), 0);
        assert_ne!(value_graph.root_fingerprint(true), 0);
        assert!(analysis.symbol_index.contains_key(&target));
        assert!(
            analysis
                .snapshot
                .symbols
                .iter()
                .any(|symbol| symbol.is_class)
        );
        assert!(
            analysis
                .snapshot
                .symbols
                .iter()
                .any(|symbol| symbol.is_instance)
        );
        assert!(!analysis.module_dependency_occurrences().is_empty());
    }

    #[test]
    fn dependency_edges_preserve_occurrence_counts() {
        let target = "intervalIntegral.integral_deriv_eq_sub".to_owned();
        let analysis = AnalysisData::new(
            extract(std::slice::from_ref(&target)).expect("targeted FTC extraction should succeed"),
        )
        .expect("targeted snapshot should satisfy its invariants");

        let statement_constant_occurrences = analysis
            .dependency_edges
            .iter()
            .filter(|edge| edge.layer == DependencyLayer::Statement)
            .map(|edge| edge.occurrences)
            .sum::<usize>();
        let declaration = &analysis.snapshot.declarations[0];

        assert_eq!(
            statement_constant_occurrences,
            declaration.type_stats.constants
        );
        assert!(
            analysis
                .dependency_edges
                .iter()
                .all(|edge| edge.source == 0 && edge.occurrences > 0)
        );
    }
}
