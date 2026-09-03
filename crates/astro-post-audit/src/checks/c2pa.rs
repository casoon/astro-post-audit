//! Opt-in, local C2PA Content Credentials validation.
//!
//! Missing credentials are only reported for paths explicitly configured in
//! `c2pa.require_for`; their absence never implies that an image is AI-made.

use c2pa::{Reader, ValidationState};
use globset::{Glob, GlobSet, GlobSetBuilder};
use std::fs::File;
use walkdir::WalkDir;

use crate::config::Config;
use crate::discovery::SiteIndex;
use crate::report::{Finding, Level};

pub fn check_all(index: &SiteIndex, config: &Config) -> Vec<Finding> {
    if !config.c2pa.enabled {
        return Vec::new();
    }
    let required = glob_set(&config.c2pa.require_for);
    WalkDir::new(&index.dist_path)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter_map(|entry| {
            let path = entry.path();
            let format = image_format(path.extension()?.to_str()?)?;
            let rel = path
                .strip_prefix(&index.dist_path)
                .ok()?
                .to_string_lossy()
                .replace('\\', "/");
            let required = required.as_ref().is_some_and(|set| set.is_match(&rel));
            validate(path, &rel, format, required, config.c2pa.require_trusted)
        })
        .collect()
}

fn image_format(extension: &str) -> Option<&'static str> {
    match extension.to_ascii_lowercase().as_str() {
        "jpg" | "jpeg" => Some("image/jpeg"),
        "png" => Some("image/png"),
        "webp" => Some("image/webp"),
        _ => None,
    }
}

fn glob_set(patterns: &[String]) -> Option<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        builder.add(Glob::new(pattern).ok()?);
    }
    (!patterns.is_empty())
        .then(|| builder.build().ok())
        .flatten()
}

fn validate(
    path: &std::path::Path,
    rel: &str,
    format: &str,
    required: bool,
    require_trusted: bool,
) -> Option<Finding> {
    let Ok(file) = File::open(path) else {
        return required.then(|| {
            finding(
                "c2pa/missing-required",
                rel,
                "The configured image asset could not be read for C2PA validation.",
                Level::Warning,
            )
        });
    };
    let reader = Reader::default().with_stream(format, file);
    let Ok(reader) = reader else {
        return required.then(|| finding("c2pa/missing-required", rel, "No readable C2PA Content Credentials found for an asset configured as requiring them.", Level::Warning));
    };
    if reader.active_manifest().is_none() {
        return required.then(|| finding("c2pa/missing-required", rel, "No embedded C2PA Content Credentials found for an asset configured as requiring them.", Level::Warning));
    }
    finding_for_state(reader.validation_state(), rel, require_trusted)
}

fn finding_for_state(state: ValidationState, rel: &str, require_trusted: bool) -> Option<Finding> {
    match state {
        ValidationState::Invalid => Some(finding(
            "c2pa/invalid",
            rel,
            "Embedded C2PA Content Credentials could not be validated.",
            Level::Warning,
        )),
        ValidationState::Valid if require_trusted => Some(finding(
            "c2pa/untrusted",
            rel,
            "Embedded C2PA Content Credentials are cryptographically valid but signed by a certificate that does not chain to a trusted root.",
            Level::Warning,
        )),
        ValidationState::Valid | ValidationState::Trusted => None,
    }
}

fn finding(rule_id: &str, file: &str, message: &str, level: Level) -> Finding {
    Finding::new(level, rule_id, file, "image asset", message, "Review the asset's provenance. Missing credentials do not prove that an image is AI-generated or non-compliant.", None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supports_the_mvp_image_formats() {
        assert_eq!(image_format("jpg"), Some("image/jpeg"));
        assert_eq!(image_format("png"), Some("image/png"));
        assert_eq!(image_format("webp"), Some("image/webp"));
        assert_eq!(image_format("avif"), None);
    }

    #[test]
    fn missing_credentials_are_only_reported_when_required() {
        let missing = std::path::Path::new("does-not-exist.png");
        assert!(validate(missing, "ai/example.png", "image/png", false, false).is_none());
        let finding = validate(missing, "ai/example.png", "image/png", true, false).unwrap();
        assert_eq!(finding.rule_id, "c2pa/missing-required");
    }

    #[test]
    fn valid_but_untrusted_is_silent_unless_require_trusted() {
        assert!(finding_for_state(ValidationState::Valid, "ai/example.png", false).is_none());
        let finding = finding_for_state(ValidationState::Valid, "ai/example.png", true).unwrap();
        assert_eq!(finding.rule_id, "c2pa/untrusted");
    }

    #[test]
    fn trusted_is_always_silent() {
        assert!(finding_for_state(ValidationState::Trusted, "ai/example.png", false).is_none());
        assert!(finding_for_state(ValidationState::Trusted, "ai/example.png", true).is_none());
    }
}
