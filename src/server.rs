use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use tower_http::compression::CompressionLayer;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;

use crate::analysis::corpus::AnalysisCorpus;
use crate::analysis::dependency::DependencyConfig;
use crate::analysis::filters::{AnalysisFilter, DependencyLayer, LayerSelection};
use crate::analysis::structure::{StructuralConfig, StructuralMode};
use crate::ui::{GraphRequest, UiFilters, UiIndex};

const DEVELOPMENT_INDEX: &str = r#"<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>Sieve</title><style>body{margin:4rem auto;max-width:42rem;padding:0 1.5rem;font:16px/1.5 system-ui;background:#111;color:#eee}code{color:#9de4c7}</style></head>
<body><h1>Sieve</h1><p>The web build is missing. Run <code>cd web &amp;&amp; npm install &amp;&amp; npm run build</code>, then restart the server.</p></body></html>"#;

#[derive(Clone)]
struct AppState {
    corpus: Arc<AnalysisCorpus>,
    ui: Arc<UiIndex>,
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn bad_request(error: impl std::fmt::Display) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: error.to_string(),
        }
    }
    fn not_found(error: impl std::fmt::Display) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: error.to_string(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(serde_json::json!({ "error": self.message, "status": self.status.as_u16() })),
        )
            .into_response()
    }
}

type ApiResult<T> = std::result::Result<Json<T>, ApiError>;

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct CommonQuery {
    include_generated: Option<bool>,
    include_infrastructure: Option<bool>,
    internal_only: Option<bool>,
    source_backed_only: Option<bool>,
    kind: Option<String>,
    module: Option<String>,
    layer: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct UiGraphQuery {
    mode: Option<String>,
    level: Option<String>,
    scope: Option<String>,
    metric: Option<String>,
    limit: Option<usize>,
    threshold: Option<f64>,
    depth: Option<usize>,
    search: Option<String>,
    include_theorems: Option<bool>,
    include_definitions: Option<bool>,
    include_technical: Option<bool>,
}

#[derive(Deserialize)]
struct NameQuery {
    name: String,
}

#[derive(Deserialize)]
struct CompareQuery {
    left: String,
    right: String,
    layer: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RepeatedQuery {
    name: String,
    limit: Option<usize>,
    exact: Option<bool>,
    minimum_size: Option<usize>,
    minimum_support: Option<usize>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PathQuery {
    source: String,
    target: String,
    limit: Option<usize>,
    max_depth: Option<usize>,
}

#[derive(Deserialize)]
struct ProofQuery {
    declaration: String,
    detail: Option<String>,
    path: Option<String>,
}

#[derive(Deserialize)]
struct WitnessQuery {
    source: String,
    target: String,
    limit: Option<usize>,
}

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
    limit: Option<usize>,
}

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
    imported_modules: Vec<String>,
    summary: crate::analysis::metrics::CorpusSummary,
    declarations: Vec<DeclarationListItem>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DeclarationResponse {
    name: String,
    kind: String,
    module: Option<String>,
    generated: bool,
    flags: DeclarationFlags,
    documentation: Option<String>,
    source_range: Option<crate::model::SourceRange>,
    type_text: String,
    level_parameters: Vec<String>,
    axioms: Vec<String>,
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

pub fn serve(corpus: Arc<AnalysisCorpus>, port: u16) -> Result<()> {
    let ui = Arc::new(UiIndex::build(&corpus));
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("failed to create the UI runtime")?
        .block_on(serve_async(AppState { corpus, ui }, port))
}

async fn serve_async(state: AppState, port: u16) -> Result<()> {
    let api = Router::new()
        .route("/health", get(health))
        .route("/corpus", get(corpus_route))
        .route("/declaration", get(declaration_route))
        .route("/compare", get(compare_route))
        .route("/repeated", get(repeated_route))
        .route("/path", get(path_route))
        .route("/ui/bootstrap", get(ui_bootstrap))
        .route("/ui/graph", get(ui_graph))
        .route("/ui/proof", get(ui_proof))
        .route("/ui/search", get(ui_search))
        .route("/ui/compare", get(compare_route))
        .route("/ui/witnesses", get(ui_witnesses))
        .fallback(api_not_found);
    let dist = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("web/dist");
    let mut app = Router::new().nest("/api", api);
    if dist.join("index.html").is_file() {
        app = app.fallback_service(
            ServeDir::new(&dist).not_found_service(ServeFile::new(dist.join("index.html"))),
        );
    } else {
        app = app.fallback(development_index);
    }
    let app = app
        .with_state(state)
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http());
    let address = format!("127.0.0.1:{port}");
    let listener = tokio::net::TcpListener::bind(&address)
        .await
        .with_context(|| format!("failed to bind the UI server to {address}"))?;
    println!("Sieve UI: http://{address}");
    axum::serve(listener, app).await.context("UI server failed")
}

async fn development_index() -> Html<&'static str> {
    Html(DEVELOPMENT_INDEX)
}
async fn api_not_found() -> ApiError {
    ApiError::not_found("API route not found")
}
async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}

