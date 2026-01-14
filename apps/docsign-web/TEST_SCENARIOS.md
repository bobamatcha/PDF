# Test Scenarios for DocSign Usability Testing

**Target User Group:** Adults aged 65 and older
**Purpose:** Detailed test cases for comprehensive usability evaluation

---

## Scenario 1: First-Time User Flow

### Description
A user who has never used DocSign before receives their first signing request via email.

### Pre-conditions
- User has never seen the DocSign interface
- User opens the signing link from an email
- User has basic computer/smartphone skills

### Test Steps

| Step | Action | Expected Result | Notes for Observer |
|------|--------|-----------------|-------------------|
| 1.1 | Click signing link from email | Loading indicator appears, then consent page loads | Note if loading time causes concern |
| 1.2 | Read the consent page | User reads sender info, document name, and consent text | Note reading time and comprehension |
| 1.3 | Ask: "What is this page asking you to do?" | User explains they need to agree to sign electronically | Tests comprehension of consent |
| 1.4 | Click "Review Document" | Document viewer appears with PDF | Note if button is easily found |
| 1.5 | Scroll through document | User can see all pages | Note scrolling difficulties |
| 1.6 | Find first signature field | User identifies the blue dotted box | Note if field is visible enough |
| 1.7 | Click signature field | Signature modal opens | Note any hesitation |
| 1.8 | Choose Draw or Type | User selects preferred method | Note which they prefer and why |
| 1.9 | Create signature | Signature is applied to field | Note motor control issues |
| 1.10 | Click "Finish Signing" | Completion confirmation appears | Note satisfaction level |

### Success Criteria
- All steps completed without assistance
- Time: Under 10 minutes total
- No more than 2 errors

### Failure Indicators
- Cannot find Review Document button
- Confused by consent language
- Cannot draw signature (motor issues)
- Does not understand when finished

---

## Scenario 2: Returning User Flow

### Description
A user who has signed documents before returns to sign another document.

### Pre-conditions
- User has successfully signed at least one document before
- User remembers basic flow
- May expect "remembered" signature

### Test Steps

| Step | Action | Expected Result | Notes for Observer |
|------|--------|-----------------|-------------------|
| 2.1 | Open signing link | Page loads to consent screen | Should feel familiar |
| 2.2 | Review consent page | Quick glance, then proceed | Note if they skip reading |
| 2.3 | Click "Review Document" | Document viewer opens | Should be confident |
| 2.4 | Navigate to signature fields | Direct navigation, less exploration | Note improved speed |
| 2.5 | Apply signature | Uses preferred method from before | Note consistency |
| 2.6 | Complete signing | Finishes quickly | Compare time to Scenario 1 |

### Success Criteria
- 30% faster than first-time flow
- Fewer hesitations
- Confident navigation

### Comparison Metrics
| Metric | First-Time | Returning | Target Improvement |
|--------|------------|-----------|-------------------|
| Total Time | ___ min | ___ min | 30% faster |
| Errors | ___ | ___ | 50% fewer |
| Help Requests | ___ | ___ | Zero |

---

## Scenario 3: Error Recovery Scenarios

### Description
Test how users recover from common mistakes and errors.

### Scenario 3A: Wrong Tab Selected (Type vs Draw)

| Step | Action | Expected Result | Notes |
|------|--------|-----------------|-------|
| 3A.1 | User types name in Draw tab | Nothing appears (wrong tab) | Note confusion |
| 3A.2 | User realizes mistake | Finds Type tab | Note discovery time |
| 3A.3 | Switches to correct tab | Types name successfully | Note recovery |

**Success:** User recovers independently within 30 seconds

### Scenario 3B: Accidental Modal Close

| Step | Action | Expected Result | Notes |
|------|--------|-----------------|-------|
| 3B.1 | Open signature modal | Modal appears | - |
| 3B.2 | Click X or outside modal | Modal closes | May be accidental |
| 3B.3 | User realizes signature not saved | Returns to field | Note frustration level |
| 3B.4 | Re-opens modal | Can try again | Note if data was lost |

**Success:** User understands how to re-open and complete

### Scenario 3C: Trying to Finish with Missing Fields

