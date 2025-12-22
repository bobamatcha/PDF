//! Streaming PDF operations using byte-level manipulation
//!
//! This module provides fast PDF split/merge by directly manipulating
//! PDF bytes rather than parsing the entire document into memory.
//!
//! PDF Structure:
//! ```text
//! %PDF-1.x
//! ... objects ...
//! xref
//! 0 N
//! 0000000000 65535 f
//! 0000000015 00000 n
//! ...
//! trailer
//! << /Root X 0 R /Size N >>
//! startxref
//! OFFSET
//! %%EOF
//! ```

use std::collections::{HashMap, HashSet};

use crate::error::PdfJoinError;

/// Object reference (object number, generation number)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjRef(pub u32, pub u16);

/// Cross-reference entry
#[derive(Debug, Clone)]
pub struct XrefEntry {
    pub offset: usize,
    pub generation: u16,
    pub in_use: bool,
}

/// Parsed PDF structure (minimal - just what we need)
#[derive(Debug)]
pub struct PdfStructure {
    pub version: String,
    pub xref: HashMap<u32, XrefEntry>,
    pub trailer: TrailerInfo,
    pub raw_bytes: Vec<u8>,
}

/// Trailer information
#[derive(Debug, Clone)]
pub struct TrailerInfo {
    pub root: ObjRef,
    pub size: u32,
    pub info: Option<ObjRef>,
}

/// Page information extracted from page tree
#[derive(Debug, Clone)]
pub struct PageRef {
    pub obj_ref: ObjRef,
    pub offset: usize,
}

impl PdfStructure {
    /// Parse PDF structure from bytes (minimal parsing - just xref and trailer)
    pub fn parse(bytes: &[u8]) -> Result<Self, PdfJoinError> {
        // Check PDF header
        if !bytes.starts_with(b"%PDF-") {
            return Err(PdfJoinError::ParseError("Not a valid PDF".into()));
        }

        // Extract version
        let version = std::str::from_utf8(&bytes[5..8])
            .unwrap_or("1.4")
            .to_string();

        // Find startxref from end of file
        let startxref_offset = find_startxref(bytes)?;

        // Parse xref table
        let (xref, trailer_offset) = parse_xref_table(bytes, startxref_offset)?;

        // Parse trailer
        let trailer = parse_trailer(bytes, trailer_offset)?;

        Ok(PdfStructure {
            version,
            xref,
            trailer,
            raw_bytes: bytes.to_vec(),
        })
    }

    /// Get page count by traversing page tree minimally
    pub fn get_page_count(&self) -> Result<u32, PdfJoinError> {
        let pages_ref = self.get_pages_ref()?;
        let pages_obj = self.read_object(pages_ref)?;

        // Extract /Count from Pages dictionary
        extract_count(&pages_obj)
    }

    /// Get reference to Pages object
    fn get_pages_ref(&self) -> Result<ObjRef, PdfJoinError> {
        let catalog = self.read_object(self.trailer.root)?;
        extract_pages_ref(&catalog)
    }

    /// Read raw object bytes at given reference
    pub fn read_object(&self, obj_ref: ObjRef) -> Result<Vec<u8>, PdfJoinError> {
        let entry = self
            .xref
            .get(&obj_ref.0)
            .ok_or_else(|| PdfJoinError::ParseError(format!("Object {} not found", obj_ref.0)))?;

        if !entry.in_use {
            return Err(PdfJoinError::ParseError(format!(
                "Object {} is free",
                obj_ref.0
            )));
        }

        // Find end of object (next "endobj")
        let start = entry.offset;
        let obj_bytes = extract_object_bytes(&self.raw_bytes, start)?;

        Ok(obj_bytes)
    }

    /// Get all page object references
    pub fn get_page_refs(&self) -> Result<Vec<PageRef>, PdfJoinError> {
        let pages_ref = self.get_pages_ref()?;
        let mut page_refs = Vec::new();
        self.collect_pages(pages_ref, &mut page_refs)?;
        Ok(page_refs)
    }

    /// Recursively collect page references from page tree
    fn collect_pages(
        &self,
        node_ref: ObjRef,
        pages: &mut Vec<PageRef>,
    ) -> Result<(), PdfJoinError> {
        let obj_bytes = self.read_object(node_ref)?;

        // Check if this is a Page or Pages node
        if contains_pattern(&obj_bytes, b"/Type /Page\n")
            || contains_pattern(&obj_bytes, b"/Type /Page ")
            || contains_pattern(&obj_bytes, b"/Type/Page")
        {
            // Check it's not /Pages
            if !contains_pattern(&obj_bytes, b"/Type /Pages")
                && !contains_pattern(&obj_bytes, b"/Type/Pages")
            {
                let entry = self.xref.get(&node_ref.0).unwrap();
                pages.push(PageRef {
                    obj_ref: node_ref,
                    offset: entry.offset,
                });
                return Ok(());
            }
        }

        // It's a Pages node - get Kids array
        let kids = extract_kids_refs(&obj_bytes)?;
        for kid_ref in kids {
            self.collect_pages(kid_ref, pages)?;
        }

        Ok(())
    }

    /// Collect all objects referenced by a page (dependencies)
    pub fn collect_page_dependencies(
        &self,
        page_ref: ObjRef,
    ) -> Result<HashSet<u32>, PdfJoinError> {
        let mut deps = HashSet::new();
        self.collect_refs_recursive(page_ref, &mut deps, 0)?;
        Ok(deps)
    }

