//! Anomaly detection for lease documents
//!
//! Detects unusual or potentially surreptitious clauses by comparing
//! parsed lease structure against canonical templates.

use crate::verifier::parser::ParsedLease;
use regex::Regex;
use serde::{Deserialize, Serialize};

/// Detector for structural and content anomalies in lease documents
pub struct AnomalyDetector;

impl AnomalyDetector {
    /// Detect anomalies in a parsed lease against a canonical structure
    pub fn detect(parsed: &ParsedLease, canonical: &CanonicalStructure) -> Vec<Anomaly> {
        let mut anomalies = vec![];

        // 1. Find sections not matching any expected pattern
        for section in &parsed.sections {
            if !Self::matches_any_expected(&section.title, &canonical.expected_sections) {
                anomalies.push(Anomaly::UnexpectedSection {
                    title: section.title.clone(),
                    content_preview: Self::truncate(&section.content, 200),
                    line_number: section.start_line,
                });
            }
        }

        // 2. Check for missing required sections
        for expected in &canonical.expected_sections {
            if expected.required {
                let found = parsed
                    .sections
                    .iter()
                    .any(|s| Self::title_matches_patterns(&s.title, &expected.title_patterns));
                if !found {
                    anomalies.push(Anomaly::MissingRequiredSection {
                        expected: expected
                            .title_patterns
                            .first()
                            .map(|s| s.to_string())
                            .unwrap_or_default(),
                    });
                }
            }
        }

        // 3. Check for suspicious language patterns
        let suspicious_patterns = Self::get_suspicious_patterns();
        for section in &parsed.sections {
            for (pattern, reason) in &suspicious_patterns {
                if pattern.is_match(&section.content) {
                    anomalies.push(Anomaly::SuspiciousLanguage {
                        text: Self::extract_match(&section.content, pattern, 100),
                        reason: reason.to_string(),
                        line_number: section.start_line,
                    });
                }
            }
        }

        // 4. Check for unusually long sections (potential hidden content)
        let avg_section_len = if parsed.sections.is_empty() {
            1000
        } else {
            parsed
                .sections
                .iter()
                .map(|s| s.content.len())
                .sum::<usize>()
                / parsed.sections.len()
        };

        for section in &parsed.sections {
            if section.content.len() > avg_section_len * 3 && section.content.len() > 2000 {
                anomalies.push(Anomaly::UnusuallyLongSection {
                    title: section.title.clone(),
                    length: section.content.len(),
                    average_length: avg_section_len,
                    line_number: section.start_line,
                });
            }
        }

        // 5. Flag unknown/unclassified sections
        for unknown in &parsed.unknown_sections {
            anomalies.push(Anomaly::UnclassifiedContent {
                title: unknown.title.clone(),
                content_preview: Self::truncate(&unknown.content, 200),
                line_number: unknown.line_number,
            });
        }

        anomalies
    }

    fn matches_any_expected(title: &str, expected_sections: &[ExpectedSection]) -> bool {
        expected_sections
            .iter()
            .any(|e| Self::title_matches_patterns(title, &e.title_patterns))
    }

    fn title_matches_patterns(title: &str, patterns: &[&str]) -> bool {
        let title_lower = title.to_lowercase();
        patterns
            .iter()
            .any(|p| title_lower.contains(&p.to_lowercase()))
    }

    fn truncate(s: &str, max_len: usize) -> String {
        if s.len() <= max_len {
            s.to_string()
        } else {
            format!("{}...", &s[..max_len])
        }
    }

    fn extract_match(text: &str, pattern: &Regex, context: usize) -> String {
        if let Some(m) = pattern.find(text) {
            let start = m.start().saturating_sub(context);
            let end = (m.end() + context).min(text.len());
            format!("...{}...", &text[start..end])
        } else {
            String::new()
        }
    }

