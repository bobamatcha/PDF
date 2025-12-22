/**
 * Mobile Optimization Tests - UX-005
 *
 * These tests verify that the signing experience is mobile-optimized.
 * Run these tests using a test runner that can simulate mobile viewports.
 *
 * For manual verification, use Puppeteer MCP (see verification steps at bottom).
 */

/**
 * Test 1: Signature modal should be full-screen on mobile (< 768px)
 */
export function testSignatureModalFullScreen() {
    // Set mobile viewport
    if (window.innerWidth < 768) {
        const modal = document.querySelector('.signature-modal .modal');
        if (!modal) {
            throw new Error('Signature modal not found');
        }

        const rect = modal.getBoundingClientRect();
        const viewportWidth = window.innerWidth;
        const viewportHeight = window.innerHeight;

        // On mobile, modal should take up the full viewport
        const isFullWidth = Math.abs(rect.width - viewportWidth) < 10; // 10px tolerance
        const isFullHeight = Math.abs(rect.height - viewportHeight) < 10;

        if (!isFullWidth || !isFullHeight) {
            throw new Error(`Signature modal is not full-screen on mobile. Width: ${rect.width} vs ${viewportWidth}, Height: ${rect.height} vs ${viewportHeight}`);
        }

        console.log('✓ Signature modal is full-screen on mobile');
        return true;
    }
    return null; // Skip test on desktop
}

/**
 * Test 2: All touch targets should be minimum 44x44px
 */
export function testTouchTargetSize() {
    const buttons = document.querySelectorAll('button, a, .field-overlay, .tab-btn');
    const minSize = 44;
    const failures = [];

    buttons.forEach((btn, idx) => {
        const rect = btn.getBoundingClientRect();
        if (rect.width < minSize || rect.height < minSize) {
            failures.push({
                element: btn.tagName + (btn.id ? `#${btn.id}` : '') + (btn.className ? `.${btn.className.split(' ')[0]}` : ''),
                width: rect.width,
                height: rect.height,
                index: idx
            });
        }
    });

    if (failures.length > 0) {
        console.error('Touch targets too small:', failures);
        throw new Error(`${failures.length} touch targets are smaller than 44x44px`);
    }

    console.log('✓ All touch targets are at least 44x44px');
    return true;
}

/**
 * Test 3: Swipe gestures should be enabled for field navigation
 */
export function testSwipeGesturesEnabled() {
    // Check if swipe handlers are attached to the viewer
    const viewerContainer = document.querySelector('.viewer-container');
    if (!viewerContainer) {
        throw new Error('Viewer container not found');
    }

    // Check if touch event listeners are present
    // This is a proxy test - we check for data attributes or handlers
    const hasSwipeSupport = viewerContainer.dataset.swipeEnabled === 'true';

    if (!hasSwipeSupport) {
        throw new Error('Swipe gesture support is not enabled on viewer container');
    }

    console.log('✓ Swipe gestures are enabled');
    return true;
}

/**
 * Test 4: Bottom sheet pattern for signature modal on mobile
 */
export function testBottomSheetPattern() {
    if (window.innerWidth < 768) {
        const modal = document.querySelector('#signature-modal .modal');
        if (!modal) {
            throw new Error('Signature modal not found');
        }

        const computedStyle = window.getComputedStyle(modal);

        // On mobile, modal should use bottom sheet pattern
        // This means it should be positioned at the bottom with border-radius on top corners only
        const hasBottomSheetClass = modal.classList.contains('bottom-sheet-mobile');

        if (!hasBottomSheetClass) {
            throw new Error('Signature modal does not use bottom sheet pattern on mobile');
        }

        console.log('✓ Bottom sheet pattern is applied on mobile');
        return true;
    }
    return null; // Skip test on desktop
}

/**
 * Test 5: No horizontal scrolling on any mobile view
 */
