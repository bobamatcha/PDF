//! PAdES signature injection into PDFs

use crate::crypto::cms::build_signed_data;
use crate::crypto::keys::SigningIdentity;
use crate::pdf::parser::PdfDocument;
use chrono::Utc;
use lopdf::{Dictionary, Object, ObjectId, Stream};
use sha2::{Digest, Sha256};

/// Escape special characters for PDF string literals
fn escape_pdf_string(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '(' => "\\(".to_string(),
            ')' => "\\)".to_string(),
            '\\' => "\\\\".to_string(),
            _ if c.is_ascii() => c.to_string(),
            _ => "?".to_string(), // Replace non-ASCII with ?
        })
        .collect()
}

/// A signature field to be placed on the document
#[derive(Debug, Clone)]
pub struct SignatureField {
    pub page: u32,
    pub rect: [f64; 4], // [x, y, width, height] in PDF coordinates
    pub signer_name: String,
    pub reason: String,
}

impl SignatureField {
    /// Create a new signature field with default positioning
    pub fn new(page: u32, signer_name: String, reason: String) -> Self {
        // Default position: bottom-left corner, 200x50 pixels
        Self {
            page,
            rect: [10.0, 10.0, 200.0, 50.0],
            signer_name,
            reason,
        }
    }

    /// Set custom position and size
    pub fn with_rect(mut self, rect: [f64; 4]) -> Self {
        self.rect = rect;
        self
    }
}

/// Handles PDF digital signature operations
pub struct PdfSigner<'a, I: SigningIdentity> {
    doc: &'a mut PdfDocument,
    identity: &'a I,
}

impl<'a, I: SigningIdentity> PdfSigner<'a, I> {
    pub fn new(doc: &'a mut PdfDocument, identity: &'a I) -> Self {
        Self { doc, identity }
    }

    /// Add a signature to the document
    pub fn sign(&mut self, field: &SignatureField) -> Result<Vec<u8>, String> {
        // Step 1: Create signature dictionary with placeholder
        let sig_dict_id = self.create_signature_dictionary()?;

        // Step 2: Create the signature field and widget annotation
        let field_id = self.create_signature_field(sig_dict_id, field)?;

        // Step 3: Add to AcroForm and page annotations
        self.add_to_acroform(field_id)?;
        self.add_to_page_annots(field.page, field_id)?;

        // Step 4: Save to get byte positions (with placeholder)
        let placeholder_size = 8192; // Reserve space for signature
        let pdf_with_placeholder = self.save_with_placeholder(sig_dict_id, placeholder_size)?;

        // Step 5: Calculate byte range and hash
        let (byte_range, hash) =
            self.calculate_hash_and_range(&pdf_with_placeholder, placeholder_size)?;

        // Step 6: Build CMS signature
        let signature = self.build_cms_signature(&hash, &field.signer_name)?;

        // Step 7: Inject signature into Contents
        let signed_pdf = self.inject_signature(
            &pdf_with_placeholder,
            &signature,
            placeholder_size,
            &byte_range,
        )?;

        Ok(signed_pdf)
    }

    /// Create the signature dictionary object
    fn create_signature_dictionary(&mut self) -> Result<ObjectId, String> {
        let mut sig_dict = Dictionary::new();
        sig_dict.set("Type", Object::Name(b"Sig".to_vec()));
        sig_dict.set("Filter", Object::Name(b"Adobe.PPKLite".to_vec()));
        sig_dict.set("SubFilter", Object::Name(b"adbe.pkcs7.detached".to_vec()));

        // Placeholder for Contents (will be replaced)
        sig_dict.set(
            "Contents",
            Object::String(vec![0; 8192], lopdf::StringFormat::Hexadecimal),
        );

        // Placeholder for ByteRange (will be replaced)
        // Use large placeholder values to ensure enough space for actual values
        // Each number needs space for up to 10 digits (for files up to ~10GB)
        sig_dict.set(
            "ByteRange",
            Object::Array(vec![
                Object::Integer(0),
                Object::Integer(9999999999),
                Object::Integer(9999999999),
                Object::Integer(9999999999),
            ]),
        );

        // Add signing time
        let now = Utc::now().format("%Y%m%d%H%M%S+00'00'").to_string();
        sig_dict.set(
            "M",
            Object::String(now.into_bytes(), lopdf::StringFormat::Literal),
        );

        let sig_dict_id = self.doc.doc_mut().add_object(Object::Dictionary(sig_dict));
        Ok(sig_dict_id)
    }

