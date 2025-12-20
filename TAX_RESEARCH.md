# Comprehensive Architecture for US Tax Preparation Document Frameworks

A Technical and Legal Roadmap for Building a TurboTax Competitor

---

## 1. Executive Summary

Building a competitive tax preparation platform requires understanding the US tax system as a **bifurcated architecture**:

1. **Frontend**: User interview layer (questionnaire-driven)
2. **Backend**: Compliance Engine generating two outputs:
   - **PDF Return** (human-readable "Substitute Form")
   - **MeF XML** (machine-readable transmission file)

### The IRS Data Flow Model

```
┌─────────────────────────────────────────────────────────────┐
│  SOURCE DOCUMENTS (Information Returns)                     │
│  W-2, 1099-NEC, 1099-DIV, 1099-INT, 1098                   │
└─────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────┐
│  CALCULATION WORKSHEETS (Internal Logic)                    │
│  Capital Gains Worksheet, SE Tax Worksheet, etc.            │
└─────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────┐
│  SCHEDULES (Specialized Reporting)                          │
│  Schedule A, B, C, D, E, SE, 1, 2, 3                        │
└─────────────────────────────────────────────────────────────┘
                              ↓
┌─────────────────────────────────────────────────────────────┐
│  BASE FORM (Aggregation)                                    │
│  Form 1040 / 1040-SR                                        │
└─────────────────────────────────────────────────────────────┘
```

---

## 2. Legal Framework: Publications 1167 & 1179

### 2.1 Publication 1167: Substitute Forms Bible

Software-generated PDFs are legally classified as **"Substitute Forms"**. The IRS accepts them only if they adhere to strict specifications.

| Requirement | Specification |
|-------------|---------------|
| **Layout** | Exact replica - cannot rearrange to "look better" |
| **Fonts** | Helvetica/Times for text, OCR-A for scanlines |
| **Margins** | Must preserve official margins exactly |
| **Data Fields** | Use monospaced font (Courier 10/12-pitch) for user data |
| **Alignment** | No scaling to "fit printable area" |

#### Approval Process

| Scenario | Action Required |
|----------|-----------------|
| Pixel-perfect replica | No approval needed |
| Layout deviations | Submit to `substituteforms@irs.gov` |
| Different orientation/fonts | Requires IRS Substitute Forms Program approval |

> **Pro Tip**: Use official IRS PDFs as background layers rather than redrawing forms from scratch.

### 2.2 Publication 1179: Information Returns

Governs W-2s, 1099s, and other forms **furnished to taxpayers**.

| Copy | Ink | Usage | Generation Notes |
|------|-----|-------|------------------|
| **Copy A** | Red (drop-out) | Filed with IRS/SSA | Cannot print from PDF - special ink required |
| **Copy B** | Black | Furnished to recipient | Standard PDF generation allowed |

#### Composite Statements

Pub 1179 allows combining multiple income types (dividends + interest) on a single document with specific graphical delimiters.

### 2.3 The "Draft" Form Hazard

| Stage | Watermark | Usage |
|-------|-----------|-------|
| Draft | "DRAFT - DO NOT FILE" | Development only |
| Final | None | Production filing |

**Critical**: Implement a flag to prevent final PDF generation using draft templates. Monitor IRS "Early Release Drafts" page.

---

## 3. The Master Construct: Form 1040 Family

### 3.1 Form 1040: U.S. Individual Income Tax Return

