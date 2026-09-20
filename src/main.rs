use anyhow::{Context, Result, bail};
use serde::Serialize;
use std::sync::Arc;

use sieve::analysis::corpus::AnalysisCorpus;
use sieve::analysis::dependency::DependencyConfig;
use sieve::analysis::filters::{AnalysisFilter, DependencyLayer, LayerSelection};
use sieve::analysis::lenses::LensAnalysisConfig;
use sieve::analysis::proof_steps::ProofStepGraph;
#[cfg(test)]
use sieve::analysis::proof_steps::build_proof_outline;
use sieve::analysis::structure::StructuralConfig;
use sieve::cli::{Cli, Command, usage};
use sieve::extraction::{ExtractionConfig, extract, extract_with_proof_steps};
use sieve::model::ExtractionSnapshot;
use sieve::report::{OutputFormat, csv, json, text};
use sieve::server;

fn emit<T: Serialize>(
    format: OutputFormat,
    value: &T,
    text_value: impl FnOnce() -> String,
) -> Result<()> {
    match format {
        OutputFormat::Text => print!("{}", text_value()),
        OutputFormat::Json => println!("{}", json::render(value)?),
        OutputFormat::Csv => bail!("CSV is available through `sieve export <table>`"),
    }
    Ok(())
}

fn dependency_config(cli: &Cli) -> DependencyConfig {
    DependencyConfig {
        filter: cli.filter.clone(),
        weighted: cli.weighted,
        max_depth: cli.max_depth,
        limit: cli.limit,
    }
}

fn structural_config(cli: &Cli) -> StructuralConfig {
    StructuralConfig {
        filter: cli.filter.clone(),
        mode: cli.structural_mode,
        minimum_size: cli.minimum_size,
        minimum_support: cli.minimum_support,
        limit: cli.limit,
    }
}

fn lens_config(cli: &Cli) -> LensAnalysisConfig {
    LensAnalysisConfig {
        filter: cli.filter.clone(),
        layer_explicit: cli.layer_explicit,
        max_depth: cli.max_depth,
        limit: cli.limit,
        representative_path_limit: 3,
    }
}

fn warn_if_empty_corpus(snapshot: &ExtractionSnapshot, targets: &[String]) {
    if targets.is_empty() && snapshot.declarations.is_empty() {
        eprintln!(
            "warning: imported module(s) {} contribute no declarations defined directly in those modules; Sieve does not traverse imported dependencies",
            snapshot.imported_modules.join(", ")
        );
    }
}

