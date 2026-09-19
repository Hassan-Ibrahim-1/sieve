use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

use anyhow::{Context, Result, bail};
use serde::Serialize;

use crate::analysis::corpus::AnalysisCorpus;
use crate::analysis::dependency::DependencyConfig;
use crate::analysis::filters::{AnalysisFilter, DependencyLayer, LayerSelection};
use crate::analysis::structure::{StructuralConfig, StructuralMode};

const INDEX_HTML: &str = r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Sieve API</title>
  <style>
    body { max-width: 48rem; margin: 4rem auto; padding: 0 1.5rem; font: 16px/1.5 system-ui, sans-serif; color: #1f2937; }
    code { background: #f3f4f6; padding: .15rem .35rem; border-radius: .25rem; }
  </style>
</head>
<body>
  <h1>Sieve API</h1>
  <p>The analysis server is running. Start with <a href="/api/health"><code>/api/health</code></a>
  or <a href="/api/corpus"><code>/api/corpus</code></a>.</p>
</body>
</html>
"#;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DeclarationListItem {
    name: String,
    kind: String,
    module: Option<String>,
    generated: bool,
    source_backed: bool,
    has_value: bool,
    statement_occurrences: usize,
    proof_occurrences: Option<usize>,
    statement_dependencies: usize,
    proof_dependencies: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CorpusResponse {
    schema_version: usize,
    lean_version: String,
    imported_module: String,
    summary: crate::analysis::metrics::CorpusSummary,
    declarations: Vec<DeclarationListItem>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DeclarationResponse<'a> {
    name: &'a str,
    kind: &'a str,
    module: Option<&'a str>,
    generated: bool,
    flags: DeclarationFlags,
    documentation: Option<&'a str>,
    source_range: Option<&'a crate::model::SourceRange>,
    type_text: &'a str,
    level_parameters: &'a [String],
    axioms: &'a [String],
    metrics: crate::analysis::metrics::DeclarationMetrics,
    dependencies: crate::analysis::dependency::DependencySummary,
    dependents: crate::analysis::dependency::DependencySummary,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DeclarationFlags {
    internal: bool,
    private: bool,
    unsafe_declaration: bool,
    partial: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RepeatedForDeclaration {
    declaration: String,
    results: Vec<crate::analysis::structure::RepeatedSubexpression>,
}

pub fn serve(corpus: &AnalysisCorpus, port: u16) -> Result<()> {
    let address = format!("127.0.0.1:{port}");
    let listener = TcpListener::bind(&address)
        .with_context(|| format!("failed to bind the UI server to {address}"))?;
    println!("Sieve UI: http://{address}");
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                if let Err(error) = handle_request(&mut stream, corpus) {
                    let _ = write_response(
                        &mut stream,
                        500,
                        "application/json; charset=utf-8",
                        &serde_json::json!({ "error": error.to_string() }).to_string(),
                    );
                }
            }
            Err(error) => eprintln!("UI connection failed: {error}"),
        }
    }
    Ok(())
}

fn handle_request(stream: &mut TcpStream, corpus: &AnalysisCorpus) -> Result<()> {
    let mut buffer = [0_u8; 16 * 1024];
    let bytes = stream
        .read(&mut buffer)
        .context("failed to read HTTP request")?;
    let request = std::str::from_utf8(&buffer[..bytes]).context("request was not UTF-8")?;
    let first_line = request.lines().next().context("empty HTTP request")?;
    let mut parts = first_line.split_whitespace();
    let method = parts.next().context("missing HTTP method")?;
    let target = parts.next().context("missing HTTP target")?;
    if method != "GET" {
        return write_response(
            stream,
            405,
            "text/plain; charset=utf-8",
            "Method not allowed",
        );
    }
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    let query = parse_query(query)?;
    match path {
        "/" | "/index.html" => write_response(stream, 200, "text/html; charset=utf-8", INDEX_HTML),
        "/api/corpus" => json_response(stream, &corpus_response(corpus, &query)),
        "/api/declaration" => declaration_response(stream, corpus, &query),
        "/api/compare" => comparison_response(stream, corpus, &query),
        "/api/repeated" => repeated_response(stream, corpus, &query),
        "/api/path" => path_response(stream, corpus, &query),
        "/api/health" => json_response(stream, &serde_json::json!({ "status": "ok" })),
        _ => write_response(stream, 404, "text/plain; charset=utf-8", "Not found"),
    }
}

fn corpus_response(corpus: &AnalysisCorpus, query: &BTreeMap<String, String>) -> CorpusResponse {
    let filter = filter_from_query(query);
    let declarations = corpus
        .filtered_declaration_ids(&filter)
        .map(|id| {
            let declaration = &corpus.declarations()[id];
            DeclarationListItem {
                name: declaration.name.clone(),
                kind: declaration.kind.clone(),
                module: declaration.module_name.clone(),
                generated: declaration.is_internal || declaration.is_private,
                source_backed: declaration.source_range.is_some(),
                has_value: declaration.has_value,
                statement_occurrences: declaration.type_stats.nodes,
                proof_occurrences: declaration.value_stats.as_ref().map(|stats| stats.nodes),
                statement_dependencies: declaration.statement_dependencies.len(),
                proof_dependencies: declaration.proof_dependencies.len(),
            }
        })
        .collect();
    CorpusResponse {
        schema_version: corpus.snapshot().schema_version,
        lean_version: corpus.snapshot().lean_version.clone(),
        imported_module: corpus.snapshot().imported_module.clone(),
        summary: corpus.summary(&filter),
        declarations,
    }
}

fn declaration_response(
    stream: &mut TcpStream,
    corpus: &AnalysisCorpus,
    query: &BTreeMap<String, String>,
) -> Result<()> {
    let name = required(query, "name")?;
    let declaration = corpus
        .declaration(name)
        .with_context(|| format!("unknown declaration {name}"))?;
    let config = dependency_config(query);
    let dependencies = corpus
        .direct_dependencies(name, &config)
        .context("declaration dependencies are unavailable")?;
    let dependents = corpus
        .direct_dependents(name, &config)
        .context("declaration dependents are unavailable")?;
    json_response(
        stream,
        &DeclarationResponse {
            name: &declaration.name,
            kind: &declaration.kind,
            module: declaration.module_name.as_deref(),
            generated: declaration.is_internal || declaration.is_private,
            flags: DeclarationFlags {
                internal: declaration.is_internal,
                private: declaration.is_private,
                unsafe_declaration: declaration.is_unsafe,
                partial: declaration.is_partial,
            },
            documentation: declaration.doc_string.as_deref(),
            source_range: declaration.source_range.as_ref(),
            type_text: &declaration.r#type,
            level_parameters: &declaration.level_parameters,
            axioms: &declaration.axioms,
            metrics: corpus
                .declaration_metrics(name)
                .expect("indexed declaration has metrics"),
            dependencies,
            dependents,
        },
    )
}

fn comparison_response(
    stream: &mut TcpStream,
    corpus: &AnalysisCorpus,
    query: &BTreeMap<String, String>,
) -> Result<()> {
    let left = required(query, "left")?;
    let right = required(query, "right")?;
    let layers = match query.get("layer").map(String::as_str) {
        Some("statement") => vec![DependencyLayer::Statement],
        Some("proof") => vec![DependencyLayer::Proof],
        _ => vec![DependencyLayer::Statement, DependencyLayer::Proof],
    };
    let results = layers
        .into_iter()
        .filter_map(|layer| corpus.compare_declarations(left, right, layer))
        .collect::<Vec<_>>();
    if results.is_empty() {
        bail!("the selected layer is absent from one or both declarations");
    }
    json_response(stream, &results)
}

fn repeated_response(
    stream: &mut TcpStream,
    corpus: &AnalysisCorpus,
    query: &BTreeMap<String, String>,
) -> Result<()> {
    let name = required(query, "name")?;
    corpus
        .declaration(name)
        .with_context(|| format!("unknown declaration {name}"))?;
    let limit = usize_query(query, "limit", 20)?;
    let mut filter = filter_from_query(query);
    filter.include_generated = true;
    let config = StructuralConfig {
        filter,
        mode: if bool_query(query, "exact") {
            StructuralMode::Exact
        } else {
            StructuralMode::AlphaEquivalent
        },
        minimum_size: usize_query(query, "minimumSize", 20)?,
        minimum_support: usize_query(query, "minimumSupport", 2)?,
        limit: 2_000,
    };
    let results = corpus
        .repeated_subexpressions(&config)
        .into_iter()
        .filter(|result| {
            result
                .locations
                .iter()
                .any(|location| location.declaration == name)
        })
        .take(limit)
        .collect();
    json_response(
        stream,
        &RepeatedForDeclaration {
            declaration: name.to_owned(),
            results,
        },
    )
}

fn path_response(
    stream: &mut TcpStream,
    corpus: &AnalysisCorpus,
    query: &BTreeMap<String, String>,
) -> Result<()> {
    let source = required(query, "source")?;
    let target = required(query, "target")?;
    let path = corpus
        .shortest_dependency_path(source, target, &dependency_config(query))
        .with_context(|| format!("no dependency path from {source} to {target}"))?;
    json_response(stream, &path)
}

fn filter_from_query(query: &BTreeMap<String, String>) -> AnalysisFilter {
    AnalysisFilter {
        include_generated: bool_query(query, "includeGenerated"),
        include_infrastructure: bool_query(query, "includeInfrastructure"),
        internal_only: bool_query(query, "internalOnly"),
        source_backed_only: bool_query(query, "sourceBackedOnly"),
        declaration_kind: query.get("kind").filter(|value| !value.is_empty()).cloned(),
        module: query
            .get("module")
            .filter(|value| !value.is_empty())
            .cloned(),
        layer: match query.get("layer").map(String::as_str) {
            Some("statement") => LayerSelection::Statement,
            Some("proof") => LayerSelection::Proof,
            _ => LayerSelection::Both,
        },
    }
}

fn dependency_config(query: &BTreeMap<String, String>) -> DependencyConfig {
    let mut filter = filter_from_query(query);
    // Direct inspection must keep a selected generated declaration visible.
    filter.include_generated = true;
    DependencyConfig {
        filter,
        weighted: bool_query(query, "weighted"),
        max_depth: usize_query(query, "maxDepth", 32).unwrap_or(32),
        limit: usize_query(query, "limit", 500).unwrap_or(500),
    }
}

fn required<'a>(query: &'a BTreeMap<String, String>, key: &str) -> Result<&'a str> {
    query
        .get(key)
        .map(String::as_str)
        .filter(|value| !value.is_empty())
        .with_context(|| format!("missing query parameter {key}"))
}

