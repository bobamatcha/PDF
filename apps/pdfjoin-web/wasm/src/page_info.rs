//! Page-level information extraction
//!
//! Extracts metadata about individual PDF pages.

use lopdf::{Document, Object};
use serde::Serialize;

/// Information about a single PDF page
#[derive(Debug, Clone, Serialize)]
pub struct PageInfo {
    /// Page number (1-indexed)
    pub page_num: u32,
    /// Page width in points (1 point = 1/72 inch)
    pub width: f32,
    /// Page height in points
    pub height: f32,
    /// Page rotation in degrees (0, 90, 180, 270)
    pub rotation: i32,
    /// Whether the page has a content stream (not blank)
    pub has_content: bool,
    /// Estimated orientation based on dimensions
    pub orientation: PageOrientation,
}

/// Page orientation
#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
pub enum PageOrientation {
    Portrait,
    Landscape,
    Square,
}

impl PageInfo {
    /// Extract page info from a document
    pub fn from_document(doc: &Document, page_num: u32) -> Result<Self, String> {
        let pages = doc.get_pages();
        let page_id = pages
            .get(&page_num)
            .ok_or_else(|| format!("Page {} not found", page_num))?;

        let page_obj = doc
            .objects
            .get(page_id)
            .ok_or_else(|| format!("Page {} object not found", page_num))?;

        let page_dict = page_obj
            .as_dict()
            .map_err(|_| format!("Page {} is not a dictionary", page_num))?;

        // Get MediaBox (required) - defines page boundaries
        let media_box = get_media_box(doc, page_dict)?;
        let (width, height) = (media_box[2] - media_box[0], media_box[3] - media_box[1]);

        // Get rotation (optional, default 0)
        let rotation = get_rotation(doc, page_dict);

        // Check for content stream
        let has_content = page_dict.get(b"Contents").is_ok();

        // Determine orientation (accounting for rotation)
        let effective_width = if rotation == 90 || rotation == 270 {
            height
        } else {
            width
        };
        let effective_height = if rotation == 90 || rotation == 270 {
            width
        } else {
            height
        };

        let orientation = if (effective_width - effective_height).abs() < 1.0 {
            PageOrientation::Square
        } else if effective_width > effective_height {
            PageOrientation::Landscape
        } else {
            PageOrientation::Portrait
        };

        Ok(Self {
            page_num,
            width: width as f32,
            height: height as f32,
            rotation,
            has_content,
            orientation,
        })
    }

    /// Get all page infos for a document
    pub fn all_from_document(doc: &Document) -> Vec<Result<Self, String>> {
        let pages = doc.get_pages();
        let mut results = Vec::with_capacity(pages.len());

        for page_num in 1..=pages.len() as u32 {
            results.push(Self::from_document(doc, page_num));
        }

        results
    }
}

/// Get MediaBox from page, inheriting from parent if necessary
fn get_media_box(doc: &Document, page_dict: &lopdf::Dictionary) -> Result<[f64; 4], String> {
    // Try to get MediaBox directly from page
    if let Ok(media_box) = page_dict.get(b"MediaBox") {
        if let Ok(array) = media_box.as_array() {
            return parse_box_array(array);
        }
    }

    // Try to inherit from parent
    if let Ok(parent_ref) = page_dict.get(b"Parent") {
        if let Ok(parent_id) = parent_ref.as_reference() {
            if let Some(parent_obj) = doc.objects.get(&parent_id) {
                if let Ok(parent_dict) = parent_obj.as_dict() {
                    if let Ok(media_box) = parent_dict.get(b"MediaBox") {
                        if let Ok(array) = media_box.as_array() {
                            return parse_box_array(array);
                        }
                    }
                }
            }
        }
    }

    // Default to US Letter size
    Ok([0.0, 0.0, 612.0, 792.0])
}

/// Parse a box array [x1, y1, x2, y2]
fn parse_box_array(array: &[Object]) -> Result<[f64; 4], String> {
    if array.len() != 4 {
        return Err("MediaBox must have 4 elements".to_string());
    }

    let mut result = [0.0; 4];
    for (i, obj) in array.iter().enumerate() {
        result[i] = match obj {
            Object::Integer(n) => *n as f64,
            Object::Real(n) => *n as f64,
            _ => return Err(format!("MediaBox element {} is not a number", i)),
        };
    }

    Ok(result)
}

/// Get rotation from page, inheriting from parent if necessary
fn get_rotation(doc: &Document, page_dict: &lopdf::Dictionary) -> i32 {
    // Try to get Rotate directly from page
    if let Ok(rotate) = page_dict.get(b"Rotate") {
        if let Ok(angle) = rotate.as_i64() {
            return normalize_rotation(angle as i32);
        }
    }

    // Try to inherit from parent
    if let Ok(parent_ref) = page_dict.get(b"Parent") {
        if let Ok(parent_id) = parent_ref.as_reference() {
            if let Some(parent_obj) = doc.objects.get(&parent_id) {
                if let Ok(parent_dict) = parent_obj.as_dict() {
                    if let Ok(rotate) = parent_dict.get(b"Rotate") {
                        if let Ok(angle) = rotate.as_i64() {
                            return normalize_rotation(angle as i32);
                        }
                    }
                }
            }
        }
    }

    0
}

/// Normalize rotation to 0, 90, 180, or 270
fn normalize_rotation(angle: i32) -> i32 {
    let normalized = angle % 360;
    if normalized < 0 {
        normalized + 360
    } else {
        normalized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_rotation() {
        assert_eq!(normalize_rotation(0), 0);
        assert_eq!(normalize_rotation(90), 90);
        assert_eq!(normalize_rotation(180), 180);
        assert_eq!(normalize_rotation(270), 270);
        assert_eq!(normalize_rotation(360), 0);
        assert_eq!(normalize_rotation(450), 90);
        assert_eq!(normalize_rotation(-90), 270);
    }

    #[test]
    fn test_parse_box_array() {
        let array = vec![
            Object::Integer(0),
            Object::Integer(0),
            Object::Real(612.0),
            Object::Real(792.0),
        ];
        let result = parse_box_array(&array).unwrap();
        assert_eq!(result, [0.0, 0.0, 612.0, 792.0]);
    }
}
