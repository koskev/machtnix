use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, RwLock},
};

use anyhow::anyhow;
use jsonnet_location::Location;
use language_server::{
    cache::Cache,
    completion::{Completion, CompletionContext},
};
use lsp_types::CompletionList;
use tree_sitter::Node;
use utils::cst::CstNodeHelper;

use crate::server::{
    NixASTGenerator,
    completion::mapping::{
        NixInputsMapping, NixInputsPrimeMapping, NixLibMapping, NixMapping, NixSelfMapping,
        NixSelfPrimeMapping, NixpkgMapping,
    },
    cst::{new_tree, node_type::NodeType},
    flake_cache::FlakeCache,
};

pub struct NixCompletion {
    pub cache: Cache<NixASTGenerator>,
    pub flake_cache: Arc<RwLock<FlakeCache>>,

    mappings: HashMap<String, Box<dyn NixMapping>>,
}

impl NixCompletion {
    pub fn new(cache: Cache<NixASTGenerator>, flake_cache: Arc<RwLock<FlakeCache>>) -> Self {
        Self {
            cache,
            flake_cache,
            mappings: HashMap::from([
                ("pkgs".to_string(), Box::new(NixpkgMapping) as _),
                ("lib".to_string(), Box::new(NixLibMapping) as _),
                ("inputs".to_string(), Box::new(NixInputsMapping) as _),
                ("inputs'".to_string(), Box::new(NixInputsPrimeMapping) as _),
                ("self".to_string(), Box::new(NixSelfMapping) as _),
                ("self'".to_string(), Box::new(NixSelfPrimeMapping) as _),
            ]),
        }
    }
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
        context: &CompletionContext,
    ) -> language_server::completion::CompletionResult {
        let doc = self.cache.get_document(&context.uri)?;
        let pos: Location = context.location.clone();
        // TODO: Use the cache
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

        let mut names: VecDeque<String> = VecDeque::new();
        names.push_front(
            node.get_name(&doc.content)
                .ok_or(anyhow!("Unable to get node name"))?,
        );

        let mut current_node = node.get_prev_node();
        while let Some(prev_node) = current_node {
            current_node = get_prev_node(prev_node);
            match NodeType::from(prev_node) {
                NodeType::Identifier => {
                    names.push_front(
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

        let first_name = names.pop_front().ok_or(anyhow!("empty"))?;
        let mapping = self
            .mappings
            .get(&first_name)
            .ok_or(anyhow!("Unsupported variable {}", first_name))?;

        mapping.map_names(&mut names);
        log::debug!("Getting flake values for {}.{:?}", first_name, names);

        let items = mapping.completion_item(names.make_contiguous(), self.flake_cache.clone())?;

        Ok(CompletionList {
            items,
            is_incomplete: false,
        })
    }
}
