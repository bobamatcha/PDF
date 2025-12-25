//! Apply operations to PDF documents

use crate::error::PdfJoinError;
use crate::operations::{EditOperation, OperationLog, PdfRect, TextStyle};
use lopdf::{Dictionary, Document, Object, ObjectId};

/// Calculate approximate text width in points for a given string
/// Uses average character widths for common PDF fonts
fn calculate_text_width(text: &str, font_size: f64, font_name: &str) -> f64 {
    // Average character width as a fraction of font size for Type1 fonts
    // Helvetica: ~0.52 em per character average
    // Times: ~0.48 em per character average
    // Courier: 0.6 em per character (monospace)
    let avg_char_width_factor = if font_name.contains("Courier") {
        0.6
    } else if font_name.contains("Times") {
        0.50
    } else {
        // Helvetica and others
        0.55
    };

    let char_count = text.chars().count() as f64;
    let text_width = char_count * font_size * avg_char_width_factor;

    // Add padding for the text positioning offset (2 points from edge)
    text_width + 10.0
}

/// Parse hex color string (e.g., "#FF0000" or "FF0000") to RGB floats (0-1 range)
fn parse_hex_color(color: &str) -> (f32, f32, f32) {
    let hex = color.trim_start_matches('#');
    if hex.len() >= 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0) as f32 / 255.0;
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0) as f32 / 255.0;
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0) as f32 / 255.0;
        (r, g, b)
    } else {
        (0.0, 0.0, 0.0) // Default to black
    }
}

/// Apply all operations from the log to a PDF document
pub fn apply_operations(pdf_bytes: &[u8], log: &OperationLog) -> Result<Vec<u8>, PdfJoinError> {
    if log.is_empty() {
        // No changes, return original
        return Ok(pdf_bytes.to_vec());
    }

    let mut doc =
        Document::load_mem(pdf_bytes).map_err(|e| PdfJoinError::ParseError(e.to_string()))?;

    let pages: Vec<(u32, ObjectId)> = doc.get_pages().into_iter().collect();

    for (page_num, page_id) in &pages {
        let page_ops = log.operations_for_page(*page_num);
        for op in page_ops {
            apply_single_operation(&mut doc, *page_id, op)?;
        }
    }

    // Compress to remove orphaned objects and reduce file size
    doc.compress();

    let mut output = Vec::new();
    doc.save_to(&mut output)
        .map_err(|e| PdfJoinError::OperationError(e.to_string()))?;

    Ok(output)
}

/// Apply all operations flattened directly into page content streams.
/// Unlike `apply_operations`, this does NOT create annotations - it burns
/// the content directly into the page, making edits permanent and non-removable.
pub fn apply_operations_flattened(
    pdf_bytes: &[u8],
    log: &OperationLog,
) -> Result<Vec<u8>, PdfJoinError> {
    if log.is_empty() {
        return Ok(pdf_bytes.to_vec());
    }

    let mut doc =
        Document::load_mem(pdf_bytes).map_err(|e| PdfJoinError::ParseError(e.to_string()))?;

    let pages: Vec<(u32, ObjectId)> = doc.get_pages().into_iter().collect();

    for (page_num, page_id) in &pages {
        let page_ops = log.operations_for_page(*page_num);
        if page_ops.is_empty() {
            continue;
        }

        // Build content stream additions for this page
        let mut content_additions = String::new();

        for op in page_ops {
            flatten_operation_to_content(&mut content_additions, op)?;
        }

        if !content_additions.is_empty() {
            // Append to page content stream
            append_to_page_content(&mut doc, *page_id, &content_additions)?;
        }
    }

    doc.compress();

    let mut output = Vec::new();
    doc.save_to(&mut output)
        .map_err(|e| PdfJoinError::OperationError(e.to_string()))?;

    Ok(output)
}

/// Convert an operation to PDF content stream operators
fn flatten_operation_to_content(
    content: &mut String,
    op: &EditOperation,
) -> Result<(), PdfJoinError> {
    use std::fmt::Write;

    match op {
        EditOperation::AddText {
            rect, text, style, ..
        } => {
            let (r, g, b) = parse_hex_color(&style.color);
            let font_name = style.pdf_font_name();

            // Save graphics state
            writeln!(content, "q").unwrap();
            // Set font and color
            writeln!(content, "{} {} {} rg", r, g, b).unwrap();
            writeln!(content, "BT").unwrap();
            writeln!(content, "/{} {} Tf", font_name, style.font_size).unwrap();
            writeln!(content, "{} {} Td", rect.x, rect.y).unwrap();
            // Escape text for PDF
            let escaped = escape_pdf_string(text);
            writeln!(content, "({}) Tj", escaped).unwrap();
            writeln!(content, "ET").unwrap();
            // Restore graphics state
            writeln!(content, "Q").unwrap();
        }
        EditOperation::AddHighlight {
            rect,
            color,
            opacity,
            ..
        } => {
            let (r, g, b) = parse_hex_color(color);

            writeln!(content, "q").unwrap();
            // Set transparency if not fully opaque
            if *opacity < 1.0 {
                writeln!(content, "/GS1 gs").unwrap(); // Assumes GS1 is defined with alpha
            }
            writeln!(content, "{} {} {} rg", r, g, b).unwrap();
            writeln!(
                content,
                "{} {} {} {} re f",
                rect.x, rect.y, rect.width, rect.height
            )
            .unwrap();
            writeln!(content, "Q").unwrap();
        }
        EditOperation::AddUnderline { rect, color, .. } => {
            let (r, g, b) = parse_hex_color(color);

            writeln!(content, "q").unwrap();
            // Set stroke color
            writeln!(content, "{} {} {} RG", r, g, b).unwrap();
            // Set line width (1-2 points for underline)
            writeln!(content, "1 w").unwrap();
            // Draw a line at the bottom of the rect
            // Move to start, line to end, stroke
            let y = rect.y; // Bottom of rect (underline goes below text)
            writeln!(
                content,
                "{} {} m {} {} l S",
                rect.x,
                y,
                rect.x + rect.width,
                y
            )
            .unwrap();
            writeln!(content, "Q").unwrap();
        }
        EditOperation::AddCheckbox { rect, checked, .. } => {
            // Draw checkbox as a rectangle with optional checkmark
            writeln!(content, "q").unwrap();
            // Draw border
            writeln!(content, "0 0 0 RG").unwrap(); // Black stroke
            writeln!(content, "1 w").unwrap(); // 1pt line width
            writeln!(
                content,
                "{} {} {} {} re S",
                rect.x, rect.y, rect.width, rect.height
            )
            .unwrap();

            if *checked {
                // Draw checkmark
                writeln!(content, "0 0 0 RG").unwrap();
                writeln!(content, "2 w").unwrap();
                let x1 = rect.x + rect.width * 0.2;
                let y1 = rect.y + rect.height * 0.5;
                let x2 = rect.x + rect.width * 0.4;
                let y2 = rect.y + rect.height * 0.2;
                let x3 = rect.x + rect.width * 0.8;
                let y3 = rect.y + rect.height * 0.8;
                writeln!(content, "{} {} m {} {} l {} {} l S", x1, y1, x2, y2, x3, y3).unwrap();
            }
            writeln!(content, "Q").unwrap();
        }
        EditOperation::AddWhiteRect { rect, .. } => {
            // Draw white filled rectangle
            writeln!(content, "q").unwrap();
            writeln!(content, "1 1 1 rg").unwrap(); // White fill
            writeln!(
                content,
                "{} {} {} {} re f",
                rect.x, rect.y, rect.width, rect.height
            )
            .unwrap();
            writeln!(content, "Q").unwrap();
        }
        EditOperation::ReplaceText {
            original_rect,
            replacement_rect,
            new_text,
            style,
            ..
        } => {
            // First draw white rectangle over original
            writeln!(content, "q").unwrap();
            writeln!(content, "1 1 1 rg").unwrap();
            writeln!(
                content,
                "{} {} {} {} re f",
                original_rect.x, original_rect.y, original_rect.width, original_rect.height
            )
            .unwrap();
            writeln!(content, "Q").unwrap();

            // Then draw new text
            let (r, g, b) = parse_hex_color(&style.color);
            let font_name = style.pdf_font_name();

            writeln!(content, "q").unwrap();
            writeln!(content, "{} {} {} rg", r, g, b).unwrap();
            writeln!(content, "BT").unwrap();
            writeln!(content, "/{} {} Tf", font_name, style.font_size).unwrap();
            writeln!(content, "{} {} Td", replacement_rect.x, replacement_rect.y).unwrap();
            let escaped = escape_pdf_string(new_text);
            writeln!(content, "({}) Tj", escaped).unwrap();
            writeln!(content, "ET").unwrap();
            writeln!(content, "Q").unwrap();
        }
    }

    Ok(())
}

