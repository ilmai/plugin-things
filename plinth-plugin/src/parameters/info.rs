use crate::ParameterId;

use super::ParameterValue;

#[derive(Clone)]
pub struct ParameterInfo {
    id: ParameterId,
    name: String,
    path: String,
    default_normalized_value: ParameterValue,
    steps: usize,
    is_bypass: bool,
    is_output: bool,
    visible: bool,
}

impl ParameterInfo {
    pub fn new(id: ParameterId, name: String) -> Self {
        Self {
            id,
            name,
            path: Default::default(),
            default_normalized_value: Default::default(),
            steps: 0,
            is_bypass: false,
            is_output: true,
            visible: true,
        }
    }

    pub fn with_path(mut self, path: String) -> Self {
        self.path = path;
        self
    }

    pub fn with_default_normalized_value(mut self, value: ParameterValue) -> Self {
        self.default_normalized_value = value;
        self
    }

    pub fn with_steps(mut self, steps: usize) -> Self {
        self.steps = steps;
        self
    }

    pub fn as_bypass(mut self) -> Self {
        self.is_bypass = true;
        self
    }

    /// By default, parameters are assumed to affect plugin output
    /// Calling this function will mark the parameter as not affecting the output,
    /// which might improve performance
    pub fn not_output(mut self) -> Self {
        self.is_output = false;
        self
    }

    pub fn hidden(mut self) -> Self {
        self.visible = false;
        self
    }

    pub fn id(&self) -> ParameterId {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn default_normalized_value(&self) -> ParameterValue {
        self.default_normalized_value
    }

    pub fn steps(&self) -> usize {
        self.steps
    }
    
    pub fn is_bypass(&self) -> bool {
        self.is_bypass
    }

    pub fn is_output(&self) -> bool {
        self.is_output
    }

    pub fn visible(&self) -> bool {
        self.visible
    }
}