async fn ui_bootstrap(State(state): State<AppState>) -> Json<crate::ui::UiBootstrap> {
    Json(state.ui.bootstrap(&state.corpus))
}

async fn ui_graph(
    State(state): State<AppState>,
    Query(query): Query<UiGraphQuery>,
) -> ApiResult<crate::ui::GraphResponse> {
    let defaults = GraphRequest::default();
    let request = GraphRequest {
        mode: query.mode.unwrap_or(defaults.mode),
        level: query.level.unwrap_or(defaults.level),
        scope: query.scope,
        metric: query.metric.unwrap_or(defaults.metric),
        limit: query.limit.unwrap_or(defaults.limit),
        threshold: query
            .threshold
            .unwrap_or(defaults.threshold)
            .clamp(0.55, 1.0),
        depth: query.depth.unwrap_or(defaults.depth),
        search: query.search,
        filters: UiFilters {
            include_theorems: query.include_theorems.unwrap_or(true),
            include_definitions: query.include_definitions.unwrap_or(true),
            include_technical: query.include_technical.unwrap_or(false),
        },
    };
    state
        .ui
        .graph(&state.corpus, &request)
        .map(Json)
        .map_err(ApiError::bad_request)
}

async fn ui_proof(
    State(state): State<AppState>,
    Query(query): Query<ProofQuery>,
) -> ApiResult<crate::ui::ProofResponse> {
    state
        .ui
        .proof(
            &state.corpus,
            &query.declaration,
            query.detail.as_deref().unwrap_or("outline"),
            query.path.as_deref(),
        )
        .map(Json)
        .map_err(ApiError::bad_request)
}

async fn ui_witnesses(
    State(state): State<AppState>,
    Query(query): Query<WitnessQuery>,
) -> ApiResult<crate::ui::WitnessResponse> {
    state
        .ui
        .witnesses(
            &state.corpus,
            &query.source,
            &query.target,
            query.limit.unwrap_or(3),
        )
        .map(Json)
        .map_err(ApiError::bad_request)
}

async fn ui_search(
    State(state): State<AppState>,
    Query(query): Query<SearchQuery>,
) -> Json<crate::ui::SearchResponse> {
    Json(
        state
            .ui
            .search(&state.corpus, &query.q, query.limit.unwrap_or(12)),
    )
}

async fn corpus_route(
    State(state): State<AppState>,
    Query(query): Query<CommonQuery>,
) -> Json<CorpusResponse> {
    Json(corpus_response_with_filter(
        &state.corpus,
        &filter_from_query(&query),
    ))
}

async fn declaration_route(
    State(state): State<AppState>,
    Query(query): Query<NameQuery>,
) -> ApiResult<DeclarationResponse> {
    let declaration = state
        .corpus
        .declaration(&query.name)
        .with_context(|| format!("unknown declaration {}", query.name))
        .map_err(ApiError::not_found)?;
    let config = DependencyConfig {
        filter: AnalysisFilter::all(),
        weighted: false,
        max_depth: 32,
        limit: 500,
    };
    Ok(Json(DeclarationResponse {
        name: declaration.name.clone(),
        kind: declaration.kind.clone(),
        module: declaration.module_name.clone(),
        generated: declaration.is_generated(),
        flags: DeclarationFlags {
            internal: declaration.is_internal,
            private: declaration.is_private,
            unsafe_declaration: declaration.is_unsafe,
            partial: declaration.is_partial,
        },
        documentation: declaration.doc_string.clone(),
        source_range: declaration.source_range.clone(),
        type_text: declaration.r#type.clone(),
        level_parameters: declaration.level_parameters.clone(),
        axioms: declaration.axioms.clone(),
        metrics: state.corpus.declaration_metrics(&query.name).unwrap(),
        dependencies: state
            .corpus
            .direct_dependencies(&query.name, &config)
            .unwrap(),
        dependents: state
            .corpus
            .direct_dependents(&query.name, &config)
            .unwrap(),
    }))
}

