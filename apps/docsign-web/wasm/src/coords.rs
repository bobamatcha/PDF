//! Coordinate transformation between DOM and PDF coordinate systems

/// Convert DOM coordinates (top-left origin, pixels) to PDF coordinates (bottom-left origin, points)
pub fn dom_to_pdf(
    dom_x: f64,
    dom_y: f64,
    container_width: f64,
    container_height: f64,
    media_box: [f64; 4],
) -> (f64, f64) {
    let [mb_x, mb_y, mb_width, mb_height] = media_box;

    // Convert to percentage
    let x_pct = dom_x / container_width;
    let y_pct = dom_y / container_height;

    // Convert to PDF coordinates (flip Y axis)
    let pdf_x = mb_x + (x_pct * mb_width);
    let pdf_y = mb_y + (mb_height - (y_pct * mb_height));

    (pdf_x, pdf_y)
}

/// Convert PDF coordinates to DOM coordinates
pub fn pdf_to_dom(
    pdf_x: f64,
    pdf_y: f64,
    container_width: f64,
    container_height: f64,
    media_box: [f64; 4],
) -> (f64, f64) {
    let [mb_x, mb_y, mb_width, mb_height] = media_box;

    // Convert to percentage
    let x_pct = (pdf_x - mb_x) / mb_width;
    let y_pct = 1.0 - ((pdf_y - mb_y) / mb_height); // Flip Y axis

    // Convert to DOM coordinates
    let dom_x = x_pct * container_width;
    let dom_y = y_pct * container_height;

    (dom_x, dom_y)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dom_to_pdf_center() {
        let media_box = [0.0, 0.0, 612.0, 792.0]; // Letter size
        let (pdf_x, pdf_y) = dom_to_pdf(300.0, 396.0, 600.0, 792.0, media_box);
        assert!((pdf_x - 306.0).abs() < 0.1);
        assert!((pdf_y - 396.0).abs() < 0.1);
    }

    #[test]
    fn test_corners() {
        let media_box = [0.0, 0.0, 612.0, 792.0];
        let container = (600.0, 800.0);

        // Top-left DOM (0, 0) should map to top-left PDF (0, 792)
        let (x, y) = dom_to_pdf(0.0, 0.0, container.0, container.1, media_box);
        assert!((x - 0.0).abs() < 0.1);
        assert!((y - 792.0).abs() < 0.1);

        // Bottom-right DOM should map to bottom-right PDF
        let (x, y) = dom_to_pdf(
            container.0,
            container.1,
            container.0,
            container.1,
            media_box,
        );
        assert!((x - 612.0).abs() < 0.1);
        assert!((y - 0.0).abs() < 0.1);
    }

    #[test]
    fn test_overlay_coordinates_round_trip() {
        let pdf_coord = (100.0, 200.0);
        let page_height = 792.0; // Letter size
        let page_width = 612.0;
        let scale = 1.5;
        let container_width = page_width * scale;
        let container_height = page_height * scale;
        let media_box = [0.0, 0.0, page_width, page_height];

        let dom = pdf_to_dom(
            pdf_coord.0,
            pdf_coord.1,
            container_width,
            container_height,
            media_box,
        );
        let back = dom_to_pdf(dom.0, dom.1, container_width, container_height, media_box);
        assert!((back.0 - pdf_coord.0).abs() < 0.001);
        assert!((back.1 - pdf_coord.1).abs() < 0.001);
    }

    #[test]
    fn test_y_axis_flip() {
        // PDF origin is bottom-left, DOM origin is top-left
        let pdf_y = 100.0;
        let page_height = 792.0;
        let page_width = 612.0;
        let scale = 1.0;
        let container_width = page_width * scale;
        let container_height = page_height * scale;
        let media_box = [0.0, 0.0, page_width, page_height];

        let dom = pdf_to_dom(0.0, pdf_y, container_width, container_height, media_box);
        // DOM y should be (792 - 100) * 1.0 = 692
        assert_eq!(dom.1, 692.0);
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    // Strategy for valid positive dimensions (1.0 to 2000.0 points/pixels)
    fn dimension() -> impl Strategy<Value = f64> {
        1.0f64..2000.0
    }

    // Strategy for a percentage (0.0 to 1.0)
    fn percentage() -> impl Strategy<Value = f64> {
        0.0f64..=1.0
    }

    proptest! {
        // =========================================================================
        // Requirement 1: DOM->PDF->DOM roundtrip invariant
        // =========================================================================

        /// Property: DOM->PDF->DOM roundtrip returns original coordinates (within tolerance)
        /// Tests with arbitrary coordinates within the container bounds
        #[test]
        fn roundtrip_dom_to_pdf_to_dom(
            container_w in dimension(),
            container_h in dimension(),
            pdf_w in dimension(),
            pdf_h in dimension(),
            x_pct in percentage(),
            y_pct in percentage(),
        ) {
            let media_box = [0.0, 0.0, pdf_w, pdf_h];

            // Generate DOM coordinates within container bounds
            let dom_x = x_pct * container_w;
            let dom_y = y_pct * container_h;

            let (pdf_x, pdf_y) = dom_to_pdf(dom_x, dom_y, container_w, container_h, media_box);
            let (back_x, back_y) = pdf_to_dom(pdf_x, pdf_y, container_w, container_h, media_box);

            let tolerance = 0.0001;
            prop_assert!(
                (back_x - dom_x).abs() < tolerance,
                "DOM->PDF->DOM X roundtrip failed: {} -> {} -> {} (expected {})",
                dom_x, pdf_x, back_x, dom_x
            );
            prop_assert!(
                (back_y - dom_y).abs() < tolerance,
                "DOM->PDF->DOM Y roundtrip failed: {} -> {} -> {} (expected {})",
                dom_y, pdf_y, back_y, dom_y
            );
        }

        // =========================================================================
        // Requirement 2: PDF->DOM->PDF roundtrip invariant (reverse)
        // =========================================================================

        /// Property: PDF->DOM->PDF roundtrip returns original coordinates
        /// Tests with arbitrary coordinates within the PDF bounds
        #[test]
        fn roundtrip_pdf_to_dom_to_pdf(
            container_w in dimension(),
            container_h in dimension(),
            pdf_w in dimension(),
            pdf_h in dimension(),
            x_pct in percentage(),
            y_pct in percentage(),
        ) {
            let media_box = [0.0, 0.0, pdf_w, pdf_h];

            // Generate PDF coordinates within PDF bounds
            let pdf_x = x_pct * pdf_w;
            let pdf_y = y_pct * pdf_h;

            let (dom_x, dom_y) = pdf_to_dom(pdf_x, pdf_y, container_w, container_h, media_box);
            let (back_x, back_y) = dom_to_pdf(dom_x, dom_y, container_w, container_h, media_box);

            let tolerance = 0.0001;
            prop_assert!(
                (back_x - pdf_x).abs() < tolerance,
                "PDF->DOM->PDF X roundtrip failed: {} -> {} -> {} (expected {})",
                pdf_x, dom_x, back_x, pdf_x
            );
            prop_assert!(
                (back_y - pdf_y).abs() < tolerance,
                "PDF->DOM->PDF Y roundtrip failed: {} -> {} -> {} (expected {})",
                pdf_y, dom_y, back_y, pdf_y
            );
        }

        // =========================================================================
        // Requirement 3: Scale preservation
        // =========================================================================

        /// Property: Scaling preserves relative positions
        /// When container is scaled, the relative positions of points should be preserved
        #[test]
        fn scale_preservation_relative_positions(
            pdf_w in dimension(),
            pdf_h in dimension(),
            scale in 0.5f64..3.0,
            x_pct in percentage(),
            y_pct in percentage(),
        ) {
            let media_box = [0.0, 0.0, pdf_w, pdf_h];

            // Container at 1x scale
            let container_w_1x = pdf_w;
            let container_h_1x = pdf_h;

            // Container at arbitrary scale
            let container_w_scaled = pdf_w * scale;
            let container_h_scaled = pdf_h * scale;

            // A point at the same percentage position in both containers
            let dom_x_1x = x_pct * container_w_1x;
            let dom_y_1x = y_pct * container_h_1x;
            let dom_x_scaled = x_pct * container_w_scaled;
            let dom_y_scaled = y_pct * container_h_scaled;

            // Both should map to the same PDF coordinates
            let (pdf_x_1x, pdf_y_1x) = dom_to_pdf(dom_x_1x, dom_y_1x, container_w_1x, container_h_1x, media_box);
            let (pdf_x_scaled, pdf_y_scaled) = dom_to_pdf(dom_x_scaled, dom_y_scaled, container_w_scaled, container_h_scaled, media_box);

            let tolerance = 0.0001;
            prop_assert!(
                (pdf_x_1x - pdf_x_scaled).abs() < tolerance,
                "Scale preservation failed for X: 1x={}, scaled={}",
                pdf_x_1x, pdf_x_scaled
            );
            prop_assert!(
                (pdf_y_1x - pdf_y_scaled).abs() < tolerance,
                "Scale preservation failed for Y: 1x={}, scaled={}",
                pdf_y_1x, pdf_y_scaled
            );
        }

        /// Property: Linear scaling - moving twice as far in DOM moves twice as far in PDF
        #[test]
        fn linear_scaling_property(
            container_w in dimension(),
            container_h in dimension(),
            pdf_w in dimension(),
            pdf_h in dimension(),
            base_pct in 0.1f64..0.4, // Keep base small so 2x doesn't exceed bounds
        ) {
            let media_box = [0.0, 0.0, pdf_w, pdf_h];

            // Test X axis linearity
            let (x1, _) = dom_to_pdf(container_w * base_pct, 0.0, container_w, container_h, media_box);
            let (x2, _) = dom_to_pdf(container_w * base_pct * 2.0, 0.0, container_w, container_h, media_box);

            // x2 should be twice as far from origin as x1
            let tolerance = 0.0001;
            prop_assert!(
                (x2 - 2.0 * x1).abs() < tolerance,
                "Linear scaling failed: x1={}, x2={}, expected x2={}",
                x1, x2, 2.0 * x1
            );
        }

        // =========================================================================
        // Requirement 4: Y-axis inversion
        // =========================================================================

        /// Property: Y-axis is correctly inverted between coordinate systems
        /// PDF Y increases upward, DOM Y increases downward
        /// A point at the TOP in DOM (y=0) should be at the TOP in PDF (y=height)
        #[test]
        fn y_axis_inversion_top(
            container_w in dimension(),
            container_h in dimension(),
            pdf_w in dimension(),
            pdf_h in dimension(),
            x_pct in percentage(),
        ) {
            let media_box = [0.0, 0.0, pdf_w, pdf_h];

            // DOM top (y=0) should map to PDF top (y=pdf_h)
            let dom_x = x_pct * container_w;
            let dom_y = 0.0;

            let (_, pdf_y) = dom_to_pdf(dom_x, dom_y, container_w, container_h, media_box);

            let tolerance = 0.0001;
            prop_assert!(
                (pdf_y - pdf_h).abs() < tolerance,
                "Y-axis inversion at top failed: DOM y=0 should map to PDF y={}, got {}",
                pdf_h, pdf_y
            );
        }

        /// Property: Y-axis inversion at bottom
        /// A point at the BOTTOM in DOM (y=container_h) should be at the BOTTOM in PDF (y=0)
        #[test]
        fn y_axis_inversion_bottom(
            container_w in dimension(),
            container_h in dimension(),
            pdf_w in dimension(),
            pdf_h in dimension(),
            x_pct in percentage(),
        ) {
            let media_box = [0.0, 0.0, pdf_w, pdf_h];

            // DOM bottom (y=container_h) should map to PDF bottom (y=0)
            let dom_x = x_pct * container_w;
            let dom_y = container_h;

            let (_, pdf_y) = dom_to_pdf(dom_x, dom_y, container_w, container_h, media_box);

            let tolerance = 0.0001;
            prop_assert!(
                pdf_y.abs() < tolerance,
                "Y-axis inversion at bottom failed: DOM y={} should map to PDF y=0, got {}",
                container_h, pdf_y
            );
        }

        /// Property: Moving down in DOM moves up in PDF (and vice versa)
        /// Increasing DOM Y should decrease PDF Y
        #[test]
        fn y_axis_movement_direction(
            container_w in dimension(),
            container_h in dimension(),
            pdf_w in dimension(),
            pdf_h in dimension(),
            x_pct in percentage(),
            y1_pct in 0.0f64..0.5,
        ) {
            let media_box = [0.0, 0.0, pdf_w, pdf_h];
            let y2_pct = y1_pct + 0.1; // y2 is further down in DOM

            let dom_x = x_pct * container_w;
            let dom_y1 = y1_pct * container_h;
            let dom_y2 = y2_pct * container_h;

            let (_, pdf_y1) = dom_to_pdf(dom_x, dom_y1, container_w, container_h, media_box);
            let (_, pdf_y2) = dom_to_pdf(dom_x, dom_y2, container_w, container_h, media_box);

            // Moving down in DOM (y2 > y1) should move DOWN in PDF (pdf_y2 < pdf_y1)
            prop_assert!(
                pdf_y2 < pdf_y1,
                "Y-axis direction failed: DOM y {} -> {} should mean PDF y {} -> {} (decreasing)",
                dom_y1, dom_y2, pdf_y1, pdf_y2
            );
        }

        // =========================================================================
        // Requirement 5: Origin transformation
        // =========================================================================

        /// Property: DOM origin (0,0) maps to PDF top-left corner
        /// DOM top-left (0,0) should map to PDF (0, pdf_height)
        #[test]
        fn origin_dom_to_pdf(
            container_w in dimension(),
            container_h in dimension(),
            pdf_w in dimension(),
            pdf_h in dimension(),
        ) {
            let media_box = [0.0, 0.0, pdf_w, pdf_h];

            let (pdf_x, pdf_y) = dom_to_pdf(0.0, 0.0, container_w, container_h, media_box);

            let tolerance = 0.0001;
            prop_assert!(
                pdf_x.abs() < tolerance,
                "DOM origin X should map to PDF x=0, got {}",
                pdf_x
            );
            prop_assert!(
                (pdf_y - pdf_h).abs() < tolerance,
                "DOM origin Y should map to PDF y={} (top), got {}",
                pdf_h, pdf_y
            );
        }

        /// Property: PDF origin (0,0) maps to DOM bottom-left corner
        /// PDF bottom-left (0,0) should map to DOM (0, container_height)
        #[test]
        fn origin_pdf_to_dom(
            container_w in dimension(),
            container_h in dimension(),
            pdf_w in dimension(),
            pdf_h in dimension(),
        ) {
            let media_box = [0.0, 0.0, pdf_w, pdf_h];

            let (dom_x, dom_y) = pdf_to_dom(0.0, 0.0, container_w, container_h, media_box);

            let tolerance = 0.0001;
            prop_assert!(
                dom_x.abs() < tolerance,
                "PDF origin X should map to DOM x=0, got {}",
                dom_x
            );
            prop_assert!(
                (dom_y - container_h).abs() < tolerance,
                "PDF origin Y should map to DOM y={} (bottom), got {}",
                container_h, dom_y
            );
        }

        /// Property: Non-zero media box origin is handled correctly in roundtrips
        #[test]
        fn offset_media_box_roundtrip(
            offset_x in 0.0f64..100.0,
            offset_y in 0.0f64..100.0,
            pdf_w in dimension(),
            pdf_h in dimension(),
            container_w in dimension(),
            container_h in dimension(),
            x_pct in percentage(),
            y_pct in percentage(),
        ) {
            let media_box = [offset_x, offset_y, pdf_w, pdf_h];

            let dom_x = x_pct * container_w;
            let dom_y = y_pct * container_h;

            let (pdf_x, pdf_y) = dom_to_pdf(dom_x, dom_y, container_w, container_h, media_box);
            let (back_x, back_y) = pdf_to_dom(pdf_x, pdf_y, container_w, container_h, media_box);

            let tolerance = 0.0001;
            prop_assert!(
                (back_x - dom_x).abs() < tolerance,
                "Offset media box roundtrip X failed: {} -> {} -> {}",
                dom_x, pdf_x, back_x
            );
            prop_assert!(
                (back_y - dom_y).abs() < tolerance,
                "Offset media box roundtrip Y failed: {} -> {} -> {}",
                dom_y, pdf_y, back_y
            );
        }

        /// Property: DOM origin maps correctly with offset media box
        #[test]
        fn origin_with_offset_media_box(
            offset_x in 0.0f64..100.0,
            offset_y in 0.0f64..100.0,
            pdf_w in dimension(),
            pdf_h in dimension(),
            container_w in dimension(),
            container_h in dimension(),
        ) {
            let media_box = [offset_x, offset_y, pdf_w, pdf_h];

            // DOM origin should map to (offset_x, offset_y + pdf_h)
            let (pdf_x, pdf_y) = dom_to_pdf(0.0, 0.0, container_w, container_h, media_box);

            let tolerance = 0.0001;
            prop_assert!(
                (pdf_x - offset_x).abs() < tolerance,
                "Offset media box origin X failed: expected {}, got {}",
                offset_x, pdf_x
            );
            prop_assert!(
                (pdf_y - (offset_y + pdf_h)).abs() < tolerance,
                "Offset media box origin Y failed: expected {}, got {}",
                offset_y + pdf_h, pdf_y
            );
        }
    }
}