    /// Create the signature field and widget annotation
    fn create_signature_field(
        &mut self,
        sig_dict_id: ObjectId,
        field: &SignatureField,
    ) -> Result<ObjectId, String> {
        let mut field_dict = Dictionary::new();

        // Widget annotation properties
        field_dict.set("Type", Object::Name(b"Annot".to_vec()));
        field_dict.set("Subtype", Object::Name(b"Widget".to_vec()));

        // Position on page
        let rect_array = vec![
            Object::Real(field.rect[0] as f32),
            Object::Real(field.rect[1] as f32),
            Object::Real((field.rect[0] + field.rect[2]) as f32),
            Object::Real((field.rect[1] + field.rect[3]) as f32),
        ];
        field_dict.set("Rect", Object::Array(rect_array));

        // Signature field properties
        field_dict.set("FT", Object::Name(b"Sig".to_vec()));
        field_dict.set(
            "T",
            Object::String(b"Signature1".to_vec(), lopdf::StringFormat::Literal),
        );

        // Reference to signature dictionary
        field_dict.set("V", Object::Reference(sig_dict_id));

        // Appearance characteristics (simple text stamp)
        let appearance_dict = self.create_appearance_dict(field)?;
        field_dict.set("AP", Object::Dictionary(appearance_dict));

        // Flags: Print (bit 3)
        field_dict.set("F", Object::Integer(4));

        // Add page reference
        if let Some(page_id) = self.doc.page_id(field.page) {
            field_dict.set("P", Object::Reference(page_id));
        }

        let field_id = self
            .doc
            .doc_mut()
            .add_object(Object::Dictionary(field_dict));
        Ok(field_id)
    }

    /// Create appearance dictionary for the signature stamp
    fn create_appearance_dict(&mut self, field: &SignatureField) -> Result<Dictionary, String> {
        let mut ap_dict = Dictionary::new();

        // Create normal appearance stream
        let appearance_stream = self.create_appearance_stream(field)?;
        let stream_id = self.doc.doc_mut().add_object(appearance_stream);

        ap_dict.set("N", Object::Reference(stream_id));
        Ok(ap_dict)
    }

    /// Create the appearance stream (visual representation)
    fn create_appearance_stream(&self, field: &SignatureField) -> Result<Object, String> {
        // rect is [x1, y1, x2, y2] - calculate actual width and height
        let width = (field.rect[2] - field.rect[0]).abs();
        let height = (field.rect[3] - field.rect[1]).abs();

        // Escape special PDF characters in text
        let signer_name = escape_pdf_string(&field.signer_name);
        let reason = escape_pdf_string(&field.reason);

        // Create minimal appearance stream - just a border and text
        // Using simpler drawing commands that work across PDF viewers
        let font_size = (height * 0.25).clamp(6.0, 10.0);
        let line1_y = height - font_size - 2.0;

        let content = format!(
            "q\n\
0.9 0.95 1 rg\n\
0 0 {w} {h} re f\n\
0.2 0.4 0.8 RG\n\
1 w\n\
0.5 0.5 {w2} {h2} re S\n\
0 0 0 rg\n\
BT\n\
/F1 {fs} Tf\n\
4 {y1} Td\n\
({signer}) Tj\n\
0 -{fs2} Td\n\
({reason}) Tj\n\
ET\n\
Q",
            w = width,
            h = height,
            w2 = width - 1.0,
            h2 = height - 1.0,
            fs = font_size,
            y1 = line1_y,
            fs2 = font_size + 2.0,
            signer = signer_name,
            reason = reason,
        );

        let mut stream_dict = Dictionary::new();
        stream_dict.set("Type", Object::Name(b"XObject".to_vec()));
        stream_dict.set("Subtype", Object::Name(b"Form".to_vec()));
        stream_dict.set("FormType", Object::Integer(1));
        stream_dict.set(
            "BBox",
            Object::Array(vec![
                Object::Integer(0),
                Object::Integer(0),
                Object::Real(width as f32),
                Object::Real(height as f32),
            ]),
        );
        // Don't set Length - lopdf will calculate it

        // Matrix for identity transform
        stream_dict.set(
            "Matrix",
            Object::Array(vec![
                Object::Integer(1),
                Object::Integer(0),
                Object::Integer(0),
                Object::Integer(1),
                Object::Integer(0),
                Object::Integer(0),
            ]),
        );

        // Add font resource
        let mut resources = Dictionary::new();
        let mut font_dict = Dictionary::new();
        let mut f1_dict = Dictionary::new();
        f1_dict.set("Type", Object::Name(b"Font".to_vec()));
        f1_dict.set("Subtype", Object::Name(b"Type1".to_vec()));
        f1_dict.set("BaseFont", Object::Name(b"Helvetica".to_vec()));
        font_dict.set("F1", Object::Dictionary(f1_dict));
        resources.set("Font", Object::Dictionary(font_dict));
        stream_dict.set("Resources", Object::Dictionary(resources));

        Ok(Object::Stream(Stream::new(
            stream_dict,
            content.into_bytes(),
        )))
    }

