use std::fs;
use std::path::Path;

/// Run the binary with JSON config on stdin and return raw stdout/stderr/code.
pub fn run_audit(dist_path: &Path, config_json: &str) -> (String, String, i32) {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let bin = env!("CARGO_BIN_EXE_astro-post-audit");
    let mut cmd = Command::new(bin);
    cmd.arg(dist_path.to_str().unwrap())
        .arg("--config-stdin")
        .env("NO_COLOR", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("failed to spawn binary");
    child
        .stdin
        .take()
        .expect("missing child stdin")
        .write_all(config_json.as_bytes())
        .expect("failed to write stdin");
    let output = child.wait_with_output().expect("failed to wait");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let code = output.status.code().unwrap_or(2);
    (stdout, stderr, code)
}

/// Run the binary with JSON config (`format=json`) and return parsed output.
pub fn run_audit_json(dist_path: &Path, config_json: &str) -> (serde_json::Value, i32) {
    let mut config: serde_json::Value =
        serde_json::from_str(config_json).unwrap_or(serde_json::json!({}));
    config
        .as_object_mut()
        .expect("config must be object")
        .insert("format".to_string(), serde_json::json!("json"));
    let merged = config.to_string();

    let (stdout, _stderr, code) = run_audit(dist_path, &merged);
    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!(
            "Failed to parse JSON output: {}\nOutput was:\n{}",
            e, stdout
        );
    });
    (json, code)
}

/// Create a minimal valid page in a temp dir.
pub fn write_valid_page(dir: &Path, rel_path: &str, title: &str, h1: &str, canonical_path: &str) {
    let full = dir.join(rel_path);
    if let Some(parent) = full.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(
        &full,
        format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{title}</title>
  <link rel="canonical" href="https://example.com{canonical_path}">
</head>
<body>
  <h1>{h1}</h1>
</body>
</html>"#
        ),
    )
    .unwrap();
}