**Resource**: [irs.gov/pub/irs-pdf/f1040.pdf](https://www.irs.gov/pub/irs-pdf/f1040.pdf)

| Page | Function | Key Elements |
|------|----------|--------------|
| **Page 1** | Identity & Income | Filing Status, Name, SSN, Dependents, Total Income |
| **Page 2** | Tax & Credits | Deductions, Credits, Withholding, Refund/Owed |

#### Mandatory Fields

```javascript
// Digital Asset Question - MUST be answered
if (digital_asset_question === null) {
  REJECT: "Digital asset question cannot be blank"
}
```

### 3.2 Form 1040-SR: Senior Version

**Resource**: [irs.gov/pub/irs-pdf/f1040s.pdf](https://www.irs.gov/pub/irs-pdf/f1040s.pdf)

| Feature | Difference from 1040 |
|---------|---------------------|
| Typography | Larger fonts |
| Standard Deduction | Chart printed on form |
| Trigger | Taxpayer born before Jan 2, 1960 |

```javascript
// Auto-select 1040-SR for seniors
if (taxpayer.birth_date < '1960-01-02') {
  form_type = '1040-SR'  // or offer choice
}
```

---

## 4. Numbered Schedules (1, 2, 3)

These are extensions of Form 1040, generated only when needed.

### 4.1 Schedule 1: Additional Income & Adjustments

**Resource**: [irs.gov/pub/irs-pdf/f1040s1.pdf](https://www.irs.gov/pub/irs-pdf/f1040s1.pdf)

| Part | Content | Common Triggers |
|------|---------|-----------------|
| **Part I** | Additional Income | Unemployment, Business (Sch C), Rental (Sch E), Gig income |
| **Part II** | Adjustments | HSA, Student Loan Interest, Educator Expenses |

#### Line 8z: "Other Income"

```javascript
// Gig economy / misc income
{
  line: "8z",
  amount: 5000,
  description: "Esports Tournament Winnings"  // Required text field
}
```

### 4.2 Schedule 2: Additional Taxes

**Resource**: [irs.gov/pub/irs-pdf/f1040s2.pdf](https://www.irs.gov/pub/irs-pdf/f1040s2.pdf)

| Part | Content | Source Forms |
|------|---------|--------------|
| **Part I** | Tax | AMT (Form 6251), Premium Tax Credit repayment (Form 8962) |
| **Part II** | Other Taxes | Self-Employment Tax (Sch SE), IRA penalties (Form 5329) |

### 4.3 Schedule 3: Additional Credits & Payments

**Resource**: [irs.gov/pub/irs-pdf/f1040s3.pdf](https://www.irs.gov/pub/irs-pdf/f1040s3.pdf)

| Part | Type | Examples |
|------|------|----------|
| **Part I** | Nonrefundable Credits | Foreign Tax (1116), Education (8863), Clean Energy (5695) |
| **Part II** | Refundable Credits | Premium Tax Credit, Excess SS Tax |

---

## 5. Lettered Schedules: The Calculation Engines

### 5.1 Schedule A: Itemized Deductions

**Resource**: [irs.gov/pub/irs-pdf/f1040sa.pdf](https://www.irs.gov/pub/irs-pdf/f1040sa.pdf)

#### Optimization Logic

```javascript
function selectDeductionMethod(itemized_total, standard_deduction) {
  // Always choose higher value to minimize tax
  return itemized_total > standard_deduction
    ? { method: 'itemized', amount: itemized_total }
    : { method: 'standard', amount: standard_deduction }
}
```

#### Key Constraints

| Deduction | Rule | Implementation |
|-----------|------|----------------|
| **Medical** | Only amount > 7.5% of AGI | `deductible = max(0, medical - AGI * 0.075)` |
| **SALT** | Capped at $10,000 ($5,000 MFS) | `salt_deduction = min(salt_paid, 10000)` |
| **Mortgage Interest** | Limited to $750K debt | Track acquisition date and amount |

### 5.2 Schedule B: Interest & Dividends

**Resource**: [irs.gov/pub/irs-pdf/f1040sb.pdf](https://www.irs.gov/pub/irs-pdf/f1040sb.pdf)

**Trigger**: Interest or dividends exceed $1,500

#### Part III: Foreign Accounts (CRITICAL)

```javascript
if (has_foreign_account === true) {
  // Trigger additional compliance requirements
  REQUIRE: "FinCEN Form 114 (FBAR)"  // Filed with Treasury, not IRS
  CONSIDER: "Form 8938 (FATCA)"

  // Penalty for non-disclosure: $10,000+
  display_warning: true
}
```

### 5.3 Schedule C: Business Profit/Loss (Gig Economy)

**Resource**: [irs.gov/pub/irs-pdf/f1040sc.pdf](https://www.irs.gov/pub/irs-pdf/f1040sc.pdf)

#### Data Ingestion Mapping

| Source | Maps To |
|--------|---------|
| Form 1099-NEC Box 1 | Schedule C Part I (Income) |
| Form 1099-K | Schedule C Part I (verify no double-count) |

#### Expense Categorization Helper

```javascript
const expenseMapping = {
  "Web hosting bill": { line: 18, category: "Office expense" },
  "Facebook ads": { line: 8, category: "Advertising" },
  "Client lunch": { line: 24b, category: "Meals (50%)" },
  "Home office": { line: 30, category: "Business Use of Home" }
}
```

#### Business Use of Home (Line 30)

Requires Form 8829 calculation:
```javascript
function calculateHomeOfficeDeduction(total_sqft, office_sqft, expenses) {
  const percentage = office_sqft / total_sqft
  return expenses.total * percentage
}
```

### 5.4 Schedule D: Capital Gains & Losses

**Resource**: [irs.gov/pub/irs-pdf/f1040sd.pdf](https://www.irs.gov/pub/irs-pdf/f1040sd.pdf)

#### Architecture

```
Form 8949 (Transaction Details)
├── Part I: Short-Term (≤1 year)
│   ├── (A) Basis reported to IRS
│   ├── (B) Basis NOT reported to IRS
│   └── (C) No 1099-B received
└── Part II: Long-Term (>1 year)
    ├── (D) Basis reported to IRS
    ├── (E) Basis NOT reported to IRS
    └── (F) No 1099-B received
            ↓
    Schedule D (Summary)
            ↓
    Qualified Dividends & Capital Gain Tax Worksheet
    (Applies 0%/15%/20% rates)
```

### 5.5 Schedule E: Rental & Passthrough Income

**Resource**: [irs.gov/pub/irs-pdf/f1040se.pdf](https://www.irs.gov/pub/irs-pdf/f1040se.pdf)

#### Passive Activity Loss (PAL) Rules

```javascript
function calculateRentalLoss(loss, agi, active_participation) {
  if (!active_participation) {
    return { deductible: 0, suspended: loss }
  }

  // Active participation: up to $25K deductible
  const phase_out_start = 100000
  const phase_out_end = 150000

  if (agi >= phase_out_end) {
    return { deductible: 0, suspended: loss }
  }

  const allowed = 25000 - ((agi - phase_out_start) * 0.5)
  const deductible = Math.min(loss, Math.max(0, allowed))

  return { deductible, suspended: loss - deductible }
}
```

#### K-1 Mapping

| K-1 Box | Schedule E Location |
|---------|---------------------|
| Box 1 (Ordinary Income) | Part II, Column (h) |
| Box 2 (Rental Income) | Part II, Column (h) |
| Box 11 (Other Income) | Part II, various |

### 5.6 Schedule SE: Self-Employment Tax

**Resource**: [irs.gov/pub/irs-pdf/f1040sse.pdf](https://www.irs.gov/pub/irs-pdf/f1040sse.pdf)

**Trigger**: Net profit from Schedule C or F exceeds $400

#### Circular Dependency Resolution

```javascript
// SE Tax creates a circular dependency:
// 1. Calculate SE Tax on Schedule SE
// 2. 50% of SE Tax is deductible on Schedule 1, Line 15
// 3. This reduces AGI, which can affect other calculations

function resolveCircularDependency(net_profit) {
  const se_tax = net_profit * 0.9235 * 0.153
  const se_deduction = se_tax * 0.5

  // Iterate until stable
  return { se_tax, se_deduction }
}
```

---

## 6. Information Returns: The Input Layer

### 6.1 Form W-2: Wage and Tax Statement

**Resource**: [irs.gov/pub/irs-pdf/fw2.pdf](https://www.irs.gov/pub/irs-pdf/fw2.pdf)

| Box | Content | Notes |
|-----|---------|-------|
| 1 | Wages, tips, other | Primary income |
| 2 | Federal tax withheld | Credit on 1040 |
| 12 | Codes | D=401k, DD=Health coverage cost |

### 6.2 Form 1099-NEC: Nonemployee Compensation

**Resource**: [irs.gov/pub/irs-pdf/f1099nec.pdf](https://www.irs.gov/pub/irs-pdf/f1099nec.pdf)

| Filed When | Mapping |
|------------|---------|
| Contractor paid ≥$600 | Box 1 → Schedule C (business) or 1040 Line 8 (hobby) |

### 6.3 Form 1099-MISC: Miscellaneous

**Resource**: [irs.gov/pub/irs-pdf/f1099msc.pdf](https://www.irs.gov/pub/irs-pdf/f1099msc.pdf)

| Box | Content | Destination |
|-----|---------|-------------|
| 1 | Rents | Schedule E |
| 2 | Royalties | Schedule E |
| 3 | Prizes/Other | Schedule 1, Line 8z |

### 6.4 Forms 1099-INT & 1099-DIV

| Form | Resource | Key Distinction |
|------|----------|-----------------|
| 1099-INT | [irs.gov/pub/irs-pdf/f1099int.pdf](https://www.irs.gov/pub/irs-pdf/f1099int.pdf) | Interest income |
| 1099-DIV | [irs.gov/pub/irs-pdf/f1099div.pdf](https://www.irs.gov/pub/irs-pdf/f1099div.pdf) | Box 1a=Ordinary (regular rates), Box 1b=Qualified (capital gains rates) |

### 6.5 Form 1098: Mortgage Interest

**Resource**: [irs.gov/pub/irs-pdf/f1098.pdf](https://www.irs.gov/pub/irs-pdf/f1098.pdf)

Box 1 → Schedule A Line 8a (if itemizing)

---

## 7. State Tax Integration

### 7.1 California (FTB)

**Resource**: [ftb.ca.gov/forms](https://www.ftb.ca.gov/forms/index.html)

| Form | Purpose |
|------|---------|
| Form 540 | California Resident Income Tax Return |
| Schedule CA | California Adjustments (non-conformity) |

#### Non-Conformity Examples

| Item | Federal | California |
|------|---------|------------|
| Lottery winnings | Taxable | Not taxed |
| Depreciation | MACRS | Different rules |

**Developer Requirement**: Letter of Intent (LOI) + FTB testing program participation

### 7.2 New York (DTF)

**Resource**: [tax.ny.gov/bus/efile](https://www.tax.ny.gov/bus/efile/Ind_income_tax_home_page.htm)

| Form | Purpose |
|------|---------|
| IT-201 | Resident Income Tax Return |

#### 2D Barcode Requirement

```
NYS requires PDF417 2D barcode encoding all return data.
Scanner reads barcode instead of OCR on text fields.

Testing Required:
- 1D Test Team (form layout)
- 2D Test Team (barcode readability)
- Publication 75 compliance
```

### 7.3 Federation of Tax Administrators (FTA)

| Resource | Purpose |
|----------|---------|
| FTA Standards | Uniform electronic filing specs |
| State Exchange System (SES) | Access to state developer specs |

> **Strategy**: Use FTA's SES to avoid building 50 separate integration pipelines.

---

## 8. Digital Transmission: MeF & XML

### 8.1 XML Schema vs PDF

| Component | Format | Purpose |
|-----------|--------|---------|
| Schema (XSD) | XML | Data structure definition |
| Stylesheet (XSL) | XSLT | Transform XML → PDF view |
| Transmission | SOAP | Wrapper with authentication |

#### Database-First Design

```
Database (Source of Truth)
├── Populates → MeF XML (transmission)
└── Populates → PDF (user view)

NEVER scrape PDF to build XML (causes rounding errors)
```

### 8.2 IRS MeF Stylesheets

**Resource**: [irs.gov/e-file-providers/modernized-e-file-mef-user-guides](https://www.irs.gov/e-file-providers/modernized-e-file-mef-user-guides-and-publications)

Using official stylesheets ensures visual compliance without manually drawing forms.

### 8.3 Transmission Process

```
┌─────────────────────────────────────────────────────────────┐
│  1. Generate XML Return                                     │
├─────────────────────────────────────────────────────────────┤
│  2. Wrap in SOAP Envelope                                   │
│     - Transmission Header                                   │
│     - Manifest                                              │
│     - ETIN Authentication                                   │
├─────────────────────────────────────────────────────────────┤
│  3. Assurance Testing System (ATS)                          │
│     - Test scenarios from Publication 5078                  │
│     - IRS test environment validation                       │
├─────────────────────────────────────────────────────────────┤
│  4. Production Transmission                                 │
└─────────────────────────────────────────────────────────────┘
```

---

## 9. Implementation Roadmap

### Short Term: Foundation

- [ ] Implement Form 1040 / 1040-SR PDF generation
- [ ] Build Schedule C engine (gig economy focus)
- [ ] Create W-2 and 1099-NEC import/display
- [ ] Implement Publication 1167 compliance checks
- [ ] Build standard deduction vs itemized optimization

### Medium Term: Full Individual Returns

- [ ] Add all numbered schedules (1, 2, 3)
- [ ] Add all lettered schedules (A, B, C, D, E, SE)
- [ ] Implement capital gains worksheet
- [ ] Build Form 8949 transaction tracking
- [ ] Add California (Form 540) state support
- [ ] Add New York (IT-201) with 2D barcode

### Long Term: Scale & E-File

- [ ] Implement MeF XML generation
- [ ] Build SOAP transmission layer
- [ ] Complete ATS certification
- [ ] Add remaining "Big States" (TX franchise, FL none)
- [ ] FTA/SES integration for multi-state
- [ ] Information return generation (1099s for businesses)

---

## 10. Architecture Recommendations

### 10.1 Database-First Design

```rust
// Schema mirrors MeF XML, not PDF layout
struct TaxReturn {
    filing_status: FilingStatus,
    income: IncomeSection,
    deductions: DeductionSection,
    credits: CreditSection,
    // ...
}

// PDF is a "view" of the data
fn generate_pdf(return: &TaxReturn) -> PDF { ... }
fn generate_mef_xml(return: &TaxReturn) -> XML { ... }
```

### 10.2 Business Rules Integration

```rust
// IRS Business Rules as unit tests
#[test]
fn test_dependent_ssn_match() {
    // Dependent SSN must match IRS records
    let dependent = Dependent { ssn: "123-45-6789", ... };
    assert!(validate_dependent_ssn(&dependent));
}
```

### 10.3 Dynamic Rendering

| Approach | Pros | Cons |
|----------|------|------|
| Static PDF filling | Simple | Brittle, alignment issues |
| XSLT → PDF | IRS-compliant | Complex setup |
| Template engine (React-PDF) | Flexible | Must verify alignment |

**Recommendation**: Use IRS stylesheets (XSLT) for guaranteed compliance.

---

## 11. Form Resource Links

### Core Forms

| Form | Link |
|------|------|
| 1040 | [irs.gov/pub/irs-pdf/f1040.pdf](https://www.irs.gov/pub/irs-pdf/f1040.pdf) |
| 1040-SR | [irs.gov/pub/irs-pdf/f1040s.pdf](https://www.irs.gov/pub/irs-pdf/f1040s.pdf) |
| Schedule 1 | [irs.gov/pub/irs-pdf/f1040s1.pdf](https://www.irs.gov/pub/irs-pdf/f1040s1.pdf) |
| Schedule 2 | [irs.gov/pub/irs-pdf/f1040s2.pdf](https://www.irs.gov/pub/irs-pdf/f1040s2.pdf) |
| Schedule 3 | [irs.gov/pub/irs-pdf/f1040s3.pdf](https://www.irs.gov/pub/irs-pdf/f1040s3.pdf) |
| Schedule A | [irs.gov/pub/irs-pdf/f1040sa.pdf](https://www.irs.gov/pub/irs-pdf/f1040sa.pdf) |
| Schedule B | [irs.gov/pub/irs-pdf/f1040sb.pdf](https://www.irs.gov/pub/irs-pdf/f1040sb.pdf) |
| Schedule C | [irs.gov/pub/irs-pdf/f1040sc.pdf](https://www.irs.gov/pub/irs-pdf/f1040sc.pdf) |
| Schedule D | [irs.gov/pub/irs-pdf/f1040sd.pdf](https://www.irs.gov/pub/irs-pdf/f1040sd.pdf) |
| Schedule E | [irs.gov/pub/irs-pdf/f1040se.pdf](https://www.irs.gov/pub/irs-pdf/f1040se.pdf) |
| Schedule SE | [irs.gov/pub/irs-pdf/f1040sse.pdf](https://www.irs.gov/pub/irs-pdf/f1040sse.pdf) |

### Information Returns

| Form | Link |
|------|------|
| W-2 | [irs.gov/pub/irs-pdf/fw2.pdf](https://www.irs.gov/pub/irs-pdf/fw2.pdf) |
| 1099-NEC | [irs.gov/pub/irs-pdf/f1099nec.pdf](https://www.irs.gov/pub/irs-pdf/f1099nec.pdf) |
| 1099-MISC | [irs.gov/pub/irs-pdf/f1099msc.pdf](https://www.irs.gov/pub/irs-pdf/f1099msc.pdf) |
| 1099-INT | [irs.gov/pub/irs-pdf/f1099int.pdf](https://www.irs.gov/pub/irs-pdf/f1099int.pdf) |
| 1099-DIV | [irs.gov/pub/irs-pdf/f1099div.pdf](https://www.irs.gov/pub/irs-pdf/f1099div.pdf) |
| 1098 | [irs.gov/pub/irs-pdf/f1098.pdf](https://www.irs.gov/pub/irs-pdf/f1098.pdf) |

### IRS Publications

| Publication | Purpose | Link |
|-------------|---------|------|
| Pub 1167 | Substitute Forms Rules | [irs.gov/pub/irs-pdf/p1167.pdf](https://www.irs.gov/pub/irs-pdf/p1167.pdf) |
| Pub 1179 | Information Returns Specs | [irs.gov/pub/irs-pdf/p1179.pdf](https://www.irs.gov/pub/irs-pdf/p1179.pdf) |

---

## References

1. IRS Publication 1167 - General Rules and Specifications for Substitute Forms
2. IRS Publication 1179 - Specifications for Information Returns
3. IRS Form 1040 Instructions
4. California FTB Developer Resources
5. New York DTF Publication 75
6. IRS MeF User Guides and Publications
7. Federation of Tax Administrators (FTA) Standards