    /// Add the signature field to the AcroForm
    fn add_to_acroform(&mut self, field_id: ObjectId) -> Result<(), String> {
        let doc = self.doc.doc_mut();
        let catalog = doc
            .catalog_mut()
            .map_err(|e| format!("Failed to get catalog: {}", e))?;

        // Get or create AcroForm
        let acroform_id = if let Ok(acroform_ref) = catalog.get(b"AcroForm") {
            acroform_ref
                .as_reference()
                .map_err(|_| "AcroForm is not a reference")?
        } else {
            // Create new AcroForm
            let mut acroform = Dictionary::new();
            acroform.set("Fields", Object::Array(vec![]));
            acroform.set("SigFlags", Object::Integer(3)); // SignaturesExist | AppendOnly
            let acroform_id = doc.add_object(Object::Dictionary(acroform));

            // Re-get catalog to set AcroForm reference
            let catalog = doc
                .catalog_mut()
                .map_err(|e| format!("Failed to get catalog: {}", e))?;
            catalog.set("AcroForm", Object::Reference(acroform_id));
            acroform_id
        };

        // Add field to Fields array
        let acroform = doc
            .get_object_mut(acroform_id)
            .map_err(|e| format!("Failed to get AcroForm: {}", e))?
            .as_dict_mut()
            .map_err(|_| "AcroForm is not a dictionary")?;

        let mut fields = if let Ok(fields_obj) = acroform.get(b"Fields") {
            fields_obj
                .as_array()
                .map_err(|_| "Fields is not an array")?
                .clone()
        } else {
            vec![]
        };

        fields.push(Object::Reference(field_id));
        acroform.set("Fields", Object::Array(fields));
        acroform.set("SigFlags", Object::Integer(3));

        Ok(())
    }

    /// Add the signature widget to the page's Annots array
    fn add_to_page_annots(&mut self, page_num: u32, field_id: ObjectId) -> Result<(), String> {
        let page_id = self
            .doc
            .page_id(page_num)
            .ok_or_else(|| format!("Page {} not found", page_num))?;

        let doc = self.doc.doc_mut();

        // First, get the annots if they exist as a reference
        let annots_ref_id = {
            let page_obj = doc
                .get_object(page_id)
                .map_err(|e| format!("Failed to get page object: {}", e))?;
            let page_dict = page_obj.as_dict().map_err(|_| "Page is not a dictionary")?;

            page_dict
                .get(b"Annots")
                .ok()
                .and_then(|annots_obj| annots_obj.as_reference().ok())
        };

        // Build the annots array
        let mut annots = if let Some(annots_id) = annots_ref_id {
            let annots_obj = doc
                .get_object(annots_id)
                .map_err(|e| format!("Failed to get annots: {}", e))?;
            annots_obj
                .as_array()
                .map_err(|_| "Annots reference is not an array")?
                .clone()
        } else {
            let page_obj = doc
                .get_object(page_id)
                .map_err(|e| format!("Failed to get page object: {}", e))?;
            let page_dict = page_obj.as_dict().map_err(|_| "Page is not a dictionary")?;

            if let Ok(annots_obj) = page_dict.get(b"Annots") {
                if let Ok(arr) = annots_obj.as_array() {
                    arr.clone()
                } else {
                    vec![]
                }
            } else {
                vec![]
            }
        };

        annots.push(Object::Reference(field_id));

        // Update the page
        let page = doc
            .get_object_mut(page_id)
            .map_err(|e| format!("Failed to get page object: {}", e))?
            .as_dict_mut()
            .map_err(|_| "Page is not a dictionary")?;
        page.set("Annots", Object::Array(annots));

        Ok(())
    }