fn run(cli: &Cli, corpus: &Arc<AnalysisCorpus>) -> Result<()> {
    match &cli.command {
        Command::Help => print!("{}", usage()),
        Command::Discover => {
            let result = corpus.discover_lenses(&cli.selected_lenses(), &lens_config(cli));
            emit(cli.format, &result, || text::discovery(&result))?;
        }
        Command::Inspect(name) => {
            let result = corpus.inspect_lenses(name, &cli.selected_lenses(), &lens_config(cli))?;
            emit(cli.format, &result, || text::theorem_lenses(&result))?;
        }
        Command::Summary => {
            let result = corpus.summary_with_histogram(&cli.filter, &cli.histogram_buckets);
            emit(cli.format, &result, || {
                text::summary(
                    &result,
                    corpus.snapshot().schema_version,
                    &corpus.snapshot().lean_version,
                    &corpus.snapshot().imported_modules.join(", "),
                )
            })?;
        }
        Command::Declaration(name) => show_declaration(cli, corpus, name)?,
        Command::ProofSteps(name) => {
            let declaration = corpus
                .declaration(name)
                .with_context(|| format!("extracted declaration {name} is missing"))?;
            let extraction = declaration
                .proof_steps
                .as_ref()
                .with_context(|| format!("proof-step extraction is absent for {name}"))?;
            let graph = ProofStepGraph::new(extraction)?;
            let report = graph.report(name, cli.selected_step)?;
            match cli.format {
                OutputFormat::Text => print!("{}", text::proof_steps(&report)),
                OutputFormat::Json => println!("{}", json::render(&report)?),
                OutputFormat::Csv => {
                    bail!("proof-step reports do not have a flat CSV representation")
                }
            }
        }
        Command::Dependencies(name) => {
            let config = dependency_config(cli);
            if cli.transitive {
                let result = corpus
                    .traverse_dependencies(name, false, &config)
                    .with_context(|| format!("unknown declaration {name}"))?;
                emit(cli.format, &result, || text::traversal(&result))?;
            } else {
                let result = corpus
                    .direct_dependencies(name, &config)
                    .with_context(|| format!("unknown declaration {name}"))?;
                emit(cli.format, &result, || text::dependencies(&result))?;
            }
        }
        Command::Dependents(name) => {
            let config = dependency_config(cli);
            if cli.transitive {
                let result = corpus
                    .traverse_dependencies(name, true, &config)
                    .with_context(|| format!("unknown symbol {name}"))?;
                emit(cli.format, &result, || text::traversal(&result))?;
            } else {
                let result = corpus
                    .direct_dependents(name, &config)
                    .with_context(|| format!("unknown symbol {name}"))?;
                emit(cli.format, &result, || text::dependencies(&result))?;
            }
        }
        Command::Path(source, target) => {
            let result = corpus
                .shortest_dependency_path(source, target, &dependency_config(cli))
                .with_context(|| format!("no dependency path from {source} to {target}"))?;
            emit(cli.format, &result, || text::path(&result))?;
        }
        Command::Rank(metric) => {
            let result = corpus
                .rank(metric, &cli.filter, cli.limit)
                .with_context(|| format!("unknown metric {metric}"))?;
            emit(cli.format, &result, || text::ranking(&result))?;
        }
        Command::Compare(left, right) => {
            let layers = match cli.filter.layer {
                LayerSelection::Statement => vec![DependencyLayer::Statement],
                LayerSelection::Proof => vec![DependencyLayer::Proof],
                LayerSelection::Both => vec![DependencyLayer::Statement, DependencyLayer::Proof],
            };
            let result = layers
                .into_iter()
                .filter_map(|layer| corpus.compare_declarations(left, right, layer))
                .collect::<Vec<_>>();
            if result.is_empty() {
                bail!("the selected expression layer is absent from one or both declarations");
            }
            emit(cli.format, &result, || {
                result.iter().map(text::comparison).collect()
            })?;
        }
        Command::Duplicates => {
            let result = corpus.find_duplicates(&structural_config(cli));
            emit(cli.format, &result, || text::duplicates(&result))?;
        }
        Command::RepeatedStructures => {
            let result = corpus.repeated_subexpressions(&structural_config(cli));
            emit(cli.format, &result, || text::repeated(&result))?;
        }
        Command::Modules => {
            let result = corpus.module_dependency_occurrences(&cli.filter).into_iter().map(|((source, target, layer), weight)| serde_json::json!({ "sourceModule": source, "targetModule": target, "layer": layer, "weight": weight })).collect::<Vec<_>>();
            emit(cli.format, &result, || {
                let mut out = String::new();
                for row in &result {
                    out.push_str(&format!(
                        "{:>8}  {} -> {} [{}]\n",
                        row["weight"],
                        row["sourceModule"].as_str().unwrap(),
                        row["targetModule"].as_str().unwrap(),
                        row["layer"].as_str().unwrap()
                    ));
                }
                out
            })?;
        }
        Command::Graph => {
            let config = dependency_config(cli);
            let centrality = corpus.centrality(&config);
            let connected = corpus.connected_components(&config);
            let components = corpus.strongly_connected_components(&config);
            let structure = corpus.graph_structure(&config);
            let result = serde_json::json!({ "centrality": centrality, "connectedComponents": connected, "stronglyConnectedComponents": components, "structure": structure });
            emit(cli.format, &result, || {
                format!(
                    "{}{}",
                    text::centrality(&centrality, cli.limit),
                    text::graph_structure(&structure)
                )
            })?;
        }
        Command::Export(table) => {
            let output = match table.as_str() {
                "nodes" => csv::nodes(corpus, &cli.filter),
                "edges" => csv::edges(corpus, &cli.filter),
                "modules" => csv::modules(corpus, &cli.filter),
                "metrics" => csv::metrics(corpus, &cli.filter),
                "duplicates" => {
                    csv::structural_matches(&corpus.find_duplicates(&structural_config(cli)))
                }
                "repeated" => csv::repeated_occurrences(
                    &corpus.repeated_subexpressions(&structural_config(cli)),
                ),
                _ => bail!("unknown export table {table}"),
            };
            print!("{output}");
        }
        Command::Serve => server::serve(Arc::clone(corpus), cli.port)?,
        Command::LegacyDeclarations(names) => {
            let filter = AnalysisFilter::all();
            let summary = corpus.summary_with_histogram(&filter, &cli.histogram_buckets);
            print!(
                "{}",
                text::summary(
                    &summary,
                    corpus.snapshot().schema_version,
                    &corpus.snapshot().lean_version,
                    &corpus.snapshot().imported_modules.join(", ")
                )
            );
            for name in names {
                show_declaration(cli, corpus, name)?;
            }
        }
    }
    Ok(())
}