async fn compare_route(
    State(state): State<AppState>,
    Query(query): Query<CompareQuery>,
) -> ApiResult<Vec<crate::analysis::comparison::DeclarationComparison>> {
    let layers = match query.layer.as_deref() {
        Some("statement") => vec![DependencyLayer::Statement],
        Some("proof") => vec![DependencyLayer::Proof],
        _ => vec![DependencyLayer::Statement, DependencyLayer::Proof],
    };
    let results = layers
        .into_iter()
        .filter_map(|layer| {
            state
                .corpus
                .compare_declarations(&query.left, &query.right, layer)
        })
        .collect::<Vec<_>>();
    if results.is_empty() {
        return Err(ApiError::bad_request(
            "the selected layer is absent from one or both declarations",
        ));
    }
    Ok(Json(results))
}

async fn repeated_route(
    State(state): State<AppState>,
    Query(query): Query<RepeatedQuery>,
) -> ApiResult<RepeatedForDeclaration> {
    state
        .corpus
        .declaration(&query.name)
        .with_context(|| format!("unknown declaration {}", query.name))
        .map_err(ApiError::not_found)?;
    let filter = AnalysisFilter {
        include_generated: true,
        ..AnalysisFilter::default()
    };
    let config = StructuralConfig {
        filter,
        mode: if query.exact.unwrap_or(false) {
            StructuralMode::Exact
        } else {
            StructuralMode::AlphaEquivalent
        },
        minimum_size: query.minimum_size.unwrap_or(20),
        minimum_support: query.minimum_support.unwrap_or(2),
        limit: 2_000,
    };
    let results = state
        .corpus
        .repeated_subexpressions(&config)
        .into_iter()
        .filter(|result| {
            result
                .locations
                .iter()
                .any(|location| location.declaration == query.name)
        })
        .take(query.limit.unwrap_or(20))
        .collect();
    Ok(Json(RepeatedForDeclaration {
        declaration: query.name,
        results,
    }))
}

async fn path_route(
    State(state): State<AppState>,
    Query(query): Query<PathQuery>,
) -> ApiResult<crate::analysis::dependency::DependencyPath> {
    let config = DependencyConfig {
        filter: AnalysisFilter::all(),
        weighted: false,
        max_depth: query.max_depth.unwrap_or(32),
        limit: query.limit.unwrap_or(500),
    };
    state
        .corpus
        .shortest_dependency_path(&query.source, &query.target, &config)
        .with_context(|| {
            format!(
                "no dependency path from {} to {}",
                query.source, query.target
            )
        })
        .map(Json)
        .map_err(ApiError::not_found)
}

fn filter_from_query(query: &CommonQuery) -> AnalysisFilter {
    AnalysisFilter {
        include_generated: query.include_generated.unwrap_or(false),
        include_infrastructure: query.include_infrastructure.unwrap_or(false),
        internal_only: query.internal_only.unwrap_or(false),
        source_backed_only: query.source_backed_only.unwrap_or(false),
        declaration_kind: query.kind.clone().filter(|value| !value.is_empty()),
        module: query.module.clone().filter(|value| !value.is_empty()),
        layer: match query.layer.as_deref() {
            Some("statement") => LayerSelection::Statement,
            Some("proof") => LayerSelection::Proof,
            _ => LayerSelection::Both,
        },
    }
}

fn corpus_response_with_filter(corpus: &AnalysisCorpus, filter: &AnalysisFilter) -> CorpusResponse {
    let declarations = corpus
        .filtered_declaration_ids(filter)
        .map(|id| {
            let declaration = &corpus.declarations()[id];
            DeclarationListItem {
                name: declaration.name.clone(),
                kind: declaration.kind.clone(),
                module: declaration.module_name.clone(),
                generated: declaration.is_generated(),
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
        imported_modules: corpus.snapshot().imported_modules.clone(),
        summary: corpus.summary(filter),
        declarations,
    }
}

/// Render the same corpus payload exposed by `GET /api/corpus`.
pub fn corpus_json(corpus: &AnalysisCorpus, filter: &AnalysisFilter) -> Result<String> {
    Ok(serde_json::to_string(&corpus_response_with_filter(
        corpus, filter,
    ))?)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_ui_query_maps_to_expected_request() {
        let query = UiGraphQuery::default();
        let defaults = GraphRequest::default();
        assert_eq!(query.mode.unwrap_or(defaults.mode), "similarity");
        assert_eq!(query.limit.unwrap_or(defaults.limit), 80);
    }
}