    /// Save PDF with placeholder signature
    fn save_with_placeholder(
        &mut self,
        sig_dict_id: ObjectId,
        placeholder_size: usize,
    ) -> Result<Vec<u8>, String> {
        // Update the signature dictionary with proper placeholder
        let sig_dict = self
            .doc
            .doc_mut()
            .get_object_mut(sig_dict_id)
            .map_err(|e| format!("Failed to get signature dict: {}", e))?
            .as_dict_mut()
            .map_err(|_| "Signature dict is not a dictionary")?;

        // Set placeholder contents (will be replaced later)
        let placeholder = vec![0u8; placeholder_size];
        sig_dict.set(
            "Contents",
            Object::String(placeholder, lopdf::StringFormat::Hexadecimal),
        );

        // Save to bytes
        self.doc.save_to_bytes()
    }

    /// Calculate the document hash and byte range
    fn calculate_hash_and_range(
        &self,
        pdf_bytes: &[u8],
        placeholder_size: usize,
    ) -> Result<([i64; 4], [u8; 32]), String> {
        // Find the /Contents field in the PDF
        let start_marker = self
            .find_last_occurrence(pdf_bytes, b"/Contents")
            .ok_or("Could not find /Contents marker")?;

        // Find the start of the hex string (after '<')
        let mut contents_start = start_marker;
        while contents_start < pdf_bytes.len() {
            if pdf_bytes[contents_start] == b'<' {
                contents_start += 1; // Skip the '<'
                break;
            }
            contents_start += 1;
        }

        // The placeholder is hex-encoded, so it's 2x the size
        let hex_placeholder_size = placeholder_size * 2;
        let contents_end = contents_start + hex_placeholder_size;

        // ByteRange: [0, contents_start, contents_end, remaining_bytes]
        let remaining_start = contents_end;
        let remaining_length = (pdf_bytes.len() - remaining_start) as i64;

        let byte_range = [
            0,
            contents_start as i64,
            remaining_start as i64,
            remaining_length,
        ];

        // Calculate hash of the byte range
        let mut hasher = Sha256::new();
        hasher.update(&pdf_bytes[0..contents_start]);
        hasher.update(&pdf_bytes[remaining_start..]);
        let hash = hasher.finalize().into();

        Ok((byte_range, hash))
    }

    /// Build the CMS signature
    fn build_cms_signature(
        &self,
        document_hash: &[u8; 32],
        signer_name: &str,
    ) -> Result<Vec<u8>, String> {
        // Sign the document hash
        let signature = self.identity.sign_prehashed(document_hash);

        // Get public key
        let public_key = self.identity.public_key_der();

        // Get signing time in UTC format
        let signing_time = Utc::now().format("%Y%m%d%H%M%SZ").to_string();

        // Build CMS SignedData structure
        let cms = build_signed_data(
            document_hash,
            &signature,
            &public_key,
            signer_name,
            &signing_time,
        );

        Ok(cms)
    }

    /// Inject the signature into the PDF
    fn inject_signature(
        &self,
        pdf_bytes: &[u8],
        signature: &[u8],
        placeholder_size: usize,
        byte_range: &[i64; 4],
    ) -> Result<Vec<u8>, String> {
        // Convert signature to hex
        let sig_hex = hex::encode(signature);

        // Check if signature fits
        let hex_placeholder_size = placeholder_size * 2;
        if sig_hex.len() > hex_placeholder_size {
            return Err(format!(
                "Signature too large: {} bytes (max {})",
                sig_hex.len(),
                hex_placeholder_size
            ));
        }

        // Pad with zeros
        let padding = hex_placeholder_size - sig_hex.len();
        let padded_sig = format!("{}{}", sig_hex, "0".repeat(padding));

        let mut result = pdf_bytes.to_vec();

        // Find and replace ByteRange
        let byte_range_str = format!(
            "[{} {} {} {}]",
            byte_range[0], byte_range[1], byte_range[2], byte_range[3]
        );
        self.replace_byte_range(&mut result, &byte_range_str)?;

        // Find and replace Contents
        let contents_start = byte_range[1] as usize;
        result[contents_start..contents_start + hex_placeholder_size]
            .copy_from_slice(padded_sig.as_bytes());

        Ok(result)
    }

