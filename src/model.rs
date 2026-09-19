use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExprStats {
    pub nodes: usize,
    pub max_depth: usize,
    pub bound_variables: usize,
    pub free_variables: usize,
    pub metavariables: usize,
    pub sorts: usize,
    pub constants: usize,
    pub applications: usize,
    pub lambdas: usize,
    pub foralls: usize,
    pub lets: usize,
    pub literals: usize,
    pub metadata: usize,
    pub projections: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpressionNode {
    pub id: usize,
    pub kind: String,
    pub children: Vec<usize>,
    pub name: Option<String>,
    pub index: Option<usize>,
    pub value: Option<String>,
    pub binder_info: Option<String>,
    pub universe_levels: Vec<String>,
}

impl ExpressionNode {
    pub fn label(&self) -> String {
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

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ExpressionGraph {
    pub root: usize,
    pub nodes: Vec<ExpressionNode>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProofContextEntry {
    pub id: String,
    pub user_name: String,
    pub kind: String,
    pub binder_info: String,
    pub r#type: String,
    pub type_graph: ExpressionGraph,
    pub value: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProofStep {
    pub id: usize,
    pub kind: String,
    pub proposition: String,
    pub proposition_graph: ExpressionGraph,
    pub context: Vec<ProofContextEntry>,
    pub scope: Vec<String>,
    pub proof_term_path: Vec<usize>,
    pub prerequisite_steps: Vec<usize>,
    pub hypothesis_references: Vec<String>,
    pub named_references: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NamedResultSnapshot {
    pub name: String,
    pub kind: String,
    pub module_name: Option<String>,
    pub r#type: String,
    pub type_graph: ExpressionGraph,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProofStepExtraction {
    pub complete: bool,
    pub truncation_reason: Option<String>,
    pub visited_terms: usize,
    pub conclusion_step: Option<usize>,
    pub steps: Vec<ProofStep>,
    pub named_results: Vec<NamedResultSnapshot>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SourcePosition {
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SourceRange {
    pub start: SourcePosition,
    pub end: SourcePosition,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeclarationSnapshot {
    pub name: String,
    pub kind: String,
    pub module_name: Option<String>,
    pub is_internal: bool,
    pub is_private: bool,
    pub is_unsafe: bool,
    pub is_partial: bool,
    pub doc_string: Option<String>,
    pub source_range: Option<SourceRange>,
    pub level_parameters: Vec<String>,
    pub r#type: String,
    pub has_value: bool,
    pub type_stats: ExprStats,
    pub value_stats: Option<ExprStats>,
    pub type_graph: ExpressionGraph,
    pub value_graph: Option<ExpressionGraph>,
    pub statement_dependencies: Vec<String>,
    pub proof_dependencies: Vec<String>,
    pub axioms: Vec<String>,
    pub proof_steps: Option<ProofStepExtraction>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SymbolSnapshot {
    pub name: String,
    pub kind: String,
    pub module_name: Option<String>,
    pub is_internal: bool,
    pub is_private: bool,
    pub is_unsafe: bool,
    pub is_partial: bool,
    pub is_class: bool,
    pub is_instance: bool,
    pub is_projection: bool,
}

impl SymbolSnapshot {
    pub fn is_infrastructure(&self) -> bool {
        self.is_class || self.is_instance || self.is_projection
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractionSnapshot {
    pub schema_version: usize,
    pub lean_version: String,
    pub imported_modules: Vec<String>,
    pub declarations: Vec<DeclarationSnapshot>,
    pub symbols: Vec<SymbolSnapshot>,
}
