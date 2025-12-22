# Puppeteer Verification Guide: UX-005 Mobile-Optimized Signing

This document provides step-by-step instructions for verifying the mobile optimizations using Puppeteer MCP.

## Prerequisites

1. Start the development server:
   ```bash
   cd apps/docsign-web
   trunk serve --port 8081
   ```

2. Ensure Puppeteer MCP is available in Claude Desktop

## Test Scenarios

### Scenario 1: iPhone SE Portrait (375x667)

**Step 1: Navigate to signing page with mobile viewport**
```javascript
await page.setViewport({ width: 375, height: 667, deviceScaleFactor: 2 });
await page.goto('http://localhost:8081/sign.html?session=test&recipient=r1&key=test123');
```

**Step 2: Take screenshot of initial view**
```javascript
await page.screenshot({ path: 'mobile-iphone-se-initial.png', fullPage: true });
```

**Expected Result:**
- Header should be compact (reduced padding)
- Loading indicator should be centered and properly sized
- No horizontal scrolling

**Step 3: Wait for document to load and start guided flow**
```javascript
await page.waitForSelector('#btn-start', { visible: true, timeout: 5000 });
await page.click('#btn-start');
await page.waitForTimeout(500); // Wait for animation
```

**Step 4: Verify progress indicator visibility**
```javascript
const progress = await page.$('.progress');
const progressBox = await progress.boundingBox();
console.log('Progress indicator position:', progressBox);
// Should be visible within viewport (top < 667)
```

**Expected Result:**
- Progress indicator should be visible at top of screen
- Navigation buttons should be at least 44x44px
- Toolbar should be sticky

**Step 5: Verify touch target sizes**
```javascript
const buttons = await page.$$('button:not(.hidden)');
for (const btn of buttons) {
  const box = await btn.boundingBox();
  const text = await page.evaluate(el => el.textContent, btn);
  console.log(`Button "${text}": ${box.width}x${box.height}`);
  if (box.width < 44 || box.height < 44) {
    console.error(`FAIL: Button too small!`);
  }
}
```

**Expected Result:**
- All buttons should be >= 44x44px

**Step 6: Open signature modal**
```javascript
await page.click('.field-overlay[data-own-index="0"]');
await page.waitForSelector('#signature-modal:not(.hidden)', { visible: true });
await page.waitForTimeout(500); // Wait for slide-up animation
```

**Step 7: Verify signature modal is bottom sheet**
```javascript
const modal = await page.$('#signature-modal .modal');
const modalBox = await modal.boundingBox();
const hasBottomSheet = await page.evaluate(() => {
  const modal = document.querySelector('#signature-modal .modal');
  return modal.classList.contains('bottom-sheet-mobile');
});

console.log('Modal dimensions:', modalBox);
console.log('Has bottom-sheet class:', hasBottomSheet);
console.log('Modal bottom position:', modalBox.y + modalBox.height);
```

**Expected Result:**
- Modal should have `bottom-sheet-mobile` class
- Modal should be positioned at bottom of screen
- Modal should have rounded corners on top only
- Modal height should be ~90% of viewport (max 600px)

**Step 8: Take screenshot of signature modal**
```javascript
await page.screenshot({ path: 'mobile-signature-modal-iphone-se.png', fullPage: false });
```

**Step 9: Verify signature canvas size**
```javascript
const canvas = await page.$('#signature-pad');
const canvasBox = await canvas.boundingBox();
const parentBox = await (await canvas.evaluateHandle(el => el.parentElement)).asElement().boundingBox();

console.log('Canvas size:', canvasBox.width, 'x', canvasBox.height);
console.log('Parent size:', parentBox.width);
console.log('Fill percentage:', (canvasBox.width / parentBox.width * 100).toFixed(1) + '%');
```

**Expected Result:**
- Canvas should fill ~100% of parent width
- Canvas height should be ~250px
- Aspect ratio should be > 1.5:1