    /// Replace the ByteRange placeholder in the PDF
    fn replace_byte_range(&self, pdf_bytes: &mut [u8], byte_range_str: &str) -> Result<(), String> {
        let marker = b"/ByteRange";
        let start = self
            .find_last_occurrence(pdf_bytes, marker)
            .ok_or("Could not find /ByteRange marker")?;

        // Find the opening bracket
        let mut bracket_start = start;
        while bracket_start < pdf_bytes.len() {
            if pdf_bytes[bracket_start] == b'[' {
                break;
            }
            bracket_start += 1;
        }

        // Find the closing bracket
        let mut bracket_end = bracket_start;
        while bracket_end < pdf_bytes.len() {
            if pdf_bytes[bracket_end] == b']' {
                bracket_end += 1; // Include the ']'
                break;
            }
            bracket_end += 1;
        }

        // Replace with the actual byte range
        let current_range = &pdf_bytes[bracket_start..bracket_end];
        let new_range = byte_range_str.as_bytes();

        // Ensure we don't exceed the placeholder size
        if new_range.len() > current_range.len() {
            return Err("ByteRange string too long".to_string());
        }

        // Copy the new range and pad with spaces if needed
        pdf_bytes[bracket_start..bracket_start + new_range.len()].copy_from_slice(new_range);

        // Pad with spaces if the new range is shorter
        for byte in pdf_bytes
            .iter_mut()
            .take(bracket_end)
            .skip(bracket_start + new_range.len())
        {
            *byte = b' ';
        }

        Ok(())
    }

    /// Find the last occurrence of a pattern in bytes
    fn find_last_occurrence(&self, haystack: &[u8], needle: &[u8]) -> Option<usize> {
        let len = needle.len();
        if len == 0 || len > haystack.len() {
            return None;
        }

        (0..=(haystack.len() - len))
            .rev()
            .find(|&i| &haystack[i..i + len] == needle)
    }

    /// Add a text field (date, text, initials) to the document
    /// This creates a simple stamp annotation with the specified text
    #[allow(clippy::too_many_arguments)]
    pub fn add_text_stamp(
        &mut self,
        page: u32,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        text: &str,
        bg_color: Option<[f64; 3]>,
    ) -> Result<(), String> {
        // Create appearance stream
        let appearance = self.create_text_appearance(width, height, text, bg_color)?;
        let ap_stream_id = self.doc.doc_mut().add_object(appearance);

        // Create AP dictionary
        let mut ap_dict = Dictionary::new();
        ap_dict.set("N", Object::Reference(ap_stream_id));

        // Create stamp annotation
        let mut annot_dict = Dictionary::new();
        annot_dict.set("Type", Object::Name(b"Annot".to_vec()));
        annot_dict.set("Subtype", Object::Name(b"Stamp".to_vec()));
        annot_dict.set(
            "Rect",
            Object::Array(vec![
                Object::Real(x as f32),
                Object::Real(y as f32),
                Object::Real((x + width) as f32),
                Object::Real((y + height) as f32),
            ]),
        );
        annot_dict.set("F", Object::Integer(4)); // Print flag
        annot_dict.set("AP", Object::Dictionary(ap_dict));

        // Add page reference
        if let Some(page_id) = self.doc.page_id(page) {
            annot_dict.set("P", Object::Reference(page_id));
        }

        let annot_id = self
            .doc
            .doc_mut()
            .add_object(Object::Dictionary(annot_dict));

        // Add to page annotations
        self.add_to_page_annots(page, annot_id)?;

        Ok(())
    }

