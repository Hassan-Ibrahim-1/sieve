use anyhow::{Context, Result, bail};
use serde::Serialize;

use sieve::analysis::corpus::AnalysisCorpus;
use sieve::analysis::dependency::DependencyConfig;
use sieve::analysis::filters::{AnalysisFilter, DependencyLayer, LayerSelection};
use sieve::analysis::structure::StructuralConfig;
use sieve::cli::{Cli, Command, usage};
use sieve::extraction::extract;
use sieve::report::{OutputFormat, csv, json, text};

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

fn run(cli: &Cli, corpus: &AnalysisCorpus) -> Result<()> {
    match &cli.command {
        Command::Help => print!("{}", usage()),
        Command::Summary => {
            let result = corpus.summary_with_histogram(&cli.filter, &cli.histogram_buckets);
            emit(cli.format, &result, || {
                text::summary(
                    &result,
                    corpus.snapshot().schema_version,
                    &corpus.snapshot().lean_version,
                    &corpus.snapshot().imported_module,
                )
            })?;
        }
        Command::Declaration(name) => show_declaration(cli, corpus, name)?,
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
        Command::LegacyDeclarations(names) => {
            let filter = AnalysisFilter::all();
            let summary = corpus.summary_with_histogram(&filter, &cli.histogram_buckets);
            print!(
                "{}",
                text::summary(
                    &summary,
                    corpus.snapshot().schema_version,
                    &corpus.snapshot().lean_version,
                    &corpus.snapshot().imported_module
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
    let snapshot = extract(&cli.extraction_targets())?;
    let corpus = AnalysisCorpus::new(snapshot)?;
    run(&cli, &corpus)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_and_navigates_a_requested_declaration() {
        let target = "intervalIntegral.integral_deriv_eq_sub'".to_owned();
        let corpus = AnalysisCorpus::new(
            extract(std::slice::from_ref(&target)).expect("targeted FTC extraction should succeed"),
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
            extract(std::slice::from_ref(&target)).expect("targeted FTC extraction should succeed"),
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
    #[ignore = "slow full-Mathlib integration test"]
    fn extracts_an_analysis_ready_ftc_module() {
        let corpus = AnalysisCorpus::new(extract(&[]).expect("FTC extraction should succeed"))
            .expect("valid snapshot");
        assert_eq!(corpus.snapshot().schema_version, 3);
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
    }
}
