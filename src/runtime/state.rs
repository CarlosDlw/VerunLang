use std::collections::HashMap;

use super::value::Value;

#[derive(Debug, Clone)]
pub struct RuntimeState {
    pub name: String,
    pub fields: HashMap<String, Value>,
    pub history: Vec<StateSnapshot>,
}

#[derive(Debug, Clone)]
pub struct StateSnapshot {
    pub transition_name: String,
    pub pre_state: HashMap<String, Value>,
    pub post_state: HashMap<String, Value>,
    pub events: Vec<RuntimeEvent>,
}

#[derive(Debug, Clone)]
pub struct RuntimeEvent {
    pub name: String,
    pub args: Vec<Value>,
}

impl RuntimeState {
    pub fn new(name: String) -> Self {
        Self {
            name,
            fields: HashMap::new(),
            history: Vec::new(),
        }
    }

    pub fn get(&self, field: &str) -> Option<&Value> {
        self.fields.get(field)
    }

    pub fn set(&mut self, field: &str, value: Value) {
        self.fields.insert(field.to_string(), value);
    }

    pub fn snapshot(&self) -> HashMap<String, Value> {
        self.fields.clone()
    }

    pub fn record_transition(
        &mut self,
        transition_name: &str,
        pre_state: HashMap<String, Value>,
        events: Vec<RuntimeEvent>,
    ) {
        let post_state = self.snapshot();
        self.history.push(StateSnapshot {
            transition_name: transition_name.to_string(),
            pre_state,
            post_state,
            events,
        });
    }
}