    /// Recursively collect object references
    fn collect_refs_recursive(
        &self,
        obj_ref: ObjRef,
        collected: &mut HashSet<u32>,
        depth: usize,
    ) -> Result<(), PdfJoinError> {
        // Prevent infinite recursion
        if depth > 100 || collected.contains(&obj_ref.0) {
            return Ok(());
        }

        collected.insert(obj_ref.0);

        if let Ok(obj_bytes) = self.read_object(obj_ref) {
            // Find all references in this object
            let refs = extract_all_refs(&obj_bytes);
            for r in refs {
                // Skip parent references to avoid cycles
                if !is_parent_ref(&obj_bytes, r) {
                    self.collect_refs_recursive(r, collected, depth + 1)?;
                }
            }
        }

        Ok(())
    }
}

/// Find startxref offset from end of file
fn find_startxref(bytes: &[u8]) -> Result<usize, PdfJoinError> {
    // Search backwards from end for "startxref"
    let search_start = bytes.len().saturating_sub(1024);
    let tail = &bytes[search_start..];

    let pos = find_pattern(tail, b"startxref")
        .ok_or_else(|| PdfJoinError::ParseError("startxref not found".into()))?;

    // Read the offset number after "startxref\n"
    let offset_start = search_start + pos + 10; // "startxref\n" = 10 bytes
    let offset_end = bytes[offset_start..]
        .iter()
        .position(|&b| !b.is_ascii_digit())
        .unwrap_or(20)
        + offset_start;

    let offset_str = std::str::from_utf8(&bytes[offset_start..offset_end])
        .map_err(|_| PdfJoinError::ParseError("Invalid startxref offset".into()))?
        .trim();

    offset_str
        .parse()
        .map_err(|_| PdfJoinError::ParseError("Invalid startxref number".into()))
}

/// Parse xref table starting at given offset
fn parse_xref_table(
    bytes: &[u8],
    offset: usize,
) -> Result<(HashMap<u32, XrefEntry>, usize), PdfJoinError> {
    let mut xref = HashMap::new();
    let mut pos = offset;

    // Skip "xref\n"
    if !bytes[pos..].starts_with(b"xref") {
        return Err(PdfJoinError::ParseError("Expected 'xref' keyword".into()));
    }
    pos += 4;
    while pos < bytes.len() && (bytes[pos] == b'\n' || bytes[pos] == b'\r' || bytes[pos] == b' ') {
        pos += 1;
    }

    // Parse subsections
    while pos < bytes.len() && bytes[pos] != b't' {
        // Not "trailer" yet
        // Read "start count\n"
        let line_end = bytes[pos..].iter().position(|&b| b == b'\n').unwrap_or(50) + pos;
        let line = std::str::from_utf8(&bytes[pos..line_end])
            .map_err(|_| PdfJoinError::ParseError("Invalid xref subsection".into()))?
            .trim();

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() != 2 {
            break; // End of xref entries
        }

        let start_obj: u32 = parts[0]
            .parse()
            .map_err(|_| PdfJoinError::ParseError("Invalid xref start".into()))?;
        let count: u32 = parts[1]
            .parse()
            .map_err(|_| PdfJoinError::ParseError("Invalid xref count".into()))?;

        pos = line_end + 1;

        // Read entries
        for i in 0..count {
            if pos + 20 > bytes.len() {
                break;
            }

            let entry_line = &bytes[pos..pos + 20];
            let entry_str = std::str::from_utf8(entry_line)
                .map_err(|_| PdfJoinError::ParseError("Invalid xref entry".into()))?;

            let offset: usize = entry_str[0..10]
                .trim()
                .parse()
                .map_err(|_| PdfJoinError::ParseError("Invalid xref offset".into()))?;
            let gen: u16 = entry_str[11..16]
                .trim()
                .parse()
                .map_err(|_| PdfJoinError::ParseError("Invalid xref generation".into()))?;
            let in_use = entry_str.chars().nth(17) == Some('n');

            xref.insert(
                start_obj + i,
                XrefEntry {
                    offset,
                    generation: gen,
                    in_use,
                },
            );

            pos += 20;
        }

        // Skip whitespace
        while pos < bytes.len()
            && (bytes[pos] == b'\n' || bytes[pos] == b'\r' || bytes[pos] == b' ')
        {
            pos += 1;
        }
    }

    Ok((xref, pos))
}

/// Parse trailer dictionary
fn parse_trailer(bytes: &[u8], offset: usize) -> Result<TrailerInfo, PdfJoinError> {
    // Find trailer dict start
    let trailer_start = find_pattern(&bytes[offset..], b"<<")
        .ok_or_else(|| PdfJoinError::ParseError("Trailer dict not found".into()))?
        + offset;

    let trailer_end = find_pattern(&bytes[trailer_start..], b">>")
        .ok_or_else(|| PdfJoinError::ParseError("Trailer end not found".into()))?
        + trailer_start
        + 2;

    let trailer_bytes = &bytes[trailer_start..trailer_end];

    // Extract /Root reference
    let root = extract_ref_after(trailer_bytes, b"/Root")
        .ok_or_else(|| PdfJoinError::ParseError("No /Root in trailer".into()))?;

    // Extract /Size
    let size = extract_int_after(trailer_bytes, b"/Size")
        .ok_or_else(|| PdfJoinError::ParseError("No /Size in trailer".into()))?
        as u32;

    // Extract /Info (optional)
    let info = extract_ref_after(trailer_bytes, b"/Info");

    Ok(TrailerInfo { root, size, info })
}

