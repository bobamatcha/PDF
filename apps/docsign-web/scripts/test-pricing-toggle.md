# Test: Pricing Page Toggle

## Test Case: Toggle should be clickable

**Steps:**
1. Navigate to /pricing.html
2. Click on the toggle switch (the visual slider element)
3. Verify prices change from monthly to annual

**Expected:**
- Personal: $10/month → $100/year
- Professional: $25/month → $250/year
- Business: $60/month → $600/year

**Bug (2026-01-11):**
- Clicking `.toggle-slider` does NOT toggle the checkbox
- Checkbox has `opacity: 0; width: 0; height: 0` making it unclickable
- No `<label for="billing-toggle">` wrapping the slider

**Fix:**
- Change `<span class="toggle-slider">` to `<label for="billing-toggle" class="toggle-slider">`
- This makes clicking the visual slider toggle the hidden checkbox

**Verification:**
- Use Puppeteer: `puppeteer_click` on `.toggle-slider` should work
- Prices should visually change after click