| Step | Action | Expected Result | Notes |
|------|--------|-----------------|-------|
| 3C.1 | Skip a required field | Field remains unsigned | - |
| 3C.2 | Click "Finish Signing" | Error message or disabled button | Note clarity of feedback |
| 3C.3 | User identifies missing field | Navigates to incomplete field | Note guidance quality |
| 3C.4 | Completes missing field | All fields filled | - |
| 3C.5 | Click "Finish" again | Success | Note total time |

**Success:** Clear error communication, user fixes within 1 minute

### Scenario 3D: Drawn Signature Too Small/Light

| Step | Action | Expected Result | Notes |
|------|--------|-----------------|-------|
| 3D.1 | Draw very small signature | Canvas shows tiny mark | - |
| 3D.2 | Click "Apply" | Signature accepted (even if poor) | Note visual feedback |
| 3D.3 | User sees result on document | May want to redo | Note if redo is easy |
| 3D.4 | If unhappy, redo signature | Can click field again | Note discoverability |

**Success:** User can redo if unsatisfied with first attempt

---

## Scenario 4: Offline/Online Transition

### Description
Test the application's behavior when internet connectivity changes.

### Pre-conditions
- Facilitator can toggle network on/off
- User is mid-signing process

### Test Steps

| Step | Action | Expected Result | Notes |
|------|--------|-----------------|-------|
| 4.1 | User starts signing process | Normal operation | - |
| 4.2 | Disable network (airplane mode) | Offline indicator appears | Note visibility of indicator |
| 4.3 | User continues signing | Signatures are saved locally | Should work normally |
| 4.4 | User completes all fields | All fields show completed | - |
| 4.5 | User clicks "Finish" | "Saved locally" message | Note clarity of message |
| 4.6 | Re-enable network | Auto-sync begins (if implemented) | Note sync indicator |
| 4.7 | Sync completes | Success confirmation | Note final state clarity |

### Questions to Ask User
- "Did you notice anything different just now?"
- "Do you feel confident your signature was saved?"
- "Would you know what to do if this happened at home?"

### Success Criteria
- User notices offline state
- User is not alarmed (understands data is safe)
- User knows to reconnect for final submission

---

## Scenario 5: Mobile vs Desktop Comparison

### Description
Compare the signing experience across devices.

### Pre-conditions
- Same user tests both devices
- Same test document
- Same session ID (or equivalent)

### Device Test Matrix

| Aspect | Desktop Test | Mobile Test | Comparison Notes |
|--------|-------------|-------------|------------------|
| **Screen Size** | 15"+ monitor | Phone/Tablet | Note visibility |
| **Input Method** | Mouse/Trackpad | Touch/Finger | Note signature quality |
| **Scrolling** | Scroll wheel | Swipe | Note ease of navigation |
| **Button Targets** | Mouse click | Tap | Note tap accuracy |
| **Modal Display** | Centered popup | Bottom sheet | Note usability |
| **Keyboard Entry** | Physical keyboard | On-screen keyboard | Note typing ease |

### Desktop-Specific Tests

| Test | Action | Expected Result |
|------|--------|-----------------|
| D1 | Mouse hover on buttons | Visual feedback (color change) |
| D2 | Tab key navigation | Moves between fields |
| D3 | Enter key to confirm | Activates focused button |
| D4 | Draw signature with mouse | Smooth line rendering |

### Mobile-Specific Tests

| Test | Action | Expected Result |
|------|--------|-----------------|
| M1 | Tap on buttons | Responsive tap feedback |
| M2 | Pinch to zoom | Document zooms (if enabled) |
| M3 | Swipe to scroll | Smooth scrolling |
| M4 | Draw signature with finger | Touch-optimized drawing |
| M5 | Landscape rotation | Layout adapts properly |

### Comparison Metrics

| Metric | Desktop | Mobile | Preference |
|--------|---------|--------|------------|
| Task Completion Time | ___ min | ___ min | |
| Signature Quality | ___/5 | ___/5 | |
| Overall Satisfaction | ___/5 | ___/5 | |
| Preferred Device | [ ] | [ ] | |

---

## Scenario 6: Accessibility Scenarios

### Description
Test accessibility features important for geriatric users.

### Scenario 6A: Vision Impairment

