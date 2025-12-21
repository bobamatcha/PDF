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

    // Strategy for valid positive dimensions
    fn dimension() -> impl Strategy<Value = f64> {
        1.0f64..2000.0
    }

    // Strategy for coordinates within a container (unused but kept for potential future use)
    #[allow(dead_code)]
    fn coord_in_range(max: f64) -> impl Strategy<Value = f64> {
        0.0f64..=max
    }

    proptest! {
        /// Property: DOM→PDF→DOM roundtrip returns original coordinates (within tolerance)
        #[test]
        fn roundtrip_dom_pdf_dom(
            container_w in dimension(),
            container_h in dimension(),
            pdf_w in dimension(),
            pdf_h in dimension(),
        ) {
            let media_box = [0.0, 0.0, pdf_w, pdf_h];

            // Test point at 50% position
            let dom_x = container_w / 2.0;
            let dom_y = container_h / 2.0;

            let (pdf_x, pdf_y) = dom_to_pdf(dom_x, dom_y, container_w, container_h, media_box);
            let (back_x, back_y) = pdf_to_dom(pdf_x, pdf_y, container_w, container_h, media_box);

            let tolerance = 0.0001;
            prop_assert!((back_x - dom_x).abs() < tolerance, "X: {} vs {}", back_x, dom_x);
            prop_assert!((back_y - dom_y).abs() < tolerance, "Y: {} vs {}", back_y, dom_y);
        }

        /// Property: PDF→DOM→PDF roundtrip returns original coordinates
        #[test]
        fn roundtrip_pdf_dom_pdf(
            container_w in dimension(),
            container_h in dimension(),
            pdf_w in dimension(),
            pdf_h in dimension(),
        ) {
            let media_box = [0.0, 0.0, pdf_w, pdf_h];

            // Test point at center of PDF
            let pdf_x = pdf_w / 2.0;
            let pdf_y = pdf_h / 2.0;

            let (dom_x, dom_y) = pdf_to_dom(pdf_x, pdf_y, container_w, container_h, media_box);
            let (back_x, back_y) = dom_to_pdf(dom_x, dom_y, container_w, container_h, media_box);

            let tolerance = 0.0001;
            prop_assert!((back_x - pdf_x).abs() < tolerance, "X: {} vs {}", back_x, pdf_x);
            prop_assert!((back_y - pdf_y).abs() < tolerance, "Y: {} vs {}", back_y, pdf_y);
        }

        /// Property: Origin maps correctly (DOM top-left → PDF top-left)
        #[test]
        fn origin_mapping(
            container_w in dimension(),
            container_h in dimension(),
            pdf_w in dimension(),
            pdf_h in dimension(),
        ) {
            let media_box = [0.0, 0.0, pdf_w, pdf_h];

            // DOM (0,0) is top-left, should map to PDF top (x=0, y=height)
            let (pdf_x, pdf_y) = dom_to_pdf(0.0, 0.0, container_w, container_h, media_box);
            prop_assert!((pdf_x - 0.0).abs() < 0.0001);
            prop_assert!((pdf_y - pdf_h).abs() < 0.0001);
        }

        /// Property: Linear scaling - doubling the position doubles the relative change
        #[test]
        fn linear_scaling(
            container_w in dimension(),
            container_h in dimension(),
            pdf_w in dimension(),
            pdf_h in dimension(),
        ) {
            let media_box = [0.0, 0.0, pdf_w, pdf_h];

            let (x1, _) = dom_to_pdf(container_w * 0.25, 0.0, container_w, container_h, media_box);
            let (x2, _) = dom_to_pdf(container_w * 0.50, 0.0, container_w, container_h, media_box);

            // x2 should be twice as far from origin as x1
            let tolerance = 0.0001;
            prop_assert!((x2 - 2.0 * x1).abs() < tolerance);
        }

        /// Property: Non-zero media box origin is handled correctly
        #[test]
        fn offset_media_box(
            offset_x in 0.0f64..100.0,
            offset_y in 0.0f64..100.0,
            pdf_w in dimension(),
            pdf_h in dimension(),
        ) {
            let media_box = [offset_x, offset_y, pdf_w, pdf_h];

            // DOM origin should map to (offset_x, offset_y + pdf_h)
            let (pdf_x, pdf_y) = dom_to_pdf(0.0, 0.0, 100.0, 100.0, media_box);
            prop_assert!((pdf_x - offset_x).abs() < 0.0001);
            prop_assert!((pdf_y - (offset_y + pdf_h)).abs() < 0.0001);
        }
    }
}