**Step 10: Test drawing on signature pad**
```javascript
// Simulate touch drawing
await page.touchscreen.tap(canvasBox.x + 50, canvasBox.y + 100);
await page.evaluate(() => {
  const canvas = document.getElementById('signature-pad');
  const ctx = canvas.getContext('2d');
  ctx.beginPath();
  ctx.moveTo(50, 100);
  ctx.lineTo(200, 100);
  ctx.stroke();
});
await page.screenshot({ path: 'mobile-signature-drawn.png' });
```

**Step 11: Test swipe gestures**
```javascript
// Close signature modal first
await page.click('#cancel-signature');
await page.waitForTimeout(300);

// Simulate swipe left (next field)
await page.evaluate(() => {
  const viewer = document.querySelector('.viewer-container');
  const startEvent = new TouchEvent('touchstart', {
    touches: [{ clientX: 300, clientY: 400, screenX: 300, screenY: 400 }],
    bubbles: true
  });
  const endEvent = new TouchEvent('touchend', {
    changedTouches: [{ clientX: 100, clientY: 400, screenX: 100, screenY: 400 }],
    bubbles: true
  });
  viewer.dispatchEvent(startEvent);
  viewer.dispatchEvent(endEvent);
});

await page.waitForTimeout(500);
const currentIndex = await page.evaluate(() => {
  return parseInt(document.getElementById('current').textContent);
});
console.log('Current field after swipe:', currentIndex);
```

**Expected Result:**
- Swipe left should advance to next field
- Current index should increase

**Step 12: Verify no horizontal scroll**
```javascript
const scrollWidth = await page.evaluate(() => {
  return {
    bodyScrollWidth: document.body.scrollWidth,
    bodyClientWidth: document.body.clientWidth,
    htmlScrollWidth: document.documentElement.scrollWidth,
    htmlClientWidth: document.documentElement.clientWidth
  };
});
console.log('Scroll dimensions:', scrollWidth);

const hasHorizontalScroll =
  scrollWidth.bodyScrollWidth > scrollWidth.bodyClientWidth ||
  scrollWidth.htmlScrollWidth > scrollWidth.htmlClientWidth;

console.log('Has horizontal scroll:', hasHorizontalScroll);
```

**Expected Result:**
- No horizontal scrolling should be detected

### Scenario 2: iPhone SE Landscape (667x375)

**Step 1: Rotate to landscape**
```javascript
await page.setViewport({ width: 667, height: 375, deviceScaleFactor: 2 });
await page.screenshot({ path: 'mobile-landscape-initial.png', fullPage: true });
```

**Step 2: Open signature modal**
```javascript
await page.click('.field-overlay[data-own-index="0"]');
await page.waitForSelector('#signature-modal:not(.hidden)', { visible: true });
await page.screenshot({ path: 'mobile-landscape-signature-modal.png' });
```

**Expected Result:**
- Modal should adapt to landscape (max-height: 95vh)
- Canvas height should be ~180px
- Header should be more compact
- All content should be accessible

### Scenario 3: iPhone 11 Portrait (414x896)

**Step 1: Set larger mobile viewport**
```javascript
await page.setViewport({ width: 414, height: 896, deviceScaleFactor: 2 });
await page.goto('http://localhost:8081/sign.html?session=test&recipient=r1&key=test123');
await page.waitForSelector('#btn-start');
```

**Step 2: Verify layout scales properly**
```javascript
await page.screenshot({ path: 'mobile-iphone11-initial.png', fullPage: true });
await page.click('#btn-start');
await page.click('.field-overlay[data-own-index="0"]');
await page.screenshot({ path: 'mobile-iphone11-signature-modal.png' });
```

**Expected Result:**
- All mobile optimizations should still apply
- More vertical space for signature canvas
- Touch targets remain >= 44x44px

### Scenario 4: Tablet (iPad Mini - 768x1024)

**Step 1: Set tablet viewport**
```javascript
await page.setViewport({ width: 768, height: 1024, deviceScaleFactor: 2 });
await page.goto('http://localhost:8081/sign.html?session=test&recipient=r1&key=test123');
```

