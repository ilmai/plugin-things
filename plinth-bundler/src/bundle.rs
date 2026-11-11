use std::{
    fs,
    io::BufReader,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use anyhow::{Context, bail};
use cargo_metadata::{Message, MetadataCommand, semver::Version};

use crate::{Result, config::BundleConfig};

const USAGE: &str = "Usage: cargo xtask bundle <package> [--universal] [cargo build options]

Builds <package> and bundles it as CLAP, VST3 and (macOS only, with an auv2-id) AUv2 into
target/bundled. Builds for the host target, or for --target, or, with --universal, as a
universal x86_64 + arm64 macOS binary.";

const PLUGIN_FORMATS: &[&str] = &["auv2", "clap", "vst3"];

fn plugin_format_name(plugin_format: &str) -> &'static str {
    match plugin_format {
        "auv2" => "AUv2",
        "clap" => "CLAP",
        "vst3" => "VST3",
        _ => unimplemented!()
    }
}

pub fn main() -> Result<()> {
    // TODO: We might want more elegant command line handling?
    let mut args = std::env::args().skip(1);
    let (Some("bundle"), Some(package_name)) = (args.next().as_deref(), args.next()) else {
        bail!(USAGE);
    };
    let mut cargo_args = args.collect::<Vec<_>>();
    let universal = match cargo_args.iter().position(|arg| arg == "--universal") {
        Some(index) => {
            cargo_args.remove(index);
            true
        }
        None => false,
    };
    let features = match cargo_args.iter().position(|arg| arg == "--features") {
        Some(index) => {
            if cargo_args.len() < index + 2 {
                bail!("Missing list of features after --features");
            }

            let default_features: Vec<_> = cargo_args[index + 1]
                .split(",")
                .map(String::from)
                .collect();

            cargo_args.remove(index);
            cargo_args.remove(index);

            default_features
        }

        None => Default::default(),
    };

    let package = Package::load(&package_name)?;
    let platform = match target_triple_argument(&cargo_args) {
        Some(triple) => {
            if universal {
                bail!("--universal can not be combined with --target");
            }
            Platform::from_target_triple(triple)?
        }
        None => Platform::from_host()?,
    };
    if universal && !matches!(platform, Platform::MacOS) {
        bail!("--universal is only supported on macOS");
    }

    // Combine command line and default features
    let mut features: Vec<_> = features
        .iter()
        .cloned()
        .chain(package.default_features.iter().cloned())
        .collect();
    features.sort();
    features.dedup();

    // Collect plugin format features
    let mut build_plugin_formats = Vec::new();
    for &plugin_format in PLUGIN_FORMATS {
        if features.iter().find(|feature| feature.as_str() == plugin_format).is_some() {
            build_plugin_formats.push(plugin_format);
        }
    }
    features.retain(|feature| !PLUGIN_FORMATS.contains(&feature.as_str()));

    let bundle_dir = package.target_dir.join("bundled");
    fs::create_dir_all(&bundle_dir)?;

    let build = |plugin_format| build_plugin_library(&package, plugin_format, &features, universal, &cargo_args);

    match platform {
        Platform::MacOS => {
            let bundle_id_prefix =
                package.config.bundle_id_prefix.as_deref().context(
                    "[package.metadata.bundle] needs a bundle-id-prefix for macOS bundles",
                )?;

            for &plugin_format in build_plugin_formats.iter() {
                eprintln!("Building {}", plugin_format_name(plugin_format));

                let library = build(plugin_format)?;

                let (extension, extra_plist_keys) = match plugin_format {
                    "auv2" => {
                        let audio_components = (package.config.auv2_id.as_deref())
                            .map(|auv2_id| audio_components_plist(&package, auv2_id))
                            .transpose()?
                            .expect("Trying to build AUv2 but plugin metadata doesn't have an AUv2 ID");

                        ("component", audio_components)
                    }

                    _ => (plugin_format, Default::default())
                };

                create_macos_bundle(
                    &package,
                    bundle_id_prefix,
                    &bundle_dir,
                    extension,
                    &library,
                    &extra_plist_keys,
                )?;
            }
        }
        Platform::Windows { vst3_arch } => {
            for &plugin_format in build_plugin_formats.iter() {
                if plugin_format == "auv2" {
                    continue;
                }

                eprintln!("Building {}", plugin_format_name(plugin_format));
                create_windows_bundle(&package, plugin_format, &bundle_dir, &build(plugin_format)?, vst3_arch)?;
            }
        }
        Platform::Linux { vst3_arch } => {
            for &plugin_format in build_plugin_formats.iter() {
                if plugin_format == "auv2" {
                    continue;
                }

                eprintln!("Building {}", plugin_format_name(plugin_format));
                create_linux_bundle(&package, plugin_format, &bundle_dir, &build(plugin_format)?, vst3_arch)?;
            }
        }
    }

    Ok(())
}