/// Extract object bytes from start offset to endobj
fn extract_object_bytes(bytes: &[u8], start: usize) -> Result<Vec<u8>, PdfJoinError> {
    // Find "endobj" after start
    let end = find_pattern(&bytes[start..], b"endobj")
        .ok_or_else(|| PdfJoinError::ParseError("endobj not found".into()))?
        + start
        + 6;

    Ok(bytes[start..end].to_vec())
}

/// Find pattern in bytes
fn find_pattern(bytes: &[u8], pattern: &[u8]) -> Option<usize> {
    bytes
        .windows(pattern.len())
        .position(|window| window == pattern)
}

/// Check if bytes contain pattern
fn contains_pattern(bytes: &[u8], pattern: &[u8]) -> bool {
    find_pattern(bytes, pattern).is_some()
}

/// Extract reference after a key (e.g., "/Root 1 0 R")
fn extract_ref_after(bytes: &[u8], key: &[u8]) -> Option<ObjRef> {
    let pos = find_pattern(bytes, key)?;
    let after = &bytes[pos + key.len()..];

    // Skip whitespace
    let start = after.iter().position(|&b| b.is_ascii_digit())?;
    let after = &after[start..];

    // Read object number
    let end = after.iter().position(|&b| !b.is_ascii_digit())?;
    let obj_num: u32 = std::str::from_utf8(&after[..end]).ok()?.parse().ok()?;

    // Skip to generation number
    let after = &after[end..];
    let start = after.iter().position(|&b| b.is_ascii_digit())?;
    let after = &after[start..];
    let end = after.iter().position(|&b| !b.is_ascii_digit())?;
    let gen: u16 = std::str::from_utf8(&after[..end]).ok()?.parse().ok()?;

    Some(ObjRef(obj_num, gen))
}

/// Extract integer after a key (e.g., "/Size 100")
fn extract_int_after(bytes: &[u8], key: &[u8]) -> Option<i64> {
    let pos = find_pattern(bytes, key)?;
    let after = &bytes[pos + key.len()..];

    // Skip whitespace
    let start = after
        .iter()
        .position(|&b| b.is_ascii_digit() || b == b'-')?;
    let after = &after[start..];

    // Read number
    let end = after
        .iter()
        .position(|&b| !b.is_ascii_digit() && b != b'-')
        .unwrap_or(after.len());

    std::str::from_utf8(&after[..end]).ok()?.parse().ok()
}

/// Extract /Pages reference from catalog
fn extract_pages_ref(catalog_bytes: &[u8]) -> Result<ObjRef, PdfJoinError> {
    extract_ref_after(catalog_bytes, b"/Pages")
        .ok_or_else(|| PdfJoinError::ParseError("No /Pages in catalog".into()))
}

/// Extract /Count from Pages dictionary
fn extract_count(pages_bytes: &[u8]) -> Result<u32, PdfJoinError> {
    extract_int_after(pages_bytes, b"/Count")
        .map(|n| n as u32)
        .ok_or_else(|| PdfJoinError::ParseError("No /Count in Pages".into()))
}

/// Extract /Kids array references
fn extract_kids_refs(bytes: &[u8]) -> Result<Vec<ObjRef>, PdfJoinError> {
    let kids_pos = find_pattern(bytes, b"/Kids")
        .ok_or_else(|| PdfJoinError::ParseError("No /Kids in Pages".into()))?;

    let after_kids = &bytes[kids_pos + 5..];
    let array_start = find_pattern(after_kids, b"[")
        .ok_or_else(|| PdfJoinError::ParseError("No [ after /Kids".into()))?;
    let array_end = find_pattern(after_kids, b"]")
        .ok_or_else(|| PdfJoinError::ParseError("No ] for /Kids".into()))?;

    let array_bytes = &after_kids[array_start + 1..array_end];

    Ok(extract_all_refs(array_bytes))
}

/// Check if a byte is a valid predecessor for an object reference
/// Object references should follow whitespace, delimiters, or be at start
#[inline]
fn is_valid_ref_predecessor(b: u8) -> bool {
    matches!(b, b' ' | b'\n' | b'\r' | b'\t' | b'[' | b'<' | b'(' | b'/')
}

