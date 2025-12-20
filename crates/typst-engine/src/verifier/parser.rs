//! Document parser for lease agreements
//!
//! This module provides functionality to parse extracted PDF text into structured
//! lease components including sections, parties, financial terms, dates, and addenda.

use crate::verifier::{ExtractedDocument, VerifierError};
use regex::Regex;
use serde::{Deserialize, Serialize};

/// A parsed lease document with structured sections and extracted data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedLease {
    /// All detected sections in the lease
    pub sections: Vec<LeaseSection>,
    /// Information about landlord and tenant
    pub parties: Parties,
    /// Property information
    pub property: PropertyInfo,
    /// Financial terms (rent, deposits, fees)
    pub financial: FinancialTerms,
    /// Important dates
    pub dates: LeaseDates,
    /// Addenda attached to the lease
    pub addenda: Vec<Addendum>,
    /// Sections that don't match known patterns
    pub unknown_sections: Vec<UnknownSection>,
}

/// A section of the lease document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseSection {
    /// Section title/heading
    pub title: String,
    /// Section number (e.g., "1.1", "2.3", "A")
    pub number: Option<String>,
    /// Full text content of the section
    pub content: String,
    /// Starting line number in source document
    pub start_line: usize,
    /// Ending line number in source document
    pub end_line: usize,
}

/// Information about the parties to the lease
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Parties {
    /// Landlord's name
    pub landlord_name: Option<String>,
    /// Landlord's address
    pub landlord_address: Option<String>,
    /// Tenant's name
    pub tenant_name: Option<String>,
    /// Tenant's address
    pub tenant_address: Option<String>,
}

/// Property information
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PropertyInfo {
    /// Full property address
    pub address: Option<String>,
    /// Unit number (if applicable)
    pub unit: Option<String>,
    /// Property type (e.g., "single family", "apartment", "condo")
    pub property_type: Option<String>,
    /// Year the property was built (important for lead paint disclosure)
    pub year_built: Option<u32>,
}

/// Financial terms of the lease
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FinancialTerms {
    /// Monthly rent amount
    pub monthly_rent: Option<f64>,
    /// Security deposit amount
    pub security_deposit: Option<f64>,
    /// Late fee amount
    pub late_fee: Option<f64>,
    /// Grace period for late fees (in days)
    pub late_fee_grace_period: Option<u32>,
    /// Pet deposit amount
    pub pet_deposit: Option<f64>,
}

/// Important dates in the lease
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LeaseDates {
    /// Lease start date
    pub start_date: Option<String>,
    /// Lease end date
    pub end_date: Option<String>,
    /// Move-in date
    pub move_in_date: Option<String>,
}

/// An addendum to the lease
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Addendum {
    /// Addendum title
    pub title: String,
    /// Addendum content
    pub content: String,
}

/// A section that doesn't match known patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnknownSection {
    /// Section title
    pub title: String,
    /// Section content
    pub content: String,
    /// Line number where found
    pub line_number: usize,
}

/// Lease document parser
pub struct LeaseParser;

impl LeaseParser {
    /// Parse an extracted document into a structured lease
    ///
    /// # Arguments
    /// * `document` - The extracted PDF document
    ///
    /// # Returns
    /// A parsed lease structure or an error if parsing fails
    pub fn parse(document: &ExtractedDocument) -> Result<ParsedLease, VerifierError> {
        let text = &document.raw_text;

        // Detect all sections in the document
        let sections = Self::detect_sections(text);

        // Extract structured information from sections
        let parties = Self::extract_parties(&sections, text);
        let property = Self::extract_property(&sections, text);
        let financial = Self::extract_financial_terms(&sections, text);
        let dates = Self::extract_dates(&sections, text);
        let (addenda, unknown_sections) = Self::categorize_sections(&sections);

        Ok(ParsedLease {
            sections,
            parties,
            property,
            financial,
            dates,
            addenda,
            unknown_sections,
        })
    }