fn bool_query(query: &BTreeMap<String, String>, key: &str) -> bool {
    query
        .get(key)
        .is_some_and(|value| matches!(value.as_str(), "1" | "true" | "yes"))
}

fn usize_query(query: &BTreeMap<String, String>, key: &str, default: usize) -> Result<usize> {
    query
        .get(key)
        .map(|value| value.parse().with_context(|| format!("invalid {key}")))
        .unwrap_or(Ok(default))
}

fn parse_query(query: &str) -> Result<BTreeMap<String, String>> {
    query
        .split('&')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let (key, value) = part.split_once('=').unwrap_or((part, ""));
            Ok((percent_decode(key)?, percent_decode(value)?))
        })
        .collect()
}

fn percent_decode(value: &str) -> Result<String> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'%' if index + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[index + 1..index + 3])?;
                decoded.push(u8::from_str_radix(hex, 16).context("invalid percent encoding")?);
                index += 3;
            }
            b'+' => {
                decoded.push(b' ');
                index += 1;
            }
            byte => {
                decoded.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8(decoded).context("query parameter was not UTF-8")
}

fn json_response(stream: &mut TcpStream, value: &impl Serialize) -> Result<()> {
    write_response(
        stream,
        200,
        "application/json; charset=utf-8",
        &serde_json::to_string(value)?,
    )
}

fn write_response(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &str,
) -> Result<()> {
    let reason = match status {
        200 => "OK",
        404 => "Not Found",
        405 => "Method Not Allowed",
        _ => "Internal Server Error",
    };
    write!(
        stream,
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_decodes_query_parameters() {
        let query =
            parse_query("name=intervalIntegral.integral_deriv_eq_sub%27&layer=proof").unwrap();
        assert_eq!(query["name"], "intervalIntegral.integral_deriv_eq_sub'");
        assert_eq!(query["layer"], "proof");
    }
}