/// Extract all "N G R" references from bytes (binary-safe)
/// This function correctly handles PDF names with embedded digits like "/F0", "/Font1"
/// by only starting to parse a reference when the digit follows valid predecessors.
pub fn extract_all_refs(bytes: &[u8]) -> Vec<ObjRef> {
    let mut refs = Vec::new();
    let mut i = 0;

    while i < bytes.len() {
        // Look for digit that could start an object reference
        // Key fix: only consider digits that follow valid predecessors
        // This prevents parsing "/F0 21 0 R" as "0 21" instead of "21 0 R"
        if bytes[i].is_ascii_digit() {
            // Check if previous char allows this to be a reference start
            let valid_start = i == 0 || is_valid_ref_predecessor(bytes[i - 1]);

            if !valid_start {
                // This digit is part of a name like "/F0", skip it
                i += 1;
                continue;
            }

            let num_start = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            let obj_num: u32 = std::str::from_utf8(&bytes[num_start..i])
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);

            // Skip whitespace
            while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\n' || bytes[i] == b'\r') {
                i += 1;
            }

            // Look for generation
            if i < bytes.len() && bytes[i].is_ascii_digit() {
                let gen_start = i;
                while i < bytes.len() && bytes[i].is_ascii_digit() {
                    i += 1;
                }
                let gen: u16 = std::str::from_utf8(&bytes[gen_start..i])
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);

                // Skip whitespace
                while i < bytes.len()
                    && (bytes[i] == b' ' || bytes[i] == b'\n' || bytes[i] == b'\r')
                {
                    i += 1;
                }

                // Check for 'R'
                if i < bytes.len() && bytes[i] == b'R' {
                    // Additional check: 'R' should be followed by non-alphanumeric
                    // to avoid matching things like "R" in the middle of a name
                    let valid_end = i + 1 >= bytes.len() || !bytes[i + 1].is_ascii_alphanumeric();
                    if valid_end {
                        refs.push(ObjRef(obj_num, gen));
                        i += 1;
                        continue;
                    }
                }
            }
        }
        i += 1;
    }

    refs
}

/// Check if reference is a /Parent reference (to avoid cycles)
fn is_parent_ref(obj_bytes: &[u8], ref_to_check: ObjRef) -> bool {
    // Look for /Parent N G R where N G matches ref_to_check
    if let Some(parent_ref) = extract_ref_after(obj_bytes, b"/Parent") {
        return parent_ref == ref_to_check;
    }
    false
}

// =============================================================================
// Streaming Split Implementation
// =============================================================================

/// Split PDF using streaming approach (minimal memory, fast)
pub fn split_streaming(bytes: &[u8], pages_to_keep: Vec<u32>) -> Result<Vec<u8>, PdfJoinError> {
    if pages_to_keep.is_empty() {
        return Err(PdfJoinError::InvalidRange("No pages specified".into()));
    }

    let pdf = PdfStructure::parse(bytes)?;
    let all_pages = pdf.get_page_refs()?;
    let page_count = all_pages.len() as u32;

    // Validate page numbers
    for &p in &pages_to_keep {
        if p == 0 || p > page_count {
            return Err(PdfJoinError::InvalidRange(format!(
                "Page {} out of range (1-{})",
                p, page_count
            )));
        }
    }

    // Collect all needed object IDs
    let mut needed_objects: HashSet<u32> = HashSet::new();

    // Get the original catalog and pages refs so we can EXCLUDE them
    // We create fresh Catalog and Pages objects at the end, so including
    // the originals would result in duplicate /Count entries which corrupts the PDF
    let catalog_ref = pdf.trailer.root.0;
    let pages_obj_bytes = pdf.read_object(pdf.trailer.root)?;
    let original_pages_ref = extract_ref_after(&pages_obj_bytes, b"/Pages")
        .map(|r| r.0)
        .unwrap_or(0);

    // For each wanted page, collect dependencies
    let pages_to_keep_set: HashSet<u32> = pages_to_keep.iter().copied().collect();
    let mut kept_page_refs: Vec<ObjRef> = Vec::new();

    for (idx, page_ref) in all_pages.iter().enumerate() {
        let page_num = (idx + 1) as u32;
        if pages_to_keep_set.contains(&page_num) {
            kept_page_refs.push(page_ref.obj_ref);
            let deps = pdf.collect_page_dependencies(page_ref.obj_ref)?;
            needed_objects.extend(deps);
        }
    }

    // Remove the original Catalog and Pages objects - we create new ones
    needed_objects.remove(&catalog_ref);
    needed_objects.remove(&original_pages_ref);

    // Build new PDF
    let mut output = Vec::new();

    // Write header
    output.extend_from_slice(format!("%PDF-{}\n", pdf.version).as_bytes());
    output.extend_from_slice(b"%\xe2\xe3\xcf\xd3\n"); // Binary marker

    // Track new object offsets
    let mut new_xref: Vec<(u32, usize)> = Vec::new();
    let mut id_mapping: HashMap<u32, u32> = HashMap::new();

    // First pass: assign new IDs to all objects so we know the Pages ID upfront
    let mut sorted_ids: Vec<u32> = needed_objects.iter().copied().collect();
    sorted_ids.sort();

    // Build id_mapping for all objects - use counter, not enumerate index
    let mut new_id_counter = 1u32;
    for &old_id in &sorted_ids {
        if let Some(entry) = pdf.xref.get(&old_id) {
            if entry.in_use {
                id_mapping.insert(old_id, new_id_counter);
                new_id_counter += 1;
            }
        }
    }

    // The new Pages object ID (we'll write it after all other objects)
    // new_id_counter is now (num_objects + 1), which is perfect for Pages
    let pages_id = new_id_counter;

    // Map the original Pages ref to our new Pages ID so /Parent refs get updated
    // This mapping is critical - all pages have /Parent pointing to the original Pages object,
    // and we need to update them to point to our new Pages object
    id_mapping.insert(original_pages_ref, pages_id);

    // Write needed objects with new IDs
    for &old_id in &sorted_ids {
        if let Some(entry) = pdf.xref.get(&old_id) {
            if entry.in_use {
                let new_id = *id_mapping.get(&old_id).unwrap();

                new_xref.push((new_id, output.len()));

                // Read and rewrite object with new ID
                if let Ok(obj_bytes) = pdf.read_object(ObjRef(old_id, entry.generation)) {
                    // Replace object header and update all references including /Parent
                    let rewritten = rewrite_object_refs(&obj_bytes, old_id, new_id, &id_mapping);
                    output.extend_from_slice(&rewritten);
                    output.push(b'\n');
                }
            }
        }
    }

    // Write new Pages object with updated Kids
    new_xref.push((pages_id, output.len()));

    output.extend_from_slice(format!("{} 0 obj\n", pages_id).as_bytes());
    output.extend_from_slice(b"<<\n/Type /Pages\n");
    output.extend_from_slice(format!("/Count {}\n", kept_page_refs.len()).as_bytes());
    output.extend_from_slice(b"/Kids [");
    for page_ref in &kept_page_refs {
        if let Some(&new_id) = id_mapping.get(&page_ref.0) {
            output.extend_from_slice(format!(" {} 0 R", new_id).as_bytes());
        }
    }
    output.extend_from_slice(b" ]\n>>\nendobj\n");

    // Update catalog to point to new Pages
    let catalog_id = pages_id + 1;
    new_xref.push((catalog_id, output.len()));

    output.extend_from_slice(format!("{} 0 obj\n", catalog_id).as_bytes());
    output.extend_from_slice(b"<<\n/Type /Catalog\n");
    output.extend_from_slice(format!("/Pages {} 0 R\n", pages_id).as_bytes());
    output.extend_from_slice(b">>\nendobj\n");

    // Write xref
    // Total objects = catalog_id (since catalog is the last object)
    let total_objects = catalog_id + 1; // +1 for the free object 0
    let xref_offset = output.len();
    output.extend_from_slice(b"xref\n");
    output.extend_from_slice(format!("0 {}\n", total_objects).as_bytes());
    output.extend_from_slice(b"0000000000 65535 f \n");

    // Sort xref by ID
    new_xref.sort_by_key(|(id, _)| *id);
    for (_, offset) in &new_xref {
        output.extend_from_slice(format!("{:010} 00000 n \n", offset).as_bytes());
    }

    // Write trailer
    output.extend_from_slice(b"trailer\n<<\n");
    output.extend_from_slice(format!("/Size {}\n", total_objects).as_bytes());
    output.extend_from_slice(format!("/Root {} 0 R\n", catalog_id).as_bytes());
    output.extend_from_slice(b">>\n");
    output.extend_from_slice(format!("startxref\n{}\n%%EOF\n", xref_offset).as_bytes());

    Ok(output)
}