**Step 2: Verify desktop-like layout**
```javascript
await page.screenshot({ path: 'tablet-initial.png', fullPage: true });
await page.click('#btn-start');
await page.click('.field-overlay[data-own-index="0"]');
await page.screenshot({ path: 'tablet-signature-modal.png' });
```

**Expected Result:**
- Modal should NOT be full-screen (max-width: 700px)
- Modal should be centered, not bottom sheet
- Touch targets still optimized (44x44px)

### Scenario 5: Run In-Browser Tests

**Execute the mobile test suite**
```javascript
const testResults = await page.evaluate(() => {
  return window.mobileTests.runAll();
});

console.log('Test Results:', testResults);
```

**Expected Result:**
- All tests should pass (or be skipped if not on mobile viewport)
- No test failures

## Summary Checklist

After running all scenarios, verify:

- [ ] Signature modal is full-screen on mobile (< 768px)
- [ ] Bottom sheet pattern applied on mobile
- [ ] All touch targets >= 44x44px
- [ ] Swipe gestures work (left = next, right = previous)
- [ ] No horizontal scrolling on any viewport
- [ ] Signature canvas fills width on mobile
- [ ] Progress indicator visible without scrolling
- [ ] Toolbar is sticky on mobile
- [ ] Modal buttons stack vertically on mobile
- [ ] Landscape orientation supported
- [ ] Tablet uses hybrid layout (touch-friendly but not full-screen)
- [ ] Touch visual feedback on active state
- [ ] Smooth animations (slide-up for bottom sheet)

## Common Issues to Check

1. **Modal not full-screen on mobile:**
   - Check that media query `@media (max-width: 767px)` is applied
   - Verify `.bottom-sheet-mobile` class is added in JavaScript

2. **Touch targets too small:**
   - Check that `min-height: 44px` and `min-width: 44px` are applied
   - Use browser DevTools to measure actual rendered sizes

3. **Swipe gestures not working:**
   - Verify `data-swipe-enabled="true"` is set on `.viewer-container`
   - Check browser console for JavaScript errors
   - Ensure touch events are not prevented elsewhere

4. **Horizontal scrolling detected:**
   - Check that all elements have `max-width: 100%` or proper constraints
   - Look for fixed-width elements that exceed viewport

5. **Canvas too small:**
   - Verify CSS sets `width: 100%` and `height: 250px` on mobile
   - Check that `_initCanvas()` properly calculates dimensions

## Automated Test Script

Save this as `puppeteer-mobile-test.js` and run with Node.js:

```javascript
const puppeteer = require('puppeteer');

(async () => {
  const browser = await puppeteer.launch({ headless: false });
  const page = await browser.newPage();

  const devices = [
    { name: 'iPhone SE', width: 375, height: 667 },
    { name: 'iPhone 11', width: 414, height: 896 },
    { name: 'iPad Mini', width: 768, height: 1024 }
  ];

  for (const device of devices) {
    console.log(`\nTesting ${device.name} (${device.width}x${device.height})...`);

    await page.setViewport({
      width: device.width,
      height: device.height,
      deviceScaleFactor: 2
    });

    await page.goto('http://localhost:8081/sign.html?session=test&recipient=r1&key=test123');
    await page.waitForSelector('#btn-start');

    // Take screenshot
    await page.screenshot({
      path: `test-${device.name.toLowerCase().replace(' ', '-')}.png`,
      fullPage: true
    });

    // Run tests
    const results = await page.evaluate(() => {
      return window.mobileTests ? window.mobileTests.runAll() : null;
    });

    console.log(`Results for ${device.name}:`, results);
  }

  await browser.close();
  console.log('\nAll tests complete!');
})();
```

## Notes

- Tests assume the development server is running on `http://localhost:8081`
- Screenshots are saved to the current working directory
- Some tests may be skipped if viewport doesn't match test requirements
- Use `session=test` for test mode with mock data
- For real session testing, create a session via the sender flow first
