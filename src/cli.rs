use anyhow::{Context, Result, bail};

use crate::analysis::filters::{AnalysisFilter, DependencyLayer, LayerSelection};
use crate::analysis::lenses::LensKind;
use crate::analysis::structure::StructuralMode;
use crate::report::OutputFormat;

#[derive(Clone, Debug)]
pub enum Command {
    Discover,
    Inspect(String),
    Summary,
    Declaration(String),
    ProofSteps(String),
    Dependencies(String),
    Dependents(String),
    Path(String, String),
    Rank(String),
    Compare(String, String),
    Duplicates,
    RepeatedStructures,
    Modules,
    Graph,
    Export(String),
    Serve,
    LegacyDeclarations(Vec<String>),
    Help,
}

#[derive(Clone, Debug)]
pub struct Cli {
    pub command: Command,
    pub filter: AnalysisFilter,
    pub format: OutputFormat,
    pub weighted: bool,
    pub transitive: bool,
    pub max_depth: usize,
    pub limit: usize,
    pub tree_depth: Option<usize>,
    pub minimum_size: usize,
    pub minimum_support: usize,
    pub structural_mode: StructuralMode,
    pub histogram_buckets: Vec<f64>,
    pub port: u16,
    pub selected_step: Option<usize>,
    pub lens: LensKind,
    pub layer_explicit: bool,
}

impl Cli {
    pub fn parse() -> Result<Self> {
        Self::parse_from(std::env::args().skip(1))
    }

    pub fn parse_from(args: impl IntoIterator<Item = String>) -> Result<Self> {
        let mut filter = AnalysisFilter::default();
        let mut format = OutputFormat::Text;
        let mut weighted = false;
        let mut transitive = false;
        let mut max_depth = 32;
        let mut limit = 100;
        let mut tree_depth = None;
        let mut minimum_size = 4;
        let mut minimum_support = 2;
        let mut structural_mode = StructuralMode::AlphaEquivalent;
        let mut histogram_buckets =
            vec![10.0, 50.0, 100.0, 250.0, 500.0, 1_000.0, 2_500.0, 5_000.0];
        let mut port = 4173;
        let mut selected_step = None;
        let mut lens = LensKind::All;
        let mut layer_explicit = false;
        let mut positional = Vec::new();
        let mut args = args.into_iter();
        while let Some(argument) = args.next() {
            match argument.as_str() {
                "--layer" => {
                    layer_explicit = true;
                    filter.layer = match args
                        .next()
                        .context("--layer requires statement, proof, or both")?
                        .as_str()
                    {
                        "statement" => LayerSelection::Statement,
                        "proof" | "value" => LayerSelection::Proof,
                        "both" => LayerSelection::Both,
                        other => bail!("invalid layer {other}"),
                    }
                }
                "--lens" => {
                    lens = args
                        .next()
                        .context("--lens requires a lens name")?
                        .parse()?;
                }
                "--include-generated" => filter.include_generated = true,
                "--include-infrastructure" => filter.include_infrastructure = true,
                "--internal-only" => filter.internal_only = true,
                "--source-backed-only" => filter.source_backed_only = true,
                "--kind" => {
                    filter.declaration_kind =
                        Some(args.next().context("--kind requires a declaration kind")?)
                }
                "--module" => {
                    filter.module = Some(args.next().context("--module requires a module name")?)
                }
                "--weighted" => weighted = true,
                "--transitive" => transitive = true,
                "--max-depth" => {
                    max_depth = args
                        .next()
                        .context("--max-depth requires a number")?
                        .parse()
                        .context("invalid maximum depth")?
                }
                "--limit" => {
                    limit = args
                        .next()
                        .context("--limit requires a number")?
                        .parse()
                        .context("invalid result limit")?
                }
                "--tree-depth" => {
                    tree_depth = Some(
                        args.next()
                            .context("--tree-depth requires a number")?
                            .parse()
                            .context("invalid tree depth")?,
                    )
                }
                "--minimum-size" => {
                    minimum_size = args
                        .next()
                        .context("--minimum-size requires a number")?
                        .parse()
                        .context("invalid minimum size")?
                }
                "--minimum-support" => {
                    minimum_support = args
                        .next()
                        .context("--minimum-support requires a number")?
                        .parse()
                        .context("invalid minimum support")?
                }
                "--exact" => structural_mode = StructuralMode::Exact,
                "--alpha" => structural_mode = StructuralMode::AlphaEquivalent,
                "--histogram-buckets" => {
                    let value = args
                        .next()
                        .context("--histogram-buckets requires comma-separated numbers")?;
                    histogram_buckets = value
                        .split(',')
                        .map(|part| part.parse().context("invalid histogram bucket"))
                        .collect::<Result<Vec<_>>>()?;
                }
                "--format" => {
                    format = match args
                        .next()
                        .context("--format requires text, json, or csv")?
                        .as_str()
                    {
                        "text" => OutputFormat::Text,
                        "json" => OutputFormat::Json,
                        "csv" => OutputFormat::Csv,
                        other => bail!("invalid output format {other}"),
                    }
                }
                "--port" => {
                    port = args
                        .next()
                        .context("--port requires a number")?
                        .parse()
                        .context("invalid port")?
                }
                "--step" => {
                    selected_step = Some(
                        args.next()
                            .context("--step requires a numeric proof-step id")?
                            .parse()
                            .context("invalid proof-step id")?,
                    )
                }
                "-h" | "--help" => positional.push("help".into()),
                option if option.starts_with('-') => bail!("unknown option {option}"),
                _ => positional.push(argument),
            }
        }
        let command = parse_command(positional)?;
        if let (Command::Discover, LensKind::Proof | LensKind::Trust) = (&command, lens) {
            bail!("discover supports influence, bridge, neighbors, or all")
        }
        if matches!(command, Command::Discover | Command::Inspect(_)) && format == OutputFormat::Csv
        {
            bail!("discover and inspect support --format text or json")
        }
        Ok(Self {
            command,
            filter,
            format,
            weighted,
            transitive,
            max_depth,
            limit,
            tree_depth,
            minimum_size,
            minimum_support,
            structural_mode,
            histogram_buckets,
            port,
            selected_step,
            lens,
            layer_explicit,
        })
    }