/// Rewrite object with new ID and update internal references (binary-safe)
fn rewrite_object_refs(
    obj_bytes: &[u8],
    old_id: u32,
    new_id: u32,
    id_mapping: &HashMap<u32, u32>,
) -> Vec<u8> {
    // For binary safety, we work with bytes directly
    // First, replace the object header
    let old_header = format!("{} 0 obj", old_id).into_bytes();
    let new_header = format!("{} 0 obj", new_id).into_bytes();

    let mut result = replace_bytes(obj_bytes, &old_header, &new_header);

    // Replace all references "X 0 R" with mapped IDs
    // CRITICAL: We must avoid replacing refs that were already replaced.
    // If old ID A maps to new ID B, and old ID B maps to new ID C,
    // then after replacing A->B, we'd incorrectly replace B->C.
    //
    // Solution: Do all replacements in a single pass by finding all refs first,
    // then replacing them from end to start (so positions don't shift).
    let refs_in_object = extract_all_refs(&result);

    // Build list of replacements (position, old_len, new_bytes)
    let mut replacements: Vec<(usize, usize, Vec<u8>)> = Vec::new();

    for obj_ref in &refs_in_object {
        if let Some(&new_ref_id) = id_mapping.get(&obj_ref.0) {
            // Find this ref in result
            let old_ref = format!("{} {} R", obj_ref.0, obj_ref.1);
            let new_ref = format!("{} {} R", new_ref_id, obj_ref.1);

            // Find all occurrences
            let old_ref_bytes = old_ref.as_bytes();
            let mut search_start = 0;
            while let Some(pos) = find_pattern(&result[search_start..], old_ref_bytes) {
                let abs_pos = search_start + pos;
                replacements.push((abs_pos, old_ref_bytes.len(), new_ref.clone().into_bytes()));
                search_start = abs_pos + old_ref_bytes.len();
            }
        }
    }

    // Sort by position descending so we can replace from end to start
    replacements.sort_by(|a, b| b.0.cmp(&a.0));

    // Apply replacements
    for (pos, old_len, new_bytes) in replacements {
        let mut new_result = Vec::with_capacity(result.len() - old_len + new_bytes.len());
        new_result.extend_from_slice(&result[..pos]);
        new_result.extend_from_slice(&new_bytes);
        new_result.extend_from_slice(&result[pos + old_len..]);
        result = new_result;
    }

    result
}

/// Replace pattern in bytes (binary-safe)
fn replace_bytes(haystack: &[u8], needle: &[u8], replacement: &[u8]) -> Vec<u8> {
    if needle.is_empty() {
        return haystack.to_vec();
    }

    let mut result = Vec::new();
    let mut i = 0;

    while i < haystack.len() {
        if i + needle.len() <= haystack.len() && &haystack[i..i + needle.len()] == needle {
            result.extend_from_slice(replacement);
            i += needle.len();
        } else {
            result.push(haystack[i]);
            i += 1;
        }
    }

    result
}

// =============================================================================
// Streaming Merge Implementation
// =============================================================================

