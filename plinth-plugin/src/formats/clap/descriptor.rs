use std::{ffi::{c_char, CStr, CString}, ptr::null};

use clap_sys::{plugin::clap_plugin_descriptor, version::CLAP_VERSION};

use super::plugin::ClapPlugin;

struct DescriptorData {
    id: CString,
    name: CString,
    vendor: CString,
    version: CString,

    features: Vec<CString>,
    feature_pointers: Vec<*const c_char>,

    url: CString,
    manual_url: CString,
    support_url: CString,
    description: CString,
}

// SAFETY: feature_pointers is never modified after creation
unsafe impl Send for DescriptorData {}

pub struct Descriptor {
    data: Box<DescriptorData>,
    raw: clap_plugin_descriptor,
}

impl Descriptor {
    pub fn new<P: ClapPlugin>() -> Self {
        let mut data = Box::new(DescriptorData {
            id: CString::new(P::CLAP_ID).unwrap(),
            name: CString::new(P::NAME).unwrap(),
            vendor: CString::new(P::VENDOR).unwrap(),
            version: CString::new(P::VERSION).unwrap(),

            features: P::FEATURES.iter().map(|feature| CString::new(feature.to_str()).unwrap()).collect(),
            feature_pointers: Vec::new(),

            url: CString::new(P::URL.unwrap_or_default()).unwrap(),
            manual_url: CString::new(P::MANUAL_URL.unwrap_or_default()).unwrap(),
            support_url: CString::new(P::SUPPORT_URL.unwrap_or_default()).unwrap(),
            description: CString::new(P::DESCRIPTION.unwrap_or_default()).unwrap(),
        });

        // Save feature string pointers
        data.feature_pointers = data.features
            .iter()
            .map(|feature| feature.as_ptr())
            .collect();
        data.feature_pointers.push(null()); // Null terminator

        let raw = clap_plugin_descriptor {
            clap_version: CLAP_VERSION,
            id: data.id.as_ptr(),
            name: data.name.as_ptr(),
            vendor: data.vendor.as_ptr(),
            url: data.url.as_ptr(),
            manual_url: data.manual_url.as_ptr(),
            support_url: data.support_url.as_ptr(),
            version: data.version.as_ptr(),
            description: data.description.as_ptr(),
            features: data.feature_pointers.as_ptr(),
        };
 
        Self {
            data,
            raw,
        }
    }

    pub fn as_raw(&self) -> &clap_plugin_descriptor {
        &self.raw
    }

    pub fn id(&self) -> &CStr {
        &self.data.id
    }
}
