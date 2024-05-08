use std::collections::HashMap;

use crate::global_context::GlobalContext;

use super::value::Value;

pub struct VmContext {
    global_ctx: GlobalContext,

    vm_variables: HashMap<String, Value>,
}

impl VmContext {
    pub fn new(global_ctx: GlobalContext) -> Self {
        Self {
            global_ctx,
            vm_variables: HashMap::new(),
        }
    }

    pub fn global_ctx(&self) -> &GlobalContext {
        &self.global_ctx
    }

    pub fn new_scope(&self, _variable_used: Vec<String>) -> Self {
        Self {
            global_ctx: self.global_ctx.clone(),
            vm_variables: self
                .vm_variables
                .iter()
                .filter_map(|(k, v)| Some((k.clone(), v.try_clone()?)))
                .collect(),
        }
    }

    pub fn set_variable(&mut self, name: &str, _value: Value) {
        self.vm_variables.insert(name.to_string(), _value);
    }

    pub fn get_variable(&self, name: &str) -> Option<&Value> {
        self.vm_variables.get(name)
    }

    pub fn take_variable(&mut self, name: &str) -> Option<Value> {
        self.vm_variables.remove(name)
    }
}