/// Merge PDFs using streaming approach
pub fn merge_streaming(documents: Vec<Vec<u8>>) -> Result<Vec<u8>, PdfJoinError> {
    if documents.is_empty() {
        return Err(PdfJoinError::OperationError("No documents to merge".into()));
    }

    if documents.len() == 1 {
        return Ok(documents.into_iter().next().unwrap());
    }

    // Parse all documents
    let mut pdfs: Vec<PdfStructure> = Vec::new();
    for (i, bytes) in documents.iter().enumerate() {
        let pdf = PdfStructure::parse(bytes)
            .map_err(|e| PdfJoinError::ParseError(format!("Document {}: {}", i, e)))?;
        pdfs.push(pdf);
    }

    let mut output = Vec::new();
    let version = pdfs
        .first()
        .map(|p| p.version.clone())
        .unwrap_or("1.7".into());

    // Write header
    output.extend_from_slice(format!("%PDF-{}\n", version).as_bytes());
    output.extend_from_slice(b"%\xe2\xe3\xcf\xd3\n");

    let mut new_xref: Vec<(u32, usize)> = Vec::new();
    let mut next_id = 1u32;
    let mut all_page_refs: Vec<u32> = Vec::new(); // New IDs of all pages

    // Process each document
    for (doc_idx, pdf) in pdfs.iter().enumerate() {
        let mut id_mapping: HashMap<u32, u32> = HashMap::new();

        // Get pages for this document
        let pages = pdf.get_page_refs()?;

        // Collect all needed objects for all pages in this doc
        let mut needed_objects: HashSet<u32> = HashSet::new();
        for page_ref in &pages {
            let deps = pdf.collect_page_dependencies(page_ref.obj_ref)?;
            needed_objects.extend(deps);
        }

        // Write objects with remapped IDs
        let mut sorted_ids: Vec<u32> = needed_objects.iter().copied().collect();
        sorted_ids.sort();

        for old_id in sorted_ids {
            if let Some(entry) = pdf.xref.get(&old_id) {
                if entry.in_use {
                    let new_id = next_id;
                    id_mapping.insert(old_id, new_id);
                    next_id += 1;

                    new_xref.push((new_id, output.len()));

                    if let Ok(obj_bytes) = pdf.read_object(ObjRef(old_id, entry.generation)) {
                        let rewritten =
                            rewrite_object_refs(&obj_bytes, old_id, new_id, &id_mapping);

                        // Also need to update /Parent references for pages
                        // (we'll fix this by not including Parent in output, or updating later)
                        output.extend_from_slice(&rewritten);
                        output.push(b'\n');
                    }
                }
            }
        }

        // Track page IDs for final page tree
        for page_ref in &pages {
            if let Some(&new_id) = id_mapping.get(&page_ref.obj_ref.0) {
                all_page_refs.push(new_id);
            }
        }

        eprintln!(
            "Document {}: {} pages, {} objects",
            doc_idx,
            pages.len(),
            id_mapping.len()
        );
    }

    // Write combined Pages object
    let pages_id = next_id;
    next_id += 1;
    new_xref.push((pages_id, output.len()));

    output.extend_from_slice(format!("{} 0 obj\n", pages_id).as_bytes());
    output.extend_from_slice(b"<<\n/Type /Pages\n");
    output.extend_from_slice(format!("/Count {}\n", all_page_refs.len()).as_bytes());
    output.extend_from_slice(b"/Kids [");
    for page_id in &all_page_refs {
        output.extend_from_slice(format!(" {} 0 R", page_id).as_bytes());
    }
    output.extend_from_slice(b" ]\n>>\nendobj\n");

    // Write catalog
    let catalog_id = next_id;
    next_id += 1;
    new_xref.push((catalog_id, output.len()));

    output.extend_from_slice(format!("{} 0 obj\n", catalog_id).as_bytes());
    output.extend_from_slice(b"<<\n/Type /Catalog\n");
    output.extend_from_slice(format!("/Pages {} 0 R\n", pages_id).as_bytes());
    output.extend_from_slice(b">>\nendobj\n");

    // Write xref
    let xref_offset = output.len();
    output.extend_from_slice(b"xref\n");
    output.extend_from_slice(format!("0 {}\n", next_id).as_bytes());
    output.extend_from_slice(b"0000000000 65535 f \n");

    new_xref.sort_by_key(|(id, _)| *id);
    let mut expected_id = 1u32;
    for (id, offset) in &new_xref {
        // Fill gaps with free entries
        while expected_id < *id {
            output.extend_from_slice(b"0000000000 65535 f \n");
            expected_id += 1;
        }
        output.extend_from_slice(format!("{:010} 00000 n \n", offset).as_bytes());
        expected_id += 1;
    }

    // Write trailer
    output.extend_from_slice(b"trailer\n<<\n");
    output.extend_from_slice(format!("/Size {}\n", next_id).as_bytes());
    output.extend_from_slice(format!("/Root {} 0 R\n", catalog_id).as_bytes());
    output.extend_from_slice(b">>\n");
    output.extend_from_slice(format!("startxref\n{}\n%%EOF\n", xref_offset).as_bytes());

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_minimal_pdf() -> Vec<u8> {
        // Build PDF with correct offsets
        let mut pdf = Vec::new();

        // Header
        pdf.extend_from_slice(b"%PDF-1.4\n");

        // Object 1: Catalog
        let obj1_offset = pdf.len();
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

        // Object 2: Pages
        let obj2_offset = pdf.len();
        pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");

        // Object 3: Page
        let obj3_offset = pdf.len();
        pdf.extend_from_slice(
            b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>\nendobj\n",
        );

        // xref
        let xref_offset = pdf.len();
        pdf.extend_from_slice(b"xref\n");
        pdf.extend_from_slice(b"0 4\n");
        pdf.extend_from_slice(b"0000000000 65535 f \n");
        pdf.extend_from_slice(format!("{:010} 00000 n \n", obj1_offset).as_bytes());
        pdf.extend_from_slice(format!("{:010} 00000 n \n", obj2_offset).as_bytes());
        pdf.extend_from_slice(format!("{:010} 00000 n \n", obj3_offset).as_bytes());

        // trailer
        pdf.extend_from_slice(b"trailer\n<< /Size 4 /Root 1 0 R >>\n");
        pdf.extend_from_slice(format!("startxref\n{}\n%%EOF\n", xref_offset).as_bytes());

        pdf
    }

    #[test]
    fn test_parse_minimal_pdf() {
        let pdf_bytes = create_minimal_pdf();
        let pdf = PdfStructure::parse(&pdf_bytes).expect("Should parse");

        assert_eq!(pdf.version, "1.4");
        assert_eq!(pdf.trailer.root.0, 1);
        assert_eq!(pdf.trailer.size, 4);
    }

    #[test]
    fn test_get_page_count() {
        let pdf_bytes = create_minimal_pdf();
        let pdf = PdfStructure::parse(&pdf_bytes).expect("Should parse");

        let count = pdf.get_page_count().expect("Should get count");
        assert_eq!(count, 1);
    }

    #[test]
    fn test_find_startxref() {
        let pdf_bytes = create_minimal_pdf();
        let offset = find_startxref(&pdf_bytes).expect("Should find startxref");
        assert!(offset > 0);
    }

    #[test]
    fn test_get_page_refs() {
        let pdf_bytes = create_minimal_pdf();
        let pdf = PdfStructure::parse(&pdf_bytes).expect("Should parse");

        let pages = pdf.get_page_refs().expect("Should get pages");
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].obj_ref.0, 3); // Page is object 3
    }

    // =========================================================================
    // Property tests for extract_all_refs
    // =========================================================================

    /// Test that simple refs are found
    #[test]
    fn test_extract_refs_simple() {
        let input = b"1 0 R";
        let refs = extract_all_refs(input);
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0], ObjRef(1, 0));
    }

    /// Test multiple refs
    #[test]
    fn test_extract_refs_multiple() {
        let input = b"1 0 R 2 0 R 3 0 R";
        let refs = extract_all_refs(input);
        assert_eq!(refs.len(), 3);
        assert_eq!(refs[0], ObjRef(1, 0));
        assert_eq!(refs[1], ObjRef(2, 0));
        assert_eq!(refs[2], ObjRef(3, 0));
    }

    /// Test refs in array
    #[test]
    fn test_extract_refs_in_array() {
        let input = b"[1 0 R 2 0 R]";
        let refs = extract_all_refs(input);
        assert_eq!(refs.len(), 2);
    }

    /// Test refs in dictionary
    #[test]
    fn test_extract_refs_in_dict() {
        let input = b"<< /Key 1 0 R /Other 2 0 R >>";
        let refs = extract_all_refs(input);
        assert_eq!(refs.len(), 2);
    }

    /// CRITICAL: Test that font names like /F0, /F1 don't break ref parsing
    #[test]
    fn test_extract_refs_with_font_names() {
        // This is the exact pattern that was broken before
        let input = b"/Font << /F0 21 0 R /F1 26 0 R /F2 31 0 R >>";
        let refs = extract_all_refs(input);
        assert_eq!(refs.len(), 3, "Should find 3 font refs");
        assert_eq!(refs[0], ObjRef(21, 0));
        assert_eq!(refs[1], ObjRef(26, 0));
        assert_eq!(refs[2], ObjRef(31, 0));
    }

    /// Test various PDF name patterns with embedded digits
    #[test]
    fn test_extract_refs_ignores_digits_in_names() {
        // Various PDF names that contain digits but aren't refs
        let inputs = vec![
            (b"/Font1 5 0 R".as_slice(), vec![ObjRef(5, 0)]),
            (b"/F0 5 0 R".as_slice(), vec![ObjRef(5, 0)]),
            (b"/Image42 5 0 R".as_slice(), vec![ObjRef(5, 0)]),
            (b"/GS0 5 0 R".as_slice(), vec![ObjRef(5, 0)]),
            (
                b"/XObject << /Im0 10 0 R >>".as_slice(),
                vec![ObjRef(10, 0)],
            ),
        ];

        for (input, expected) in inputs {
            let refs = extract_all_refs(input);
            assert_eq!(
                refs,
                expected,
                "Failed for input: {:?}",
                String::from_utf8_lossy(input)
            );
        }
    }

    /// Test refs with various whitespace patterns
    #[test]
    fn test_extract_refs_whitespace_variations() {
        let inputs = vec![
            b"1 0 R".as_slice(),       // single space
            b"1  0  R".as_slice(),     // double space
            b"1\n0\nR".as_slice(),     // newlines
            b"1\r\n0\r\nR".as_slice(), // CRLF
            b"1 \n 0 \n R".as_slice(), // mixed
        ];

        for input in inputs {
            let refs = extract_all_refs(input);
            assert_eq!(
                refs.len(),
                1,
                "Should find 1 ref in: {:?}",
                String::from_utf8_lossy(input)
            );
            assert_eq!(refs[0], ObjRef(1, 0));
        }
    }

    /// Test that refs at boundaries are found
    #[test]
    fn test_extract_refs_at_boundaries() {
        // Start of input
        let refs = extract_all_refs(b"1 0 R rest");
        assert_eq!(refs.len(), 1);

        // After newline (common in PDF)
        let refs = extract_all_refs(b"stuff\n1 0 R");
        assert_eq!(refs.len(), 1);

        // After [ (array start)
        let refs = extract_all_refs(b"[1 0 R");
        assert_eq!(refs.len(), 1);

        // After << (dict start)
        let refs = extract_all_refs(b"<<1 0 R");
        assert_eq!(refs.len(), 1);
    }

    /// Test large object numbers (real PDFs can have hundreds of objects)
    #[test]
    fn test_extract_refs_large_numbers() {
        let input = b"999 0 R 1000 0 R 12345 0 R";
        let refs = extract_all_refs(input);
        assert_eq!(refs.len(), 3);
        assert_eq!(refs[0], ObjRef(999, 0));
        assert_eq!(refs[1], ObjRef(1000, 0));
        assert_eq!(refs[2], ObjRef(12345, 0));
    }

    /// Test non-zero generation numbers
    #[test]
    fn test_extract_refs_with_generation() {
        let input = b"1 1 R 2 5 R 3 99 R";
        let refs = extract_all_refs(input);
        assert_eq!(refs.len(), 3);
        assert_eq!(refs[0], ObjRef(1, 1));
        assert_eq!(refs[1], ObjRef(2, 5));
        assert_eq!(refs[2], ObjRef(3, 99));
    }

    /// Test realistic Resources dictionary (the exact pattern that was broken)
    #[test]
    fn test_extract_refs_realistic_resources() {
        let input = br#"20 0 obj
<<
  /XObject 64 0 R
  /Pattern 65 0 R
  /ExtGState 66 0 R
  /ColorSpace 67 0 R
  /Font <<
    /F0 21 0 R
    /F1 26 0 R
    /F2 31 0 R
    /F3 36 0 R
    /F4 41 0 R
  >>
>>
endobj"#;

        let refs = extract_all_refs(input);

        // Should find: 64, 65, 66, 67, 21, 26, 31, 36, 41 = 9 refs
        assert_eq!(refs.len(), 9, "Should find all 9 refs in Resources dict");

        // Verify the font refs specifically (these were the ones being missed)
        let font_refs: Vec<u32> = refs.iter().map(|r| r.0).collect();
        assert!(font_refs.contains(&21), "Should find font ref 21");
        assert!(font_refs.contains(&26), "Should find font ref 26");
        assert!(font_refs.contains(&31), "Should find font ref 31");
        assert!(font_refs.contains(&36), "Should find font ref 36");
        assert!(font_refs.contains(&41), "Should find font ref 41");
    }
}

