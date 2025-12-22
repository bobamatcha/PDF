# Quick Test Guide: UX-005 Mobile Optimizations

## Quick Start (5 minutes)

### 1. Start Development Server
```bash
cd apps/docsign-web
trunk serve --port 8081
```

### 2. Open in Browser with Mobile Simulation

**Chrome DevTools:**
1. Open `http://localhost:8081/sign.html?session=test&recipient=r1&key=test123`
2. Press F12 to open DevTools
3. Click the device toolbar icon (or Ctrl+Shift+M / Cmd+Shift+M)
4. Select "iPhone SE" from device dropdown
5. Refresh the page

**Firefox:**
1. Open `http://localhost:8081/sign.html?session=test&recipient=r1&key=test123`
2. Press F12 to open Developer Tools
3. Click the Responsive Design Mode icon (or Ctrl+Shift+M / Cmd+Opt+M)
4. Select "iPhone SE" (375 x 667)

### 3. Visual Checklist (2 minutes)

Open the page and verify:

- [ ] **Loading Screen:** Spinner centered, no horizontal scroll
- [ ] **Click "Start Signing":** Button is large and easy to tap
- [ ] **Progress Indicator:** Shows "1 of 3" at top of toolbar
- [ ] **Navigation Buttons:** "Back" and "Next" buttons are clearly visible
- [ ] **Click First Field:** Signature modal slides up from bottom
- [ ] **Bottom Sheet:** Modal has rounded corners on top, fills ~90% of screen height
- [ ] **Signature Canvas:** Canvas fills full width of modal body
- [ ] **Draw Signature:** Lines are smooth and thick enough to see
- [ ] **Click Apply:** Modal closes smoothly
- [ ] **Swipe Left:** (Touch simulation or drag with mouse) - Advances to next field
- [ ] **Swipe Right:** Goes back to previous field
- [ ] **No Horizontal Scroll:** Scroll horizontally - page should not scroll left/right

### 4. Run Automated Tests (1 minute)

In browser console:
```javascript
window.mobileTests.runAll()
```

Expected output:
```
Running mobile optimization tests...

✓ All touch targets are at least 44x44px
✓ Swipe gestures are enabled
✓ No horizontal scrolling detected
✓ Signature pad fills available width with proper aspect ratio

=== Test Results ===
Passed: X
Failed: 0
Skipped: Y
```

## Test Different Devices

### iPhone 11 (Larger Mobile)
```javascript
// In DevTools console or device selector:
// Width: 414, Height: 896
```
- Modal should still be bottom sheet
- More vertical space for signature

### iPad Mini (Tablet)
```javascript
// Width: 768, Height: 1024
```
- Modal should be centered (NOT bottom sheet)
- Modal max-width: 700px
- Still touch-friendly (44x44px targets)

### Landscape Mode
Rotate device to landscape (or swap width/height in DevTools)
- Modal should adapt (max-height: 95vh)
- Canvas height reduces to 180px
- All content accessible without scrolling

## Common Test Scenarios

### Scenario 1: Complete Signing Flow on Mobile
1. Open page in iPhone SE mode
2. Click "Start Signing"
3. Click first field (signature)
4. Draw signature
5. Click "Apply"
6. Swipe left to next field
7. Draw initials
8. Click "Apply"
9. Swipe left to date field
10. Date auto-fills
11. Click "Finish"
12. ✅ Success modal appears

### Scenario 2: Touch Target Validation
1. Open page in iPhone SE mode
2. Use DevTools to measure buttons:
   - Right-click button → Inspect
   - Check Computed styles
   - Verify: height >= 44px, width >= 44px
3. All interactive elements should meet minimum

### Scenario 3: Bottom Sheet Behavior
1. Open page in iPhone SE mode
2. Click first signature field
3. Verify:
   - Modal slides up from bottom
   - Rounded corners on top only
   - Modal takes ~90% of screen height
   - Can see a bit of the PDF behind modal
4. Close modal (click Cancel)
5. Verify modal slides down smoothly

## Troubleshooting

### Issue: Modal Not Full Screen on Mobile
**Check:**
- Is viewport < 768px?
- Does modal have `bottom-sheet-mobile` class?
- Check browser console for JavaScript errors

**Fix:**
```javascript
// In console, check:
window.innerWidth // Should be < 768
document.querySelector('#signature-modal .modal').classList.contains('bottom-sheet-mobile')
```

### Issue: Touch Targets Too Small
**Check:**
- Inspect button element
- Look at Computed styles
- Check if `min-height: 44px` is applied

**Fix:**
- Clear browser cache
- Hard refresh (Ctrl+Shift+R / Cmd+Shift+R)
- Check CSS is loading properly

### Issue: Swipe Not Working
**Check:**
- Is guided flow active? (Did you click "Start Signing"?)
- Check console for errors
- Verify `data-swipe-enabled="true"` on `.viewer-container`

**Fix:**
```javascript
// In console:
document.querySelector('.viewer-container').dataset.swipeEnabled // Should be "true"
```

### Issue: Horizontal Scrolling
**Check:**
- Zoom level (should be 100%)
- DevTools device mode active
- Any fixed-width elements

**Fix:**
```javascript
// In console, find overflowing element:
document.querySelectorAll('*').forEach(el => {
  if (el.scrollWidth > window.innerWidth) {
    console.log('Overflow:', el);
  }
});
```

## Success Criteria Summary

After testing, you should have:

✅ All buttons and interactive elements are easy to tap (44x44px minimum)
✅ Signature modal slides up from bottom on mobile
✅ Signature canvas is large enough to draw comfortably
✅ Swipe gestures navigate between fields smoothly
✅ No horizontal scrolling on any screen
✅ Progress indicator always visible
✅ Layout adapts to portrait and landscape
✅ Tablet uses hybrid layout (centered modal, touch-friendly)

## Next Steps

For comprehensive testing, see:
- `PUPPETEER_VERIFICATION_UX005.md` - Full Puppeteer test suite
- `mobile-tests.js` - Automated test implementation
- `UX005_IMPLEMENTATION_SUMMARY.md` - Complete implementation details

## Quick Puppeteer Test

If you have Puppeteer MCP available:

```javascript
// Navigate to page with mobile viewport
await page.setViewport({ width: 375, height: 667 });
await page.goto('http://localhost:8081/sign.html?session=test&recipient=r1&key=test123');

// Take screenshot
await page.screenshot({ path: 'mobile-test.png' });

// Run tests
const results = await page.evaluate(() => window.mobileTests.runAll());
console.log(results);
```

---

**Total Test Time:** ~5-10 minutes for quick validation
**Full Test Time:** ~30 minutes with Puppeteer scenarios
