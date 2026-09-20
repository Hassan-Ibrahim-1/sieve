use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;

use sieve::AnalysisCorpus;
use sieve::analysis::filters::AnalysisFilter;
use sieve::extraction::{ExtractionConfig, extract};
use sieve::ui::{GraphRequest, UiIndex};

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn fixture_root() -> PathBuf {
    repository_root().join("examples/ftc-slop")
}

fn run(mut command: Command) -> Output {
    let output = command.output().expect("process should start");
    assert!(
        output.status.success(),
        "process failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn built_fixture() -> &'static Path {
    static FIXTURE: OnceLock<PathBuf> = OnceLock::new();
    FIXTURE.get_or_init(|| {
        let fixture = fixture_root();
        let mut command = Command::new(sieve::extraction::lake_path());
        command.current_dir(&fixture).arg("build");
        run(command);
        assert!(
            fixture.join(".lake/build/lib/lean/FtcSlop.olean").is_file(),
            "the default build must compile the import-wrapper module"
        );
        fixture
    })
}

fn sieve_command() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_sieve"));
    command
        .current_dir(repository_root())
        .args(["--project", "examples/ftc-slop"]);
    command
}

#[test]
fn default_build_and_both_entry_points_execute_project_code() {
    let fixture = built_fixture();
    let expected = "FtcSlop: integral square 0 4 = 16\n";

    let mut interpreted = Command::new(sieve::extraction::lake_path());
    interpreted
        .current_dir(fixture)
        .args(["env", "lean", "--run", "FtcSlop/Main.lean"]);
    assert_eq!(
        String::from_utf8(run(interpreted).stdout).unwrap(),
        expected
    );

    let mut native = Command::new(fixture.join(".lake/build/bin/ftc_slop"));
    native.current_dir(fixture);
    assert_eq!(String::from_utf8(run(native).stdout).unwrap(), expected);
}

#[test]
fn real_equation_theorems_have_generated_provenance_and_are_filtered() {
    let fixture = built_fixture();
    let config = ExtractionConfig::new(fixture, vec!["FtcSlop.Calculus".into()]).unwrap();
    let corpus = AnalysisCorpus::new(extract(&config, &[]).unwrap()).unwrap();

    let authored = corpus
        .filtered_declaration_ids(&AnalysisFilter::default())
        .map(|id| &corpus.declarations()[id])
        .collect::<Vec<_>>();
    assert_eq!(authored.len(), 7);
    assert_eq!(
        authored
            .iter()
            .filter(|declaration| declaration.kind == "theorem")
            .count(),
        5
    );

    let all = corpus
        .filtered_declaration_ids(&AnalysisFilter::all())
        .map(|id| &corpus.declarations()[id])
        .collect::<Vec<_>>();
    assert_eq!(all.len(), 9);
    for name in ["FtcSlop.derivative.eq_1", "FtcSlop.integral.eq_1"] {
        let declaration = corpus.declaration(name).expect("equation theorem");
        assert!(declaration.is_generated());
        assert_eq!(
            declaration.generation_kind.as_deref(),
            Some("equationTheorem")
        );
        assert!(declaration.source_range.is_none());
    }
    assert!(
        authored
            .iter()
            .all(|declaration| !declaration.is_generated())
    );

    let server_payload: serde_json::Value =
        serde_json::from_str(&sieve::server::corpus_json(&corpus, &AnalysisFilter::all()).unwrap())
            .unwrap();
    let equation_theorem = server_payload["declarations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|declaration| declaration["name"] == "FtcSlop.derivative.eq_1")
        .unwrap();
    assert_eq!(equation_theorem["generated"], true);
}

#[test]
fn cli_reports_generated_flags_in_json_csv_and_trust_outputs() {
    built_fixture();

    let mut summary = sieve_command();
    summary.args([
        "--import",
        "FtcSlop.Calculus",
        "--include-generated",
        "summary",
        "--format",
        "json",
    ]);
    let summary: serde_json::Value = serde_json::from_slice(&run(summary).stdout).unwrap();
    assert_eq!(summary["declarationCount"], 9);
    assert_eq!(summary["theoremCount"], 7);
    assert_eq!(summary["generatedCount"], 2);

    let mut declaration = sieve_command();
    declaration.args([
        "--import",
        "FtcSlop.Calculus",
        "declaration",
        "FtcSlop.derivative.eq_1",
        "--format",
        "json",
    ]);
    let declaration: serde_json::Value = serde_json::from_slice(&run(declaration).stdout).unwrap();
    assert_eq!(
        declaration["declaration"]["generationKind"],
        "equationTheorem"
    );
    assert_eq!(declaration["metrics"]["generated"], true);

    let mut csv = sieve_command();
    csv.args([
        "--import",
        "FtcSlop.Calculus",
        "--include-generated",
        "export",
        "nodes",
    ]);
    let csv = String::from_utf8(run(csv).stdout).unwrap();
    assert!(
        csv.lines().any(|line| {
            line.contains("FtcSlop.derivative.eq_1") && line.contains("theorem,true,false")
        }),
        "equation theorem should be generated in CSV:\n{csv}"
    );

    let mut trust = sieve_command();
    trust.args([
        "--import",
        "FtcSlop.Calculus",
        "inspect",
        "FtcSlop.derivative.eq_1",
        "--lens",
        "trust",
        "--format",
        "json",
    ]);
    let trust: serde_json::Value = serde_json::from_slice(&run(trust).stdout).unwrap();
    assert_eq!(trust["trust"]["isGenerated"], true);
}

#[test]
fn import_only_module_warns_without_failing() {
    built_fixture();
    let mut command = sieve_command();
    command.args(["--import", "FtcSlop", "summary"]);
    let output = run(command);
    let stderr = String::from_utf8(output.stderr).unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stderr.contains("FtcSlop contribute no declarations defined directly"));
    assert!(stderr.contains("does not traverse imported dependencies"));
    assert!(stdout.contains("corpus: 0 declarations"));
}

#[test]
fn similarity_circle_fixture_has_three_connected_groups() {
    let fixture = built_fixture();
    let config = ExtractionConfig::new(fixture, vec!["FtcSlop.SimilarityCircles".into()]).unwrap();
    let corpus = AnalysisCorpus::new(extract(&config, &[]).unwrap()).unwrap();
    let graph = UiIndex::build(&corpus)
        .graph(&corpus, &GraphRequest::default())
        .unwrap();

    assert_eq!(graph.totals.declarations, 9);
    assert_eq!(graph.totals.groups, 3);
    let mut group_sizes = graph
        .nodes
        .iter()
        .filter(|node| node.node_kind == "group")
        .map(|node| node.member_count)
        .collect::<Vec<_>>();
    group_sizes.sort_unstable();
    assert_eq!(group_sizes, vec![2, 3, 3]);
    assert!(
        graph
            .edges
            .iter()
            .any(|edge| { edge.source.starts_with("group-") && edge.target.starts_with("group-") })
    );
}
