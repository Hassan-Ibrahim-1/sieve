use crate::analysis::corpus::AnalysisCorpus;
use crate::analysis::filters::AnalysisFilter;
use crate::analysis::structure::{RepeatedSubexpression, StructuralMatch};

fn cell(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

pub fn nodes(corpus: &AnalysisCorpus, filter: &AnalysisFilter) -> String {
    let mut output = "id,display_name,module,kind,generated,source_backed,statement_occurrences,value_occurrences\n".to_owned();
    for id in corpus.filtered_declaration_ids(filter) {
        let declaration = &corpus.declarations()[id];
        output.push_str(&format!(
            "{},{},{},{},{},{},{},{}\n",
            id,
            cell(&declaration.name),
            cell(declaration.module_name.as_deref().unwrap_or("")),
            cell(&declaration.kind),
            declaration.is_generated(),
            declaration.source_range.is_some(),
            declaration.type_stats.nodes,
            declaration
                .value_stats
                .as_ref()
                .map_or(0, |stats| stats.nodes)
        ));
    }
    output
}

pub fn edges(corpus: &AnalysisCorpus, filter: &AnalysisFilter) -> String {
    let mut output = "source,target,layer,weight,target_in_corpus,source_module,target_module,target_kind,is_class,is_instance,is_projection\n".to_owned();
    for edge in corpus.filtered_edges(filter) {
        output.push_str(&format!(
            "{},{},{:?},{},{},{},{},{},{},{},{}\n",
            cell(&corpus.declarations()[edge.source].name),
            cell(&edge.target),
            edge.layer,
            edge.occurrences,
            edge.target_in_corpus,
            cell(edge.source_module.as_deref().unwrap_or("")),
            cell(edge.target_module.as_deref().unwrap_or("")),
            cell(&edge.target_kind),
            edge.target_is_class,
            edge.target_is_instance,
            edge.target_is_projection
        ));
    }
    output
}

pub fn modules(corpus: &AnalysisCorpus, filter: &AnalysisFilter) -> String {
    let mut output = "source_module,target_module,layer,weight\n".to_owned();
    for ((source, target, layer), weight) in corpus.module_dependency_occurrences(filter) {
        output.push_str(&format!(
            "{},{},{:?},{}\n",
            cell(&source),
            cell(&target),
            layer,
            weight
        ));
    }
    output
}

pub fn metrics(corpus: &AnalysisCorpus, filter: &AnalysisFilter) -> String {
    let mut output = "name,module,kind,generated,source_backed,has_documentation,axiom_count,statement_occurrences,statement_unique_nodes,statement_depth,statement_dependencies,value_occurrences,value_unique_nodes,value_depth,value_dependencies\n".to_owned();
    for id in corpus.filtered_declaration_ids(filter) {
        let metrics = corpus.declaration_metrics_by_id(id);
        let value = metrics.value.as_ref();
        output.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
            cell(&metrics.name),
            cell(metrics.module.as_deref().unwrap_or("")),
            cell(&metrics.kind),
            metrics.generated,
            metrics.source_backed,
            metrics.has_documentation,
            metrics.axiom_count,
            metrics.statement.total_occurrences,
            metrics.statement.unique_nodes,
            metrics.statement.maximum_depth,
            metrics.statement.direct_dependencies,
            value.map_or(0, |m| m.total_occurrences),
            value.map_or(0, |m| m.unique_nodes),
            value.map_or(0, |m| m.maximum_depth),
            value.map_or(0, |m| m.direct_dependencies)
        ));
    }
    output
}

pub fn structural_matches(matches: &[StructuralMatch]) -> String {
    let mut output =
        "fingerprint,expression_size,expression_depth,total_occurrences,declarations\n".to_owned();
    for group in matches {
        output.push_str(&format!(
            "{},{},{},{},{}\n",
            group.fingerprint,
            group.expression_size,
            group.expression_depth,
            group.total_occurrences,
            cell(&group.declarations.join(";"))
        ));
    }
    output
}

pub fn repeated_occurrences(repeated: &[RepeatedSubexpression]) -> String {
    let mut output = "fingerprint,expression_size,expression_depth,unique_declarations,total_occurrences,declaration,layer,node,location_occurrences\n".to_owned();
    for group in repeated {
        for location in &group.locations {
            output.push_str(&format!(
                "{},{},{},{},{},{},{:?},{},{}\n",
                group.fingerprint,
                group.expression_size,
                group.expression_depth,
                group.unique_declarations,
                group.total_occurrences,
                cell(&location.declaration),
                location.layer,
                location.node,
                location.occurrences
            ));
        }
    }
    output
}
