//! Lease verification module
//!
//! Provides PDF text extraction, document parsing, compliance rule checking,
//! and anomaly detection for Florida residential leases.

pub mod anomaly;
pub mod extract;
pub mod parser;
pub mod rules;

// Re-export main types
pub use anomaly::{Anomaly, AnomalyDetector, CanonicalStructure};
pub use extract::{ExtractedDocument, PageContent, PdfExtractor, PdfMetadata};
pub use parser::{FinancialTerms, LeaseDates, LeaseParser, LeaseSection, ParsedLease, Parties};
pub use rules::{get_florida_rules, ComplianceRule, RuleResult, Severity};

use thiserror::Error;

/// Errors that can occur during lease verification
#[derive(Error, Debug)]
pub enum VerifierError {
    #[error("PDF extraction failed: {0}")]
    ExtractionError(String),

    #[error("Document parsing failed: {0}")]
    ParsingError(String),

    #[error("Invalid PDF: {0}")]
    InvalidPdf(String),

    #[error("Password-protected PDF")]
    PasswordProtected,

    #[error("Scanned PDF detected - OCR required")]
    ScannedPdfNeedsOcr,

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Complete verification report for a lease document
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VerificationReport {
    /// Input file name or identifier
    pub input_file: String,

    /// Results of all compliance checks
    pub compliance_results: Vec<ComplianceCheckResult>,

    /// Detected anomalies
    pub anomalies: Vec<Anomaly>,

    /// Overall summary
    pub summary: VerificationSummary,
}

/// Result of a single compliance check
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ComplianceCheckResult {
    /// Name of the rule
    pub rule_name: String,

    /// Statute or regulation reference
    pub statute: String,

    /// Result of the check
    pub result: RuleResult,
}

/// Summary of verification results
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VerificationSummary {
    /// Total number of rules checked
    pub total_rules: usize,

    /// Number of rules that passed
    pub passed: usize,

    /// Number of rules that failed
    pub failed: usize,

    /// Number of warnings
    pub warnings: usize,

    /// Number of anomalies detected
    pub anomalies_found: usize,

    /// Overall status
    pub status: VerificationStatus,
}

/// Overall verification status
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationStatus {
    /// All rules passed, no anomalies
    Compliant,
    /// Some warnings but no failures
    CompliantWithWarnings,
    /// One or more rules failed
    NonCompliant,
}

impl VerificationReport {
    /// Generate a text report
    pub fn to_text(&self) -> String {
        let mut output = String::new();

        output.push_str(&format!("Lease Verification Report: {}\n", self.input_file));
        output.push_str(&"=".repeat(60));
        output.push_str("\n\n");

        // Summary
        output.push_str(&format!("Status: {:?}\n", self.summary.status));
        output.push_str(&format!(
            "Rules: {} passed, {} failed, {} warnings\n",
            self.summary.passed, self.summary.failed, self.summary.warnings
        ));
        output.push_str(&format!("Anomalies: {}\n\n", self.summary.anomalies_found));

        // Compliance results
        output.push_str("Compliance Checks:\n");
        output.push_str(&"-".repeat(40));
        output.push('\n');

        for result in &self.compliance_results {
            let status = match &result.result {
                RuleResult::Pass => "✓ PASS",
                RuleResult::Fail { .. } => "✗ FAIL",
                RuleResult::Warning { .. } => "⚠ WARN",
                RuleResult::NotApplicable => "- N/A",
            };
            output.push_str(&format!(
                "{} [{}] {}\n",
                status, result.statute, result.rule_name
            ));

            if let RuleResult::Fail { reason, severity } = &result.result {
                output.push_str(&format!("    Severity: {:?}\n", severity));
                output.push_str(&format!("    Reason: {}\n", reason));
            } else if let RuleResult::Warning { reason } = &result.result {
                output.push_str(&format!("    Reason: {}\n", reason));
            }
        }

        // Anomalies
        if !self.anomalies.is_empty() {
            output.push_str("\nAnomalies Detected:\n");
            output.push_str(&"-".repeat(40));
            output.push('\n');

            for (i, anomaly) in self.anomalies.iter().enumerate() {
                output.push_str(&format!("{}. {}\n", i + 1, anomaly.description()));
            }
        }

        output
    }
}

/// Main verification function
pub fn verify_lease(
    pdf_bytes: &[u8],
    state: &str,
    detect_anomalies: bool,
) -> Result<VerificationReport, VerifierError> {
    // 1. Extract text from PDF
    let document = PdfExtractor::extract_text(pdf_bytes)?;

    // 2. Parse the lease structure
    let parsed = LeaseParser::parse(&document)?;

    // 3. Get rules for the specified state
    let rules = match state.to_lowercase().as_str() {
        "florida" | "fl" => get_florida_rules(),
        _ => {
            return Err(VerifierError::ParsingError(format!(
                "Unsupported state: {}",
                state
            )))
        }
    };

    // 4. Run compliance checks
    let compliance_results: Vec<ComplianceCheckResult> = rules
        .iter()
        .map(|rule| ComplianceCheckResult {
            rule_name: rule.name().to_string(),
            statute: rule.statute_reference().to_string(),
            result: rule.check(&parsed),
        })
        .collect();

    // 5. Detect anomalies if requested
    let anomalies = if detect_anomalies {
        AnomalyDetector::detect(&parsed, &CanonicalStructure::florida_residential())
    } else {
        vec![]
    };

    // 6. Generate summary
    let passed = compliance_results
        .iter()
        .filter(|r| matches!(r.result, RuleResult::Pass))
        .count();
    let failed = compliance_results
        .iter()
        .filter(|r| matches!(r.result, RuleResult::Fail { .. }))
        .count();
    let warnings = compliance_results
        .iter()
        .filter(|r| matches!(r.result, RuleResult::Warning { .. }))
        .count();
    let anomalies_count = anomalies.len();

    let status = if failed > 0 {
        VerificationStatus::NonCompliant
    } else if warnings > 0 || anomalies_count > 0 {
        VerificationStatus::CompliantWithWarnings
    } else {
        VerificationStatus::Compliant
    };

    Ok(VerificationReport {
        input_file: "lease.pdf".to_string(),
        compliance_results,
        anomalies,
        summary: VerificationSummary {
            total_rules: rules.len(),
            passed,
            failed,
            warnings,
            anomalies_found: anomalies_count,
            status,
        },
    })
}
