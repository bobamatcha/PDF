//! Regex patterns and detection logic for prohibited provisions

/// Waiver keywords that indicate a tenant is relinquishing rights
pub const WAIVER_KEYWORDS: &[&str] = &[
    "waive",
    "waives",
    "waiver",
    "relinquish",
    "relinquishes",
    "forgo",
    "forgoes",
    "surrender",
    "surrenders",
];

/// Notice-related keywords
pub const NOTICE_KEYWORDS: &[&str] = &["notice", "notification", "notify", "advance notice"];

/// Termination/eviction keywords
pub const TERMINATION_KEYWORDS: &[&str] =
    &["termination", "eviction", "evict", "terminate", "removal"];

/// Property disposal keywords
pub const DISPOSAL_KEYWORDS: &[&str] = &[
    "dispose",
    "disposal",
    "discard",
    "throw away",
    "remove",
    "destroy",
];

/// Property reference keywords
pub const PROPERTY_KEYWORDS: &[&str] = &[
    "property",
    "belongings",
    "possessions",
    "items",
    "personal property",
];

/// Tenant reference keywords
pub const TENANT_KEYWORDS: &[&str] = &["tenant", "lessee", "renter"];

/// Structural/maintenance keywords that landlord is responsible for
pub const STRUCTURAL_KEYWORDS: &[&str] = &[
    "roof",
    "roofing",
    "plumbing",
    "pipes",
    "termite",
    "termites",
    "structural",
    "foundation",
    "hvac",
    "heating",
    "air conditioning",
    "electrical",
    "wiring",
];

/// AS-IS keywords
pub const AS_IS_KEYWORDS: &[&str] = &["as-is", "as is", "asis"];

/// Rights reference keywords
pub const RIGHTS_KEYWORDS: &[&str] = &["rights", "right", "protections", "remedies"];

/// Florida landlord-tenant law references
pub const FL_LAW_KEYWORDS: &[&str] = &[
    "florida",
    "chapter 83",
    "landlord tenant law",
    "landlord-tenant law",
    "statute",
];

/// Check if text contains semantic clustering of keywords
/// Returns true if text contains words from multiple keyword groups within proximity
pub fn contains_semantic_cluster(text: &str, keyword_groups: &[&[&str]]) -> bool {
    let text_lower = text.to_lowercase();
    let mut found_groups = 0;

    for group in keyword_groups {
        if group.iter().any(|keyword| text_lower.contains(keyword)) {
            found_groups += 1;
        }
    }

    // Require at least 2 groups to be present for semantic clustering
    found_groups >= 2
}

/// Extract a snippet around a keyword match (up to 150 characters)
pub fn extract_snippet(text: &str, keyword: &str) -> String {
    let text_lower = text.to_lowercase();
    let keyword_lower = keyword.to_lowercase();

    if let Some(pos) = text_lower.find(&keyword_lower) {
        let start = pos.saturating_sub(50);
        let end = (pos + keyword.len() + 50).min(text.len());
        let snippet = &text[start..end];
        format!("...{}...", snippet.trim())
    } else {
        text.chars().take(150).collect::<String>()
    }
}

/// Find the position of a keyword match for text highlighting
/// Returns (start_offset, end_offset) if found
pub fn find_text_position(text: &str, keyword: &str) -> Option<(usize, usize)> {
    let text_lower = text.to_lowercase();
    let keyword_lower = keyword.to_lowercase();

    if let Some(start) = text_lower.find(&keyword_lower) {
        // Extend to capture the full phrase (up to 100 chars around the keyword)
        let context_start = start.saturating_sub(20);
        let context_end = (start + keyword.len() + 80).min(text.len());
        Some((context_start, context_end))
    } else {
        None
    }
}