    fn get_suspicious_patterns() -> Vec<(Regex, &'static str)> {
        vec![
            // Waiver of tenant rights
            (
                Regex::new(r"(?i)tenant\s+(waives?|relinquish|forfeit|surrender)\s+(any|all)?\s*(rights?|claims?|remedies?)").unwrap(),
                "Potential waiver of tenant rights"
            ),
            // Hidden fee clauses
            (
                Regex::new(r"(?i)(additional|hidden|undisclosed)\s+fees?").unwrap(),
                "Reference to additional/hidden fees"
            ),
            // Automatic renewal traps
            (
                Regex::new(r"(?i)automatic(ally)?\s+(renew|extend|continue)\s+.*unless\s+.*written\s+notice\s+.*(\d+)\s+days?").unwrap(),
                "Automatic renewal clause - review notice period"
            ),
            // Confession of judgment
            (
                Regex::new(r"(?i)confess(ion)?\s+(of\s+)?judgment").unwrap(),
                "Illegal confession of judgment clause"
            ),
            // Jury trial waiver
            (
                Regex::new(r"(?i)waive[rs]?\s+(the\s+)?right\s+to\s+(a\s+)?(jury|trial)").unwrap(),
                "Waiver of jury trial rights"
            ),
            // Attorney fee abuse
            (
                Regex::new(r"(?i)tenant\s+(shall|will|must)\s+(pay|be\s+(liable|responsible))\s+(for\s+)?(all\s+)?attorney('?s)?\s+fees?\s+.*regardless").unwrap(),
                "One-sided attorney fee clause"
            ),
            // Access without notice
            (
                Regex::new(r"(?i)landlord\s+(may|can|shall|will)\s+(enter|access)\s+.*without\s+(prior\s+)?notice").unwrap(),
                "Entry without notice clause - may violate F.S. § 83.53"
            ),
            // Unilateral modification
            (
                Regex::new(r"(?i)landlord\s+(may|can|reserves?\s+the\s+right\s+to)\s+(change|modify|alter|amend)\s+.*at\s+any\s+time").unwrap(),
                "Unilateral modification clause"
            ),
            // Security deposit non-refundable
            (
                Regex::new(r"(?i)(security\s+)?deposit\s+.*non[- ]?refundable").unwrap(),
                "Non-refundable deposit - review legality"
            ),
            // Excessive penalties
            (
                Regex::new(r"(?i)penalty\s+of\s+\$?(\d{3,}|\d+,\d{3})").unwrap(),
                "Large penalty clause - review reasonableness"
            ),
        ]
    }
}

/// Canonical structure defining expected lease sections
#[derive(Debug, Clone)]
pub struct CanonicalStructure {
    pub expected_sections: Vec<ExpectedSection>,
}

/// Expected section in a canonical lease structure
#[derive(Debug, Clone)]
pub struct ExpectedSection {
    /// Patterns that match this section's title
    pub title_patterns: Vec<&'static str>,
    /// Whether this section is required
    pub required: bool,
    /// Typical subsections within this section
    pub typical_subsections: Vec<&'static str>,
}

impl CanonicalStructure {
    /// Create canonical structure for Florida residential leases
    pub fn florida_residential() -> Self {
        Self {
            expected_sections: vec![
                ExpectedSection {
                    title_patterns: vec![
                        "basic terms",
                        "summary",
                        "key terms",
                        "lease terms",
                        "fundamental terms",
                    ],
                    required: true,
                    typical_subsections: vec!["rent", "deposit", "term", "utilities"],
                },
                ExpectedSection {
                    title_patterns: vec!["parties", "landlord", "tenant", "lessee", "lessor"],
                    required: true,
                    typical_subsections: vec!["name", "address", "contact"],
                },
                ExpectedSection {
                    title_patterns: vec!["premises", "property", "rental property", "unit"],
                    required: true,
                    typical_subsections: vec!["address", "description", "parking"],
                },
                ExpectedSection {
                    title_patterns: vec!["rent", "payment", "monthly rent"],
                    required: true,
                    typical_subsections: vec!["amount", "due date", "method"],
                },
                ExpectedSection {
                    title_patterns: vec!["deposit", "security deposit", "security"],
                    required: true,
                    typical_subsections: vec!["amount", "return", "deductions"],
                },
                ExpectedSection {
                    title_patterns: vec!["term", "lease term", "duration", "period"],
                    required: true,
                    typical_subsections: vec!["start", "end", "renewal"],
                },
                ExpectedSection {
                    title_patterns: vec!["utilities", "services"],
                    required: false,
                    typical_subsections: vec!["electric", "water", "gas", "internet"],
                },
                ExpectedSection {
                    title_patterns: vec!["maintenance", "repairs", "upkeep"],
                    required: false,
                    typical_subsections: vec!["tenant", "landlord", "emergency"],
                },
                ExpectedSection {
                    title_patterns: vec!["access", "entry", "right of entry"],
                    required: false,
                    typical_subsections: vec!["notice", "emergency", "showing"],
                },
                ExpectedSection {
                    title_patterns: vec!["default", "breach", "violation"],
                    required: false,
                    typical_subsections: vec!["cure period", "remedies"],
                },
                ExpectedSection {
                    title_patterns: vec!["termination", "early termination", "ending"],
                    required: false,
                    typical_subsections: vec!["notice", "penalties", "buyout"],
                },
                ExpectedSection {
                    title_patterns: vec!["pet", "animal", "pet policy"],
                    required: false,
                    typical_subsections: vec!["deposit", "restrictions", "breeds"],
                },
                ExpectedSection {
                    title_patterns: vec!["parking", "vehicle"],
                    required: false,
                    typical_subsections: vec!["assigned", "guest", "restrictions"],
                },
                ExpectedSection {
                    title_patterns: vec!["radon", "radon gas"],
                    required: true, // Required by Florida law
                    typical_subsections: vec![],
                },
                ExpectedSection {
                    title_patterns: vec!["signature", "execution", "agreement"],
                    required: true,
                    typical_subsections: vec!["date", "witness"],
                },
                ExpectedSection {
                    title_patterns: vec!["addendum", "exhibit", "attachment", "appendix"],
                    required: false,
                    typical_subsections: vec![],
                },
            ],
        }
    }
}