    /// Detect section boundaries and headers in the document
    ///
    /// Matches patterns like:
    /// - "1. BASIC TERMS"
    /// - "SECTION 2: ADDITIONAL TERMS"
    /// - "ADDENDUM A: PET ADDENDUM"
    /// - "2.3 Maintenance Responsibilities"
    fn detect_sections(text: &str) -> Vec<LeaseSection> {
        let mut sections = Vec::new();
        let lines: Vec<&str> = text.lines().collect();

        // Regex patterns for section headers
        let patterns = vec![
            // "1. SECTION TITLE" or "1.1 Section Title"
            Regex::new(r"^(\d+(?:\.\d+)?)\s*\.?\s+([A-Z][A-Za-z\s:]+)").unwrap(),
            // "SECTION 1:" or "SECTION 1.1:"
            Regex::new(r"^SECTION\s+(\d+(?:\.\d+)?)\s*:?\s*(.*)").unwrap(),
            // "ADDENDUM A:" or "EXHIBIT B:"
            Regex::new(r"^(ADDENDUM|EXHIBIT|ATTACHMENT)\s+([A-Z0-9]+)\s*:?\s*(.*)").unwrap(),
            // All caps titles on their own line
            Regex::new(r"^([A-Z][A-Z\s]{3,})$").unwrap(),
        ];

        let mut current_section: Option<(String, Option<String>, usize, Vec<String>)> = None;

        for (line_num, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                if let Some((_, _, _, ref mut content)) = current_section {
                    content.push(String::new());
                }
                continue;
            }

            let mut is_header = false;
            let mut section_num = None;
            let mut section_title = None;

            // Check each pattern
            for pattern in &patterns {
                if let Some(captures) = pattern.captures(trimmed) {
                    is_header = true;

                    // Extract section number and title based on pattern
                    if captures.len() == 3 {
                        section_num = Some(captures.get(1).unwrap().as_str().to_string());
                        section_title = Some(captures.get(2).unwrap().as_str().trim().to_string());
                    } else if captures.len() == 4 {
                        // ADDENDUM/EXHIBIT pattern
                        let prefix = captures.get(1).unwrap().as_str();
                        let id = captures.get(2).unwrap().as_str();
                        let title = captures.get(3).unwrap().as_str();
                        section_num = Some(id.to_string());
                        section_title = Some(if title.is_empty() {
                            format!("{} {}", prefix, id)
                        } else {
                            format!("{} {}: {}", prefix, id, title)
                        });
                    } else if captures.len() == 2 {
                        // All caps title only
                        section_title = Some(captures.get(1).unwrap().as_str().trim().to_string());
                    }
                    break;
                }
            }

            if is_header {
                // Save previous section if it exists
                if let Some((title, num, start, content)) = current_section.take() {
                    sections.push(LeaseSection {
                        title,
                        number: num,
                        content: content.join("\n").trim().to_string(),
                        start_line: start,
                        end_line: line_num.saturating_sub(1),
                    });
                }

                // Start new section
                if let Some(title) = section_title {
                    current_section = Some((title, section_num, line_num, Vec::new()));
                }
            } else if let Some((_, _, _, ref mut content)) = current_section {
                // Add to current section content
                content.push(line.to_string());
            } else {
                // Content before first section - create preamble section
                if sections.is_empty() {
                    current_section =
                        Some(("Preamble".to_string(), None, 0, vec![line.to_string()]));
                }
            }
        }

        // Save final section
        if let Some((title, num, start, content)) = current_section {
            sections.push(LeaseSection {
                title,
                number: num,
                content: content.join("\n").trim().to_string(),
                start_line: start,
                end_line: lines.len().saturating_sub(1),
            });
        }

