use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use crate::error::DgmError;
use crate::interpreter::DgmValue;

#[derive(Debug, Clone)]
pub struct Environment {
    values: HashMap<String, DgmValue>,
    parent: Option<Rc<RefCell<Environment>>>,
}

impl Environment {
    pub fn new() -> Self {
        Self { values: HashMap::new(), parent: None }
    }

    pub fn new_child(parent: Rc<RefCell<Environment>>) -> Self {
        Self { values: HashMap::new(), parent: Some(parent) }
    }

    pub fn get(&self, name: &str) -> Option<DgmValue> {
        if let Some(v) = self.values.get(name) {
            Some(v.clone())
        } else if let Some(p) = &self.parent {
            p.borrow().get(name)
        } else {
            None
        }
    }

    pub fn set(&mut self, name: &str, value: DgmValue) {
        self.values.insert(name.to_string(), value);
    }

    /// Assign to existing variable, walking up scope chain
    pub fn assign(&mut self, name: &str, value: DgmValue) -> Result<(), DgmError> {
        if self.values.contains_key(name) {
            self.values.insert(name.to_string(), value);
            Ok(())
        } else if let Some(p) = &self.parent {
            p.borrow_mut().assign(name, value)
        } else {
            Err(DgmError::undefined_variable(name))
        }
    }

    /// Check if a variable exists in current or parent scope
    pub fn has(&self, name: &str) -> bool {
        if self.values.contains_key(name) {
            true
        } else if let Some(p) = &self.parent {
            p.borrow().has(name)
        } else {
            false
        }
    }

    /// Get all variable names in current scope (not parents)
    pub fn keys(&self) -> Vec<String> {
        self.values.keys().cloned().collect()
    }

    /// Remove a variable from current scope
    pub fn remove(&mut self, name: &str) {
        self.values.remove(name);
    }
}