/// Append content to a page's content stream
fn append_to_page_content(
    doc: &mut Document,
    page_id: ObjectId,
    new_content: &str,
) -> Result<(), PdfJoinError> {
    // Get the page object
    let page = doc
        .get_object(page_id)
        .map_err(|e| PdfJoinError::OperationError(e.to_string()))?
        .clone();

    let page_dict = match page {
        Object::Dictionary(d) => d,
        _ => {
            return Err(PdfJoinError::OperationError(
                "Page is not a dictionary".into(),
            ))
        }
    };

    // Get existing content
    let existing_content = if let Ok(contents) = page_dict.get(b"Contents") {
        get_content_bytes(doc, contents)?
    } else {
        Vec::new()
    };

    // Combine existing content with new content
    let mut combined = existing_content;
    combined.extend_from_slice(b"\n");
    combined.extend_from_slice(new_content.as_bytes());

    // Create new content stream
    let new_stream = lopdf::Stream::new(Dictionary::new(), combined);
    let new_content_id = doc.add_object(Object::Stream(new_stream));

    // Update page to point to new content
    let mut new_page_dict = page_dict.clone();
    new_page_dict.set("Contents", Object::Reference(new_content_id));

    // Ensure page has required font resources for flattened text
    ensure_font_resources(&mut new_page_dict, doc)?;

    doc.set_object(page_id, Object::Dictionary(new_page_dict));

    Ok(())
}

/// Get bytes from content stream (handles both direct and reference)
fn get_content_bytes(doc: &Document, contents: &Object) -> Result<Vec<u8>, PdfJoinError> {
    match contents {
        Object::Reference(id) => {
            let obj = doc
                .get_object(*id)
                .map_err(|e| PdfJoinError::OperationError(e.to_string()))?;
            get_content_bytes(doc, obj)
        }
        Object::Stream(stream) => Ok(stream
            .decompressed_content()
            .unwrap_or_else(|_| stream.content.clone())),
        Object::Array(arr) => {
            // Multiple content streams - concatenate
            let mut result = Vec::new();
            for item in arr {
                result.extend(get_content_bytes(doc, item)?);
                result.push(b'\n');
            }
            Ok(result)
        }
        _ => Ok(Vec::new()),
    }
}

/// Ensure the page has font resources for the standard fonts we use
fn ensure_font_resources(
    page_dict: &mut Dictionary,
    doc: &mut Document,
) -> Result<(), PdfJoinError> {
    // Get or create Resources dictionary
    let resources = if let Ok(res) = page_dict.get(b"Resources") {
        match res {
            Object::Dictionary(d) => d.clone(),
            Object::Reference(id) => match doc.get_object(*id) {
                Ok(Object::Dictionary(d)) => d.clone(),
                _ => Dictionary::new(),
            },
            _ => Dictionary::new(),
        }
    } else {
        Dictionary::new()
    };

    // Get or create Font dictionary
    let mut fonts = if let Ok(f) = resources.get(b"Font") {
        match f {
            Object::Dictionary(d) => d.clone(),
            Object::Reference(id) => match doc.get_object(*id) {
                Ok(Object::Dictionary(d)) => d.clone(),
                _ => Dictionary::new(),
            },
            _ => Dictionary::new(),
        }
    } else {
        Dictionary::new()
    };

    // Add standard fonts if not present
    let standard_fonts = [
        ("Helvetica", "Helvetica"),
        ("Helvetica-Bold", "Helvetica-Bold"),
        ("Helvetica-Oblique", "Helvetica-Oblique"),
        ("Helvetica-BoldOblique", "Helvetica-BoldOblique"),
    ];

    for (name, base_font) in standard_fonts {
        if fonts.get(name.as_bytes()).is_err() {
            let mut font_dict = Dictionary::new();
            font_dict.set("Type", Object::Name(b"Font".to_vec()));
            font_dict.set("Subtype", Object::Name(b"Type1".to_vec()));
            font_dict.set("BaseFont", Object::Name(base_font.as_bytes().to_vec()));
            fonts.set(name, Object::Dictionary(font_dict));
        }
    }

    // Update resources with fonts
    let mut new_resources = resources.clone();
    new_resources.set("Font", Object::Dictionary(fonts));
    page_dict.set("Resources", Object::Dictionary(new_resources));

    Ok(())
}

fn apply_single_operation(
    doc: &mut Document,
    page_id: ObjectId,
    op: &EditOperation,
) -> Result<(), PdfJoinError> {
    match op {
        EditOperation::AddText {
            rect, text, style, ..
        } => add_text_annotation(doc, page_id, rect, text, style),
        EditOperation::AddHighlight {
            rect,
            color,
            opacity,
            ..
        } => add_highlight_annotation(doc, page_id, rect, color, *opacity),
        EditOperation::AddUnderline { rect, color, .. } => {
            add_underline_annotation(doc, page_id, rect, color)
        }
        EditOperation::AddCheckbox { rect, checked, .. } => {
            add_checkbox_annotation(doc, page_id, rect, *checked)
        }
        EditOperation::ReplaceText {
            original_rect,
            replacement_rect,
            new_text,
            style,
            ..
        } => add_text_replacement(
            doc,
            page_id,
            original_rect,
            replacement_rect,
            new_text,
            style,
        ),
        EditOperation::AddWhiteRect { rect, .. } => add_white_rect_annotation(doc, page_id, rect),
    }
}

