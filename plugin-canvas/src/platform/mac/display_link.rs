use std::{ptr::null_mut, ffi::{c_void, c_double}};

use objc2_foundation::CGRect;

pub(super) fn get_displays_with_rect(rect: CGRect) -> Vec<u32> {
    const MAX_DISPLAYS: usize = 10;
    let mut displays = vec![0; MAX_DISPLAYS];
    let mut matching_displays = 0;

    unsafe {
        let result = CGGetDisplaysWithRect(rect, MAX_DISPLAYS as u32, displays.as_mut_ptr(), &mut matching_displays);
        assert!(result == CGError::Success, "CGGetDisplaysWithRect failed: {result:?}");
    }

    displays.resize(matching_displays as usize, 0);
    displays
}

pub(super) fn create_with_active_cg_displays() -> CVDisplayLinkRef {
    let mut display_link_ptr = null_mut();
    unsafe {
        let result = CVDisplayLinkCreateWithActiveCGDisplays(&mut display_link_ptr);
        assert!(result == CVReturn::Success, "CVDisplayLinkCreateWithActiveCGDisplays failed: {result:?}");
    }

    CVDisplayLinkRef(display_link_ptr)
}

pub(super) fn set_output_callback(display_link: &mut CVDisplayLinkRef, callback: CVDisplayLinkOutputCallback, user_info: *mut c_void) {
    assert!(!display_link.0.is_null());

    unsafe {
        let result = CVDisplayLinkSetOutputCallback(display_link.0, callback, user_info);
        assert!(result == CVReturn::Success, "CVDisplayLinkSetOutputCallback failed: {result:?}");
    }
}

pub(super) fn set_current_display(display_link: &mut CVDisplayLinkRef, display: u32) {
    assert!(!display_link.0.is_null());

    unsafe {
        let result = CVDisplayLinkSetCurrentCGDisplay(display_link.0, display);
        assert!(result == CVReturn::Success, "CVDisplayLinkSetCurrentCGDisplay failed: {result:?}");
    }
}

pub(super) fn start(display_link: &mut CVDisplayLinkRef) {
    assert!(!display_link.0.is_null());

    unsafe {
        let result = CVDisplayLinkStart(display_link.0);
        assert!(result == CVReturn::Success, "CVDisplayLinkStart failed: {result:?}");
    }
}

pub(super) fn release(display_link: &mut CVDisplayLinkRef) {
    assert!(!display_link.0.is_null());

    unsafe {
        CVDisplayLinkRelease(display_link.0);
    }
}

#[repr(C)]
#[derive(Debug, Clone)]
pub(super) struct CVSMPTETime {
    pub subframes: i16,
    pub subframe_divisor: i16,
    pub counter: u32,
    pub type_: u32,
    pub flags: u32,
    pub hours: i16,
    pub minutes: i16,
    pub seconds: i16,
    pub frames: i16,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub(super) struct CVTimeStamp {
    pub version: u32,
    pub video_time_scale: i32,
    pub video_time: i64,
    pub host_time: u64,
    pub rate_scalar: c_double,
    pub video_refresh_period: i64,
    pub smpte_time: CVSMPTETime,
    pub flags: u64,
    pub reserved: u64,
}

#[repr(i32)]
#[derive(Debug, PartialEq)]
#[allow(dead_code)]
pub(super) enum CGError {
    Success = 0,
    Failure = 1000,
    IllegalArgument = 1001,
    InvalidConnection = 1002,
    InvalidContext = 1003,
    CannotComplete = 1004,
    NotImplemented = 1006,
    RangeCheck = 1007,
    TypeCheck = 1008,
    InvalidOperation = 1010,
    NoneAvailable = 1011,
}

#[repr(i32)]
#[derive(Debug, PartialEq)]
#[allow(dead_code)]
pub(super) enum CVReturn {
    Success = 0,
    Error = -6660,
    InvalidArgument = -6661,
    AllocationFailed = -6662,
    Unsupported = -6663,
    Last = -6699,
}

#[repr(C)]
struct CVDisplayLink(c_void);

#[repr(C)]
pub struct CVDisplayLinkRef(*mut CVDisplayLink);

type CVDisplayLinkOutputCallback = unsafe extern "C" fn(
    display_link: CVDisplayLinkRef,
    in_now: *mut CVTimeStamp,
    in_output_time: *mut CVTimeStamp,
    flags_in: u64,
    flags_out: *mut u64,
    display_link_context: *mut c_void,
) -> CVReturn;

#[link(name = "CoreFoundation", kind = "framework")]
#[link(name = "CoreVideo", kind = "framework")]
#[allow(improper_ctypes)]
extern "C" {
    fn CGGetDisplaysWithRect(
        rect: CGRect,
        maxDisplays: u32,
        displays: *mut u32,
        matchingDisplayCount: *mut u32
    ) -> CGError;

    fn CVDisplayLinkCreateWithActiveCGDisplays(
        display_link_out: *mut *mut CVDisplayLink,
    ) -> CVReturn;

    fn CVDisplayLinkSetOutputCallback(
        display_link: *mut CVDisplayLink,
        callback: CVDisplayLinkOutputCallback,
        user_info: *mut c_void,
    ) -> CVReturn;

    fn CVDisplayLinkSetCurrentCGDisplay(
        display_link: *mut CVDisplayLink,
        display_id: u32,
    ) -> CVReturn;

    fn CVDisplayLinkStart(display_link: *mut CVDisplayLink) -> CVReturn;
    fn _CVDisplayLinkStop(display_link: *mut CVDisplayLink) -> CVReturn;
    fn CVDisplayLinkRelease(display_link: *mut CVDisplayLink);
    fn _CVDisplayLinkRetain(display_link: *mut CVDisplayLink) -> *mut CVDisplayLink;
}