export function testNoHorizontalScroll() {
    const body = document.body;
    const html = document.documentElement;

    const bodyScrollWidth = body.scrollWidth;
    const bodyClientWidth = body.clientWidth;
    const htmlScrollWidth = html.scrollWidth;
    const htmlClientWidth = html.clientWidth;

    const hasHorizontalScroll =
        bodyScrollWidth > bodyClientWidth ||
        htmlScrollWidth > htmlClientWidth;

    if (hasHorizontalScroll) {
        throw new Error(`Horizontal scroll detected. Body: ${bodyScrollWidth} vs ${bodyClientWidth}, HTML: ${htmlScrollWidth} vs ${htmlClientWidth}`);
    }

    console.log('✓ No horizontal scrolling detected');
    return true;
}

/**
 * Test 6: Signature pad fills available width with proper aspect ratio
 */
export function testSignaturePadSize() {
    const canvas = document.getElementById('signature-pad');
    if (!canvas) {
        throw new Error('Signature pad canvas not found');
    }

    const parent = canvas.parentElement;
    const parentWidth = parent.getBoundingClientRect().width;
    const canvasWidth = canvas.getBoundingClientRect().width;

    // Canvas should fill at least 90% of parent width
    const fillsWidth = (canvasWidth / parentWidth) >= 0.9;

    if (!fillsWidth) {
        throw new Error(`Signature pad does not fill width. Canvas: ${canvasWidth}px, Parent: ${parentWidth}px`);
    }

    // Check aspect ratio (should be reasonable for signatures, e.g., 2:1 or wider)
    const canvasHeight = canvas.getBoundingClientRect().height;
    const aspectRatio = canvasWidth / canvasHeight;

    if (aspectRatio < 1.5) {
        throw new Error(`Signature pad aspect ratio is too narrow: ${aspectRatio.toFixed(2)}`);
    }

    console.log('✓ Signature pad fills available width with proper aspect ratio');
    return true;
}

/**
 * Test 7: Progress indicator should be visible without scrolling on mobile
 */
export function testProgressIndicatorVisibility() {
    const progress = document.querySelector('.progress');
    if (!progress || progress.classList.contains('hidden')) {
        return null; // Skip if progress not visible (flow not started)
    }

    const rect = progress.getBoundingClientRect();
    const viewportHeight = window.innerHeight;

    // Progress should be in viewport
    const isInViewport = rect.top >= 0 && rect.bottom <= viewportHeight;

    if (!isInViewport) {
        throw new Error('Progress indicator is not visible without scrolling');
    }

    console.log('✓ Progress indicator is visible without scrolling');
    return true;
}

/**
 * Test 8: "Next" button should be sticky at bottom on mobile
 */
export function testStickyNextButton() {
    if (window.innerWidth < 768) {
        const nextBtn = document.getElementById('btn-next');
        if (!nextBtn || nextBtn.classList.contains('hidden')) {
            return null; // Skip if button not visible
        }

        const computedStyle = window.getComputedStyle(nextBtn.parentElement);
        const isSticky = computedStyle.position === 'fixed' || computedStyle.position === 'sticky';

        if (!isSticky) {
            throw new Error('Next button is not sticky on mobile');
        }

        console.log('✓ Next button is sticky at bottom on mobile');
        return true;
    }
    return null; // Skip test on desktop
}

/**
 * Run all tests
 */