fn add_text_annotation(
    doc: &mut Document,
    page_id: ObjectId,
    rect: &PdfRect,
    text: &str,
    style: &TextStyle,
) -> Result<(), PdfJoinError> {
    // Parse color from hex to RGB components (0-1 range)
    let (r, g, b) = parse_hex_color(&style.color);

    // Font name for PDF standard fonts
    let font_name = style.pdf_font_name();

    // Calculate minimum required width for text content
    // This prevents text truncation/clipping when the provided rect is too small
    let min_text_width = calculate_text_width(text, style.font_size, font_name);
    let actual_width = rect.width.max(min_text_width);

    // Calculate annotation rectangle with adjusted width
    let x1 = rect.x as f32;
    let y1 = rect.y as f32;
    let x2 = (rect.x + actual_width) as f32;
    let y2 = (rect.y + rect.height) as f32;

    // Create appearance stream content
    // This explicitly draws the text, ensuring reliable rendering across all PDF viewers
    let ap_content = create_text_appearance_content(text, style.font_size, r, g, b, font_name);

    // Create the appearance stream (Form XObject)
    let mut ap_stream_dict = Dictionary::new();
    ap_stream_dict.set("Type", Object::Name(b"XObject".to_vec()));
    ap_stream_dict.set("Subtype", Object::Name(b"Form".to_vec()));
    ap_stream_dict.set("FormType", Object::Integer(1));
    ap_stream_dict.set(
        "BBox",
        Object::Array(vec![
            Object::Real(0.0),
            Object::Real(0.0),
            Object::Real(x2 - x1),
            Object::Real(y2 - y1),
        ]),
    );

    // Resources for the appearance stream (font)
    let mut font_dict = Dictionary::new();
    let mut font_entry = Dictionary::new();
    font_entry.set("Type", Object::Name(b"Font".to_vec()));
    font_entry.set("Subtype", Object::Name(b"Type1".to_vec()));
    font_entry.set("BaseFont", Object::Name(font_name.as_bytes().to_vec()));
    font_dict.set("F1", Object::Dictionary(font_entry));

    let mut resources = Dictionary::new();
    resources.set("Font", Object::Dictionary(font_dict));
    ap_stream_dict.set("Resources", Object::Dictionary(resources));

    // Create the stream object
    let ap_stream = lopdf::Stream::new(ap_stream_dict, ap_content.into_bytes());
    let ap_stream_id = doc.add_object(Object::Stream(ap_stream));

    // Create AP dictionary pointing to the normal appearance
    let mut ap_dict = Dictionary::new();
    ap_dict.set("N", Object::Reference(ap_stream_id));

    // Create the annotation
    let mut annot = Dictionary::new();
    annot.set("Type", Object::Name(b"Annot".to_vec()));
    annot.set("Subtype", Object::Name(b"FreeText".to_vec()));
    annot.set(
        "Rect",
        Object::Array(vec![
            Object::Real(x1),
            Object::Real(y1),
            Object::Real(x2),
            Object::Real(y2),
        ]),
    );
    annot.set(
        "Contents",
        Object::String(text.as_bytes().to_vec(), lopdf::StringFormat::Literal),
    );

    // Default appearance (fallback for viewers that don't use AP)
    let da = format!("/F1 {} Tf {} {} {} rg", style.font_size, r, g, b);
    annot.set(
        "DA",
        Object::String(da.into_bytes(), lopdf::StringFormat::Literal),
    );

    // Add the appearance stream - this is critical for reliable rendering!
    annot.set("AP", Object::Dictionary(ap_dict));

    let annot_id = doc.add_object(Object::Dictionary(annot));
    add_annotation_to_page(doc, page_id, annot_id)
}

/// Create PDF content stream for text appearance
fn create_text_appearance_content(
    text: &str,
    font_size: f64,
    r: f32,
    g: f32,
    b: f32,
    _font_name: &str,
) -> String {
    // Escape special characters in PDF string
    let escaped_text = escape_pdf_string(text);

    // Content stream that draws text
    // Position text with small offset from bottom-left of BBox
    format!(
        "BT\n\
         /F1 {} Tf\n\
         {} {} {} rg\n\
         2 4 Td\n\
         ({}) Tj\n\
         ET",
        font_size, r, g, b, escaped_text
    )
}

/// Escape special characters for PDF string literals
fn escape_pdf_string(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => result.push_str("\\\\"),
            '(' => result.push_str("\\("),
            ')' => result.push_str("\\)"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            _ => result.push(c),
        }
    }
    result
}

fn add_highlight_annotation(
    doc: &mut Document,
    page_id: ObjectId,
    rect: &PdfRect,
    _color: &str,
    opacity: f64,
) -> Result<(), PdfJoinError> {
    let mut annot = Dictionary::new();
    annot.set("Type", Object::Name(b"Annot".to_vec()));
    annot.set("Subtype", Object::Name(b"Highlight".to_vec()));
    annot.set(
        "Rect",
        Object::Array(vec![
            Object::Real(rect.x as f32),
            Object::Real(rect.y as f32),
            Object::Real((rect.x + rect.width) as f32),
            Object::Real((rect.y + rect.height) as f32),
        ]),
    );
    // QuadPoints for highlight
    annot.set(
        "QuadPoints",
        Object::Array(vec![
            Object::Real(rect.x as f32),
            Object::Real((rect.y + rect.height) as f32),
            Object::Real((rect.x + rect.width) as f32),
            Object::Real((rect.y + rect.height) as f32),
            Object::Real(rect.x as f32),
            Object::Real(rect.y as f32),
            Object::Real((rect.x + rect.width) as f32),
            Object::Real(rect.y as f32),
        ]),
    );
    annot.set("CA", Object::Real(opacity as f32));
    // Yellow color
    annot.set(
        "C",
        Object::Array(vec![
            Object::Real(1.0),
            Object::Real(1.0),
            Object::Real(0.0),
        ]),
    );

    let annot_id = doc.add_object(Object::Dictionary(annot));
    add_annotation_to_page(doc, page_id, annot_id)
}

/// Add an Underline annotation to the PDF
/// Uses the standard PDF Underline annotation type (distinct from Highlight)
fn add_underline_annotation(
    doc: &mut Document,
    page_id: ObjectId,
    rect: &PdfRect,
    color: &str,
) -> Result<(), PdfJoinError> {
    let (r, g, b) = parse_hex_color(color);

    let mut annot = Dictionary::new();
    annot.set("Type", Object::Name(b"Annot".to_vec()));
    annot.set("Subtype", Object::Name(b"Underline".to_vec()));
    annot.set(
        "Rect",
        Object::Array(vec![
            Object::Real(rect.x as f32),
            Object::Real(rect.y as f32),
            Object::Real((rect.x + rect.width) as f32),
            Object::Real((rect.y + rect.height) as f32),
        ]),
    );
    // QuadPoints for underline (required for text markup annotations)
    annot.set(
        "QuadPoints",
        Object::Array(vec![
            Object::Real(rect.x as f32),
            Object::Real((rect.y + rect.height) as f32),
            Object::Real((rect.x + rect.width) as f32),
            Object::Real((rect.y + rect.height) as f32),
            Object::Real(rect.x as f32),
            Object::Real(rect.y as f32),
            Object::Real((rect.x + rect.width) as f32),
            Object::Real(rect.y as f32),
        ]),
    );
    // Use the specified color (unlike highlight which hardcodes yellow)
    annot.set(
        "C",
        Object::Array(vec![Object::Real(r), Object::Real(g), Object::Real(b)]),
    );

    let annot_id = doc.add_object(Object::Dictionary(annot));
    add_annotation_to_page(doc, page_id, annot_id)
}

fn add_checkbox_annotation(
    doc: &mut Document,
    page_id: ObjectId,
    rect: &PdfRect,
    checked: bool,
) -> Result<(), PdfJoinError> {
    let mut annot = Dictionary::new();
    annot.set("Type", Object::Name(b"Annot".to_vec()));
    annot.set("Subtype", Object::Name(b"Square".to_vec()));
    annot.set(
        "Rect",
        Object::Array(vec![
            Object::Real(rect.x as f32),
            Object::Real(rect.y as f32),
            Object::Real((rect.x + rect.width) as f32),
            Object::Real((rect.y + rect.height) as f32),
        ]),
    );

    if checked {
        // Green interior for checked
        annot.set(
            "IC",
            Object::Array(vec![
                Object::Real(0.2),
                Object::Real(0.8),
                Object::Real(0.2),
            ]),
        );
    }
    // Black border
    annot.set(
        "C",
        Object::Array(vec![
            Object::Real(0.0),
            Object::Real(0.0),
            Object::Real(0.0),
        ]),
    );

    let annot_id = doc.add_object(Object::Dictionary(annot));
    add_annotation_to_page(doc, page_id, annot_id)
}

fn add_text_replacement(
    doc: &mut Document,
    page_id: ObjectId,
    original_rect: &PdfRect,
    replacement_rect: &PdfRect,
    new_text: &str,
    style: &TextStyle,
) -> Result<(), PdfJoinError> {
    // Liberal padding to ensure the white cover fully hides original text
    // Text rendering extends beyond reported bounds due to:
    // - Descenders (g, y, p, q, j) extend below baseline
    // - Ascenders and accents extend above cap height
    // - Kerning and letter spacing variations
    // - PDF viewer rendering differences
    // Use generous padding (15pt) to guarantee complete coverage
    const COVER_PADDING: f64 = 15.0;

    // 1. Add white Square annotation to cover original text with padding
    let padded_rect = PdfRect {
        x: original_rect.x - COVER_PADDING,
        y: original_rect.y - COVER_PADDING,
        width: original_rect.width + COVER_PADDING * 2.0,
        height: original_rect.height + COVER_PADDING * 2.0,
    };
    add_white_rect_annotation(doc, page_id, &padded_rect)?;

    // 2. Add FreeText annotation with replacement text
    add_text_annotation(doc, page_id, replacement_rect, new_text, style)
}

