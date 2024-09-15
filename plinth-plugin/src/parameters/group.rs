use std::rc::Rc;

use crate::Parameters;

pub type ParameterGroupRef = Rc<ParameterGroup>;

pub fn from_parameters(parameters: &impl Parameters) -> Vec<ParameterGroupRef> {
    let mut groups = Vec::new();

    for &id in parameters.ids().iter() {
        let parameter = parameters.get(id).unwrap();
        let path = parameter.info().path();
        if path.is_empty() {
            continue;
        }

        // Create all units for the path
        let sub_paths: Vec<_> = path.split('/').collect();
        let mut parameter_groups: Vec<_> = sub_paths
            .iter()
            .map(|sub_path| ParameterGroup::from_path(sub_path.to_string())).collect();

        let mut parent = None;

        for mut group in parameter_groups.drain(..) {
            group.parent = parent.clone();

            // Add unit to state if it doesn't exist already
            let group_ref = Rc::new(group);
            parent = Some(group_ref.clone());

            if !groups.contains(&group_ref) {
                groups.push(group_ref);
            }
        }
    }

    groups
}

pub struct ParameterGroup {
    pub path: String,
    pub name: String,
    pub parent: Option<ParameterGroupRef>,
}

impl PartialEq for ParameterGroup {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

impl ParameterGroup {
    pub fn from_path(path: String) -> Self {
        let name = path.split_once('/')
            .map(|(head, _)| head)
            .unwrap_or(&path)
            .to_string();

        Self {
            path,
            name,
            parent: None,
        }
    }
}