fn show_declaration(cli: &Cli, corpus: &AnalysisCorpus, name: &str) -> Result<()> {
    let declaration = corpus
        .declaration(name)
        .with_context(|| format!("extracted declaration {name} is missing"))?;
    let metrics = corpus
        .declaration_metrics(name)
        .expect("declaration is indexed");
    let tree = cli.tree_depth.and_then(|depth| {
        declaration
            .value_graph
            .as_ref()
            .map(|graph| graph.render_tree(depth))
    });
    if cli.format == OutputFormat::Json {
        println!(
            "{}",
            json::render(
                &serde_json::json!({ "declaration": declaration, "metrics": metrics, "valueTree": tree })
            )?
        );
    } else if cli.format == OutputFormat::Text {
        print!(
            "{}",
            text::declaration(declaration, &metrics, tree.as_deref())
        );
    } else {
        bail!("declaration reports do not have a flat CSV representation");
    }
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse()?;
    if matches!(cli.command, Command::Help) {
        print!("{}", usage());
        return Ok(());
    }
    let extraction = ExtractionConfig::new(&cli.project, cli.imports.clone())?;
    let targets = cli.extraction_targets();
    let mut snapshot = if matches!(cli.command, Command::ProofSteps(_) | Command::Serve) {
        extract_with_proof_steps(&extraction, &targets)?
    } else {
        extract(&extraction, &targets)?
    };
    warn_if_empty_corpus(&snapshot, &targets);
    if cli.needs_targeted_proof_steps()
        && let Command::Inspect(name) = &cli.command
    {
        let full_declaration = snapshot
            .declarations
            .iter_mut()
            .find(|declaration| declaration.name == *name)
            .with_context(|| format!("unknown theorem {name}"))?;
        if full_declaration.has_value {
            let targeted = extract_with_proof_steps(&extraction, std::slice::from_ref(name))?;
            let proof_steps = targeted
                .declarations
                .into_iter()
                .find(|declaration| declaration.name == *name)
                .and_then(|declaration| declaration.proof_steps)
                .with_context(|| format!("proof-step extraction is absent for {name}"))?;
            full_declaration.proof_steps = Some(proof_steps);
        }
    }
    let corpus = Arc::new(AnalysisCorpus::new(snapshot)?);
    run(&cli, &corpus)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ftc_extraction() -> ExtractionConfig {
        ExtractionConfig::new(
            ".",
            vec!["Mathlib.MeasureTheory.Integral.IntervalIntegral.FundThmCalculus".into()],
        )
        .expect("valid FTC extraction configuration")
    }

    #[test]
    fn extracts_and_navigates_a_requested_declaration() {
        let target = "intervalIntegral.integral_deriv_eq_sub'".to_owned();
        let corpus = AnalysisCorpus::new(
            extract(&ftc_extraction(), std::slice::from_ref(&target))
                .expect("targeted FTC extraction should succeed"),
        )
        .expect("valid snapshot");
        let declaration = corpus
            .declaration(&target)
            .expect("target declaration should be indexed");
        let value_graph = declaration.value_graph.as_ref().expect("proof graph");
        assert!(
            value_graph
                .constant_occurrences()
                .contains_key("intervalIntegral.integral_deriv_eq_sub")
        );
        assert!(!value_graph.render_tree(2).is_empty());
        assert_ne!(
            value_graph.root_fingerprint(sieve::analysis::structure::StructuralMode::Exact),
            0
        );
        assert!(corpus.symbols().iter().any(|symbol| symbol.is_class));
        assert!(
            !corpus
                .module_dependency_occurrences(&AnalysisFilter::all())
                .is_empty()
        );
    }

    #[test]
    fn dependency_edges_preserve_occurrence_counts() {
        let target = "intervalIntegral.integral_deriv_eq_sub".to_owned();
        let corpus = AnalysisCorpus::new(
            extract(&ftc_extraction(), std::slice::from_ref(&target))
                .expect("targeted FTC extraction should succeed"),
        )
        .expect("valid snapshot");
        let occurrences = corpus
            .edges()
            .iter()
            .filter(|edge| edge.layer == DependencyLayer::Statement)
            .map(|edge| edge.occurrences)
            .sum::<usize>();
        assert_eq!(occurrences, corpus.declarations()[0].type_stats.constants);
    }

    #[test]
    fn extracts_the_reference_ftc_argument_as_scoped_steps() {
        let target = "intervalIntegral.integral_eq_sub_of_hasDeriv_right_of_le".to_owned();
        let corpus = AnalysisCorpus::new(
            extract_with_proof_steps(&ftc_extraction(), std::slice::from_ref(&target))
                .expect("reference FTC step extraction should succeed"),
        )
        .expect("valid proof-step snapshot");
        let extraction = corpus
            .declaration(&target)
            .and_then(|declaration| declaration.proof_steps.as_ref())
            .expect("reference theorem should include proof steps");
        assert!(extraction.complete);
        let expected_spine = [
            "SeparatingDual.eq_iff_forall_dual_eq",
            "ContinuousLinearMap.intervalIntegral_comp_comm",
            "ContinuousLinearMap.map_sub",
            "intervalIntegral.integral_eq_sub_of_hasDeriv_right_of_le_real",
        ];
        for expected in expected_spine {
            assert!(
                extraction
                    .steps
                    .iter()
                    .any(|step| step.named_references.iter().any(|name| name == expected)),
                "missing reference step {expected}"
            );
        }
        let scalar_ftc = extraction
            .steps
            .iter()
            .find(|step| {
                step.named_references.iter().any(|name| {
                    name == "intervalIntegral.integral_eq_sub_of_hasDeriv_right_of_le_real"
                })
            })
            .expect("scalar FTC step");
        assert!(
            scalar_ftc
                .context
                .iter()
                .any(|entry| entry.user_name == "g")
        );
        let graph = ProofStepGraph::new(extraction).expect("valid reference step graph");
        assert!(
            !graph
                .paths_to_conclusion(scalar_ftc.id, 10)
                .expect("conclusion exists")
                .0
                .is_empty()
        );
        let outline = build_proof_outline(extraction).expect("valid proof outline");
        for expected in expected_spine {
            assert!(
                outline
                    .retained_nodes
                    .iter()
                    .any(|step| step.named_references.iter().any(|name| name == expected)),
                "outline omitted reference step {expected}"
            );
        }
        let raw_edges = outline
            .raw_edges
            .iter()
            .map(|edge| (edge.source, edge.target))
            .collect::<std::collections::BTreeSet<_>>();
        assert!(outline.condensed_edges.iter().all(|edge| {
            edge.raw_step_path
                .windows(2)
                .all(|pair| raw_edges.contains(&(pair[0], pair[1])))
        }));
        let conclusion = outline.conclusion_step.expect("outline conclusion");
        for node in &outline.retained_nodes {
            let mut seen = std::collections::BTreeSet::from([node.raw_step_id]);
            let mut queue = std::collections::VecDeque::from([node.raw_step_id]);
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
            assert!(
                seen.contains(&conclusion),
                "retained step {} does not reach the conclusion",
                node.raw_step_id
            );
        }
    }

    #[test]
    #[ignore = "slow full-Mathlib integration test"]
    fn extracts_an_analysis_ready_ftc_module() {
        let corpus = AnalysisCorpus::new(
            extract(&ftc_extraction(), &[]).expect("FTC extraction should succeed"),
        )
        .expect("valid snapshot");
        assert_eq!(corpus.snapshot().schema_version, 6);
        assert!(corpus.declarations().len() >= 70);
        assert!(corpus.symbols().len() > corpus.declarations().len());
        assert!(corpus.internal_dependency_edge_count() > 0);
        let theorem = corpus
            .declaration("intervalIntegral.integral_deriv_eq_sub")
            .expect("FTC-2");
        assert_eq!(theorem.kind, "theorem");
        assert!(theorem.has_value);
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
        let neighbors = corpus
            .inspect_lenses(
                "intervalIntegral.integral_deriv_eq_sub",
                &[sieve::analysis::lenses::LensKind::Neighbors],
                &sieve::analysis::lenses::LensAnalysisConfig {
                    limit: 20,
                    ..Default::default()
                },
            )
            .expect("FTC theorem should be inspectable");
        let names = neighbors.neighbors[0]
            .neighbors
            .iter()
            .map(|neighbor| neighbor.name.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        assert!(names.contains("intervalIntegral.integral_deriv_eq_sub'"));
        assert!(names.contains("intervalIntegral.integral_deriv_eq_sub_uIoo"));
    }
}