        sections
    }

    /// Extract party information (landlord and tenant)
    fn extract_parties(sections: &[LeaseSection], _full_text: &str) -> Parties {
        let mut parties = Parties::default();

        // Regex patterns for party information
        let landlord_patterns = vec![
            Regex::new(r"(?i)landlord[:\s]+([A-Z][A-Za-z\s.,]+?)(?:,|\n|$)").unwrap(),
            Regex::new(r"(?i)lessor[:\s]+([A-Z][A-Za-z\s.,]+?)(?:,|\n|$)").unwrap(),
            Regex::new(r"(?i)owner[:\s]+([A-Z][A-Za-z\s.,]+?)(?:,|\n|$)").unwrap(),
        ];

        let tenant_patterns = vec![
            Regex::new(r"(?i)tenant[:\s]+([A-Z][A-Za-z\s.,]+?)(?:,|\n|$)").unwrap(),
            Regex::new(r"(?i)lessee[:\s]+([A-Z][A-Za-z\s.,]+?)(?:,|\n|$)").unwrap(),
            Regex::new(r"(?i)resident[:\s]+([A-Z][A-Za-z\s.,]+?)(?:,|\n|$)").unwrap(),
        ];

        let address_pattern = Regex::new(
            r"(?i)(?:address|located at)[:\s]+([0-9]+\s+[A-Za-z\s.,]+(?:Street|St|Avenue|Ave|Road|Rd|Drive|Dr|Lane|Ln|Boulevard|Blvd|Court|Ct)[A-Za-z\s.,#0-9]*)"
        ).unwrap();

        // Search in the first few sections (usually where parties are defined)
        let search_text = sections
            .iter()
            .take(3)
            .map(|s| s.content.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        // Extract landlord name
        for pattern in &landlord_patterns {
            if let Some(captures) = pattern.captures(&search_text) {
                if let Some(name) = captures.get(1) {
                    parties.landlord_name = Some(name.as_str().trim().to_string());
                    break;
                }
            }
        }

        // Extract tenant name
        for pattern in &tenant_patterns {
            if let Some(captures) = pattern.captures(&search_text) {
                if let Some(name) = captures.get(1) {
                    parties.tenant_name = Some(name.as_str().trim().to_string());
                    break;
                }
            }
        }

        // Extract addresses (simplified - would need more context to distinguish landlord vs tenant)
        if let Some(captures) = address_pattern.captures(&search_text) {
            if let Some(address) = captures.get(1) {
                // Heuristic: first address found is likely tenant/property address
                parties.tenant_address = Some(address.as_str().trim().to_string());
            }
        }

        parties
    }

    /// Extract property information
    fn extract_property(sections: &[LeaseSection], _full_text: &str) -> PropertyInfo {
        let mut property = PropertyInfo::default();

        // Property address pattern
        let address_pattern = Regex::new(
            r"(?i)(?:property|premises|unit)(?:\s+(?:address|located at))?[:\s]+([0-9]+\s+[A-Za-z\s.,]+(?:Street|St|Avenue|Ave|Road|Rd|Drive|Dr|Lane|Ln|Boulevard|Blvd|Court|Ct)[A-Za-z\s.,#0-9]*(?:,\s*[A-Z]{2}\s+\d{5})?)"
        ).unwrap();

        // Unit number pattern
        let unit_pattern =
            Regex::new(r"(?i)(?:unit|apartment|apt\.?|suite|#)\s*([A-Z0-9-]+)").unwrap();

        // Property type pattern
        let type_pattern = Regex::new(
            r"(?i)(?:property type|premises type)[:\s]+(single family|apartment|condo(?:minium)?|townhouse|duplex)"
        ).unwrap();

        // Year built pattern
        let year_built_pattern =
            Regex::new(r"(?i)(?:built|constructed|year built)[:\s]+(\d{4})").unwrap();

        let search_text = sections
            .iter()
            .take(5)
            .map(|s| s.content.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        // Extract property address
        if let Some(captures) = address_pattern.captures(&search_text) {
            if let Some(addr) = captures.get(1) {
                property.address = Some(addr.as_str().trim().to_string());
            }
        }

        // Extract unit number
        if let Some(captures) = unit_pattern.captures(&search_text) {
            if let Some(unit) = captures.get(1) {
                property.unit = Some(unit.as_str().trim().to_string());
            }
        }

        // Extract property type
        if let Some(captures) = type_pattern.captures(&search_text) {
            if let Some(ptype) = captures.get(1) {
                property.property_type = Some(ptype.as_str().trim().to_string());
            }
        }

        // Extract year built
        if let Some(captures) = year_built_pattern.captures(&search_text) {
            if let Some(year_str) = captures.get(1) {
                if let Ok(year) = year_str.as_str().parse::<u32>() {
                    property.year_built = Some(year);
                }
            }
        }

        property
    }

    /// Extract financial terms from the lease
    ///
    /// Searches for patterns like:
    /// - "Monthly Rent: $1,500.00"
    /// - "Security Deposit $2,500"
    /// - "Late Fee: $50 after 5 days"
    fn extract_financial_terms(sections: &[LeaseSection], _full_text: &str) -> FinancialTerms {
        let mut financial = FinancialTerms::default();

        // Specific financial term patterns
        let rent_pattern = Regex::new(
            r"(?i)(?:monthly\s+)?rent(?:al)?(?:\s+(?:amount|payment))?[:\s]+\$\s*([\d,]+\.?\d*)",
        )
        .unwrap();

        let deposit_pattern =
            Regex::new(r"(?i)security\s+deposit[:\s]+\$\s*([\d,]+\.?\d*)").unwrap();

        let late_fee_pattern =
            Regex::new(r"(?i)late\s+(?:fee|charge)[:\s]+\$\s*([\d,]+\.?\d*)").unwrap();

        let grace_period_pattern =
            Regex::new(r"(?i)(?:grace\s+period|after|within)\s+(\d+)\s+days?").unwrap();

        let pet_deposit_pattern =
            Regex::new(r"(?i)pet\s+(?:deposit|fee)[:\s]+\$\s*([\d,]+\.?\d*)").unwrap();

        let search_text = sections
            .iter()
            .map(|s| format!("{}: {}", s.title, s.content))
            .collect::<Vec<_>>()
            .join("\n");

        // Extract monthly rent
        if let Some(captures) = rent_pattern.captures(&search_text) {
            if let Some(amount_str) = captures.get(1) {
                let cleaned = amount_str.as_str().replace(',', "");
                if let Ok(amount) = cleaned.parse::<f64>() {
                    financial.monthly_rent = Some(amount);
                }
            }
        }

        // Extract security deposit
        if let Some(captures) = deposit_pattern.captures(&search_text) {
            if let Some(amount_str) = captures.get(1) {
                let cleaned = amount_str.as_str().replace(',', "");
                if let Ok(amount) = cleaned.parse::<f64>() {
                    financial.security_deposit = Some(amount);
                }
            }
        }

        // Extract late fee
        if let Some(captures) = late_fee_pattern.captures(&search_text) {
            if let Some(amount_str) = captures.get(1) {
                let cleaned = amount_str.as_str().replace(',', "");
                if let Ok(amount) = cleaned.parse::<f64>() {
                    financial.late_fee = Some(amount);
                }
            }
        }

        // Extract grace period for late fees
        if let Some(captures) = grace_period_pattern.captures(&search_text) {
            if let Some(days_str) = captures.get(1) {
                if let Ok(days) = days_str.as_str().parse::<u32>() {
                    financial.late_fee_grace_period = Some(days);
                }
            }
        }

        // Extract pet deposit
        if let Some(captures) = pet_deposit_pattern.captures(&search_text) {
            if let Some(amount_str) = captures.get(1) {
                let cleaned = amount_str.as_str().replace(',', "");
                if let Ok(amount) = cleaned.parse::<f64>() {
                    financial.pet_deposit = Some(amount);
                }
            }
        }

        financial
    }

    /// Extract important dates from the lease
    fn extract_dates(sections: &[LeaseSection], _full_text: &str) -> LeaseDates {
        let mut dates = LeaseDates::default();

        // Date patterns - various formats
        let date_patterns = vec![
            // MM/DD/YYYY or MM-DD-YYYY
            Regex::new(r"\d{1,2}[/-]\d{1,2}[/-]\d{4}").unwrap(),
            // Month DD, YYYY
            Regex::new(r"(?:January|February|March|April|May|June|July|August|September|October|November|December)\s+\d{1,2},\s+\d{4}").unwrap(),
            // DD Month YYYY
            Regex::new(r"\d{1,2}\s+(?:January|February|March|April|May|June|July|August|September|October|November|December)\s+\d{4}").unwrap(),
        ];

        // Context patterns for different date types
        let start_date_pattern = Regex::new(
            r"(?i)(?:lease\s+)?(?:start|commencement|effective)\s+date[:\s]+(.*?)(?:\n|$)",
        )
        .unwrap();

        let end_date_pattern = Regex::new(
            r"(?i)(?:lease\s+)?(?:end|termination|expiration)\s+date[:\s]+(.*?)(?:\n|$)",
        )
        .unwrap();

        let move_in_pattern = Regex::new(r"(?i)move[- ]?in\s+date[:\s]+(.*?)(?:\n|$)").unwrap();

        let search_text = sections
            .iter()
            .take(5)
            .map(|s| format!("{}: {}", s.title, s.content))
            .collect::<Vec<_>>()
            .join("\n");

        // Helper function to extract date from text
        let extract_date = |text: &str| -> Option<String> {
            for pattern in &date_patterns {
                if let Some(m) = pattern.find(text) {
                    return Some(m.as_str().to_string());
                }
            }
            None
        };

        // Extract start date
        if let Some(captures) = start_date_pattern.captures(&search_text) {
            if let Some(text) = captures.get(1) {
                dates.start_date = extract_date(text.as_str());
            }
        }

        // Extract end date
        if let Some(captures) = end_date_pattern.captures(&search_text) {
            if let Some(text) = captures.get(1) {
                dates.end_date = extract_date(text.as_str());
            }
        }

        // Extract move-in date
        if let Some(captures) = move_in_pattern.captures(&search_text) {
            if let Some(text) = captures.get(1) {
                dates.move_in_date = extract_date(text.as_str());
            }
        }

        dates
    }

    /// Categorize sections into addenda and unknown sections
    fn categorize_sections(sections: &[LeaseSection]) -> (Vec<Addendum>, Vec<UnknownSection>) {
        let mut addenda = Vec::new();
        let mut unknown = Vec::new();

        // Known section title keywords (lowercase for case-insensitive matching)
        let known_keywords = vec![
            "basic",
            "terms",
            "summary",
            "parties",
            "property",
            "premises",
            "rent",
            "payment",
            "deposit",
            "security",
            "fees",
            "utilities",
            "maintenance",
            "repairs",
            "access",
            "entry",
            "default",
            "termination",
            "notices",
            "disclosures",
            "radon",
            "lead",
            "insurance",
            "liability",
            "pets",
            "parking",
            "storage",
            "rules",
            "regulations",
            "compliance",
            "signatures",
            "preamble",
        ];

        for section in sections {
            let title_lower = section.title.to_lowercase();

            // Check if it's an addendum
            if title_lower.contains("addendum")
                || title_lower.contains("exhibit")
                || title_lower.contains("attachment")
            {
                addenda.push(Addendum {
                    title: section.title.clone(),
                    content: section.content.clone(),
                });
            }
            // Check if it's a known section type
            else if !known_keywords
                .iter()
                .any(|keyword| title_lower.contains(keyword))
            {
                // This is an unknown section - flag it
                unknown.push(UnknownSection {
                    title: section.title.clone(),
                    content: section.content.clone(),
                    line_number: section.start_line,
                });
            }
        }

        (addenda, unknown)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_sections_numbered() {
        let text = r#"
1. BASIC TERMS
This section contains basic terms.

2. ADDITIONAL PROVISIONS
This section has more details.

3.1 Subsection Example
This is a subsection.
        "#;

        let sections = LeaseParser::detect_sections(text);
        assert!(sections.len() >= 3);
        assert_eq!(sections[0].title, "BASIC TERMS");
        assert_eq!(sections[0].number, Some("1".to_string()));
    }

    #[test]
    fn test_detect_sections_addendum() {
        let text = r#"
MAIN LEASE CONTENT
Content here.

ADDENDUM A: PET POLICY
Pet policy details.

EXHIBIT B: PARKING RULES
Parking rules here.
        "#;

        let sections = LeaseParser::detect_sections(text);
        assert!(sections.iter().any(|s| s.title.contains("PET POLICY")));
        assert!(sections.iter().any(|s| s.title.contains("PARKING RULES")));
    }

    #[test]
    fn test_extract_financial_rent() {
        let section = LeaseSection {
            title: "Basic Terms".to_string(),
            number: Some("1".to_string()),
            content: "Monthly Rent: $1,500.00\nSecurity Deposit: $2,500".to_string(),
            start_line: 0,
            end_line: 2,
        };

        let financial = LeaseParser::extract_financial_terms(&[section], "");
        assert_eq!(financial.monthly_rent, Some(1500.0));
        assert_eq!(financial.security_deposit, Some(2500.0));
    }

    #[test]
    fn test_extract_dates() {
        let section = LeaseSection {
            title: "Lease Term".to_string(),
            number: Some("2".to_string()),
            content: "Start Date: 01/01/2024\nEnd Date: 12/31/2024".to_string(),
            start_line: 0,
            end_line: 2,
        };

        let dates = LeaseParser::extract_dates(&[section], "");
        assert_eq!(dates.start_date, Some("01/01/2024".to_string()));
        assert_eq!(dates.end_date, Some("12/31/2024".to_string()));
    }

    #[test]
    fn test_categorize_sections() {
        let sections = vec![
            LeaseSection {
                title: "BASIC TERMS".to_string(),
                number: Some("1".to_string()),
                content: "Content".to_string(),
                start_line: 0,
                end_line: 1,
            },
            LeaseSection {
                title: "ADDENDUM A: PETS".to_string(),
                number: Some("A".to_string()),
                content: "Pet rules".to_string(),
                start_line: 2,
                end_line: 3,
            },
            LeaseSection {
                title: "MYSTERY SECTION".to_string(),
                number: None,
                content: "Unknown content".to_string(),
                start_line: 4,
                end_line: 5,
            },
        ];

        let (addenda, unknown) = LeaseParser::categorize_sections(&sections);
        assert_eq!(addenda.len(), 1);
        assert_eq!(addenda[0].title, "ADDENDUM A: PETS");
        assert_eq!(unknown.len(), 1);
        assert_eq!(unknown[0].title, "MYSTERY SECTION");
    }
}
