//! Core rendering logic
//!
//! This module handles the actual Typst compilation with timeout handling
//! and error extraction.

use std::collections::HashMap;
use std::time::Duration;

use base64::Engine;
use typst::diag::{Severity, SourceDiagnostic};
use typst::foundations::Bytes;
use typst::model::Document;

use super::errors::{CompileError, RenderStatus, ServerError};
use super::output::OutputFormat;
use super::{RenderArtifact, RenderRequest, RenderResponse};
use crate::templates;
use crate::world::VirtualWorld;

/// Compile a Typst document with timeout
pub async fn compile_document(
    request: RenderRequest,
    timeout_ms: u64,
) -> Result<RenderResponse, ServerError> {
    // 1. Resolve source - check if it's a template URI or raw source
    let source = if request.source.starts_with("typst://templates/") {
        let template_name = request
            .source
            .strip_prefix("typst://templates/")
            .unwrap_or(&request.source);
        templates::get_template_source(template_name)?
    } else {
        request.source.clone()
    };

    // 2. Decode base64 assets
    let assets = decode_assets(&request.assets)?;

    // 3. Create VirtualWorld
    let world = VirtualWorld::new(source, request.inputs.clone(), assets)?;

    // 4. Compile with timeout
    let result = tokio::time::timeout(
        Duration::from_millis(timeout_ms),
        tokio::task::spawn_blocking(move || typst::compile(&world)),
    )
    .await;

    // 5. Handle timeout and join error
    let compile_result = match result {
        Ok(Ok(result)) => result,
        Ok(Err(join_error)) => {
            return Err(ServerError::SourceError(format!(
                "Compilation task panicked: {}",
                join_error
            )));
        }
        Err(_timeout) => {
            return Err(ServerError::Timeout(timeout_ms));
        }
    };

    // 6. Process compilation result
    // Warned<Result<Document, EcoVec<SourceDiagnostic>>> - access .output field
    let warned = compile_result;
    let compilation_warnings = warned.warnings.clone();

    match warned.output {
        Ok(document) => {
            // Export to requested format
            let artifact = export_document(&document, request.format, request.ppi)?;
            let (_, warnings) = categorize_diagnostics(&compilation_warnings);
            Ok(RenderResponse {
                status: RenderStatus::Success,
                artifact: Some(artifact),
                errors: vec![],
                warnings,
            })
        }
        Err(diagnostics) => {
            let (errors, warnings) = categorize_diagnostics(&diagnostics);

            if errors.is_empty() {
                // Warnings only - shouldn't happen with Err result, but handle it
                Err(ServerError::CompileError(vec![CompileError::new(
                    "Compilation failed with unknown error",
                )]))
            } else {
                Ok(RenderResponse {
                    status: RenderStatus::Error,
                    artifact: None,
                    errors,
                    warnings,
                })
            }
        }
    }
}

/// Validate Typst syntax without full compilation
pub fn validate_syntax(source: &str) -> Vec<CompileError> {
    use typst::syntax::parse;

    let parsed = parse(source);

    parsed
        .errors()
        .into_iter()
        .map(|error| {
            // Span doesn't expose start/end directly in typst 0.12
            CompileError::new(error.message.to_string())
        })
        .collect()
}

/// Decode base64-encoded assets
fn decode_assets(assets: &HashMap<String, String>) -> Result<HashMap<String, Bytes>, ServerError> {
    let engine = base64::engine::general_purpose::STANDARD;

    assets
        .iter()
        .map(|(path, data)| {
            let bytes = engine
                .decode(data)
                .map_err(|e| ServerError::AssetError(path.clone(), e.to_string()))?;
            Ok((path.clone(), Bytes::from(bytes)))
        })
        .collect()
}

