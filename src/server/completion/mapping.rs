use std::{
    collections::VecDeque,
    sync::{Arc, RwLock},
};

use anyhow::Result;
use lsp_types::{CompletionItem, CompletionItemKind};
use nix_bindings_expr::value::ValueType;
use utils::{MutexPanic, RwLockPanic};

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
    fn completion_item(
        &self,
        names: &[String],
        flake_cache: Arc<RwLock<FlakeCache>>,
    ) -> Result<Vec<CompletionItem>> {
        let vals = flake_cache.write().unwrap().get_flake_value_name(names)?;
        Ok(vals
            .iter()
            .filter_map(|val| {
                let mut current_path: Vec<String> = vec![];
                current_path.extend(names.iter().cloned());
                current_path.push(val.clone());
                let mut cache_lock = flake_cache.write_or_panic();
                // Also filters out values that do not evaluate at all (renamed etc.)
                let flake_value = cache_lock.get_flake_value(&current_path).ok()?;
                let mut eval_lock = cache_lock.eval_state.lock_or_panic();
                let mut value_type = eval_lock.value_type(&flake_value).ok()?;
                if value_type == ValueType::AttrSet
                    && let Ok(_) = eval_lock.require_attrs_select(&flake_value, "__functor")
                {
                    value_type = ValueType::Function;
                };
                let mut priority = "9999";
                let mut kind = CompletionItemKind::FIELD;
                if value_type == ValueType::Function {
                    priority = "0000";
                    kind = CompletionItemKind::FUNCTION;
                }
                Some(CompletionItem {
                    label: val.to_string(),
                    kind: Some(kind),
                    sort_text: Some(format!("{}_{}", priority, val)),
                    ..Default::default()
                })
            })
            .collect())
    }
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