    /// Create an appearance stream for text content
    fn create_text_appearance(
        &self,
        width: f64,
        height: f64,
        text: &str,
        bg_color: Option<[f64; 3]>,
    ) -> Result<Object, String> {
        let escaped_text = escape_pdf_string(text);
        let font_size = (height * 0.6).clamp(8.0, 14.0);
        let text_y = (height - font_size) / 2.0;

        // Default to light yellow background
        let bg = bg_color.unwrap_or([1.0, 1.0, 0.9]);

        let content = format!(
            "q\n\
{r} {g} {b} rg\n\
0 0 {w} {h} re f\n\
0 0 0 RG\n\
0.5 w\n\
0 0 {w} {h} re S\n\
0 0 0 rg\n\
BT\n\
/F1 {fs} Tf\n\
4 {ty} Td\n\
({text}) Tj\n\
ET\n\
Q",
            r = bg[0],
            g = bg[1],
            b = bg[2],
            w = width,
            h = height,
            fs = font_size,
            ty = text_y,
            text = escaped_text,
        );

        let content_bytes = content.into_bytes();

        // Create resources dictionary with Helvetica font
        let mut font_dict = Dictionary::new();
        font_dict.set("Type", Object::Name(b"Font".to_vec()));
        font_dict.set("Subtype", Object::Name(b"Type1".to_vec()));
        font_dict.set("BaseFont", Object::Name(b"Helvetica".to_vec()));

        let mut fonts = Dictionary::new();
        fonts.set("F1", Object::Dictionary(font_dict));

        let mut resources = Dictionary::new();
        resources.set("Font", Object::Dictionary(fonts));

        let mut stream_dict = Dictionary::new();
        stream_dict.set("Type", Object::Name(b"XObject".to_vec()));
        stream_dict.set("Subtype", Object::Name(b"Form".to_vec()));
        stream_dict.set(
            "BBox",
            Object::Array(vec![
                Object::Integer(0),
                Object::Integer(0),
                Object::Real(width as f32),
                Object::Real(height as f32),
            ]),
        );
        stream_dict.set("Resources", Object::Dictionary(resources));
        stream_dict.set("Length", Object::Integer(content_bytes.len() as i64));

        Ok(Object::Stream(Stream::new(stream_dict, content_bytes)))
    }

    /// Add a checkbox field to the document
    pub fn add_checkbox(
        &mut self,
        page: u32,
        x: f64,
        y: f64,
        size: f64,
        checked: bool,
    ) -> Result<(), String> {
        let appearance = self.create_checkbox_appearance(size, checked)?;
        let ap_stream_id = self.doc.doc_mut().add_object(appearance);

        let mut ap_dict = Dictionary::new();
        ap_dict.set("N", Object::Reference(ap_stream_id));

        let mut annot_dict = Dictionary::new();
        annot_dict.set("Type", Object::Name(b"Annot".to_vec()));
        annot_dict.set("Subtype", Object::Name(b"Stamp".to_vec()));
        annot_dict.set(
            "Rect",
            Object::Array(vec![
                Object::Real(x as f32),
                Object::Real(y as f32),
                Object::Real((x + size) as f32),
                Object::Real((y + size) as f32),
            ]),
        );
        annot_dict.set("F", Object::Integer(4));
        annot_dict.set("AP", Object::Dictionary(ap_dict));

        if let Some(page_id) = self.doc.page_id(page) {
            annot_dict.set("P", Object::Reference(page_id));
        }

        let annot_id = self
            .doc
            .doc_mut()
            .add_object(Object::Dictionary(annot_dict));
        self.add_to_page_annots(page, annot_id)?;

        Ok(())
    }

