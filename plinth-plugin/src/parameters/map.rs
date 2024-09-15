use std::{collections::HashMap, sync::Arc};

use crate::{Parameter, ParameterId, Parameters};

#[derive(Clone)]
pub struct ParameterMap {
    ids: Vec<ParameterId>,
    map: HashMap<ParameterId, Arc<dyn Parameter>>,
}

impl ParameterMap {
    pub fn new() -> Self {
        Self {
            ids: Default::default(),
            map: Default::default(),
        }
    }

    pub fn add(&mut self, parameter: impl Parameter) {
        let id = parameter.info().id();
        assert!(!self.map.contains_key(&id),
            "Duplicate parameter id {id}. Old parameter was \"{}\", new parameter is \"{}\"",
            self.map.get(&id).unwrap().info().name(),
            parameter.info().name(),
        );

        self.ids.push(id);
        self.map.insert(id, Arc::new(parameter));
    }

    pub fn index_of(&self, parameter_id: impl Into<ParameterId>) -> Option<usize> {
        let parameter_id = parameter_id.into();
        self.ids.iter().position(|&id| id == parameter_id)
    }
}

impl Parameters for ParameterMap {
    fn ids(&self) -> &[ParameterId] {
        &self.ids
    }

    fn get(&self, id: impl Into<ParameterId>) -> Option<&dyn Parameter> {
        self.map.get(&id.into())
            .map(|parameter| parameter.as_ref())
    }
}