    pub fn extraction_targets(&self) -> Vec<String> {
        match &self.command {
            Command::LegacyDeclarations(targets) => targets.clone(),
            Command::ProofSteps(target) => vec![target.clone()],
            _ => vec![],
        }
    }

    pub fn selected_lenses(&self) -> Vec<LensKind> {
        self.lens
            .expand_for_discovery(matches!(self.command, Command::Discover))
    }

    pub fn needs_targeted_proof_steps(&self) -> bool {
        matches!(self.command, Command::Inspect(_))
            && self
                .selected_lenses()
                .iter()
                .any(|lens| matches!(lens, LensKind::Proof | LensKind::Trust))
    }
}

fn parse_command(mut args: Vec<String>) -> Result<Command> {
    if args.is_empty() {
        return Ok(Command::Summary);
    }
    let command = args.remove(0);
    let take_one = |args: Vec<String>, usage: &str| -> Result<String> {
        if args.len() != 1 {
            bail!("usage: {usage}");
        }
        Ok(args.into_iter().next().unwrap())
    };
    Ok(match command.as_str() {
        "discover" => {
            if !args.is_empty() {
                bail!("discover takes no arguments");
            }
            Command::Discover
        }
        "inspect" => Command::Inspect(take_one(args, "sieve inspect <name>")?),
        "summary" => {
            if !args.is_empty() {
                bail!("summary takes no arguments");
            }
            Command::Summary
        }
        "declaration" => Command::Declaration(take_one(args, "sieve declaration <name>")?),
        "proof-steps" => Command::ProofSteps(take_one(args, "sieve proof-steps <name>")?),
        "dependencies" => Command::Dependencies(take_one(args, "sieve dependencies <name>")?),
        "dependents" => Command::Dependents(take_one(args, "sieve dependents <name>")?),
        "path" => {
            if args.len() != 2 {
                bail!("usage: sieve path <source> <target>");
            }
            Command::Path(args.remove(0), args.remove(0))
        }
        "rank" => Command::Rank(take_one(args, "sieve rank <metric>")?),
        "compare" => {
            if args.len() != 2 {
                bail!("usage: sieve compare <left> <right>");
            }
            Command::Compare(args.remove(0), args.remove(0))
        }
        "duplicates" => {
            if !args.is_empty() {
                bail!("duplicates takes no arguments");
            }
            Command::Duplicates
        }
        "repeated-structures" => {
            if !args.is_empty() {
                bail!("repeated-structures takes no arguments");
            }
            Command::RepeatedStructures
        }
        "modules" => {
            if !args.is_empty() {
                bail!("modules takes no arguments");
            }
            Command::Modules
        }
        "graph" => {
            if !args.is_empty() {
                bail!("graph takes no arguments");
            }
            Command::Graph
        }
        "export" => Command::Export(take_one(
            args,
            "sieve export <nodes|edges|modules|metrics>",
        )?),
        "serve" => {
            if !args.is_empty() {
                bail!("serve takes no arguments");
            }
            Command::Serve
        }
        "help" => Command::Help,
        _ => {
            args.insert(0, command);
            Command::LegacyDeclarations(args)
        }
    })
}

