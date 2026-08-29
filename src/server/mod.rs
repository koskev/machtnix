pub mod completion;
pub mod cst;
pub mod flake_cache;

use std::sync::{Arc, RwLock};

use anyhow::{Result, anyhow};
use jsonnet_location::{Location, LocationRange};
use language_server::{
    cache::{ASTGenerator, ASTNode, Cache},
    completion::{Completion, CompletionContext},
    server::{LSPConnection, LSPError, LSPResponse, LSPServer},
};
use lsp_types::{
    CompletionOptions, CompletionParams, CompletionResponse, DidSaveTextDocumentParams,
    InitializeParams, PositionEncodingKind, ServerCapabilities, TextDocumentSyncCapability,
    TextDocumentSyncKind, TextDocumentSyncOptions, Uri,
};
use rnix::{NixLanguage, SyntaxNode};
use rowan::{GreenNode, ast::SyntaxNodePtr};
use utils::{RwLockPanic, rope::RopeHelper, uri::UriHelper};

use crate::server::{completion::nix::NixCompletion, flake_cache::FlakeCache};

#[derive(Debug, Default, Clone)]
pub struct NixASTGenerator {}

#[derive(Debug, Clone)]
pub struct NixASTNode {
    /// The green node to ensure Send + Sync
    pub root: GreenNode,
    pub node_ptr: SyntaxNodePtr<NixLanguage>,
}

impl Default for NixASTNode {
    fn default() -> Self {
        todo!()
        //let green = rnix::Root::parse("null").syntax().green().into();
        //Self {
        //    green,
        //}
    }
}

impl From<SyntaxNode> for NixASTNode {
    fn from(value: SyntaxNode) -> Self {
        let mut root = value.clone();
        while let Some(parent) = root.parent() {
            root = parent;
        }
        Self {
            root: root.green().into(),
            node_ptr: SyntaxNodePtr::new(&value),
        }
    }
}

#[allow(dead_code)]
impl NixASTNode {
    fn get_location(&self, content: &str) -> Option<LocationRange> {
        let node = self.get_node();
        let node_pos = node.text_range();
        let rope = ropey::Rope::from(content);
        let start = rope.get_location(node_pos.start().into())?;
        let end = rope.get_location(node_pos.end().into())?;
        let range = LocationRange {
            begin: start.into(),
            end: end.into(),
            ..Default::default()
        };
        Some(range)
    }

    fn get_node(&self) -> SyntaxNode {
        self.node_ptr.to_node(&self.get_root_node())
    }

    fn get_root_node(&self) -> SyntaxNode {
        SyntaxNode::new_root(self.root.clone())
    }

    fn get_node_at_pos(&self, pos: Location, content: &str) -> Option<Vec<NixASTNode>> {
        let range = self.get_location(content)?;
        let node = self.get_node();
        if !range.in_range(&pos) {
            log::trace!("Not In range {} {:?} {:?}", node.text(), range, pos);
            return None;
        };
        log::trace!("In range {} {:?} {:?}", node.text(), range, pos);
        let mut children_in_range: Vec<_> = node
            .children()
            .map(NixASTNode::from)
            //.filter(|child| {
            //    log::info!("NODE: {:?}", child.get_node().kind());
            //    let loc = child.get_location(content).unwrap();
            //    loc.in_range(&range.begin)
            //})
            .flat_map(|child| {
                child
                    .get_node_at_pos(pos.clone(), content)
                    .unwrap_or(vec![])
            })
            .collect();
        children_in_range.push(node.into());

        Some(children_in_range)
    }
}

impl ASTNode for NixASTNode {}

#[derive(Default, Clone)]
pub struct NixLSPServer {
    pub connection: LSPConnection,
    pub cache: Cache<NixASTGenerator>,
    pub flake_cache: Option<Arc<RwLock<FlakeCache>>>,
}

fn print_child_pos(node: SyntaxNode) {
    for child in node.children() {
        log::info!(
            "CHILD: {} {:?} {:?}",
            child.text(),
            child.text_range().start(),
            child.text_range().end()
        );
        print_child_pos(child);
    }
}

impl NixLSPServer {
    fn get_encoding(&self) -> PositionEncodingKind {
        self.get_capabilities()
            .position_encoding
            .unwrap_or(PositionEncodingKind::UTF16)
    }
}

impl ASTGenerator for NixASTGenerator {
    type Node = NixASTNode;
    fn update_ast(&self, _source_file: &str, new_content: &str) -> Result<Arc<Self::Node>> {
        let parse = rnix::Root::parse(new_content).syntax();
        print_child_pos(parse.clone());
        Ok(Arc::new(parse.into()))
    }

    fn update_cst(
        &self,
        new_content: &str,
        old_tree: Option<&tree_sitter::Tree>,
    ) -> Result<tree_sitter::Tree> {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_nix::LANGUAGE.into())
            .expect("Something is really wrong with the tresitter setup!");

        parser
            .parse(new_content, old_tree)
            .ok_or(anyhow!("unable to parse CST"))
    }
}

impl LSPServer for NixLSPServer {
    type AstGenerator = NixASTGenerator;

    fn connection(&self) -> &LSPConnection {
        &self.connection
    }

    fn cache(&self) -> &Cache<Self::AstGenerator> {
        &self.cache
    }

    fn handle_init_parameters(&self, params: InitializeParams) {
        if let Some(flake_cache) = &self.flake_cache {
            let path = params
                .workspace_folders
                .unwrap()
                .first()
                .unwrap()
                .uri
                .to_file_path_string()
                .unwrap();
            log::info!("Loading flake from {}", path);
            if let Err(e) = flake_cache.write_or_panic().load_flake(&path) {
                log::error!("Unable to load flake. Reloading on next save. Err: {e}");
            };
        }
    }

    fn get_capabilities(&self) -> ServerCapabilities {
        ServerCapabilities {
            completion_provider: Some(CompletionOptions::default()),
            text_document_sync: Some(TextDocumentSyncCapability::Options(
                TextDocumentSyncOptions {
                    open_close: Some(true),
                    change: Some(TextDocumentSyncKind::INCREMENTAL),
                    ..Default::default()
                },
            )),
            ..Default::default()
        }
    }

    fn queue_diagnostics(&self, _uri: &Uri) {}

    fn completion(&self, params: CompletionParams) -> Result<LSPResponse, LSPError> {
        let list = NixCompletion::new(self.cache.clone(), self.flake_cache.clone().unwrap())
            .complete(&CompletionContext {
                location: params.text_document_position.position.into(),
                uri: params.text_document_position.text_document.uri.clone(),
                encoding: self.get_encoding(),
            })?;

        Ok(CompletionResponse::List(list).into())
    }

    fn did_save(&self, _params: DidSaveTextDocumentParams) -> Result<(), LSPError> {
        if let Some(flake_cache) = &self.flake_cache {
            flake_cache.write_or_panic().refresh_flake()?
        }
        Ok(())
    }
}