    /// Create checkbox appearance stream
    fn create_checkbox_appearance(&self, size: f64, checked: bool) -> Result<Object, String> {
        let checkmark = if checked {
            // Draw a checkmark
            format!(
                "q\n\
0 G\n\
2 w\n\
{x1} {y1} m\n\
{x2} {y2} l\n\
{x3} {y3} l\n\
S\n\
Q",
                x1 = size * 0.2,
                y1 = size * 0.5,
                x2 = size * 0.4,
                y2 = size * 0.3,
                x3 = size * 0.8,
                y3 = size * 0.8,
            )
        } else {
            String::new()
        };

        let content = format!(
            "q\n\
1 1 1 rg\n\
0 0 {s} {s} re f\n\
0 0 0 RG\n\
1 w\n\
0 0 {s} {s} re S\n\
{check}\n\
Q",
            s = size,
            check = checkmark,
        );

        let content_bytes = content.into_bytes();

        let mut stream_dict = Dictionary::new();
        stream_dict.set("Type", Object::Name(b"XObject".to_vec()));
        stream_dict.set("Subtype", Object::Name(b"Form".to_vec()));
        stream_dict.set(
            "BBox",
            Object::Array(vec![
                Object::Integer(0),
                Object::Integer(0),
                Object::Real(size as f32),
                Object::Real(size as f32),
            ]),
        );
        stream_dict.set("Length", Object::Integer(content_bytes.len() as i64));

        Ok(Object::Stream(Stream::new(stream_dict, content_bytes)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::keys::EphemeralIdentity;

    #[test]
    fn test_signature_field_creation() {
        let field = SignatureField::new(1, "Test Signer".to_string(), "Testing".to_string());
        assert_eq!(field.page, 1);
        assert_eq!(field.signer_name, "Test Signer");
        assert_eq!(field.reason, "Testing");
    }

    #[test]
    fn test_find_last_occurrence() {
        let data = b"Hello /Contents world /Contents end";
        let needle = b"/Contents";

        // For this test, we don't need a full PDF document
        // Just test the find_last_occurrence method directly
        // We'll create a dummy PdfDocument and use the method
        let pdf_bytes = b"%PDF-1.4\n1 0 obj\n<</Type/Catalog/Pages 2 0 R>>\nendobj\n2 0 obj\n<</Type/Pages/Kids[3 0 R]/Count 1>>\nendobj\n3 0 obj\n<</Type/Page/MediaBox[0 0 612 792]/Parent 2 0 R>>\nendobj\nxref\n0 4\n0000000000 65535 f \n0000000009 00000 n \n0000000060 00000 n \n0000000118 00000 n \ntrailer\n<</Size 4/Root 1 0 R>>\nstartxref\n187\n%%EOF\n";

        // Test the search functionality on test data
        if let Ok(mut doc) = PdfDocument::from_bytes(pdf_bytes.to_vec()) {
            let identity = EphemeralIdentity::generate();
            let signer = PdfSigner::new(&mut doc, &identity);
            let result = signer.find_last_occurrence(data, needle);
            assert_eq!(result, Some(22)); // Should find the last occurrence
        } else {
            // If PDF parsing fails (due to lopdf version differences),
            // just test the core logic independently
            let dummy_bytes = b"test";
            let mut doc = PdfDocument {
                doc: lopdf::Document::new(),
                bytes: dummy_bytes.to_vec(),
            };
            let identity = EphemeralIdentity::generate();
            let signer = PdfSigner::new(&mut doc, &identity);
            let result = signer.find_last_occurrence(data, needle);
            assert_eq!(result, Some(22)); // Should find the last occurrence
        }
    }

    #[test]
    fn test_escape_pdf_string_basic() {
        assert_eq!(escape_pdf_string("Hello"), "Hello");
        assert_eq!(escape_pdf_string("(test)"), "\\(test\\)");
        assert_eq!(escape_pdf_string("back\\slash"), "back\\\\slash");
    }

    #[test]
    fn test_signature_field_with_rect() {
        let field = SignatureField::new(1, "Test".to_string(), "Reason".to_string())
            .with_rect([100.0, 200.0, 150.0, 50.0]);
        assert_eq!(field.rect, [100.0, 200.0, 150.0, 50.0]);
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        /// Property: PDF string escaping handles all special characters correctly
        #[test]
        fn escape_pdf_string_preserves_safe_chars(s in "[a-zA-Z0-9 .,!?@#$%^&*-_+=:;<>/'\"\\[\\]{}|~`]{0,100}") {
            let escaped = escape_pdf_string(&s);
            // Result should not contain unescaped parens or backslashes
            let unescaped_paren = escaped.chars().enumerate().any(|(i, c)| {
                (c == '(' || c == ')') && (i == 0 || escaped.chars().nth(i - 1) != Some('\\'))
            });
            // Allow the escaped versions
            prop_assert!(!unescaped_paren || !s.contains('(') && !s.contains(')'));
        }

        /// Property: Escaping parentheses produces valid escape sequences
        #[test]
        fn escape_parentheses_correct(s in ".*") {
            let escaped = escape_pdf_string(&s);

            // Count unescaped parens in original
            let orig_open = s.chars().filter(|&c| c == '(').count();
            let orig_close = s.chars().filter(|&c| c == ')').count();

            // Count escaped parens in result
            let escaped_open = escaped.matches("\\(").count();
            let escaped_close = escaped.matches("\\)").count();

            prop_assert_eq!(orig_open, escaped_open);
            prop_assert_eq!(orig_close, escaped_close);
        }

        /// Property: Escaping backslashes doubles them
        #[test]
        fn escape_backslashes_doubled(s in "[^\\\\]*\\\\[^\\\\]*") {
            // String with exactly one backslash
            let escaped = escape_pdf_string(&s);
            let orig_backslashes = s.chars().filter(|&c| c == '\\').count();
            let escaped_backslashes = escaped.matches("\\\\").count();
            prop_assert_eq!(orig_backslashes, escaped_backslashes);
        }

        /// Property: SignatureField::with_rect updates rectangle correctly
        #[test]
        fn signature_field_rect_update(
            x in 0.0f64..1000.0,
            y in 0.0f64..1000.0,
            w in 10.0f64..500.0,
            h in 10.0f64..200.0,
        ) {
            let field = SignatureField::new(1, "Test".to_string(), "Reason".to_string())
                .with_rect([x, y, w, h]);

            prop_assert_eq!(field.rect[0], x);
            prop_assert_eq!(field.rect[1], y);
            prop_assert_eq!(field.rect[2], w);
            prop_assert_eq!(field.rect[3], h);
        }

        /// Property: Page number is preserved in SignatureField
        #[test]
        fn signature_field_preserves_page(page in 1u32..100) {
            let field = SignatureField::new(page, "Signer".to_string(), "Reason".to_string());
            prop_assert_eq!(field.page, page);
        }

        /// Property: find_last_occurrence returns None for empty needle
        #[test]
        fn find_last_empty_needle(haystack in prop::collection::vec(any::<u8>(), 0..100)) {
            // Create a minimal signer for testing
            let mut doc = PdfDocument {
                doc: lopdf::Document::new(),
                bytes: vec![],
            };
            let identity = crate::crypto::keys::EphemeralIdentity::generate();
            let signer = PdfSigner::new(&mut doc, &identity);

            let result = signer.find_last_occurrence(&haystack, &[]);
            prop_assert!(result.is_none());
        }

        /// Property: find_last_occurrence returns None when needle is longer than haystack
        #[test]
        fn find_last_needle_too_long(
            haystack_len in 1usize..50,
            needle_len in 51usize..100,
        ) {
            let haystack: Vec<u8> = (0..haystack_len).map(|i| i as u8).collect();
            let needle: Vec<u8> = (0..needle_len).map(|i| i as u8).collect();

            let mut doc = PdfDocument {
                doc: lopdf::Document::new(),
                bytes: vec![],
            };
            let identity = crate::crypto::keys::EphemeralIdentity::generate();
            let signer = PdfSigner::new(&mut doc, &identity);

            let result = signer.find_last_occurrence(&haystack, &needle);
            prop_assert!(result.is_none());
        }

        /// Property: find_last_occurrence finds last match when multiple exist
        #[test]
        fn find_last_finds_last_match(prefix_len in 5usize..20, suffix_len in 5usize..20) {
            let needle = b"MARKER";
            let mut haystack = Vec::new();
            haystack.extend(vec![b'X'; prefix_len]);
            haystack.extend(needle); // First occurrence
            haystack.extend(vec![b'Y'; 10]);
            let expected_pos = haystack.len();
            haystack.extend(needle); // Second (last) occurrence
            haystack.extend(vec![b'Z'; suffix_len]);

            let mut doc = PdfDocument {
                doc: lopdf::Document::new(),
                bytes: vec![],
            };
            let identity = crate::crypto::keys::EphemeralIdentity::generate();
            let signer = PdfSigner::new(&mut doc, &identity);

            let result = signer.find_last_occurrence(&haystack, needle);
            prop_assert_eq!(result, Some(expected_pos));
        }
    }
}
