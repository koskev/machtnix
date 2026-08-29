use std::{
    ops::Deref,
    sync::{Arc, Mutex, Once, RwLock},
};

use anyhow::{Result, anyhow};
use nix_bindings_expr::{
    eval_state::{EvalState, EvalStateBuilder, ThreadRegistrationGuard, gc_register_my_thread},
    value::Value,
};
use nix_bindings_flake::{EvalStateBuilderExt, FlakeSettings};
use nix_bindings_store::store::Store;
use utils::{MutexPanic, RwLockPanic};

static INIT: Once = Once::new();

pub struct SendEval(Mutex<EvalState>);
struct SendValue(RwLock<Option<Value>>);

impl Deref for SendEval {
    type Target = Mutex<EvalState>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for SendValue {
    type Target = RwLock<Option<Value>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

unsafe impl Send for SendEval {}
unsafe impl Sync for SendEval {}
unsafe impl Send for SendValue {}
unsafe impl Sync for SendValue {}

pub struct FlakeCache {
    _gc_registration: ThreadRegistrationGuard,

    pub eval_state: Arc<SendEval>,
    flake_value: Arc<SendValue>,
    flake_path: String,
}

impl FlakeCache {
    pub fn try_new() -> Result<Self> {
        INIT.call_once(|| {
            nix_bindings_expr::eval_state::init().unwrap();
            nix_bindings_util::settings::set("experimental-features", "flakes").unwrap();
        });
        let gc_registration = gc_register_my_thread()?;
        let store = Store::open(None, [])?;
        let eval_state = EvalStateBuilder::new(store)?
            .flakes(&FlakeSettings::new().unwrap())?
            .build()?;

        Ok(Self {
            _gc_registration: gc_registration,
            eval_state: Arc::new(SendEval(Mutex::new(eval_state))),
            flake_value: Arc::new(SendValue(RwLock::default())),
            flake_path: "".to_string(),
        })
    }
    pub fn load_flake(&mut self, path: &str) -> Result<()> {
        self.flake_path = path.to_string();
        self.refresh_flake()
    }

    pub fn refresh_flake(&mut self) -> Result<()> {
        *self.flake_value.write_or_panic() =
            Some(self.eval_state.lock_or_panic().eval_from_string(
                &format!("builtins.getFlake git+file://{}", self.flake_path),
                &self.flake_path,
            )?);
        Ok(())
    }

    pub fn get_flake_value_name(&mut self, attrs: &[String]) -> Result<Vec<String>> {
        let val = self.get_flake_value(attrs)?;
        self.eval_state.lock_or_panic().require_attrs_names(&val)
    }

    pub fn get_flake_value(&mut self, attrs: &[String]) -> Result<Value> {
        let mut val = self
            .flake_value
            .read_or_panic()
            .clone()
            .ok_or(anyhow!("Flake not loaded"))?;
        for attr in attrs {
            log::info!("Getting {}", attr);
            val = self
                .eval_state
                .lock_or_panic()
                .require_attrs_select(&val, attr)?;
            log::info!("Got {}", attr);
        }
        Ok(val)
    }
}