/// Types of anomalies that can be detected
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Anomaly {
    /// Section that doesn't match any expected section type
    UnexpectedSection {
        title: String,
        content_preview: String,
        line_number: usize,
    },

    /// Unexpected or potentially hidden clause within a section
    UnexpectedClause {
        within_section: String,
        content: String,
        line_number: usize,
        risk_score: f32,
    },

    /// Required section is missing
    MissingRequiredSection { expected: String },

    /// Suspicious language detected
    SuspiciousLanguage {
        text: String,
        reason: String,
        line_number: usize,
    },

    /// Section is unusually long compared to average
    UnusuallyLongSection {
        title: String,
        length: usize,
        average_length: usize,
        line_number: usize,
    },

    /// Content that couldn't be classified into known sections
    UnclassifiedContent {
        title: String,
        content_preview: String,
        line_number: usize,
    },
}

impl Anomaly {
    /// Get a human-readable description of the anomaly
    pub fn description(&self) -> String {
        match self {
            Anomaly::UnexpectedSection {
                title, line_number, ..
            } => {
                format!(
                    "Unexpected section '{}' at line {} - review contents carefully",
                    title, line_number
                )
            }
            Anomaly::UnexpectedClause {
                within_section,
                line_number,
                risk_score,
                ..
            } => {
                format!(
                    "Unexpected clause in '{}' at line {} (risk: {:.0}%)",
                    within_section,
                    line_number,
                    risk_score * 100.0
                )
            }
            Anomaly::MissingRequiredSection { expected } => {
                format!("Missing required section: '{}'", expected)
            }
            Anomaly::SuspiciousLanguage {
                reason,
                line_number,
                ..
            } => {
                format!("{} at line {}", reason, line_number)
            }
            Anomaly::UnusuallyLongSection {
                title,
                length,
                average_length,
                line_number,
            } => {
                format!(
                    "Section '{}' at line {} is unusually long ({} chars vs {} avg)",
                    title, line_number, length, average_length
                )
            }
            Anomaly::UnclassifiedContent {
                title, line_number, ..
            } => {
                format!("Unclassified content '{}' at line {}", title, line_number)
            }
        }
    }

    /// Get the risk level of this anomaly
    pub fn risk_level(&self) -> &'static str {
        match self {
            Anomaly::SuspiciousLanguage { .. } => "high",
            Anomaly::UnexpectedClause { risk_score, .. } if *risk_score > 0.7 => "high",
            Anomaly::MissingRequiredSection { .. } => "medium",
            Anomaly::UnexpectedSection { .. } => "medium",
            Anomaly::UnexpectedClause { .. } => "medium",
            Anomaly::UnusuallyLongSection { .. } => "low",
            Anomaly::UnclassifiedContent { .. } => "low",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_suspicious_patterns_compile() {
        // Ensure all regex patterns compile successfully
        let patterns = AnomalyDetector::get_suspicious_patterns();
        assert!(!patterns.is_empty());
    }

    #[test]
    fn test_truncate() {
        assert_eq!(AnomalyDetector::truncate("hello", 10), "hello");
        assert_eq!(AnomalyDetector::truncate("hello world", 5), "hello...");
    }

    #[test]
    fn test_florida_canonical_structure() {
        let canonical = CanonicalStructure::florida_residential();
        assert!(!canonical.expected_sections.is_empty());

        // Check that required sections exist
        let required: Vec<_> = canonical
            .expected_sections
            .iter()
            .filter(|s| s.required)
            .collect();
        assert!(required.len() >= 5); // At least basic terms, parties, property, rent, deposit
    }
}