fn add_white_rect_annotation(
    doc: &mut Document,
    page_id: ObjectId,
    rect: &PdfRect,
) -> Result<(), PdfJoinError> {
    // Calculate rectangle bounds
    let x1 = rect.x as f32;
    let y1 = rect.y as f32;
    let x2 = (rect.x + rect.width) as f32;
    let y2 = (rect.y + rect.height) as f32;
    let width = x2 - x1;
    let height = y2 - y1;

    // Create appearance stream content that draws a filled white rectangle
    // This is critical for reliable rendering across all PDF viewers
    let ap_content = format!(
        "1 1 1 rg\n\
         0 0 {} {} re\n\
         f",
        width, height
    );

    // Create the appearance stream (Form XObject)
    let mut ap_stream_dict = Dictionary::new();
    ap_stream_dict.set("Type", Object::Name(b"XObject".to_vec()));
    ap_stream_dict.set("Subtype", Object::Name(b"Form".to_vec()));
    ap_stream_dict.set("FormType", Object::Integer(1));
    ap_stream_dict.set(
        "BBox",
        Object::Array(vec![
            Object::Real(0.0),
            Object::Real(0.0),
            Object::Real(width),
            Object::Real(height),
        ]),
    );

    let ap_stream = lopdf::Stream::new(ap_stream_dict, ap_content.into_bytes());
    let ap_stream_id = doc.add_object(Object::Stream(ap_stream));

    // Create AP dictionary pointing to the normal appearance
    let mut ap_dict = Dictionary::new();
    ap_dict.set("N", Object::Reference(ap_stream_id));

    // Create the annotation
    let mut annot = Dictionary::new();
    annot.set("Type", Object::Name(b"Annot".to_vec()));
    annot.set("Subtype", Object::Name(b"Square".to_vec()));
    annot.set(
        "Rect",
        Object::Array(vec![
            Object::Real(x1),
            Object::Real(y1),
            Object::Real(x2),
            Object::Real(y2),
        ]),
    );
    // White fill (IC = Interior Color) - fallback for viewers that don't use AP
    annot.set(
        "IC",
        Object::Array(vec![
            Object::Real(1.0),
            Object::Real(1.0),
            Object::Real(1.0),
        ]),
    );
    // White border (C = Color)
    annot.set(
        "C",
        Object::Array(vec![
            Object::Real(1.0),
            Object::Real(1.0),
            Object::Real(1.0),
        ]),
    );
    // No border width
    let mut bs = Dictionary::new();
    bs.set("W", Object::Integer(0));
    annot.set("BS", Object::Dictionary(bs));

    // Add the appearance stream - critical for reliable rendering!
    annot.set("AP", Object::Dictionary(ap_dict));

    let annot_id = doc.add_object(Object::Dictionary(annot));
    add_annotation_to_page(doc, page_id, annot_id)
}

