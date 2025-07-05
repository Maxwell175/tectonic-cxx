use std::env;
use std::fs;
use std::path::PathBuf;
use anyhow::{Context, Result};

fn main() -> Result<()> {
    cxx_build::bridge("src/lib.rs")
        .include("include")
        .compile("tectonic");

    let out_dir = PathBuf::from(env::var("OUT_DIR").context("OUT_DIR environment variable not set")?);

    let generated_header_path = out_dir.join("cxxbridge/include/tectonic-cxx/src/lib.rs.h");

    let target_dir = PathBuf::from(env::var("CARGO_TARGET_DIR").unwrap_or_else(|_| "target".to_string()));
    let target_header_path = target_dir.join("tectonic.h");

    // Read the generated header
    let generated_content = fs::read_to_string(&generated_header_path)
        .with_context(|| format!("Failed to read generated header from {:?}", generated_header_path))?;

    // Create a deployment-ready version by:
    // 1. Removing the interface include from the top
    // 2. Adding it after the rust namespace is defined
    let mut deployment_content = generated_content.replace(
        r#"#include "tectonic-cxx-interface.h""#,
        ""
    );

    // Find where to insert the interface include (after rust namespace definitions)
    if let Some(pos) = deployment_content.find("} // namespace rust") {
        // Find the end of that line
        if let Some(line_end) = deployment_content[pos..].find('\n') {
            let insert_pos = pos + line_end + 1;
            deployment_content.insert_str(insert_pos, "\n#include \"tectonic-cxx-interface.h\"\n");
        }
    }

    fs::write(&target_header_path, deployment_content)
        .with_context(|| {
            format!(
                "Failed to write deployment header to {:?}",
                target_header_path
            )
        })?;

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/lib.rs");
    
    Ok(())
}