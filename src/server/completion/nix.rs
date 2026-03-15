use std::sync::{Arc, RwLock};

use anyhow::anyhow;
use jsonnet_location::Location;
use language_server::{cache::Cache, completion::Completion, utils::cst::CstNodeHelper};
use lsp_types::{CompletionItem, CompletionItemKind, CompletionList};
use tree_sitter::Node;

use crate::server::{
    NixASTGenerator,
    cst::{new_tree, node_type::NodeType},
    flake_cache::FlakeCache,
};

pub struct NixCompletion {
    pub cache: Cache<NixASTGenerator>,
    pub flake_cache: Arc<RwLock<FlakeCache>>,
}

// TODO: copied from helper due to lifetime stuff
fn get_prev_node(node: Node) -> Option<Node> {
    match node.prev_sibling() {
        Some(sibling) => {
            let mut cursor = sibling.walk();
            while cursor.goto_last_child() {}
            Some(cursor.node())
        }
        None => node.parent(),
    }
}

impl Completion for NixCompletion {
    fn complete(
        &self,
        location: lsp_types::Position,
        uri: &lsp_types::Uri,
    ) -> language_server::completion::CompletionResult {
        let doc = self.cache.get_document(uri)?;
        let pos: Location = location.into();
        let tree = new_tree(&doc.content).ok_or(anyhow!("Unable to generate tree"))?;
        let root_node = tree.root_node();
        let mut node = root_node
            .get_node_at(pos.into())
            .ok_or(anyhow!("Unable to find node"))?;
        // If dot, Identifier, or Attrpath -> Skip to next Identifier or Attrpath to skip the
        // currently selected one
        match NodeType::from(node) {
            NodeType::Identifier | NodeType::Dot | NodeType::Attrpath => {
                node = get_prev_node(node).ok_or(anyhow!("No parent left"))?;
                // Now get the next Identifier
                while NodeType::from(node) != NodeType::Identifier {
                    node = get_prev_node(node).ok_or(anyhow!("No parent left"))?;
                    log::trace!(
                        "Searching for Identifier or path: parent is of type {}",
                        node.grammar_name()
                    );
                }
            }
            _ => (),
        };

        let mut names: Vec<String> = vec![];
        names.push(
            node.get_name(&doc.content)
                .ok_or(anyhow!("Unable to get node name"))?,
        );

        let mut current_node = node.get_prev_node();
        while let Some(prev_node) = current_node {
            current_node = get_prev_node(prev_node);
            match NodeType::from(prev_node) {
                NodeType::Identifier => {
                    names.push(
                        prev_node
                            .get_name(&doc.content)
                            .ok_or(anyhow!("Unable to get node name"))?,
                    );
                }
                NodeType::Attrpath => (),
                NodeType::Dot => (),
                _ => {
                    log::info!("Breaking at {}", prev_node.grammar_name());
                    break;
                }
            }
        }

        let first_name = names.pop().ok_or(anyhow!("empty"))?;
        match first_name.as_str() {
            "inputs" => names.push("inputs".into()),
            "self" => (),
            "self'" => {
                // Map self' to self.<second>.${system}
                if !names.is_empty() {
                    names.insert(names.len() - 1, "x86_64-linux".into());
                }
            }
            _ => return Err(anyhow!("Unsupported variable")),
        };
        names.reverse();
        log::debug!("Getting flake values for {}.{:?}", first_name, names);

        let vals = self.flake_cache.write().unwrap().get_flake_values(&names)?;
        let items = vals
            .iter()
            .map(|val| CompletionItem {
                label: val.to_string(),
                kind: Some(CompletionItemKind::FIELD),
                ..Default::default()
            })
            .collect();
        Ok(CompletionList {
            items,
            is_incomplete: false,
        })
    }
}
