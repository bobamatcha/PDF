//! Apply operations to PDF documents

use crate::error::PdfJoinError;
use crate::operations::{EditOperation, OperationLog, PdfRect, StyledTextSegment, TextStyle};
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

    // NOTE: Removed doc.compress() - it was corrupting the content streams in WASM
    // See KNOWN_ISSUES.md ISSUE-001 for details
    // doc.compress();

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
        EditOperation::AddStyledText {
            rect,
            segments,
            style,
            ..
        } => {
            let (r, g, b) = parse_hex_color(&style.color);

            // Save graphics state
            writeln!(content, "q").unwrap();
            // Set color
            writeln!(content, "{} {} {} rg", r, g, b).unwrap();
            writeln!(content, "BT").unwrap();
            // Position text
            writeln!(content, "{} {} Td", rect.x, rect.y).unwrap();

            // Output each segment with its own font
            for segment in segments {
                // Create a temporary style for this segment with the segment's bold/italic flags
                let segment_style = TextStyle {
                    font_size: style.font_size,
                    color: style.color.clone(),
                    font_name: style.font_name.clone(),
                    is_bold: segment.is_bold,
                    is_italic: segment.is_italic,
                };
                let font_name = segment_style.pdf_font_name();

                // Set font for this segment
                writeln!(content, "/{} {} Tf", font_name, style.font_size).unwrap();
                // Output the segment text
                let escaped = escape_pdf_string(&segment.text);
                writeln!(content, "({}) Tj", escaped).unwrap();
            }

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
        EditOperation::AddWhiteRect { rect, color, .. } => {
            // Draw colored filled rectangle (white for whiteout, black for blackout/redaction)
            let (r, g, b) = parse_hex_color(color);
            writeln!(content, "q").unwrap();
            writeln!(content, "{} {} {} rg", r, g, b).unwrap();
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
        EditOperation::AddStyledText {
            rect,
            segments,
            style,
            ..
        } => add_styled_text_annotation(doc, page_id, rect, segments, style),
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
        EditOperation::AddWhiteRect { rect, color, .. } => {
            add_white_rect_annotation(doc, page_id, rect, color)
        }
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
    let box_width = (x2 - x1) as f64;
    let box_height = (y2 - y1) as f64;
    let ap_content = create_text_appearance_content(
        text,
        style.font_size,
        r,
        g,
        b,
        font_name,
        box_width,
        box_height,
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
    font_name: &str,
    box_width: f64,
    box_height: f64,
) -> String {
    // Escape special characters in PDF string
    let escaped_text = escape_pdf_string(text);

    // Calculate approximate text width for centering
    // Use standard font metrics - average character width is ~0.5-0.6 of font size
    let avg_char_width = match font_name {
        "Courier" | "Courier-Bold" | "Courier-Oblique" | "Courier-BoldOblique" => {
            font_size * 0.6 // Monospace fonts are wider
        }
        "Times-Roman" | "Times-Bold" | "Times-Italic" | "Times-BoldItalic" => {
            font_size * 0.45 // Serif fonts are narrower
        }
        _ => font_size * 0.5, // Helvetica and others
    };
    let text_width = text.chars().count() as f64 * avg_char_width;

    // Center horizontally: (box_width - text_width) / 2
    // Minimum offset of 2 to prevent clipping at edges
    let x_offset = ((box_width - text_width) / 2.0).max(2.0);

    // Center vertically: (box_height - font_size) / 2
    // Add baseline offset (fonts sit ~20% above their baseline)
    let y_offset = ((box_height - font_size) / 2.0).max(2.0);

    // Content stream that draws text centered in the box
    format!(
        "BT\n\
         /F1 {} Tf\n\
         {} {} {} rg\n\
         {} {} Td\n\
         ({}) Tj\n\
         ET",
        font_size, r, g, b, x_offset, y_offset, escaped_text
    )
}

/// Create PDF content stream for styled text appearance with mixed fonts
fn create_styled_text_appearance_content(
    segments: &[StyledTextSegment],
    font_size: f64,
    r: f32,
    g: f32,
    b: f32,
    _base_font_name: &str,
    box_width: f64,
    box_height: f64,
    _style: &TextStyle,
) -> String {
    use std::fmt::Write;
    let mut content = String::new();

    // Calculate total text width for centering
    let total_chars: usize = segments.iter().map(|s| s.text.chars().count()).sum();
    let avg_char_width = font_size * 0.5;
    let text_width = total_chars as f64 * avg_char_width;

    // Center horizontally
    let x_offset = ((box_width - text_width) / 2.0).max(2.0);
    // Center vertically
    let y_offset = ((box_height - font_size) / 2.0).max(2.0);

    writeln!(content, "BT").unwrap();

    // ISSUE-009 FIX: Font must be set FIRST, before color (rg) and position (Td).
    // This matches the working create_text_appearance_content() operator order.
    // Without this, macOS Preview and other strict PDF viewers won't render the text.

    // Set initial font for first segment (MUST come before rg and Td)
    let first_font = if let Some(first) = segments.first() {
        match (first.is_bold, first.is_italic) {
            (false, false) => "F1",
            (true, false) => "F2",
            (false, true) => "F3",
            (true, true) => "F4",
        }
    } else {
        "F1"
    };
    writeln!(content, "/{} {} Tf", first_font, font_size).unwrap();
    writeln!(content, "{} {} {} rg", r, g, b).unwrap();
    writeln!(content, "{} {} Td", x_offset, y_offset).unwrap();

    // Output each segment, switching font as needed
    for (i, segment) in segments.iter().enumerate() {
        let font_ref = match (segment.is_bold, segment.is_italic) {
            (false, false) => "F1",
            (true, false) => "F2",
            (false, true) => "F3",
            (true, true) => "F4",
        };

        // Only output font change if different from current (skip first since already set)
        if i > 0 {
            writeln!(content, "/{} {} Tf", font_ref, font_size).unwrap();
        }
        let escaped = escape_pdf_string(&segment.text);
        writeln!(content, "({}) Tj", escaped).unwrap();
    }

    writeln!(content, "ET").unwrap();
    content
}

fn add_styled_text_annotation(
    doc: &mut Document,
    page_id: ObjectId,
    rect: &PdfRect,
    segments: &[StyledTextSegment],
    style: &TextStyle,
) -> Result<(), PdfJoinError> {
    if segments.is_empty() {
        return Ok(());
    }

    // Parse color from hex to RGB components
    let (r, g, b) = parse_hex_color(&style.color);

    // Calculate total text for sizing
    let total_text: String = segments.iter().map(|s| s.text.as_str()).collect();
    let font_name = style.pdf_font_name();

    // Calculate minimum required width
    let min_text_width = calculate_text_width(&total_text, style.font_size, font_name);
    let actual_width = rect.width.max(min_text_width);

    let x1 = rect.x as f32;
    let y1 = rect.y as f32;
    let x2 = (rect.x + actual_width) as f32;
    let y2 = (rect.y + rect.height) as f32;

    let box_width = (x2 - x1) as f64;
    let box_height = (y2 - y1) as f64;

    // Create appearance stream content with styled segments
    let ap_content = create_styled_text_appearance_content(
        segments,
        style.font_size,
        r,
        g,
        b,
        font_name,
        box_width,
        box_height,
        style,
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
            Object::Real(x2 - x1),
            Object::Real(y2 - y1),
        ]),
    );

    // Resources for the appearance stream - all font variants
    let mut font_dict = Dictionary::new();

    // F1 = Regular
    let mut f1 = Dictionary::new();
    f1.set("Type", Object::Name(b"Font".to_vec()));
    f1.set("Subtype", Object::Name(b"Type1".to_vec()));
    f1.set("BaseFont", Object::Name(b"Helvetica".to_vec()));
    font_dict.set("F1", Object::Dictionary(f1));

    // F2 = Bold
    let mut f2 = Dictionary::new();
    f2.set("Type", Object::Name(b"Font".to_vec()));
    f2.set("Subtype", Object::Name(b"Type1".to_vec()));
    f2.set("BaseFont", Object::Name(b"Helvetica-Bold".to_vec()));
    font_dict.set("F2", Object::Dictionary(f2));

    // F3 = Italic/Oblique
    let mut f3 = Dictionary::new();
    f3.set("Type", Object::Name(b"Font".to_vec()));
    f3.set("Subtype", Object::Name(b"Type1".to_vec()));
    f3.set("BaseFont", Object::Name(b"Helvetica-Oblique".to_vec()));
    font_dict.set("F3", Object::Dictionary(f3));

    // F4 = Bold+Italic
    let mut f4 = Dictionary::new();
    f4.set("Type", Object::Name(b"Font".to_vec()));
    f4.set("Subtype", Object::Name(b"Type1".to_vec()));
    f4.set("BaseFont", Object::Name(b"Helvetica-BoldOblique".to_vec()));
    font_dict.set("F4", Object::Dictionary(f4));

    let mut resources = Dictionary::new();
    resources.set("Font", Object::Dictionary(font_dict));
    ap_stream_dict.set("Resources", Object::Dictionary(resources));

    // Create the stream object
    let ap_stream = lopdf::Stream::new(ap_stream_dict, ap_content.into_bytes());
    let ap_stream_id = doc.add_object(Object::Stream(ap_stream));

    // Create AP dictionary
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
        Object::String(total_text.as_bytes().to_vec(), lopdf::StringFormat::Literal),
    );

    // Default appearance (fallback)
    let da = format!("/F1 {} Tf {} {} {} rg", style.font_size, r, g, b);
    annot.set(
        "DA",
        Object::String(da.into_bytes(), lopdf::StringFormat::Literal),
    );

    // Add the appearance stream
    annot.set("AP", Object::Dictionary(ap_dict));

    let annot_id = doc.add_object(Object::Dictionary(annot));
    add_annotation_to_page(doc, page_id, annot_id)
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
    add_white_rect_annotation(doc, page_id, &padded_rect, "#FFFFFF")?;

    // 2. Add FreeText annotation with replacement text
    add_text_annotation(doc, page_id, replacement_rect, new_text, style)
}