/// Export a compiled document to the requested format
fn export_document(
    document: &Document,
    format: OutputFormat,
    ppi: Option<u32>,
) -> Result<RenderArtifact, ServerError> {
    let engine = base64::engine::general_purpose::STANDARD;

    match format {
        OutputFormat::Pdf => {
            let pdf_bytes = typst_pdf::pdf(document, &typst_pdf::PdfOptions::default())
                .map_err(|e| ServerError::SourceError(format!("PDF export failed: {:?}", e)))?;
            Ok(RenderArtifact {
                data_base64: engine.encode(&pdf_bytes),
                mime_type: format.mime_type().to_string(),
                page_count: document.pages.len(),
            })
        }
        OutputFormat::Svg => {
            // Export first page as SVG
            if let Some(page) = document.pages.first() {
                let svg = typst_svg::svg(page);
                Ok(RenderArtifact {
                    data_base64: engine.encode(svg.as_bytes()),
                    mime_type: format.mime_type().to_string(),
                    page_count: document.pages.len(),
                })
            } else {
                Err(ServerError::SourceError(
                    "Document has no pages".to_string(),
                ))
            }
        }
        OutputFormat::Png => {
            // Export first page as PNG
            let pixels_per_point = ppi.unwrap_or(144) as f32 / 72.0;

            if let Some(page) = document.pages.first() {
                let pixmap = typst_render::render(page, pixels_per_point);
                let png_bytes = pixmap
                    .encode_png()
                    .map_err(|e| ServerError::SourceError(format!("PNG encoding failed: {}", e)))?;
                Ok(RenderArtifact {
                    data_base64: engine.encode(&png_bytes),
                    mime_type: format.mime_type().to_string(),
                    page_count: document.pages.len(),
                })
            } else {
                Err(ServerError::SourceError(
                    "Document has no pages".to_string(),
                ))
            }
        }
    }
}

/// Categorize diagnostics into errors and warnings
fn categorize_diagnostics(
    diagnostics: &[SourceDiagnostic],
) -> (Vec<CompileError>, Vec<CompileError>) {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    for diag in diagnostics {
        let mut compile_error = CompileError::new(diag.message.to_string());

        // Add hints if present
        if !diag.hints.is_empty() {
            let hint = diag
                .hints
                .iter()
                .map(|h| h.to_string())
                .collect::<Vec<_>>()
                .join("; ");
            compile_error = compile_error.with_hint(hint);
        }

        match diag.severity {
            Severity::Error => errors.push(compile_error),
            Severity::Warning => warnings.push(compile_error.as_warning()),
        }
    }

    (errors, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_compile_simple_document() {
        let request = RenderRequest {
            source: "Hello, *World*!".to_string(),
            inputs: HashMap::new(),
            assets: HashMap::new(),
            format: OutputFormat::Pdf,
            ppi: None,
        };

        let result = compile_document(request, 5000).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response.status, RenderStatus::Success);
        assert!(response.artifact.is_some());
    }

    #[tokio::test]
    async fn test_compile_with_inputs() {
        let mut inputs = HashMap::new();
        inputs.insert("name".to_string(), serde_json::json!("Alice"));

        let request = RenderRequest {
            source: r#"#let name = sys.inputs.at("name", default: "World")
Hello, #name!"#
                .to_string(),
            inputs,
            assets: HashMap::new(),
            format: OutputFormat::Pdf,
            ppi: None,
        };

        let result = compile_document(request, 5000).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_compile_syntax_error() {
        let request = RenderRequest {
            source: "#invalid{{{{".to_string(),
            inputs: HashMap::new(),
            assets: HashMap::new(),
            format: OutputFormat::Pdf,
            ppi: None,
        };

        let result = compile_document(request, 5000).await;
        assert!(result.is_ok()); // Returns Ok with error status

        let response = result.unwrap();
        assert_eq!(response.status, RenderStatus::Error);
        assert!(!response.errors.is_empty());
    }

    #[test]
    fn test_validate_syntax_valid() {
        let errors = validate_syntax("Hello, World!");
        assert!(errors.is_empty());
    }

    #[test]
    fn test_validate_syntax_invalid() {
        let errors = validate_syntax("#let x = ");
        assert!(!errors.is_empty());
    }
}
