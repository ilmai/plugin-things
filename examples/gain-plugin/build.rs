use std::{error::Error, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let config = slint_build::CompilerConfiguration::new()
        .with_include_paths(vec![
            PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("ui/"),
        ]);

    slint_build::compile_with_config("ui/main.slint", config)?;

    Ok(())
}
