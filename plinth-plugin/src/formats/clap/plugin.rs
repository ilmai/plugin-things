use crate::plugin::Plugin;

use super::features::Feature;

pub trait ClapPlugin : Plugin {
    const CLAP_ID: &'static str;

    const FEATURES: &'static [Feature];

    const MANUAL_URL: Option<&'static str> = None;
    const SUPPORT_URL: Option<&'static str> = None;
    const DESCRIPTION: Option<&'static str> = None;
}