export async function runAllMobileTests() {
    console.log('Running mobile optimization tests...\n');

    const tests = [
        { name: 'Signature modal full-screen on mobile', fn: testSignatureModalFullScreen },
        { name: 'Touch target sizes (44x44px minimum)', fn: testTouchTargetSize },
        { name: 'Swipe gestures enabled', fn: testSwipeGesturesEnabled },
        { name: 'Bottom sheet pattern on mobile', fn: testBottomSheetPattern },
        { name: 'No horizontal scrolling', fn: testNoHorizontalScroll },
        { name: 'Signature pad sizing', fn: testSignaturePadSize },
        { name: 'Progress indicator visibility', fn: testProgressIndicatorVisibility },
        { name: 'Sticky next button on mobile', fn: testStickyNextButton }
    ];

    const results = [];

    for (const test of tests) {
        try {
            const result = test.fn();
            if (result !== null) {
                results.push({ name: test.name, passed: true });
            } else {
                results.push({ name: test.name, passed: true, skipped: true });
            }
        } catch (err) {
            results.push({ name: test.name, passed: false, error: err.message });
            console.error(`✗ ${test.name}: ${err.message}`);
        }
    }

    console.log('\n=== Test Results ===');
    const passed = results.filter(r => r.passed && !r.skipped).length;
    const failed = results.filter(r => !r.passed).length;
    const skipped = results.filter(r => r.skipped).length;

    console.log(`Passed: ${passed}`);
    console.log(`Failed: ${failed}`);
    console.log(`Skipped: ${skipped}`);

    if (failed > 0) {
        throw new Error(`${failed} tests failed`);
    }

    return results;
}

// Expose for console testing
if (typeof window !== 'undefined') {
    window.mobileTests = {
        runAll: runAllMobileTests,
        testSignatureModalFullScreen,
        testTouchTargetSize,
        testSwipeGesturesEnabled,
        testBottomSheetPattern,
        testNoHorizontalScroll,
        testSignaturePadSize,
        testProgressIndicatorVisibility,
        testStickyNextButton
    };
}

/**
 * PUPPETEER VERIFICATION STEPS
 *
 * Run these steps manually using Puppeteer MCP to verify mobile behavior:
 *
 * 1. Set viewport to iPhone SE (375x667):
 *    await page.setViewport({ width: 375, height: 667 });
 *
 * 2. Navigate to signing URL:
 *    await page.goto('http://localhost:8081/sign.html?session=test&recipient=r1&key=test123');
 *
 * 3. Take screenshot of initial view:
 *    await page.screenshot({ path: 'mobile-initial.png' });
 *
 * 4. Start guided flow:
 *    await page.click('#btn-start');
 *
 * 5. Take screenshot of guided flow:
 *    await page.screenshot({ path: 'mobile-guided-flow.png' });
 *
 * 6. Open signature modal by clicking first field:
 *    await page.click('.field-overlay[data-own-index="0"]');
 *
 * 7. Take screenshot of signature modal (should be full-screen):
 *    await page.screenshot({ path: 'mobile-signature-modal.png' });
 *
 * 8. Verify modal fills viewport:
 *    const modal = await page.$('#signature-modal .modal');
 *    const box = await modal.boundingBox();
 *    console.log('Modal dimensions:', box);
 *    // Should be close to 375x667
 *
 * 9. Verify touch target sizes:
 *    const buttons = await page.$$('button');
 *    for (const btn of buttons) {
 *      const box = await btn.boundingBox();
 *      console.log('Button:', box.width, 'x', box.height);
 *      // All should be >= 44x44px
 *    }
 *
 * 10. Test swipe navigation:
 *     await page.touchscreen.tap(200, 400);
 *     await page.evaluate(() => {
 *       const event = new TouchEvent('touchstart', { touches: [{ clientX: 300, clientY: 400 }] });
 *       document.querySelector('.viewer-container').dispatchEvent(event);
 *     });
 *
 * 11. Test landscape orientation (iPhone SE landscape: 667x375):
 *     await page.setViewport({ width: 667, height: 375 });
 *     await page.screenshot({ path: 'mobile-landscape.png' });
 *
 * 12. Test larger device (iPhone 11: 414x896):
 *     await page.setViewport({ width: 414, height: 896 });
 *     await page.screenshot({ path: 'mobile-iphone11.png' });
 *
 * 13. Run in-browser tests:
 *     await page.evaluate(() => window.mobileTests.runAll());
 */
