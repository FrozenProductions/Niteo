use crate::import_graph::ImportGraph;
use crate::import_graph::model::ImportKind;

pub(crate) struct GraphFormatter<'a> {
    graph: &'a ImportGraph,
}

impl<'a> GraphFormatter<'a> {
    pub(crate) fn new(graph: &'a ImportGraph) -> Self {
        Self { graph }
    }

    pub(crate) fn to_dot(&self) -> String {
        let mut output = String::new();
        output.push_str("digraph imports {\n");
        output.push_str("  rankdir=LR;\n");
        output.push_str("  node [shape=box];\n\n");

        for (path, node) in self.graph.iter_files() {
            let label = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("unknown");
            let style = if node.is_barrel {
                ", style=filled, fillcolor=lightblue"
            } else if node.is_test {
                ", style=filled, fillcolor=lightyellow"
            } else {
                ""
            };
            output.push_str(&format!(
                "  \"{}\" [label=\"{}\"{}];\n",
                path.display(),
                label,
                style
            ));
        }

        output.push('\n');

        for edge in self.graph.edges() {
            if let Some(target) = &edge.resolved_target {
                let style = match edge.kind {
                    ImportKind::Import => "",
                    ImportKind::ReExport => ", style=bold",
                    ImportKind::DynamicImport => ", style=dotted",
                };
                output.push_str(&format!(
                    "  \"{}\" -> \"{}\" [label=\"{}\"{}];\n",
                    edge.source_file.display(),
                    target.display(),
                    edge.specifier,
                    style
                ));
            }
        }

        output.push_str("}\n");
        output
    }
}
