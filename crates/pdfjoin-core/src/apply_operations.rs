//! Apply operations to PDF documents

use crate::error::PdfJoinError;
use crate::operations::{EditOperation, OperationLog, PdfRect, TextStyle};
use lopdf::{Dictionary, Document, Object, ObjectId};

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

    let mut output = Vec::new();
    doc.save_to(&mut output)
        .map_err(|e| PdfJoinError::OperationError(e.to_string()))?;

    Ok(output)
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
    let mut annot = Dictionary::new();
    annot.set("Type", Object::Name(b"Annot".to_vec()));
    annot.set("Subtype", Object::Name(b"FreeText".to_vec()));
    annot.set(
        "Rect",
        Object::Array(vec![
            Object::Real(rect.x as f32),
            Object::Real(rect.y as f32),
            Object::Real((rect.x + rect.width) as f32),
            Object::Real((rect.y + rect.height) as f32),
        ]),
    );
    annot.set(
        "Contents",
        Object::String(text.as_bytes().to_vec(), lopdf::StringFormat::Literal),
    );

    // Parse color from hex to RGB components (0-1 range)
    let (r, g, b) = parse_hex_color(&style.color);

    // Default appearance with font preservation
    let font_name = style.pdf_font_name();
    let da = format!("/{} {} Tf {} {} {} rg", font_name, style.font_size, r, g, b);
    annot.set(
        "DA",
        Object::String(da.into_bytes(), lopdf::StringFormat::Literal),
    );

    let annot_id = doc.add_object(Object::Dictionary(annot));
    add_annotation_to_page(doc, page_id, annot_id)
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

    // 1. Add white Square annotation to cover original text
    // Use padding to ensure complete coverage of original text glyphs
    let mut cover = Dictionary::new();
    cover.set("Type", Object::Name(b"Annot".to_vec()));
    cover.set("Subtype", Object::Name(b"Square".to_vec()));
    cover.set(
        "Rect",
        Object::Array(vec![
            Object::Real((original_rect.x - COVER_PADDING) as f32),
            Object::Real((original_rect.y - COVER_PADDING) as f32),
            Object::Real((original_rect.x + original_rect.width + COVER_PADDING) as f32),
            Object::Real((original_rect.y + original_rect.height + COVER_PADDING) as f32),
        ]),
    );
    // White fill (IC = Interior Color)
    cover.set(
        "IC",
        Object::Array(vec![
            Object::Real(1.0),
            Object::Real(1.0),
            Object::Real(1.0),
        ]),
    );
    // White border (C = Color)
    cover.set(
        "C",
        Object::Array(vec![
            Object::Real(1.0),
            Object::Real(1.0),
            Object::Real(1.0),
        ]),
    );
    // No border width - create BS dictionary inline
    let mut bs = Dictionary::new();
    bs.set("W", Object::Integer(0));
    cover.set("BS", Object::Dictionary(bs));

    let cover_id = doc.add_object(Object::Dictionary(cover));
    add_annotation_to_page(doc, page_id, cover_id)?;

    // 2. Add FreeText annotation with replacement text
    add_text_annotation(doc, page_id, replacement_rect, new_text, style)
}

fn add_white_rect_annotation(
    doc: &mut Document,
    page_id: ObjectId,
    rect: &PdfRect,
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
    // White fill (IC = Interior Color)
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
}
