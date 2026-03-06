pub mod keyboard;
pub mod view;
pub mod window;

fn is_os_version_at_least(major: isize, minor: isize, patch: isize) -> bool {
    let process_info = objc2_foundation::NSProcessInfo::processInfo();
    let version = objc2_foundation::NSOperatingSystemVersion {
        majorVersion: major,
        minorVersion: minor,
        patchVersion: patch
    };

    process_info.isOperatingSystemAtLeastVersion(version)
}