pub fn usage() -> &'static str {
    "Sieve — analysis of elaborated Lean declarations\n\nCommands:\n  discover [--lens influence|bridge|neighbors|all]\n  inspect <name> [--lens influence|bridge|neighbors|proof|trust|all]\n  serve [--port N]\n  summary\n  declaration <name>\n  proof-steps <name> [--step ID] [--format text|json]\n  dependencies <name> [--transitive]\n  dependents <name> [--transitive]\n  path <source> <target>\n  rank <metric>\n  compare <left> <right>\n  duplicates\n  repeated-structures\n  modules\n  graph\n  export <nodes|edges|modules|metrics|duplicates|repeated>\n\nCommon options:\n  --lens influence|bridge|neighbors|proof|trust|all\n  --layer statement|proof|both\n  --include-generated\n  --include-infrastructure\n  --internal-only\n  --source-backed-only\n  --weighted\n  --max-depth N\n  --limit N\n  --format text|json|csv\n  --histogram-buckets N,N,...\n"
}

pub fn selected_layer(selection: LayerSelection) -> DependencyLayer {
    match selection {
        LayerSelection::Proof => DependencyLayer::Proof,
        LayerSelection::Statement | LayerSelection::Both => DependencyLayer::Statement,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_filters_explicitly() {
        let cli = Cli::parse_from(
            ["summary", "--include-generated", "--layer", "proof"].map(str::to_owned),
        )
        .unwrap();
        assert!(cli.filter.include_generated);
        assert_eq!(cli.filter.layer, LayerSelection::Proof);
    }

    #[test]
    fn parses_proof_step_selection() {
        let cli = Cli::parse_from(
            [
                "proof-steps",
                "Example.theorem",
                "--step",
                "7",
                "--format",
                "json",
            ]
            .map(str::to_owned),
        )
        .unwrap();
        assert!(matches!(cli.command, Command::ProofSteps(ref name) if name == "Example.theorem"));
        assert_eq!(cli.selected_step, Some(7));
        assert_eq!(cli.format, OutputFormat::Json);
    }

    #[test]
    fn parses_lens_commands_and_explicit_layers() {
        let cli = Cli::parse_from(
            [
                "inspect",
                "Example.theorem",
                "--lens",
                "bridge",
                "--layer",
                "both",
            ]
            .map(str::to_owned),
        )
        .unwrap();
        assert!(matches!(cli.command, Command::Inspect(ref name) if name == "Example.theorem"));
        assert_eq!(cli.lens, LensKind::Bridge);
        assert!(cli.layer_explicit);
    }

    #[test]
    fn rejects_unknown_or_discovery_only_lenses() {
        assert!(Cli::parse_from(["discover", "--lens", "proof"].map(str::to_owned)).is_err());
        assert!(Cli::parse_from(["inspect", "T", "--lens", "meaning"].map(str::to_owned)).is_err());
        assert!(Cli::parse_from(["discover", "--format", "csv"].map(str::to_owned)).is_err());
    }
}
