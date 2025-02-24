use crate::{parameters::info::ParameterInfo, ParameterId};

pub(super) struct CachedParameter {
    pub id: ParameterId,
    pub info: ParameterInfo,
    pub value: f32,
}
