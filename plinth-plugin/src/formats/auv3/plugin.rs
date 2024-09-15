use crate::Plugin;

pub trait Auv3Plugin : Plugin {
    const AUV3_ID: &'static str;
}
