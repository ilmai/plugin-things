use std::collections::HashSet;

use crate::{parameters::ParameterValue, Event, Parameter, ParameterId};

use super::parameter::ParameterPlain;

pub fn has_duplicates(ids: &[ParameterId]) -> bool {
    let mut set = HashSet::new();
    ids.iter().any(|id| !set.insert(id))
}

pub trait Parameters {
    fn ids(&self) -> &[ParameterId];
    fn get(&self, id: impl Into<ParameterId>) -> Option<&dyn Parameter>;

    fn typed<T: Parameter>(&self, id: impl Into<ParameterId>) -> Option<&T> {
        self.get(id)
            .and_then(|parameter| {
                let any_parameter = parameter.as_any();
                any_parameter.downcast_ref::<T>()
            })
    }

    fn value<T: ParameterPlain>(&self, id: impl Into<ParameterId>) -> T::Plain {
        self.typed::<T>(id).unwrap().plain()
    }

    fn modulated_value<T: ParameterPlain>(&self, id: impl Into<ParameterId>) -> T::Plain {
        self.typed::<T>(id).unwrap().modulated_plain()
    }

    fn process_event(&self, event: &Event) {
        match event {
            Event::ParameterValue { id, value, .. } => {
                let parameter = self.get(*id).expect(&format!("Tried to get parameter with id {id} but it doesn't exist"));
                parameter.set_normalized_value(*value).unwrap();
            },

            Event::ParameterModulation { id, amount, .. } => {
                let parameter = self.get(*id).expect(&format!("Tried to get parameter with id {id} but it doesn't exist"));
                parameter.set_normalized_modulation(*amount);
            },

            _ => {},
        }
    }

    fn serialize(&self) -> impl Iterator<Item = (ParameterId, ParameterValue)> {
        self.ids().iter()
            .map(|&id| {
                let parameter = self.get(id);
                (id, parameter.unwrap().serialize_value())
            })
    }

    fn deserialize(&self, parameters: impl IntoIterator<Item = (ParameterId, ParameterValue)>) {
        // Reset parameters to default and apply whatever we read
        for id in self.ids().iter().copied() {
            let parameter = self.get(id).unwrap();
            parameter.set_normalized_value(parameter.info().default_normalized_value()).unwrap();
        }

        for (id, value) in parameters.into_iter() {
            // TODO: Error handling
            let parameter = self.get(id).unwrap();
            parameter.deserialize_value(value).unwrap();
        }
    }
}
