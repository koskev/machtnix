use std::{
    collections::VecDeque,
    sync::{Arc, RwLock},
};

use anyhow::Result;
use lsp_types::{CompletionItem, CompletionItemKind};

use crate::server::flake_cache::FlakeCache;

pub trait NixMapping: Send + Sync {
    fn map_names(&self, names: &mut VecDeque<String>);

    fn completion_item(
        &self,
        names: &[String],
        flake_cache: Arc<RwLock<FlakeCache>>,
    ) -> Result<Vec<CompletionItem>> {
        let vals = flake_cache.write().unwrap().get_flake_value_name(names)?;
        Ok(vals
            .iter()
            .map(|val| CompletionItem {
                label: val.to_string(),
                kind: Some(CompletionItemKind::FIELD),
                ..Default::default()
            })
            .collect())
    }
}

pub struct NixpkgMapping;
impl NixMapping for NixpkgMapping {
    fn map_names(&self, names: &mut VecDeque<String>) {
        names.extend(
            "inputs.nixpkgs.legacyPackages.x86_64-linux"
                .split(".")
                .map(|s| s.to_string()),
        );
    }
}

pub struct NixLibMapping;
impl NixMapping for NixLibMapping {
    fn map_names(&self, names: &mut VecDeque<String>) {
        NixpkgMapping {}.map_names(names);
        names.push_back("lib".to_string());
    }
}

pub struct NixInputsMapping;
impl NixMapping for NixInputsMapping {
    fn map_names(&self, names: &mut VecDeque<String>) {
        names.push_back("input".to_string());
    }
}

pub struct NixInputsPrimeMapping;
impl NixMapping for NixInputsPrimeMapping {
    fn map_names(&self, names: &mut VecDeque<String>) {
        names.push_front("inputs".into());
        // Map inputs' to inputs.<flake>.<output>.${system}
        if names.len() >= 3 {
            names.insert(3, "x86_64-linux".into());
        }
    }
}

pub struct NixSelfPrimeMapping;
impl NixMapping for NixSelfPrimeMapping {
    fn map_names(&self, names: &mut VecDeque<String>) {
        // Map self' to self.<second>.${system}
        if !names.is_empty() {
            names.insert(1, "x86_64-linux".into());
        }
    }
}

pub struct NixSelfMapping;
impl NixMapping for NixSelfMapping {
    fn map_names(&self, _names: &mut VecDeque<String>) {}
}