| Test | Action | Expected Result | Notes |
|------|--------|-----------------|-------|
| 6A.1 | View at 150% browser zoom | All elements scale properly | Note any clipping |
| 6A.2 | View at 200% browser zoom | Still usable | Note layout issues |
| 6A.3 | Check color contrast | Text readable on all backgrounds | Use contrast checker |
| 6A.4 | Read all button labels | Labels are clear and descriptive | Note ambiguous labels |

### Scenario 6B: Motor Control Issues

| Test | Action | Expected Result | Notes |
|------|--------|-----------------|-------|
| 6B.1 | Click small targets | 60px minimum touch targets | Measure actual sizes |
| 6B.2 | Draw with unsteady hand | Signature still legible | Simulate tremor |
| 6B.3 | Type alternative to draw | Easy to switch | Note tab clarity |
| 6B.4 | Click adjacent buttons | No accidental clicks | Check spacing |

### Scenario 6C: Cognitive Load

| Test | Action | Expected Result | Notes |
|------|--------|-----------------|-------|
| 6C.1 | Count steps to completion | Minimal steps (under 6) | Document flow |
| 6C.2 | Read all instructions | Clear, simple language | Note jargon |
| 6C.3 | Identify progress | Clear indication of remaining work | Check progress bar |
| 6C.4 | Understand completion | Clear "you're done" state | Note confirmation clarity |

---

## Scenario 7: Edge Cases

### Description
Test unusual but possible situations.

### Scenario 7A: Very Long Document (17+ pages)

| Test | Expected Behavior |
|------|-------------------|
| Scroll through all pages | Performance remains smooth |
| Find signature on last page | Can navigate without losing place |
| Time to load | Under 5 seconds on good connection |

### Scenario 7B: Multiple Signature Fields

| Test | Expected Behavior |
|------|-------------------|
| 3+ signature fields | Progress indicator accurate |
| Fields on different pages | Navigation helps find each |
| All fields completion | Clear indication when done |

### Scenario 7C: Session Expiry

| Test | Expected Behavior |
|------|-------------------|
| Open expired link | Clear expiry message shown |
| Request new link | Button works, feedback given |
| Contact sender option | Email link works |

### Scenario 7D: Invalid Link

| Test | Expected Behavior |
|------|-------------------|
| Corrupted URL | Clear error message |
| Missing parameters | Explains what went wrong |
| Help guidance | Suggests checking email |

---

## Test Session Recording Template

### Session Information
- **Session ID:** _______________
- **Participant ID:** _______________
- **Date:** _______________
- **Scenarios Tested:** _______________

### Per-Scenario Results

| Scenario | Completed | Time | Issues Found |
|----------|-----------|------|--------------|
| 1 (First-Time) | Y/N | ___ min | |
| 2 (Returning) | Y/N | ___ min | |
| 3A (Wrong Tab) | Y/N | ___ sec | |
| 3B (Modal Close) | Y/N | ___ sec | |
| 3C (Missing Fields) | Y/N | ___ min | |
| 3D (Poor Signature) | Y/N | ___ sec | |
| 4 (Offline) | Y/N | ___ min | |
| 5 (Mobile vs Desktop) | Y/N | ___ min | |
| 6A (Vision) | Y/N | - | |
| 6B (Motor) | Y/N | - | |
| 6C (Cognitive) | Y/N | - | |
| 7A (Long Doc) | Y/N | - | |
| 7B (Multi-Sig) | Y/N | - | |
| 7C (Expiry) | Y/N | - | |
| 7D (Invalid Link) | Y/N | - | |

### Priority Issues Found

| Issue | Scenario | Severity (1-5) | Fix Required? |
|-------|----------|----------------|---------------|
| | | | |
| | | | |
| | | | |

---

## Scenario Selection Guide

### For Quick Testing (30 min)
- Scenario 1: First-Time User Flow
- Scenario 3C: Missing Fields Recovery

### For Full Testing (60 min)
- Scenario 1: First-Time User Flow
- Scenario 3A-D: All Error Recovery
- Scenario 6A-B: Vision and Motor

### For Mobile Focus (45 min)
- Scenario 5: Mobile vs Desktop (mobile portion)
- Scenario 1: First-Time User Flow (on mobile)
- Scenario 3D: Drawn Signature (touch input)

### For Accessibility Audit
- Scenario 6A-C: All Accessibility Scenarios
- Scenario 7A: Long Document (performance)