/// The package to bundle.
struct Package {
    name: String,
    version: Version,
    config: BundleConfig,
    target_dir: PathBuf,
    default_features: Vec<String>,
}

impl Package {
    fn load(name: &str) -> Result<Self> {
        let metadata = MetadataCommand::new()
            .no_deps()
            .exec()
            .context("Could not run `cargo metadata`")?;
        let package = metadata
            .packages
            .into_iter()
            .find(|package| package.name == name)
            .with_context(|| format!("Package '{name}' not found in workspace"))?;
        let config =
            package.metadata.get("bundle").cloned().with_context(|| {
                format!("Cargo.toml of '{name}' has no [package.metadata.bundle]")
            })?;
        let default_features = package.features.get("default").cloned().unwrap_or_default();

        Ok(Self {
            name: name.to_string(),
            version: package.version,
            target_dir: metadata.target_directory.into_std_path_buf(),
            config: serde_json::from_value(config).context("Invalid [package.metadata.bundle]")?,
            default_features,
        })
    }
}

/// The platform to bundle for, along with a VST3 architecture folder for Linux and Windows.
#[derive(Clone, Copy)]
enum Platform {
    MacOS,
    Windows { vst3_arch: &'static str },
    Linux { vst3_arch: &'static str },
}

impl Platform {
    fn new(os: &str, arch: &str) -> Result<Self> {
        let (windows_arch, linux_arch) = match arch {
            "x86_64" => ("x86_64-win", "x86_64-linux"),
            "aarch64" => ("arm64-win", "aarch64-linux"),
            _ => bail!("Unsupported architecture '{arch}'"),
        };
        match os {
            "macos" => Ok(Self::MacOS),
            "windows" => Ok(Self::Windows {
                vst3_arch: windows_arch,
            }),
            "linux" => Ok(Self::Linux {
                vst3_arch: linux_arch,
            }),
            _ => bail!("Unsupported OS '{os}'"),
        }
    }

    fn from_host() -> Result<Self> {
        Self::new(std::env::consts::OS, std::env::consts::ARCH)
    }

    fn from_target_triple(triple: &str) -> Result<Self> {
        let arch = triple.split('-').next().unwrap_or_default();
        let os = if triple.contains("darwin") {
            "macos"
        } else if triple.contains("windows") {
            "windows"
        } else if triple.contains("linux") {
            "linux"
        } else {
            bail!("Unsupported target '{triple}'");
        };
        Self::new(os, arch)
    }
}

fn target_triple_argument(cargo_args: &[String]) -> Option<&str> {
    cargo_args.iter().enumerate().find_map(|(index, arg)| {
        if arg == "--target" {
            cargo_args.get(index + 1).map(String::as_str)
        } else {
            arg.strip_prefix("--target=")
        }
    })
}

/// Builds the plugin library with *only* the given format features.
/// When `universal` is true, both macOS architectures are built and merged via lipo.
fn build_plugin_library(
    package: &Package,
    plugin_format: &str,
    features: &[String],
    universal: bool,
    cargo_args: &[String],
) -> Result<PathBuf> {
    if cfg!(target_os = "macos") && universal {
        let mut libraries = Vec::new();
        for triple in ["x86_64-apple-darwin", "aarch64-apple-darwin"] {
            let mut cargo_args = cargo_args.to_vec();
            cargo_args.push(format!("--target={triple}"));
            libraries.push(cargo_build(package, plugin_format, features, &cargo_args)?);
        }
        let universal_library = package
            .target_dir
            .join("universal-apple-darwin")
            .join(libraries[0].file_name().unwrap());
        fs::create_dir_all(universal_library.parent().unwrap())?;
        run_command(
            Command::new("lipo")
                .arg("-create")
                .arg("-output")
                .arg(&universal_library)
                .args(&libraries),
        )?;
        Ok(universal_library)
    } else {
        cargo_build(package, plugin_format, features, cargo_args)
    }
}

/// Runs `cargo build` and returns the path of the built library.
fn cargo_build(
    package: &Package,
    plugin_format: &str,
    features: &[String],
    cargo_args: &[String],
) -> Result<PathBuf> {
    let features = format!("{plugin_format},{}", features.join(","));
    let mut cargo = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
        .args(["build", "--lib", "--package", &package.name])
        .args(["--no-default-features", "--features", &features])
        .arg("--message-format=json-render-diagnostics")
        .args(cargo_args)
        .stdout(Stdio::piped())
        .spawn()
        .context("Could not run `cargo build`")?;

    let library_name = package.name.replace('-', "_");
    let mut library = None;
    for message in Message::parse_stream(BufReader::new(cargo.stdout.take().unwrap())) {
        if let Message::CompilerArtifact(artifact) = message?
            && artifact.target.is_cdylib()
            && artifact.target.name == library_name
        {
            library = artifact
                .filenames
                .into_iter()
                .find(|file| matches!(file.extension(), Some("dylib" | "so" | "dll")));
        }
    }

    if !cargo.wait()?.success() {
        bail!("`cargo build` failed");
    }
    Ok(library
        .context("`cargo build` produced no plugin library")?
        .into_std_path_buf())
}

/// Creates and signs a macOS bundle (vst3, clap or component) around the given library file.
fn create_macos_bundle(
    package: &Package,
    bundle_id_prefix: &str,
    bundle_dir: &Path,
    extension: &str,
    library: &Path,
    extra_plist_keys: &str,
) -> Result<()> {
    let name = &package.config.name;

    let bundle = bundle_dir.join(format!("{name}.{extension}"));
    let contents = bundle.join("Contents");
    let bundle_id = format!("{bundle_id_prefix}.{extension}");

    remove_dir(&bundle)?;
    copy_file(library, &contents.join("MacOS").join(name))?;
    fs::write(contents.join("PkgInfo"), "BNDL????")?;
    fs::write(
        contents.join("Info.plist"),
        info_plist(package, &bundle_id, extra_plist_keys),
    )?;
    // The linker only ad-hoc signs the binary, not the bundle around it
    run_command(
        Command::new("codesign")
            .args(["--force", "--sign", "-"])
            .arg(&bundle),
    )?;

    eprintln!("Created '{}'", bundle.display());
    Ok(())
}

/// Creates a plain `.clap` library file and a `.vst3` folder with the library in its architecture
/// subfolder and a `desktop.ini` + `PlugIn.ico` as Explorer icon.
fn create_windows_bundle(
    package: &Package,
    plugin_format: &str,
    bundle_dir: &Path,
    library: &Path,
    vst3_arch: &str,
) -> Result<()> {
    let name = &package.config.name;

    match plugin_format {
        "clap" => {
            let clap = bundle_dir.join(format!("{name}.clap"));
            copy_file(library, &clap)?;
            eprintln!("Created '{}'", clap.display());
        }

        "vst3" => {
            let vst3 = bundle_dir.join(format!("{name}.vst3"));
            let vst3_library = vst3
                .join("Contents")
                .join(vst3_arch)
                .join(format!("{name}.vst3"));
            let desktop_ini = vst3.join("desktop.ini");
            let icon = vst3.join("PlugIn.ico");
            remove_dir(&vst3)?;
            copy_file(library, &vst3_library)?;
            fs::write(
                &desktop_ini,
                "[.ShellClassInfo]\r\nIconResource=PlugIn.ico,0\r\n",
            )?;
            // MIT licensed, see icons/README.md
            fs::write(&icon, include_bytes!("../icons/VST_Logo_Steinberg.ico"))?;

            // `attrib` only exists on Windows: when cross-building, the installer has to set them
            if cfg!(windows) {
                run_command(
                    Command::new("attrib")
                        .args(["+s", "+r", "+h"])
                        .arg(&desktop_ini),
                )?;
                run_command(Command::new("attrib").args(["+r", "+h"]).arg(&icon))?;
                run_command(Command::new("attrib").arg("+s").arg(&vst3))?;
            }
            eprintln!("Created '{}'", vst3.display());
        }

        _ => unimplemented!()
    }

    Ok(())
}

/// Creates a plain `.clap` library file and a `.vst3` folder with the library in its architecture
/// subfolder.
fn create_linux_bundle(
    package: &Package,
    plugin_format: &str,
    bundle_dir: &Path,
    library: &Path,
    vst3_arch: &str,
) -> Result<()> {
    let name = &package.config.name;

    match plugin_format {
        "clap" => {
            let clap = bundle_dir.join(format!("{name}.clap"));
            copy_file(library, &clap)?;
            eprintln!("Created '{}'", clap.display());
        }

        "vst3" => {
            let vst3 = bundle_dir.join(format!("{name}.vst3"));
            let vst3_library = vst3
                .join("Contents")
                .join(vst3_arch)
                .join(format!("{name}.so"));
            remove_dir(&vst3)?;
            copy_file(library, &vst3_library)?;
            eprintln!("Created '{}'", vst3.display());
        }

        _ => unimplemented!()
    }

    Ok(())
}

fn info_plist(package: &Package, bundle_id: &str, extra_keys: &str) -> String {
    let BundleConfig { name, vendor, .. } = &package.config;
    let version = &package.version;
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
  <dict>
    <key>CFBundleExecutable</key>
    <string>{name}</string>
    <key>CFBundleIdentifier</key>
    <string>{bundle_id}</string>
    <key>CFBundleName</key>
    <string>{name}</string>
    <key>CFBundleDisplayName</key>
    <string>{name}</string>
    <key>CFBundlePackageType</key>
    <string>BNDL</string>
    <key>CFBundleSignature</key>
    <string>????</string>
    <key>CFBundleShortVersionString</key>
    <string>{version}</string>
    <key>CFBundleVersion</key>
    <string>{version}</string>
    <key>CFBundleSupportedPlatforms</key>
    <array>
      <string>MacOSX</string>
    </array>
    <key>NSHumanReadableCopyright</key>
    <string>{vendor}</string>
    <key>NSHighResolutionCapable</key>
    <true/>{extra_keys}
  </dict>
</plist>
"#
    )
}

fn audio_components_plist(package: &Package, auv2_id: &str) -> Result<String> {
    let BundleConfig { name, vendor, .. } = &package.config;
    let codes = auv2_id.split(':').collect::<Vec<_>>();
    let [au_type, au_subtype, au_manufacturer] = codes[..] else {
        bail!("auv2-id must be 'type:subtype:manufacturer'");
    };
    if codes.iter().any(|code| code.len() != 4 || !code.is_ascii()) {
        bail!("auv2-id codes must be 4 ASCII characters, got '{auv2_id}'");
    }
    let version = &package.version;
    let au_version = (version.major << 16) | (version.minor << 8) | version.patch;

    Ok(format!(
        r#"
    <key>AudioComponents</key>
    <array>
      <dict>
        <key>name</key>
        <string>{vendor}: {name}</string>
        <key>description</key>
        <string>{vendor}: {name}</string>
        <key>factoryFunction</key>
        <string>GetPluginFactoryAUV2</string>
        <key>type</key>
        <string>{au_type}</string>
        <key>subtype</key>
        <string>{au_subtype}</string>
        <key>manufacturer</key>
        <string>{au_manufacturer}</string>
        <key>version</key>
        <integer>{au_version}</integer>
        <key>resourceUsage</key>
        <dict>
          <key>temporary-exception.files.all.read-write</key>
          <true/>
        </dict>
      </dict>
    </array>"#
    ))
}

fn remove_dir(path: &Path) -> Result<()> {
    if path.is_dir() {
        fs::remove_dir_all(path)?;
    }
    Ok(())
}

fn copy_file(from: &Path, to: &Path) -> Result<()> {
    fs::create_dir_all(to.parent().unwrap())?;
    fs::copy(from, to).with_context(|| format!("Could not copy to '{}'", to.display()))?;
    Ok(())
}

fn run_command(command: &mut Command) -> Result<()> {
    let program = command.get_program().to_string_lossy().into_owned();
    let status = command
        .status()
        .with_context(|| format!("Could not run `{program}`"))?;
    if !status.success() {
        bail!("`{program}` failed with {status}");
    }
    Ok(())
}