// Property tests using proptest
#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        /// Property: extract_all_refs never panics on arbitrary input
        #[test]
        fn extract_refs_never_panics(input in prop::collection::vec(any::<u8>(), 0..1000)) {
            let _ = extract_all_refs(&input);
        }

        /// Property: a properly formatted ref "N G R" is always found when preceded by valid char
        #[test]
        fn valid_ref_is_found(obj_num in 1u32..10000, gen in 0u16..100) {
            let input = format!(" {} {} R", obj_num, gen);
            let refs = extract_all_refs(input.as_bytes());
            prop_assert_eq!(refs.len(), 1);
            prop_assert_eq!(refs[0], ObjRef(obj_num, gen));
        }

        /// Property: refs in arrays are found
        #[test]
        fn refs_in_array_found(
            obj_nums in prop::collection::vec(1u32..1000, 1..10)
        ) {
            let refs_str: Vec<String> = obj_nums.iter()
                .map(|n| format!("{} 0 R", n))
                .collect();
            let input = format!("[{}]", refs_str.join(" "));
            let refs = extract_all_refs(input.as_bytes());
            prop_assert_eq!(refs.len(), obj_nums.len());
        }

        /// Property: digit after letter should not start a ref
        /// e.g., "/F0 5 0 R" should only find ref 5, not misparse "/F0"
        #[test]
        fn digit_after_letter_not_ref(
            name in "[A-Za-z]+[0-9]+",
            obj_num in 1u32..1000
        ) {
            let input = format!("/{} {} 0 R", name, obj_num);
            let refs = extract_all_refs(input.as_bytes());
            // Should only find the actual ref, not the name
            prop_assert_eq!(refs.len(), 1);
            prop_assert_eq!(refs[0].0, obj_num);
        }

        /// Property: multiple refs separated by various whitespace are all found
        #[test]
        fn multiple_refs_with_whitespace(
            obj_nums in prop::collection::vec(1u32..1000, 2..5),
            sep in "[ \n\r\t]+"
        ) {
            let refs_str: Vec<String> = obj_nums.iter()
                .map(|n| format!("{} 0 R", n))
                .collect();
            let input = refs_str.join(&sep);
            let refs = extract_all_refs(input.as_bytes());
            prop_assert_eq!(refs.len(), obj_nums.len());
        }
    }
}