fn add_annotation_to_page(
    doc: &mut Document,
    page_id: ObjectId,
    annot_id: ObjectId,
) -> Result<(), PdfJoinError> {
    let page = doc
        .get_object_mut(page_id)
        .map_err(|e| PdfJoinError::OperationError(e.to_string()))?;

    if let Object::Dictionary(ref mut page_dict) = page {
        if let Ok(Object::Array(ref mut arr)) = page_dict.get_mut(b"Annots") {
            arr.push(Object::Reference(annot_id));
        } else {
            page_dict.set("Annots", Object::Array(vec![Object::Reference(annot_id)]));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operations::{EditOperation, OperationLog, PdfRect, TextStyle};

    fn create_test_pdf() -> Vec<u8> {
        use lopdf::{dictionary, Document, Object};

        let mut doc = Document::with_version("1.7");
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
        });
        let pages_id = doc.add_object(dictionary! {
            "Type" => "Pages",
            "Kids" => vec![Object::Reference(page_id)],
            "Count" => 1,
        });
        if let Ok(page) = doc.get_object_mut(page_id) {
            if let Ok(dict) = page.as_dict_mut() {
                dict.set("Parent", Object::Reference(pages_id));
            }
        }
        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => Object::Reference(pages_id),
        });
        doc.trailer.set("Root", Object::Reference(catalog_id));

        let mut buffer = Vec::new();
        doc.save_to(&mut buffer).unwrap();
        buffer
    }

    #[test]
    fn test_empty_log_returns_original() {
        let pdf = create_test_pdf();
        let log = OperationLog::new();
        let result = apply_operations(&pdf, &log).unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn test_add_text_produces_valid_pdf() {
        let pdf = create_test_pdf();
        let mut log = OperationLog::new();
        log.add(EditOperation::AddText {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 100.0,
                y: 700.0,
                width: 200.0,
                height: 20.0,
            },
            text: "Hello World".to_string(),
            style: TextStyle::default(),
        });

        let result = apply_operations(&pdf, &log).unwrap();
        assert!(result.starts_with(b"%PDF-"));

        // Verify the result is valid by loading it
        let doc = Document::load_mem(&result).unwrap();
        assert_eq!(doc.get_pages().len(), 1);
    }

    #[test]
    fn test_add_highlight_produces_valid_pdf() {
        let pdf = create_test_pdf();
        let mut log = OperationLog::new();
        log.add(EditOperation::AddHighlight {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 50.0,
                y: 600.0,
                width: 300.0,
                height: 20.0,
            },
            color: "#FFFF00".to_string(),
            opacity: 0.5,
        });

        let result = apply_operations(&pdf, &log).unwrap();
        let doc = Document::load_mem(&result).unwrap();
        assert_eq!(doc.get_pages().len(), 1);
    }

    #[test]
    fn test_add_checkbox_produces_valid_pdf() {
        let pdf = create_test_pdf();
        let mut log = OperationLog::new();
        log.add(EditOperation::AddCheckbox {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 100.0,
                y: 500.0,
                width: 20.0,
                height: 20.0,
            },
            checked: true,
        });

        let result = apply_operations(&pdf, &log).unwrap();
        let doc = Document::load_mem(&result).unwrap();
        assert_eq!(doc.get_pages().len(), 1);
    }

    #[test]
    fn test_multiple_operations_on_same_page() {
        let pdf = create_test_pdf();
        let mut log = OperationLog::new();
        log.add(EditOperation::AddText {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 100.0,
                y: 700.0,
                width: 200.0,
                height: 20.0,
            },
            text: "Title".to_string(),
            style: TextStyle::default(),
        });
        log.add(EditOperation::AddHighlight {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 100.0,
                y: 650.0,
                width: 200.0,
                height: 15.0,
            },
            color: "#FFFF00".to_string(),
            opacity: 0.3,
        });
        log.add(EditOperation::AddCheckbox {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 100.0,
                y: 600.0,
                width: 20.0,
                height: 20.0,
            },
            checked: false,
        });

        let result = apply_operations(&pdf, &log).unwrap();
        let doc = Document::load_mem(&result).unwrap();
        assert_eq!(doc.get_pages().len(), 1);
    }

    #[test]
    fn test_replace_text_white_cover_is_larger_than_original() {
        // The white rectangle must be LARGER than the original text rect
        // to ensure full coverage (no text bleeding through)
        let pdf = create_test_pdf();
        let mut log = OperationLog::new();

        // Original text at position (100, 700) with size 200x24
        let original_rect = PdfRect {
            x: 100.0,
            y: 700.0,
            width: 200.0,
            height: 24.0,
        };

        log.add(EditOperation::ReplaceText {
            id: 0,
            page: 1,
            original_rect: original_rect.clone(),
            replacement_rect: original_rect.clone(),
            original_text: "Original Text".to_string(),
            new_text: "Replacement".to_string(),
            style: TextStyle::default(),
        });

        let result = apply_operations(&pdf, &log).unwrap();

        // Parse the output PDF text to verify white rectangle coordinates
        let output_str = String::from_utf8_lossy(&result);

        // Find the Square annotation (white cover)
        // It should be larger than the original rect to ensure coverage
        // The rect should include padding: at least 2-4 points on each side
        // Original: x=100, y=700, x2=300, y2=724
        // Expected with padding: x<100, y<700, x2>300, y2>724

        // Verify the PDF contains a Square annotation
        assert!(
            output_str.contains("/Square"),
            "Should contain Square annotation for white cover"
        );

        // Verify the white interior color (IC)
        assert!(
            output_str.contains("/IC"),
            "Square should have interior color (IC) for white fill"
        );

        // The rectangle bounds in the PDF
        // Look for Rect array that is LARGER than original bounds
        // Original would be: [100 700 300 724]
        // With padding should be something like: [98 698 302 728] (2pt padding)
        // or [96 696 304 732] (4pt padding)

        // For now, just verify the annotation exists and has white color
        // We'll check the actual bounds in a separate test
        let doc = Document::load_mem(&result).unwrap();
        assert_eq!(doc.get_pages().len(), 1);
    }

    #[test]
    fn test_whiteout_then_text_has_correct_annotation_order() {
        // CRITICAL: When whiteout is drawn first, then text added on top,
        // the FreeText annotation MUST appear AFTER the Square in /Annots
        // for proper z-ordering (text on top of white rect)
        let pdf = create_test_pdf();
        let mut log = OperationLog::new();

        // 1. First add white rect (drawn first by user)
        log.add(EditOperation::AddWhiteRect {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 100.0,
                y: 700.0,
                width: 150.0,
                height: 30.0,
            },
        });

        // 2. Then add text on top (typed by user after drawing whiteout)
        log.add(EditOperation::AddText {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 100.0,
                y: 700.0,
                width: 150.0,
                height: 30.0,
            },
            text: "Replacement Text".to_string(),
            style: TextStyle::default(),
        });

        let result = apply_operations(&pdf, &log).unwrap();
        let doc = Document::load_mem(&result).unwrap();

        // Get the page and its annotations
        let pages: Vec<_> = doc.get_pages().into_iter().collect();
        let (_page_num, page_id) = pages[0];

        let page = doc.get_object(page_id).unwrap();
        if let Object::Dictionary(page_dict) = page {
            if let Ok(Object::Array(annots)) = page_dict.get(b"Annots") {
                assert_eq!(
                    annots.len(),
                    2,
                    "Should have exactly 2 annotations (Square + FreeText)"
                );

                // Get subtypes in order
                let mut subtypes = Vec::new();
                for annot_ref in annots {
                    if let Object::Reference(annot_id) = annot_ref {
                        if let Ok(Object::Dictionary(annot)) = doc.get_object(*annot_id) {
                            if let Ok(Object::Name(subtype)) = annot.get(b"Subtype") {
                                subtypes.push(String::from_utf8_lossy(subtype).to_string());
                            }
                        }
                    }
                }

                // Square (white rect) must come FIRST, FreeText (text) must come SECOND
                // This ensures FreeText renders ON TOP of Square
                assert_eq!(
                    subtypes,
                    vec!["Square", "FreeText"],
                    "Annotation order must be [Square, FreeText] for proper z-ordering. \
                     Square (white rect) should be first (bottom), FreeText (text) second (top)"
                );
            } else {
                panic!("Page should have Annots array");
            }
        } else {
            panic!("Page should be a Dictionary");
        }
    }

    #[test]
    fn test_freetext_annotation_has_appearance_stream() {
        // FreeText annotations need an appearance stream (AP) for reliable
        // rendering across different PDF viewers
        let pdf = create_test_pdf();
        let mut log = OperationLog::new();

        log.add(EditOperation::AddText {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 100.0,
                y: 700.0,
                width: 200.0,
                height: 20.0,
            },
            text: "Test Text".to_string(),
            style: TextStyle::default(),
        });

        let result = apply_operations(&pdf, &log).unwrap();
        let doc = Document::load_mem(&result).unwrap();

        // Get the page and its annotations
        let pages: Vec<_> = doc.get_pages().into_iter().collect();
        let (_page_num, page_id) = pages[0];

        let page = doc.get_object(page_id).unwrap();
        if let Object::Dictionary(page_dict) = page {
            if let Ok(Object::Array(annots)) = page_dict.get(b"Annots") {
                // Find the FreeText annotation
                for annot_ref in annots {
                    if let Object::Reference(annot_id) = annot_ref {
                        if let Ok(Object::Dictionary(annot)) = doc.get_object(*annot_id) {
                            if let Ok(Object::Name(subtype)) = annot.get(b"Subtype") {
                                if subtype == b"FreeText" {
                                    // FreeText MUST have an appearance stream for reliable rendering
                                    assert!(
                                        annot.has(b"AP"),
                                        "FreeText annotation MUST have an appearance stream (AP) \
                                         for reliable text rendering across PDF viewers"
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_square_annotation_has_appearance_stream() {
        // CRITICAL: Square annotations (white rect) need an appearance stream (AP)
        // for reliable rendering. Without AP, the interior color (IC) may not
        // be rendered by all PDF viewers, causing the whiteout to be invisible.
        let pdf = create_test_pdf();
        let mut log = OperationLog::new();

        log.add(EditOperation::AddWhiteRect {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 100.0,
                y: 700.0,
                width: 200.0,
                height: 30.0,
            },
        });

        let result = apply_operations(&pdf, &log).unwrap();
        let doc = Document::load_mem(&result).unwrap();

        // Get the page and its annotations
        let pages: Vec<_> = doc.get_pages().into_iter().collect();
        let (_page_num, page_id) = pages[0];

        let page = doc.get_object(page_id).unwrap();
        if let Object::Dictionary(page_dict) = page {
            if let Ok(Object::Array(annots)) = page_dict.get(b"Annots") {
                // Find the Square annotation
                for annot_ref in annots {
                    if let Object::Reference(annot_id) = annot_ref {
                        if let Ok(Object::Dictionary(annot)) = doc.get_object(*annot_id) {
                            if let Ok(Object::Name(subtype)) = annot.get(b"Subtype") {
                                if subtype == b"Square" {
                                    // Square MUST have an appearance stream for reliable rendering
                                    assert!(
                                        annot.has(b"AP"),
                                        "Square annotation MUST have an appearance stream (AP) \
                                         for reliable white rectangle rendering across PDF viewers"
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_replace_text_white_cover_has_padding() {
        // REGRESSION TEST: White cover must have padding to prevent text bleed-through
        let pdf = create_test_pdf();
        let mut log = OperationLog::new();

        let original_rect = PdfRect {
            x: 100.0,
            y: 700.0,
            width: 200.0,
            height: 24.0,
        };

        log.add(EditOperation::ReplaceText {
            id: 0,
            page: 1,
            original_rect: original_rect.clone(),
            replacement_rect: original_rect.clone(),
            original_text: "Original".to_string(),
            new_text: "New".to_string(),
            style: TextStyle::default(),
        });

        let result = apply_operations(&pdf, &log).unwrap();
        let doc = Document::load_mem(&result).unwrap();

        // Get the page and its annotations
        let pages: Vec<_> = doc.get_pages().into_iter().collect();
        let (_page_num, page_id) = pages[0];

        let page = doc.get_object(page_id).unwrap();
        if let Object::Dictionary(page_dict) = page {
            if let Ok(Object::Array(annots)) = page_dict.get(b"Annots") {
                // Find the Square annotation (white cover)
                let mut found_square = false;
                let mut cover_rect: Option<(f32, f32, f32, f32)> = None;

                for annot_ref in annots {
                    if let Object::Reference(annot_id) = annot_ref {
                        if let Ok(Object::Dictionary(annot)) = doc.get_object(*annot_id) {
                            if let Ok(Object::Name(subtype)) = annot.get(b"Subtype") {
                                if subtype == b"Square" {
                                    found_square = true;
                                    if let Ok(Object::Array(rect)) = annot.get(b"Rect") {
                                        let x1 = match &rect[0] {
                                            Object::Real(v) => *v,
                                            Object::Integer(v) => *v as f32,
                                            _ => 0.0,
                                        };
                                        let y1 = match &rect[1] {
                                            Object::Real(v) => *v,
                                            Object::Integer(v) => *v as f32,
                                            _ => 0.0,
                                        };
                                        let x2 = match &rect[2] {
                                            Object::Real(v) => *v,
                                            Object::Integer(v) => *v as f32,
                                            _ => 0.0,
                                        };
                                        let y2 = match &rect[3] {
                                            Object::Real(v) => *v,
                                            Object::Integer(v) => *v as f32,
                                            _ => 0.0,
                                        };
                                        cover_rect = Some((x1, y1, x2, y2));
                                    }
                                }
                            }
                        }
                    }
                }

                assert!(
                    found_square,
                    "Should have Square annotation for white cover"
                );

                if let Some((x1, y1, x2, y2)) = cover_rect {
                    // Verify the cover rect has GENEROUS padding (at least 10pt on each side)
                    // Original: x=100, y=700, x2=300 (100+200), y2=724 (700+24)
                    let orig_x1 = 100.0_f32;
                    let orig_y1 = 700.0_f32;
                    let orig_x2 = 300.0_f32; // 100 + 200
                    let orig_y2 = 724.0_f32; // 700 + 24

                    // Minimum required padding to prevent text bleed-through
                    const MIN_PADDING: f32 = 10.0;

                    // Cover should extend WELL BEYOND original bounds
                    let left_padding = orig_x1 - x1;
                    let bottom_padding = orig_y1 - y1;
                    let right_padding = x2 - orig_x2;
                    let top_padding = y2 - orig_y2;

                    assert!(
                        left_padding >= MIN_PADDING,
                        "INSUFFICIENT PADDING: Cover left padding ({}) must be >= {}pt to prevent bleed-through",
                        left_padding,
                        MIN_PADDING
                    );
                    assert!(
                        bottom_padding >= MIN_PADDING,
                        "INSUFFICIENT PADDING: Cover bottom padding ({}) must be >= {}pt to prevent bleed-through",
                        bottom_padding,
                        MIN_PADDING
                    );
                    assert!(
                        right_padding >= MIN_PADDING,
                        "INSUFFICIENT PADDING: Cover right padding ({}) must be >= {}pt to prevent bleed-through",
                        right_padding,
                        MIN_PADDING
                    );
                    assert!(
                        top_padding >= MIN_PADDING,
                        "INSUFFICIENT PADDING: Cover top padding ({}) must be >= {}pt to prevent bleed-through",
                        top_padding,
                        MIN_PADDING
                    );
                }
            }
        }
    }

    #[test]
    fn test_bold_text_annotation_uses_bold_font() {
        // REGRESSION TEST: Bold text must use bold font in appearance stream
        // When is_bold=true, the font should be e.g., "Helvetica-Bold" not "Helvetica"
        let pdf = create_test_pdf();
        let mut log = OperationLog::new();

        log.add(EditOperation::AddText {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 100.0,
                y: 700.0,
                width: 200.0,
                height: 20.0,
            },
            text: "Bold Text".to_string(),
            style: TextStyle {
                font_size: 14.0,
                color: "#000000".to_string(),
                font_name: Some("sans-serif".to_string()),
                is_italic: false,
                is_bold: true,
            },
        });

        let result = apply_operations(&pdf, &log).unwrap();

        // The appearance stream should contain "Helvetica-Bold" as the font
        let output_str = String::from_utf8_lossy(&result);
        assert!(
            output_str.contains("Helvetica-Bold"),
            "Bold text annotation must use Helvetica-Bold font, not regular Helvetica"
        );
    }

    #[test]
    fn test_italic_text_annotation_uses_italic_font() {
        // REGRESSION TEST: Italic text must use italic font in appearance stream
        let pdf = create_test_pdf();
        let mut log = OperationLog::new();

        log.add(EditOperation::AddText {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 100.0,
                y: 700.0,
                width: 200.0,
                height: 20.0,
            },
            text: "Italic Text".to_string(),
            style: TextStyle {
                font_size: 14.0,
                color: "#000000".to_string(),
                font_name: Some("sans-serif".to_string()),
                is_italic: true,
                is_bold: false,
            },
        });

        let result = apply_operations(&pdf, &log).unwrap();

        // The appearance stream should contain "Helvetica-Oblique" as the font
        let output_str = String::from_utf8_lossy(&result);
        assert!(
            output_str.contains("Helvetica-Oblique"),
            "Italic text annotation must use Helvetica-Oblique font, not regular Helvetica"
        );
    }

    #[test]
    fn test_bold_italic_text_annotation_uses_bold_italic_font() {
        // REGRESSION TEST: Bold+Italic text must use bold-italic font
        let pdf = create_test_pdf();
        let mut log = OperationLog::new();

        log.add(EditOperation::AddText {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 100.0,
                y: 700.0,
                width: 200.0,
                height: 20.0,
            },
            text: "Bold Italic Text".to_string(),
            style: TextStyle {
                font_size: 14.0,
                color: "#000000".to_string(),
                font_name: Some("serif".to_string()),
                is_italic: true,
                is_bold: true,
            },
        });

        let result = apply_operations(&pdf, &log).unwrap();

        // The appearance stream should contain "Times-BoldItalic" as the font
        let output_str = String::from_utf8_lossy(&result);
        assert!(
            output_str.contains("Times-BoldItalic"),
            "Bold+Italic text annotation must use Times-BoldItalic font"
        );
    }

    #[test]
    fn test_text_annotation_bbox_fits_text_content() {
        // REGRESSION TEST: The appearance stream BBox must be wide enough
        // to fit the text content, otherwise text will be clipped/truncated.
        // This was happening when the frontend passed a small fixed width (200)
        // regardless of actual text length.
        let pdf = create_test_pdf();
        let mut log = OperationLog::new();

        // Long text that won't fit in 200 points at 12pt font
        // At ~6 points per character (for Helvetica 12pt), this needs ~330 points
        let long_text = "REAL ESTATE CONTRACT AGREEMENT";

        log.add(EditOperation::AddText {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 100.0,
                y: 700.0,
                width: 200.0, // Intentionally too small!
                height: 20.0,
            },
            text: long_text.to_string(),
            style: TextStyle {
                font_size: 12.0,
                color: "#000000".to_string(),
                font_name: None,
                is_italic: false,
                is_bold: false,
            },
        });

        let result = apply_operations(&pdf, &log).unwrap();
        let doc = Document::load_mem(&result).unwrap();

        // Get the page and its annotations
        let pages: Vec<_> = doc.get_pages().into_iter().collect();
        let (_page_num, page_id) = pages[0];

        let page = doc.get_object(page_id).unwrap();
        if let Object::Dictionary(page_dict) = page {
            if let Ok(Object::Array(annots)) = page_dict.get(b"Annots") {
                // Find the FreeText annotation
                for annot_ref in annots {
                    if let Object::Reference(annot_id) = annot_ref {
                        if let Ok(Object::Dictionary(annot)) = doc.get_object(*annot_id) {
                            if let Ok(Object::Name(subtype)) = annot.get(b"Subtype") {
                                if subtype == b"FreeText" {
                                    // Get the appearance stream
                                    if let Ok(Object::Dictionary(ap)) = annot.get(b"AP") {
                                        if let Ok(Object::Reference(ap_stream_id)) = ap.get(b"N") {
                                            if let Ok(Object::Stream(stream)) =
                                                doc.get_object(*ap_stream_id)
                                            {
                                                // Get BBox from appearance stream
                                                if let Ok(Object::Array(bbox)) =
                                                    stream.dict.get(b"BBox")
                                                {
                                                    let bbox_width = match &bbox[2] {
                                                        Object::Real(v) => *v,
                                                        Object::Integer(v) => *v as f32,
                                                        _ => 0.0,
                                                    };

                                                    // Calculate minimum required width for text
                                                    // Helvetica average char width ≈ 0.5 * font_size
                                                    let font_size = 12.0_f32;
                                                    let char_count = long_text.len() as f32;
                                                    let min_width =
                                                        char_count * font_size * 0.55 + 10.0; // ~0.55 * font_size per char + padding

                                                    assert!(
                                                        bbox_width >= min_width,
                                                        "Appearance stream BBox width ({}) must be at least {} to fit '{}' ({} chars at {}pt). \
                                                         Text will be truncated/clipped otherwise!",
                                                        bbox_width, min_width, long_text, char_count, font_size
                                                    );
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_text_annotation_bbox_fits_large_font() {
        // REGRESSION TEST: Same as above but with larger font (36pt)
        // This matches the real-world case where whiteout text uses detected font size
        let pdf = create_test_pdf();
        let mut log = OperationLog::new();

        // Text that needs more width at 36pt
        let long_text = "TEST TEST TEST TEST TEST";

        log.add(EditOperation::AddText {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 100.0,
                y: 700.0,
                width: 400.0, // Smaller than needed for 36pt text
                height: 50.0,
            },
            text: long_text.to_string(),
            style: TextStyle {
                font_size: 36.0, // Large font!
                color: "#000000".to_string(),
                font_name: None,
                is_italic: false,
                is_bold: false,
            },
        });

        let result = apply_operations(&pdf, &log).unwrap();
        let doc = Document::load_mem(&result).unwrap();

        // Get the page and its annotations
        let pages: Vec<_> = doc.get_pages().into_iter().collect();
        let (_page_num, page_id) = pages[0];

        let page = doc.get_object(page_id).unwrap();
        if let Object::Dictionary(page_dict) = page {
            if let Ok(Object::Array(annots)) = page_dict.get(b"Annots") {
                for annot_ref in annots {
                    if let Object::Reference(annot_id) = annot_ref {
                        if let Ok(Object::Dictionary(annot)) = doc.get_object(*annot_id) {
                            if let Ok(Object::Name(subtype)) = annot.get(b"Subtype") {
                                if subtype == b"FreeText" {
                                    if let Ok(Object::Dictionary(ap)) = annot.get(b"AP") {
                                        if let Ok(Object::Reference(ap_stream_id)) = ap.get(b"N") {
                                            if let Ok(Object::Stream(stream)) =
                                                doc.get_object(*ap_stream_id)
                                            {
                                                if let Ok(Object::Array(bbox)) =
                                                    stream.dict.get(b"BBox")
                                                {
                                                    let bbox_width = match &bbox[2] {
                                                        Object::Real(v) => *v,
                                                        Object::Integer(v) => *v as f32,
                                                        _ => 0.0,
                                                    };

                                                    // For 24 chars at 36pt: 24 * 36 * 0.55 + 10 = 485.2
                                                    let font_size = 36.0_f32;
                                                    let char_count = long_text.len() as f32;
                                                    let min_width =
                                                        char_count * font_size * 0.55 + 10.0;

                                                    assert!(
                                                        bbox_width >= min_width,
                                                        "BBox width ({}) must be >= {} for '{}' ({} chars at {}pt)",
                                                        bbox_width, min_width, long_text, char_count, font_size
                                                    );
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_flattened_output_has_no_annotations() {
        let pdf = create_test_pdf();
        let mut log = OperationLog::new();
        log.add(EditOperation::AddText {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 100.0,
                y: 700.0,
                width: 200.0,
                height: 20.0,
            },
            text: "Flattened Text".to_string(),
            style: TextStyle::default(),
        });

        let result = apply_operations_flattened(&pdf, &log).unwrap();

        // Load the result and check for annotations
        let doc = Document::load_mem(&result).unwrap();
        let pages: Vec<_> = doc.get_pages().into_iter().collect();
        let (_, page_id) = pages[0];

        // Get the page dictionary
        if let Ok(Object::Dictionary(page_dict)) = doc.get_object(page_id) {
            // Check that there are no annotations, or Annots is empty
            if let Ok(annots) = page_dict.get(b"Annots") {
                match annots {
                    Object::Array(arr) => {
                        assert!(arr.is_empty(), "Flattened PDF should have no annotations");
                    }
                    Object::Reference(annots_ref) => {
                        if let Ok(Object::Array(arr)) = doc.get_object(*annots_ref) {
                            assert!(arr.is_empty(), "Flattened PDF should have no annotations");
                        }
                    }
                    _ => {}
                }
            }
            // If no Annots key at all, that's also correct
        }
    }

    #[test]
    fn test_flattened_text_is_in_content_stream() {
        let pdf = create_test_pdf();
        let mut log = OperationLog::new();
        log.add(EditOperation::AddText {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 100.0,
                y: 700.0,
                width: 200.0,
                height: 20.0,
            },
            text: "FLATTEN_TEST_STRING".to_string(),
            style: TextStyle::default(),
        });

        let result = apply_operations_flattened(&pdf, &log).unwrap();

        // The text should appear somewhere in the PDF content
        // (either in content stream or as a visible string)
        let result_str = String::from_utf8_lossy(&result);
        assert!(
            result_str.contains("FLATTEN_TEST_STRING"),
            "Flattened text should appear in PDF content"
        );
    }

    #[test]
    fn test_flattened_whiteout_produces_white_color() {
        let pdf = create_test_pdf();
        let mut log = OperationLog::new();
        log.add(EditOperation::AddWhiteRect {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 100.0,
                y: 700.0,
                width: 200.0,
                height: 50.0,
            },
        });

        let result = apply_operations_flattened(&pdf, &log).unwrap();

        // The white color (1 1 1 rg) should appear in the PDF
        let result_str = String::from_utf8_lossy(&result);
        assert!(
            result_str.contains("1 1 1 rg"),
            "Flattened whiteout should have white fill color (1 1 1 rg). PDF content: {}",
            &result_str[..std::cmp::min(500, result_str.len())]
        );
    }

    /// Test that mimics the browser test PDF structure
    #[test]
    fn test_flattened_with_multipage_pdf() {
        // Create a 2-page PDF similar to test_pdf_base64
        use lopdf::content::{Content, Operation};
        use lopdf::{dictionary, Dictionary, Document, Object, Stream};

        let mut doc = Document::with_version("1.7");
        let pages_id = doc.new_object_id();
        let mut page_ids = Vec::new();

        for i in 0..2 {
            let content = Content {
                operations: vec![
                    Operation::new("BT", vec![]),
                    Operation::new(
                        "Tf",
                        vec![Object::Name(b"F1".to_vec()), Object::Integer(12)],
                    ),
                    Operation::new("Td", vec![Object::Integer(100), Object::Integer(700)]),
                    Operation::new(
                        "Tj",
                        vec![Object::String(
                            format!("Page {}", i + 1).into_bytes(),
                            lopdf::StringFormat::Literal,
                        )],
                    ),
                    Operation::new("ET", vec![]),
                ],
            };
            let content_id =
                doc.add_object(Stream::new(Dictionary::new(), content.encode().unwrap()));

            let page = dictionary! {
                "Type" => "Page",
                "Parent" => Object::Reference(pages_id),
                "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
                "Contents" => Object::Reference(content_id),
            };
            let page_id = doc.add_object(page);
            page_ids.push(page_id);
        }

        let pages = dictionary! {
            "Type" => "Pages",
            "Count" => 2,
            "Kids" => page_ids.iter().map(|id| Object::Reference(*id)).collect::<Vec<_>>(),
        };
        doc.objects.insert(pages_id, Object::Dictionary(pages));

        let catalog = dictionary! {
            "Type" => "Catalog",
            "Pages" => Object::Reference(pages_id),
        };
        let catalog_id = doc.add_object(catalog);
        doc.trailer.set("Root", Object::Reference(catalog_id));

        let mut pdf_bytes = Vec::new();
        doc.save_to(&mut pdf_bytes).unwrap();

        // Now add a whiteout on page 1
        let mut log = OperationLog::new();
        log.add(EditOperation::AddWhiteRect {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 66.67,
                y: 687.05,
                width: 134.68,
                height: 38.53,
            },
        });

        eprintln!("Original PDF size: {} bytes", pdf_bytes.len());
        eprintln!("Operations: {:?}", log.operations());
        eprintln!("Is empty: {}", log.is_empty());

        let result = apply_operations_flattened(&pdf_bytes, &log).unwrap();

        eprintln!("Result PDF size: {} bytes", result.len());

        let result_str = String::from_utf8_lossy(&result);
        assert!(
            result_str.contains("1 1 1 rg"),
            "Flattened whiteout should have white fill color (1 1 1 rg)"
        );
    }

    // ============ Underline Annotation Tests (Bug 5 fix) ============

    #[test]
    fn test_add_underline_produces_underline_annotation_not_highlight() {
        // Bug 5: Underlines were being saved as Highlight annotations instead of Underline
        let pdf = create_test_pdf();
        let mut log = OperationLog::new();
        log.add(EditOperation::AddUnderline {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 100.0,
                y: 500.0,
                width: 200.0,
                height: 2.0,
            },
            color: "#000000".to_string(),
        });

        let result = apply_operations(&pdf, &log).unwrap();
        let doc = Document::load_mem(&result).unwrap();

        // Get the page and its annotations
        let pages: Vec<_> = doc.get_pages().into_iter().collect();
        let (_page_num, page_id) = pages[0];

        let page = doc.get_object(page_id).unwrap();
        if let Object::Dictionary(page_dict) = page {
            if let Ok(Object::Array(annots)) = page_dict.get(b"Annots") {
                assert_eq!(annots.len(), 1, "Should have exactly 1 annotation");

                // Find the annotation and check its subtype
                if let Object::Reference(annot_id) = &annots[0] {
                    if let Ok(Object::Dictionary(annot)) = doc.get_object(*annot_id) {
                        if let Ok(Object::Name(subtype)) = annot.get(b"Subtype") {
                            let subtype_str = String::from_utf8_lossy(subtype);
                            assert_eq!(
                                subtype_str, "Underline",
                                "Underline operations must create Underline annotations, not {} annotations",
                                subtype_str
                            );
                        } else {
                            panic!("Annotation missing Subtype");
                        }
                    }
                }
            } else {
                panic!("Page should have Annots array");
            }
        }
    }

    #[test]
    fn test_underline_annotation_uses_correct_color() {
        // Verify the underline uses the specified color, not hardcoded yellow like highlight
        let pdf = create_test_pdf();
        let mut log = OperationLog::new();
        log.add(EditOperation::AddUnderline {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 100.0,
                y: 500.0,
                width: 200.0,
                height: 2.0,
            },
            color: "#FF0000".to_string(), // Red underline
        });

        let result = apply_operations(&pdf, &log).unwrap();
        let doc = Document::load_mem(&result).unwrap();

        let pages: Vec<_> = doc.get_pages().into_iter().collect();
        let (_page_num, page_id) = pages[0];

        let page = doc.get_object(page_id).unwrap();
        if let Object::Dictionary(page_dict) = page {
            if let Ok(Object::Array(annots)) = page_dict.get(b"Annots") {
                if let Object::Reference(annot_id) = &annots[0] {
                    if let Ok(Object::Dictionary(annot)) = doc.get_object(*annot_id) {
                        // Check color (C key)
                        if let Ok(Object::Array(color)) = annot.get(b"C") {
                            let r = match &color[0] {
                                Object::Real(v) => *v,
                                Object::Integer(v) => *v as f32,
                                _ => 0.0,
                            };
                            // Red = 1.0, not yellow (1.0, 1.0, 0.0)
                            assert!(
                                r > 0.9,
                                "Underline color should be red (specified), not yellow (highlight default)"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn test_flattened_underline_draws_thin_line_not_filled_rect() {
        // In flattened mode, underline should draw a thin line, not a filled rectangle
        let pdf = create_test_pdf();
        let mut log = OperationLog::new();
        log.add(EditOperation::AddUnderline {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 100.0,
                y: 500.0,
                width: 200.0,
                height: 2.0,
            },
            color: "#000000".to_string(),
        });

        let result = apply_operations_flattened(&pdf, &log).unwrap();
        let result_str = String::from_utf8_lossy(&result);

        // Underline should use stroke (S) not fill (f)
        // It should draw a line from x to x+width at the baseline
        assert!(
            result_str.contains(" l S") || result_str.contains(" l\nS"),
            "Flattened underline should draw a stroked line, not a filled rectangle. Content: {}",
            &result_str[..std::cmp::min(1000, result_str.len())]
        );
    }

    #[test]
    fn test_get_pages_returns_correct_page_numbers() {
        // Create a PDF like the browser test does
        use lopdf::content::{Content, Operation};
        use lopdf::{dictionary, Dictionary, Document, Object, Stream};

        let mut doc = Document::with_version("1.7");
        let pages_id = doc.new_object_id();
        let mut page_ids = Vec::new();

        for i in 0..2 {
            let content = Content {
                operations: vec![
                    Operation::new("BT", vec![]),
                    Operation::new(
                        "Tf",
                        vec![Object::Name(b"F1".to_vec()), Object::Integer(12)],
                    ),
                    Operation::new("Td", vec![Object::Integer(100), Object::Integer(700)]),
                    Operation::new(
                        "Tj",
                        vec![Object::String(
                            format!("Page {}", i + 1).into_bytes(),
                            lopdf::StringFormat::Literal,
                        )],
                    ),
                    Operation::new("ET", vec![]),
                ],
            };
            let content_id =
                doc.add_object(Stream::new(Dictionary::new(), content.encode().unwrap()));

            let page = dictionary! {
                "Type" => "Page",
                "Parent" => Object::Reference(pages_id),
                "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
                "Contents" => Object::Reference(content_id),
            };
            let page_id = doc.add_object(page);
            page_ids.push(page_id);
        }

        let pages = dictionary! {
            "Type" => "Pages",
            "Count" => 2,
            "Kids" => page_ids.iter().map(|id| Object::Reference(*id)).collect::<Vec<_>>(),
        };
        doc.objects.insert(pages_id, Object::Dictionary(pages));

        let catalog = dictionary! {
            "Type" => "Catalog",
            "Pages" => Object::Reference(pages_id),
        };
        let catalog_id = doc.add_object(catalog);
        doc.trailer.set("Root", Object::Reference(catalog_id));

        // Save and reload
        let mut pdf_bytes = Vec::new();
        doc.save_to(&mut pdf_bytes).unwrap();

        let reloaded = Document::load_mem(&pdf_bytes).unwrap();
        let pages = reloaded.get_pages();

        eprintln!("Pages map: {:?}", pages);

        // Should have pages 1 and 2
        assert!(pages.contains_key(&1), "Should have page 1");
        assert!(pages.contains_key(&2), "Should have page 2");
        assert_eq!(pages.len(), 2, "Should have exactly 2 pages");
    }
}
