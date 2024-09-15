use crate::plugin::Plugin;

use super::subcategories::Subcategory;

pub trait Vst3Plugin : Plugin {
    const CLASS_ID: u128;
    const SUBCATEGORIES: &'static [Subcategory];

    const EMAIL: Option<&'static str> = None;
}
