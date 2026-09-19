use serde::{Deserialize, Serialize};

use crate::model::{DeclarationSnapshot, SymbolSnapshot};

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LayerSelection {
    Statement,
    Proof,
    #[default]
    Both,
}

impl LayerSelection {
    pub fn includes(self, layer: DependencyLayer) -> bool {
        matches!(self, Self::Both)
            || matches!(
                (self, layer),
                (Self::Statement, DependencyLayer::Statement)
                    | (Self::Proof, DependencyLayer::Proof)
            )
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DependencyLayer {
    Statement,
    Proof,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisFilter {
    pub include_generated: bool,
    pub include_infrastructure: bool,
    pub internal_only: bool,
    pub source_backed_only: bool,
    pub declaration_kind: Option<String>,
    pub module: Option<String>,
    pub layer: LayerSelection,
}

impl Default for AnalysisFilter {
    fn default() -> Self {
        Self {
            include_generated: false,
            include_infrastructure: false,
            internal_only: false,
            source_backed_only: false,
            declaration_kind: None,
            module: None,
            layer: LayerSelection::Both,
        }
    }
}

impl AnalysisFilter {
    pub fn all() -> Self {
        Self {
            include_generated: true,
            include_infrastructure: true,
            ..Self::default()
        }
    }

    pub fn matches_declaration(&self, declaration: &DeclarationSnapshot) -> bool {
        (self.include_generated || !declaration.is_hidden_by_default())
            && (!self.source_backed_only || declaration.source_range.is_some())
            && self
                .declaration_kind
                .as_ref()
                .is_none_or(|kind| kind == &declaration.kind)
            && self
                .module
                .as_ref()
                .is_none_or(|module| declaration.module_name.as_ref() == Some(module))
    }

    pub fn matches_symbol(&self, symbol: &SymbolSnapshot, belongs_to_corpus: bool) -> bool {
        (!self.internal_only || belongs_to_corpus)
            && (self.include_generated || !symbol.is_hidden_by_default())
            && (self.include_infrastructure || !symbol.is_infrastructure())
    }
}
