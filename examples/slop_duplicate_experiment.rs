use std::collections::BTreeMap;

use anyhow::{Result, ensure};
use sieve::analysis::filters::DependencyLayer;
use sieve::analysis::lenses::{LensAnalysisConfig, LensKind};
use sieve::{AnalysisCorpus, ExtractionSnapshot, extract};

const COPIES: [&str; 3] = [
    "Sieve.SlopCorpus.resultOne",
    "Sieve.SlopCorpus.resultTwo",
    "Sieve.SlopCorpus.resultThree",
];
const REUSED_RESULT: &str = "intervalIntegral.integral_deriv_eq_sub";

fn merge_snapshots(
    mut base: ExtractionSnapshot,
    additions: ExtractionSnapshot,
) -> Result<ExtractionSnapshot> {
    ensure!(base.schema_version == additions.schema_version);
    for declaration in additions.declarations {
        ensure!(
            !base
                .declarations
                .iter()
                .any(|existing| existing.name == declaration.name),
            "duplicate experiment declaration {}",
            declaration.name
        );
        base.declarations.push(declaration);
    }
    let mut symbols = base
        .symbols
        .into_iter()
        .map(|symbol| (symbol.name.clone(), symbol))
        .collect::<BTreeMap<_, _>>();
    for symbol in additions.symbols {
        symbols.entry(symbol.name.clone()).or_insert(symbol);
    }
    base.symbols = symbols.into_values().collect();
    Ok(base)
}

fn influence(corpus: &AnalysisCorpus, name: &str) -> Result<(usize, usize, Option<usize>)> {
    let config = LensAnalysisConfig {
        limit: usize::MAX,
        ..LensAnalysisConfig::default()
    };
    let inspected = corpus.inspect_lenses(name, &[LensKind::Influence], &config)?;
    let evidence = &inspected.influence[0];
    let discovered = corpus.discover_lenses(&[LensKind::Influence], &config);
    let rank = discovered
        .influence
        .iter()
        .position(|entry| entry.name == name)
        .map(|index| index + 1);
    Ok((
        evidence.direct_dependent_count,
        evidence.reachable_dependent_count,
        rank,
    ))
}

fn main() -> Result<()> {
    let full = extract(&[])?;
    let baseline = AnalysisCorpus::new(full.clone())?;
    let additions = extract(&COPIES.map(str::to_owned))?;
    let augmented = AnalysisCorpus::new(merge_snapshots(full, additions)?)?;

    println!("neutral-name duplicate experiment");
    println!("statement comparisons:");
    for right in [COPIES[1], COPIES[2], REUSED_RESULT] {
        let comparison = augmented
            .compare_declarations(COPIES[0], right, DependencyLayer::Statement)
            .expect("all experiment statements are present");
        println!(
            "  {} vs {}: exact={}, alpha={}, combined={:.6}",
            COPIES[0],
            right,
            comparison.exact_equal,
            comparison.alpha_equivalent,
            comparison.similarity.combined_score
        );
    }

    let neighbors = augmented.inspect_lenses(
        COPIES[0],
        &[LensKind::Neighbors],
        &LensAnalysisConfig {
            limit: usize::MAX,
            ..LensAnalysisConfig::default()
        },
    )?;
    println!("top neighbors for {}:", COPIES[0]);
    for neighbor in neighbors.neighbors[0].neighbors.iter().take(5) {
        println!(
            "  {}: exact={}, alpha={}, combined={:.6}",
            neighbor.name,
            neighbor.exact_equal,
            neighbor.alpha_equivalent,
            neighbor.similarity.combined_score
        );
    }

    let before = influence(&baseline, REUSED_RESULT)?;
    let after = influence(&augmented, REUSED_RESULT)?;
    println!("reused result influence:");
    println!(
        "  before: direct={}, reachable={}, discovery_rank={:?}",
        before.0, before.1, before.2
    );
    println!(
        "  after:  direct={}, reachable={}, discovery_rank={:?}",
        after.0, after.1, after.2
    );
    println!(
        "interpretation: exact/alpha flags establish structural identity; influence reports reuse, not mathematical importance or family membership"
    );
    Ok(())
}