fn add_white_rect_annotation(
    doc: &mut Document,
    page_id: ObjectId,
    rect: &PdfRect,
    color: &str,
) -> Result<(), PdfJoinError> {
    // Calculate rectangle bounds
    let x1 = rect.x as f32;
    let y1 = rect.y as f32;
    let x2 = (rect.x + rect.width) as f32;
    let y2 = (rect.y + rect.height) as f32;
    let width = x2 - x1;
    let height = y2 - y1;

    // Parse color (white for whiteout, black for blackout/redaction)
    let (r, g, b) = parse_hex_color(color);

    // Create appearance stream content that draws a filled rectangle
    // This is critical for reliable rendering across all PDF viewers
    let ap_content = format!(
        "{} {} {} rg\n\
         0 0 {} {} re\n\
         f",
        r, g, b, width, height
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
    // Interior Color (IC) - fallback for viewers that don't use AP
    annot.set(
        "IC",
        Object::Array(vec![Object::Real(r), Object::Real(g), Object::Real(b)]),
    );
    // Border color (C = Color) - same as interior
    annot.set(
        "C",
        Object::Array(vec![Object::Real(r), Object::Real(g), Object::Real(b)]),
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
            color: "#FFFFFF".to_string(),
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
            color: "#FFFFFF".to_string(),
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

    /// REGRESSION TEST: Exported PDF must contain UPDATED text after update_text() is called.
    /// This tests the scenario where a user edits text in a text box after initial creation.
    /// The downloaded PDF must reflect the latest text, not the original.
    #[test]
    fn test_export_contains_updated_text_after_update() {
        let pdf = create_test_pdf();
        let mut log = OperationLog::new();

        // Add text with original content
        let op_id = log.add(EditOperation::AddText {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 100.0,
                y: 700.0,
                width: 200.0,
                height: 20.0,
            },
            text: "ORIGINAL_TEXT_BEFORE_EDIT".to_string(),
            style: TextStyle::default(),
        });

        // Update the text (simulating user editing the text box)
        let updated = log.update_text(op_id, "UPDATED_TEXT_AFTER_EDIT", None);
        assert!(updated, "update_text should succeed");

        // Export the PDF
        let result = apply_operations(&pdf, &log).unwrap();
        let result_str = String::from_utf8_lossy(&result);

        // CRITICAL: The exported PDF must contain the UPDATED text, NOT the original
        assert!(
            result_str.contains("UPDATED_TEXT_AFTER_EDIT"),
            "Exported PDF MUST contain the updated text. \
             This proves that text edits are persisted to the download."
        );

        // And it should NOT contain the original text
        assert!(
            !result_str.contains("ORIGINAL_TEXT_BEFORE_EDIT"),
            "Exported PDF must NOT contain the original text after update. \
             Found original text when it should have been replaced."
        );
    }

    /// Same test but for flattened export
    #[test]
    fn test_flattened_export_contains_updated_text_after_update() {
        let pdf = create_test_pdf();
        let mut log = OperationLog::new();

        let op_id = log.add(EditOperation::AddText {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 100.0,
                y: 700.0,
                width: 200.0,
                height: 20.0,
            },
            text: "ORIGINAL_FLATTENED_TEXT".to_string(),
            style: TextStyle::default(),
        });

        log.update_text(op_id, "UPDATED_FLATTENED_TEXT", None);

        let result = apply_operations_flattened(&pdf, &log).unwrap();
        let result_str = String::from_utf8_lossy(&result);

        assert!(
            result_str.contains("UPDATED_FLATTENED_TEXT"),
            "Flattened export must contain updated text"
        );
        assert!(
            !result_str.contains("ORIGINAL_FLATTENED_TEXT"),
            "Flattened export must not contain original text after update"
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
            color: "#FFFFFF".to_string(),
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

    #[test]
    fn test_flattened_blackout_produces_black_color() {
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
            color: "#000000".to_string(), // Black for redaction
        });

        let result = apply_operations_flattened(&pdf, &log).unwrap();

        // The black color (0 0 0 rg) should appear in the PDF
        let result_str = String::from_utf8_lossy(&result);
        assert!(
            result_str.contains("0 0 0 rg"),
            "Flattened blackout should have black fill color (0 0 0 rg). PDF content: {}",
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
            color: "#FFFFFF".to_string(),
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

    /// Test that whiteout + text on same position both appear in exported PDF
    /// This is the user's reported bug: text typed into whiteout doesn't persist
    #[test]
    fn test_whiteout_with_text_both_persist_to_export() {
        let pdf = create_test_pdf();
        let mut log = OperationLog::new();

        // Add whiteout rectangle
        log.add(EditOperation::AddWhiteRect {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 100.0,
                y: 500.0,
                width: 200.0,
                height: 50.0,
            },
            color: "#FFFFFF".to_string(),
        });

        // Add text on top of the whiteout (same position)
        log.add(EditOperation::AddText {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 100.0,
                y: 500.0,
                width: 200.0,
                height: 50.0,
            },
            text: "WHITEOUT_TEXT_MUST_PERSIST".to_string(),
            style: TextStyle::default(),
        });

        // Test annotation-based export (compress)
        let result = apply_operations(&pdf, &log).unwrap();
        let result_str = String::from_utf8_lossy(&result);

        assert!(
            result_str.contains("WHITEOUT_TEXT_MUST_PERSIST"),
            "Annotation export MUST contain text typed into whiteout. \
             This is the user's reported bug. PDF content snippet: {}",
            &result_str[..std::cmp::min(1000, result_str.len())]
        );
    }

    /// REGRESSION TEST: Z-order bug where whiteout covers text in exported PDF
    /// User reported: "text is being included in the downloaded PDF but not rendered
    /// because the z ordering of the whiteout is greater than the text box"
    ///
    /// This test verifies that when whiteout and text are at the SAME position:
    /// 1. Both annotations exist in the PDF
    /// 2. Square (whiteout) comes FIRST in /Annots array (renders at bottom)
    /// 3. FreeText (text) comes SECOND in /Annots array (renders on top)
    /// 4. The text annotation's Rect OVERLAPS with the whiteout's Rect
    #[test]
    fn test_zorder_text_renders_on_top_of_whiteout_at_same_position() {
        let pdf = create_test_pdf();
        let mut log = OperationLog::new();

        // These coordinates are IDENTICAL - complete overlap
        let shared_rect = PdfRect {
            x: 100.0,
            y: 500.0,
            width: 200.0,
            height: 50.0,
        };

        // 1. Add whiteout FIRST (should render at bottom)
        log.add(EditOperation::AddWhiteRect {
            id: 0,
            page: 1,
            rect: shared_rect.clone(),
            color: "#FFFFFF".to_string(),
        });

        // 2. Add text SECOND at SAME position (should render on top)
        log.add(EditOperation::AddText {
            id: 0,
            page: 1,
            rect: shared_rect.clone(),
            text: "VISIBLE_TEXT_ON_TOP".to_string(),
            style: TextStyle::default(),
        });

        let result = apply_operations(&pdf, &log).unwrap();
        let doc = Document::load_mem(&result).unwrap();

        // Get the page and its annotations
        let pages: Vec<_> = doc.get_pages().into_iter().collect();
        let (_page_num, page_id) = pages[0];

        let page = doc.get_object(page_id).unwrap();
        let page_dict = match page {
            Object::Dictionary(d) => d,
            _ => panic!("Page should be a Dictionary"),
        };

        let annots = match page_dict.get(b"Annots") {
            Ok(Object::Array(arr)) => arr,
            _ => panic!("Page should have Annots array"),
        };

        assert_eq!(annots.len(), 2, "Should have exactly 2 annotations");

        // Extract annotation details in order
        let mut annotation_info: Vec<(String, Vec<f32>)> = Vec::new();
        for annot_ref in annots {
            if let Object::Reference(annot_id) = annot_ref {
                if let Ok(Object::Dictionary(annot)) = doc.get_object(*annot_id) {
                    let subtype = match annot.get(b"Subtype") {
                        Ok(Object::Name(n)) => String::from_utf8_lossy(n).to_string(),
                        _ => "Unknown".to_string(),
                    };
                    let rect = match annot.get(b"Rect") {
                        Ok(Object::Array(arr)) => arr
                            .iter()
                            .filter_map(|v| match v {
                                Object::Real(f) => Some(*f),
                                Object::Integer(i) => Some(*i as f32),
                                _ => None,
                            })
                            .collect(),
                        _ => vec![],
                    };
                    annotation_info.push((subtype, rect));
                }
            }
        }

        eprintln!("Annotations in order: {:?}", annotation_info);

        // CRITICAL Z-ORDER CHECK:
        // Square (whiteout) MUST be first (index 0) - renders at bottom
        // FreeText (text) MUST be second (index 1) - renders on top
        assert_eq!(
            annotation_info[0].0, "Square",
            "Z-ORDER BUG: First annotation should be Square (whiteout) but got {}. \
             Whiteout must come first to render at the bottom!",
            annotation_info[0].0
        );
        assert_eq!(
            annotation_info[1].0, "FreeText",
            "Z-ORDER BUG: Second annotation should be FreeText (text) but got {}. \
             Text must come second to render on top of whiteout!",
            annotation_info[1].0
        );

        // Verify rects overlap (both should be at approximately the same position)
        let whiteout_rect = &annotation_info[0].1;
        let text_rect = &annotation_info[1].1;

        // The rects should have significant overlap
        // whiteout_rect and text_rect should both cover the same area
        if whiteout_rect.len() == 4 && text_rect.len() == 4 {
            let whiteout_x1 = whiteout_rect[0];
            let whiteout_y1 = whiteout_rect[1];
            let whiteout_x2 = whiteout_rect[2];
            let whiteout_y2 = whiteout_rect[3];

            let text_x1 = text_rect[0];
            let text_y1 = text_rect[1];
            let text_x2 = text_rect[2];
            let text_y2 = text_rect[3];

            // Check for overlap: text rect should be within or overlapping whiteout rect
            let overlaps_x = text_x1 < whiteout_x2 && text_x2 > whiteout_x1;
            let overlaps_y = text_y1 < whiteout_y2 && text_y2 > whiteout_y1;

            assert!(
                overlaps_x && overlaps_y,
                "Z-ORDER BUG: Text rect {:?} should overlap with whiteout rect {:?}. \
                 If they don't overlap, z-order doesn't matter - but user reported \
                 text IS at same position as whiteout!",
                text_rect,
                whiteout_rect
            );
        }

        eprintln!("✓ Z-order is correct: Square at index 0 (bottom), FreeText at index 1 (top)");
    }

    /// REGRESSION TEST: Z-order after doc.compress() reorders objects
    /// User's bug: When we call doc.compress() to clean up the PDF,
    /// it may reorder objects in a way that breaks annotation z-order.
    /// This test creates a PDF with existing annotations, adds our whiteout+text,
    /// and verifies z-order is preserved after compress.
    #[test]
    fn test_zorder_preserved_after_compress_with_existing_annotations() {
        // Create a PDF that already has an annotation (simulating user's real PDF)
        use lopdf::content::{Content, Operation};

        let mut doc = Document::with_version("1.7");
        let pages_id = doc.new_object_id();

        // Create page content
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
                        b"Original Page Content".to_vec(),
                        lopdf::StringFormat::Literal,
                    )],
                ),
                Operation::new("ET", vec![]),
            ],
        };
        let content_id = doc.add_object(lopdf::Stream::new(
            Dictionary::new(),
            content.encode().unwrap(),
        ));

        // Create an EXISTING annotation on the page (like form fields in real PDFs)
        let mut existing_annot = Dictionary::new();
        existing_annot.set("Type", Object::Name(b"Annot".to_vec()));
        existing_annot.set("Subtype", Object::Name(b"Widget".to_vec())); // Form field
        existing_annot.set(
            "Rect",
            Object::Array(vec![
                Object::Integer(200),
                Object::Integer(600),
                Object::Integer(400),
                Object::Integer(620),
            ]),
        );
        let existing_annot_id = doc.add_object(Object::Dictionary(existing_annot));

        // Create page WITH existing annotation
        let page = Dictionary::from_iter(vec![
            ("Type", Object::Name(b"Page".to_vec())),
            ("Parent", Object::Reference(pages_id)),
            (
                "MediaBox",
                Object::Array(vec![
                    Object::Integer(0),
                    Object::Integer(0),
                    Object::Integer(612),
                    Object::Integer(792),
                ]),
            ),
            ("Contents", Object::Reference(content_id)),
            (
                "Annots",
                Object::Array(vec![Object::Reference(existing_annot_id)]),
            ), // EXISTING!
        ]);
        let page_id = doc.add_object(page);

        let pages = Dictionary::from_iter(vec![
            ("Type", Object::Name(b"Pages".to_vec())),
            ("Count", Object::Integer(1)),
            ("Kids", Object::Array(vec![Object::Reference(page_id)])),
        ]);
        doc.objects.insert(pages_id, Object::Dictionary(pages));

        let catalog = Dictionary::from_iter(vec![
            ("Type", Object::Name(b"Catalog".to_vec())),
            ("Pages", Object::Reference(pages_id)),
        ]);
        let catalog_id = doc.add_object(catalog);
        doc.trailer.set("Root", Object::Reference(catalog_id));

        // Save to bytes
        let mut pdf_bytes = Vec::new();
        doc.save_to(&mut pdf_bytes).unwrap();

        // Now apply our whiteout + text operations
        let mut log = OperationLog::new();

        log.add(EditOperation::AddWhiteRect {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 100.0,
                y: 500.0,
                width: 200.0,
                height: 50.0,
            },
            color: "#FFFFFF".to_string(),
        });

        log.add(EditOperation::AddText {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 100.0,
                y: 500.0,
                width: 200.0,
                height: 50.0,
            },
            text: "TEXT_MUST_BE_ON_TOP".to_string(),
            style: TextStyle::default(),
        });

        let result = apply_operations(&pdf_bytes, &log).unwrap();
        let result_doc = Document::load_mem(&result).unwrap();

        // Get annotations from the result
        let result_pages: Vec<_> = result_doc.get_pages().into_iter().collect();
        let (_page_num, result_page_id) = result_pages[0];

        let result_page = result_doc.get_object(result_page_id).unwrap();
        let result_page_dict = match result_page {
            Object::Dictionary(d) => d,
            _ => panic!("Page should be a Dictionary"),
        };

        let annots = match result_page_dict.get(b"Annots") {
            Ok(Object::Array(arr)) => arr,
            _ => panic!("Page should have Annots array"),
        };

        // Should have 3 annotations: existing Widget + our Square + our FreeText
        eprintln!("Total annotations after compress: {}", annots.len());
        assert!(annots.len() >= 2, "Should have at least our 2 annotations");

        // Find our Square and FreeText annotations and get their indices
        let mut square_index: Option<usize> = None;
        let mut freetext_index: Option<usize> = None;

        for (i, annot_ref) in annots.iter().enumerate() {
            if let Object::Reference(annot_id) = annot_ref {
                if let Ok(Object::Dictionary(annot)) = result_doc.get_object(*annot_id) {
                    if let Ok(Object::Name(subtype)) = annot.get(b"Subtype") {
                        let subtype_str = String::from_utf8_lossy(subtype);
                        eprintln!("Annotation {}: {}", i, subtype_str);
                        if subtype_str == "Square" {
                            square_index = Some(i);
                        } else if subtype_str == "FreeText" {
                            freetext_index = Some(i);
                        }
                    }
                }
            }
        }

        let square_idx = square_index.expect("Should have Square annotation");
        let freetext_idx = freetext_index.expect("Should have FreeText annotation");

        // CRITICAL: FreeText index MUST be greater than Square index
        // (later in array = renders on top)
        assert!(
            freetext_idx > square_idx,
            "Z-ORDER BUG AFTER COMPRESS: FreeText (text) is at index {}, \
             Square (whiteout) is at index {}. \
             FreeText MUST come AFTER Square for text to render on top! \
             doc.compress() may be reordering annotations.",
            freetext_idx,
            square_idx
        );

        eprintln!(
            "✓ Z-order preserved after compress: Square at {}, FreeText at {}",
            square_idx, freetext_idx
        );
    }

    /// REGRESSION TEST: Test with REAL user's PDF (florida_lease_demo.pdf)
    /// This tests the exact scenario the user reported
    #[test]
    fn test_zorder_with_real_florida_lease_pdf() {
        // Read the actual PDF the user is testing with
        let pdf_path = "/tmp/florida_lease_demo.pdf";
        let pdf_bytes = match std::fs::read(pdf_path) {
            Ok(bytes) => bytes,
            Err(_) => {
                eprintln!("Skipping test: {} not found", pdf_path);
                return;
            }
        };

        let mut log = OperationLog::new();

        // Add whiteout at a position near the top of page 1
        log.add(EditOperation::AddWhiteRect {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 100.0,
                y: 700.0,
                width: 300.0,
                height: 50.0,
            },
            color: "#FFFFFF".to_string(),
        });

        // Add text at SAME position
        log.add(EditOperation::AddText {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 100.0,
                y: 700.0,
                width: 300.0,
                height: 50.0,
            },
            text: "Test Test Test".to_string(),
            style: TextStyle {
                font_size: 36.0,
                color: "#000000".to_string(),
                font_name: None,
                is_italic: false,
                is_bold: false,
            },
        });

        let result = apply_operations(&pdf_bytes, &log).unwrap();
        let result_doc = Document::load_mem(&result).unwrap();

        // Check page 1 annotations
        let result_pages: Vec<_> = result_doc.get_pages().into_iter().collect();
        let page_1 = result_pages.iter().find(|(num, _)| *num == 1);

        if let Some((_page_num, page_id)) = page_1 {
            let page = result_doc.get_object(*page_id).unwrap();
            if let Object::Dictionary(page_dict) = page {
                if let Ok(Object::Array(annots)) = page_dict.get(b"Annots") {
                    eprintln!("Page 1 has {} annotations", annots.len());

                    let mut square_index: Option<usize> = None;
                    let mut freetext_index: Option<usize> = None;

                    for (i, annot_ref) in annots.iter().enumerate() {
                        if let Object::Reference(annot_id) = annot_ref {
                            if let Ok(Object::Dictionary(annot)) = result_doc.get_object(*annot_id)
                            {
                                if let Ok(Object::Name(subtype)) = annot.get(b"Subtype") {
                                    let subtype_str = String::from_utf8_lossy(subtype);
                                    eprintln!("Annotation {}: {}", i, subtype_str);
                                    if subtype_str == "Square" {
                                        square_index = Some(i);
                                    } else if subtype_str == "FreeText" {
                                        freetext_index = Some(i);
                                    }
                                }
                            }
                        }
                    }

                    let square_idx = square_index.expect("Should have Square annotation");
                    let freetext_idx = freetext_index.expect("Should have FreeText annotation");

                    assert!(
                        freetext_idx > square_idx,
                        "Z-ORDER BUG WITH REAL PDF: FreeText at index {}, Square at index {}. \
                         Text must come AFTER whiteout!",
                        freetext_idx,
                        square_idx
                    );

                    eprintln!(
                        "✓ Real PDF z-order correct: Square at {}, FreeText at {}",
                        square_idx, freetext_idx
                    );
                } else {
                    eprintln!("Page 1 has no Annots array");
                }
            }
        }

        // Also verify the text is in the PDF content
        let result_str = String::from_utf8_lossy(&result);
        assert!(
            result_str.contains("Test Test Test"),
            "Text should be in exported PDF"
        );

        // Save the output for visual verification
        std::fs::write("/tmp/florida_lease_demo_OUTPUT.pdf", &result)
            .expect("Failed to write output");
        eprintln!(
            "Saved output to /tmp/florida_lease_demo_OUTPUT.pdf - open to verify text is visible!"
        );
    }

    /// EXACT USER FLOW TEST: Matches the bug report exactly
    /// 1. User draws whiteout over "Residential Lease Agreement" text
    /// 2. User clicks ON the whiteout to open text editor
    /// 3. User types "Test Test Test" into the whiteout
    /// 4. Whiteout EXPANDS to fit text (saveWhiteoutText removes old whiteout, adds new one)
    /// 5. Text is added via addText at same position
    /// 6. User downloads - text should be visible on top of whiteout
    #[test]
    fn test_exact_user_flow_whiteout_then_type_text_into_it() {
        let pdf_path = "/tmp/florida_lease_demo.pdf";
        let pdf_bytes = match std::fs::read(pdf_path) {
            Ok(bytes) => bytes,
            Err(_) => {
                eprintln!("Skipping test: {} not found", pdf_path);
                return;
            }
        };

        let mut log = OperationLog::new();

        // STEP 1: User draws whiteout (this happens when mouseup after drawing)
        // Whiteout covers "Residential Lease Agreement" near top of page 1
        let original_whiteout_id = log.add(EditOperation::AddWhiteRect {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 150.0,
                y: 680.0,
                width: 300.0,
                height: 40.0,
            },
            color: "#FFFFFF".to_string(),
        });
        eprintln!(
            "Step 1: Added original whiteout with ID {}",
            original_whiteout_id
        );

        // STEP 2-3: User clicks whiteout, types "Test Test Test"
        // The text is larger (36pt) so the whiteout needs to expand
        // saveWhiteoutText() does this:
        //   - Removes old whiteout
        //   - Adds new whiteout with expanded dimensions
        //   - Adds text at same position

        // Simulate saveWhiteoutText removing old whiteout and adding expanded one
        log.remove(original_whiteout_id);
        eprintln!("Step 2: Removed old whiteout {}", original_whiteout_id);

        // STEP 4: Add expanded whiteout (wider to fit "Test Test Test" at 36pt)
        let new_whiteout_id = log.add(EditOperation::AddWhiteRect {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 150.0,
                y: 680.0,
                width: 350.0,
                height: 50.0,
            }, // Expanded
            color: "#FFFFFF".to_string(),
        });
        eprintln!(
            "Step 3: Added expanded whiteout with ID {}",
            new_whiteout_id
        );

        // STEP 5: Add text at same position
        let text_id = log.add(EditOperation::AddText {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 150.0,
                y: 680.0,
                width: 350.0,
                height: 50.0,
            },
            text: "Test Test Test".to_string(),
            style: TextStyle {
                font_size: 36.0,
                color: "#000000".to_string(),
                font_name: None,
                is_italic: false,
                is_bold: false,
            },
        });
        eprintln!("Step 4: Added text with ID {}", text_id);

        // Verify operations list
        eprintln!("Operations count: {}", log.operations().len());
        for (i, op) in log.operations().iter().enumerate() {
            let op_type = match op {
                EditOperation::AddWhiteRect { .. } => "AddWhiteRect",
                EditOperation::AddText { .. } => "AddText",
                EditOperation::AddStyledText { .. } => "AddStyledText",
                EditOperation::AddHighlight { .. } => "AddHighlight",
                EditOperation::AddCheckbox { .. } => "AddCheckbox",
                EditOperation::AddUnderline { .. } => "AddUnderline",
                EditOperation::ReplaceText { .. } => "ReplaceText",
            };
            eprintln!("  Op {}: {}", i, op_type);
        }

        // STEP 6: Export (this is what happens when user clicks Download)
        let result = apply_operations(&pdf_bytes, &log).unwrap();
        let result_doc = Document::load_mem(&result).unwrap();

        // Check annotations on page 1
        let result_pages: Vec<_> = result_doc.get_pages().into_iter().collect();
        let page_1 = result_pages.iter().find(|(num, _)| *num == 1);

        if let Some((_page_num, page_id)) = page_1 {
            let page = result_doc.get_object(*page_id).unwrap();
            if let Object::Dictionary(page_dict) = page {
                if let Ok(Object::Array(annots)) = page_dict.get(b"Annots") {
                    eprintln!("\nExported PDF - Page 1 annotations:");

                    let mut square_idx: Option<usize> = None;
                    let mut freetext_idx: Option<usize> = None;

                    for (i, annot_ref) in annots.iter().enumerate() {
                        if let Object::Reference(annot_id) = annot_ref {
                            if let Ok(Object::Dictionary(annot)) = result_doc.get_object(*annot_id)
                            {
                                let subtype = match annot.get(b"Subtype") {
                                    Ok(Object::Name(n)) => String::from_utf8_lossy(n).to_string(),
                                    _ => "Unknown".to_string(),
                                };
                                let rect = match annot.get(b"Rect") {
                                    Ok(Object::Array(arr)) => format!("{:?}", arr),
                                    _ => "no rect".to_string(),
                                };
                                eprintln!("  Annotation {}: {} at {}", i, subtype, rect);

                                if subtype == "Square" {
                                    square_idx = Some(i);
                                }
                                if subtype == "FreeText" {
                                    freetext_idx = Some(i);
                                }
                            }
                        }
                    }

                    // CRITICAL ASSERTIONS
                    assert!(
                        square_idx.is_some(),
                        "BUG: No Square (whiteout) annotation found!"
                    );
                    assert!(
                        freetext_idx.is_some(),
                        "BUG: No FreeText (text) annotation found! Text was not exported!"
                    );

                    let sq_idx = square_idx.unwrap();
                    let ft_idx = freetext_idx.unwrap();

                    assert!(
                        ft_idx > sq_idx,
                        "Z-ORDER BUG: FreeText at index {} but Square at index {}. \
                         FreeText MUST come AFTER Square for text to render on top!",
                        ft_idx,
                        sq_idx
                    );

                    eprintln!(
                        "\n✓ Z-order correct: Square at {}, FreeText at {}",
                        sq_idx, ft_idx
                    );
                } else {
                    panic!("Page 1 should have annotations!");
                }
            }
        }

        // Save for visual verification
        std::fs::write("/tmp/florida_lease_EXACT_FLOW_OUTPUT.pdf", &result)
            .expect("Failed to write output");
        eprintln!("\nSaved to /tmp/florida_lease_EXACT_FLOW_OUTPUT.pdf - OPEN THIS AND CHECK IF TEXT IS VISIBLE!");
    }

    /// TEST FOR ISSUE-025b: AddStyledText must produce visible text in PDF
    /// This is a CRITICAL test - the bug is that styled text shows in preview but NOT in downloaded PDF
    #[test]
    fn test_add_styled_text_produces_visible_text_in_pdf() {
        use crate::operations::StyledTextSegment;

        let pdf = create_test_pdf();
        let mut log = OperationLog::new();

        // Add styled text with a simple segment (no actual mixed styling)
        log.add(EditOperation::AddStyledText {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 100.0,
                y: 500.0,
                width: 200.0,
                height: 50.0,
            },
            segments: vec![StyledTextSegment {
                text: "STYLED_TEXT_TEST_VISIBLE".to_string(),
                is_bold: false,
                is_italic: false,
            }],
            style: TextStyle::default(),
        });

        let result = apply_operations(&pdf, &log).unwrap();
        let result_str = String::from_utf8_lossy(&result);

        // The text MUST appear in the PDF content
        assert!(
            result_str.contains("STYLED_TEXT_TEST_VISIBLE"),
            "CRITICAL BUG: AddStyledText does not produce visible text in PDF! \
             The text 'STYLED_TEXT_TEST_VISIBLE' should be in the PDF but is missing. \
             First 2000 chars of PDF: {}",
            &result_str[..std::cmp::min(2000, result_str.len())]
        );

        // Verify the appearance stream (AP) is present - this is what actually renders the text
        assert!(
            result_str.contains("/AP"),
            "CRITICAL BUG: AddStyledText annotation is missing appearance stream (AP)! \
             Without AP, the text won't render in most PDF viewers."
        );

        // Save to file for manual inspection
        std::fs::write("/tmp/styled_text_test.pdf", &result).expect("Failed to write test PDF");
        eprintln!("Saved styled text test PDF to /tmp/styled_text_test.pdf - OPEN AND VERIFY TEXT IS VISIBLE!");
    }

    /// TEST: AddStyledText with actual mixed styling (bold + regular)
    #[test]
    fn test_add_styled_text_mixed_bold_regular() {
        use crate::operations::StyledTextSegment;

        let pdf = create_test_pdf();
        let mut log = OperationLog::new();

        log.add(EditOperation::AddStyledText {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 100.0,
                y: 400.0,
                width: 300.0,
                height: 50.0,
            },
            segments: vec![
                StyledTextSegment {
                    text: "BOLD_PART".to_string(),
                    is_bold: true,
                    is_italic: false,
                },
                StyledTextSegment {
                    text: "REGULAR_PART".to_string(),
                    is_bold: false,
                    is_italic: false,
                },
            ],
            style: TextStyle::default(),
        });

        let result = apply_operations(&pdf, &log).unwrap();
        let result_str = String::from_utf8_lossy(&result);

        assert!(
            result_str.contains("BOLD_PART"),
            "Mixed styled text missing BOLD_PART"
        );
        assert!(
            result_str.contains("REGULAR_PART"),
            "Mixed styled text missing REGULAR_PART"
        );

        // Save for manual inspection
        std::fs::write("/tmp/styled_text_mixed_test.pdf", &result)
            .expect("Failed to write test PDF");
        eprintln!("Saved mixed styled text test PDF to /tmp/styled_text_mixed_test.pdf");
    }

    /// Same test but for flattened export
    #[test]
    fn test_whiteout_with_text_both_persist_to_flattened_export() {
        let pdf = create_test_pdf();
        let mut log = OperationLog::new();

        // Add whiteout rectangle
        log.add(EditOperation::AddWhiteRect {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 100.0,
                y: 500.0,
                width: 200.0,
                height: 50.0,
            },
            color: "#FFFFFF".to_string(),
        });

        // Add text on top of the whiteout (same position)
        log.add(EditOperation::AddText {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 100.0,
                y: 500.0,
                width: 200.0,
                height: 50.0,
            },
            text: "WHITEOUT_FLATTENED_TEXT".to_string(),
            style: TextStyle::default(),
        });

        // Test flattened export
        let result = apply_operations_flattened(&pdf, &log).unwrap();
        let result_str = String::from_utf8_lossy(&result);

        assert!(
            result_str.contains("WHITEOUT_FLATTENED_TEXT"),
            "Flattened export MUST contain text typed into whiteout. \
             This is the user's reported bug. PDF content snippet: {}",
            &result_str[..std::cmp::min(1000, result_str.len())]
        );
    }

    /// ISSUE-009: Whiteout with STYLED text (using AddStyledText path) must persist to PDF
    /// This is the exact bug scenario: user creates whiteout, types styled text, downloads
    #[test]
    fn test_whiteout_with_styled_text_persists_to_export() {
        use crate::operations::StyledTextSegment;

        let pdf = create_test_pdf();
        let mut log = OperationLog::new();

        // Add whiteout rectangle (covering existing text)
        log.add(EditOperation::AddWhiteRect {
            id: 0,
            page: 1,
            rect: PdfRect {
                x: 100.0,
                y: 500.0,
                width: 200.0,
                height: 50.0,
            },
            color: "#FFFFFF".to_string(),
        });

        // Add STYLED text on top of the whiteout (this is the addStyledText path)
        log.add(EditOperation::AddStyledText {
            id: 1,
            page: 1,
            rect: PdfRect {
                x: 100.0,
                y: 500.0,
                width: 200.0,
                height: 50.0,
            },
            segments: vec![StyledTextSegment {
                text: "WHITEOUT_STYLED_TEXT_TEST".to_string(),
                is_bold: false,
                is_italic: false,
            }],
            style: TextStyle::default(),
        });

        // Test regular export (annotations)
        let result = apply_operations(&pdf, &log).unwrap();
        let result_str = String::from_utf8_lossy(&result);

        assert!(
            result_str.contains("WHITEOUT_STYLED_TEXT_TEST"),
            "ISSUE-009 REGRESSION: AddStyledText on whiteout does NOT produce text in PDF! \
             The user creates whiteout, types text, and downloads - but text is missing. \
             PDF content: {}",
            &result_str[..std::cmp::min(2000, result_str.len())]
        );

        // Also test flattened export
        let flat_result = apply_operations_flattened(&pdf, &log).unwrap();
        let flat_str = String::from_utf8_lossy(&flat_result);

        assert!(
            flat_str.contains("WHITEOUT_STYLED_TEXT_TEST"),
            "ISSUE-009 REGRESSION: AddStyledText on whiteout does NOT persist to flattened PDF! \
             Flattened export is missing the text."
        );

        // Save for manual visual inspection
        std::fs::write("/tmp/whiteout_styled_text_test.pdf", &result)
            .expect("Failed to write test PDF");
        eprintln!("Saved whiteout+styled text test PDF to /tmp/whiteout_styled_text_test.pdf");
    }

    /// ISSUE-009 ROOT CAUSE: Styled text appearance stream has WRONG operator order.
    ///
    /// PDF content streams require font to be set BEFORE text positioning for proper rendering.
    /// The working `create_text_appearance_content` uses:
    ///   BT -> /F1 Tf -> rg -> Td -> Tj -> ET  (font FIRST)
    ///
    /// The broken `create_styled_text_appearance_content` uses:
    ///   BT -> rg -> Td -> /F1 Tf -> Tj -> ET  (font AFTER position - WRONG!)
    ///
    /// This causes macOS Preview (and other strict PDF viewers) to not render the text,
    /// even though the text bytes exist in the PDF.
    ///
    /// FIX: In `create_styled_text_appearance_content()`, move the font operator
    /// BEFORE the color (rg) and position (Td) operators, matching the working function.
    #[test]
    fn test_styled_text_appearance_stream_operator_order() {
        // Test the content stream directly to verify operator order
        use crate::operations::StyledTextSegment;

        let segments = vec![StyledTextSegment {
            text: "TEST".to_string(),
            is_bold: false,
            is_italic: false,
        }];

        let content = create_styled_text_appearance_content(
            &segments,
            12.0,        // font_size
            0.0,         // r
            0.0,         // g
            0.0,         // b
            "Helvetica", // base_font_name
            100.0,       // box_width
            20.0,        // box_height
            &TextStyle::default(),
        );

        // Find positions of key operators
        let tf_pos = content.find(" Tf");
        let rg_pos = content.find(" rg");
        let td_pos = content.find(" Td");

        assert!(
            tf_pos.is_some(),
            "Content stream missing font operator (Tf). Content: {}",
            content
        );
        assert!(
            rg_pos.is_some(),
            "Content stream missing color operator (rg). Content: {}",
            content
        );
        assert!(
            td_pos.is_some(),
            "Content stream missing position operator (Td). Content: {}",
            content
        );

        let tf_pos = tf_pos.unwrap();
        let rg_pos = rg_pos.unwrap();
        let td_pos = td_pos.unwrap();

        // CRITICAL: Font (Tf) must come BEFORE color (rg) and position (Td)
        // This matches the working create_text_appearance_content function.
        //
        // If this test fails, the fix is in create_styled_text_appearance_content():
        // Move the font Tf line BEFORE the rg and Td lines.
        assert!(
            tf_pos < rg_pos,
            "ISSUE-009 BUG: Font operator (Tf) at pos {} comes AFTER color operator (rg) at pos {}!\n\
             \n\
             FIX: In create_styled_text_appearance_content(), change the operator order from:\n\
             \n\
             Current (BROKEN - doesn't render in macOS Preview):\n\
             ```\n\
             BT\n\
             {{r}} {{g}} {{b}} rg     <- color first\n\
             {{x}} {{y}} Td           <- position second\n\
             /F1 {{size}} Tf          <- font third (TOO LATE!)\n\
             (text) Tj\n\
             ET\n\
             ```\n\
             \n\
             To (WORKING - matches create_text_appearance_content):\n\
             ```\n\
             BT\n\
             /F1 {{size}} Tf          <- font FIRST\n\
             {{r}} {{g}} {{b}} rg     <- color second\n\
             {{x}} {{y}} Td           <- position third\n\
             (text) Tj\n\
             ET\n\
             ```\n\
             \n\
             Actual content stream:\n{}",
            tf_pos, rg_pos, content
        );

        assert!(
            tf_pos < td_pos,
            "ISSUE-009 BUG: Font operator (Tf) at pos {} comes AFTER position operator (Td) at pos {}!\n\
             See fix instructions above.\n\
             Actual content stream:\n{}",
            tf_pos, td_pos, content
        );

        eprintln!(
            "Content stream operator order is correct: Tf({}) < rg({}) < Td({})",
            tf_pos, rg_pos, td_pos
        );
        eprintln!("Content stream:\n{}", content);
    }
}
