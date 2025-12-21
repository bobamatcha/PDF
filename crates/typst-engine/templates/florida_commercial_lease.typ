// ============================================================================
// FLORIDA COMMERCIAL LEASE AGREEMENT
// ============================================================================
// Governed by Florida Statutes Chapter 83, Part I (Non-Residential Tenancies)
// NOT Chapter 83, Part II (Residential) - Commercial has no habitability rights
// Updated for October 1, 2025 sales tax repeal
// ============================================================================

#let data = sys.inputs

// Helper functions
#let get(key, default: "") = data.at(key, default: default)
#let get_bool(key) = {
  let val = data.at(key, default: false)
  if type(val) == str { val == "true" } else { val == true }
}
#let get_num(key, default: 0) = {
  let val = data.at(key, default: default)
  if type(val) == str { float(val) } else { float(val) }
}
#let format_money(amount) = {
  let num = if type(amount) == str { float(amount) } else { float(amount) }
  "$" + str(calc.round(num, digits: 2))
}

// Page setup
#set page(
  paper: "us-letter",
  margin: (top: 0.75in, bottom: 0.75in, left: 1in, right: 1in),
  numbering: "1",
  number-align: center,
)
#set text(font: "New Computer Modern", size: 10pt)
#set par(justify: true, leading: 0.65em)

// ============================================================================
// COVER PAGE
// ============================================================================

#align(center)[
  #v(1.5in)

  #text(size: 24pt, weight: "bold")[COMMERCIAL LEASE AGREEMENT]

  #v(0.5em)

  #text(size: 14pt)[State of Florida]
  #v(0.2em)
  #text(size: 11pt, style: "italic")[Chapter 83, Part I - Non-Residential Tenancies]

  #v(2em)

  #rect(
    width: 80%,
    inset: 20pt,
    stroke: 2pt + black,
    radius: 4pt,
  )[
    #align(center)[
      #text(size: 12pt, weight: "bold")[PREMISES]
      #v(0.5em)
      #text(size: 14pt)[#get("property_address", default: "[Property Address]")]
      #v(0.3em)
      #text(size: 11pt)[#get("property_city", default: "[City]"), FL #get("property_zip", default: "[ZIP]")]
      #v(0.3em)
      #if get("suite_number", default: "") != "" [
        #text(size: 11pt)[Suite/Unit: #get("suite_number")]
        #v(0.2em)
      ]
      #text(size: 10pt)[Rentable Square Feet: #get("square_feet", default: "[SF]")]
    ]
  ]

  #v(2em)

  #grid(
    columns: (1fr, 1fr),
    gutter: 40pt,
    [
      #text(weight: "bold")[LANDLORD]
      #v(0.3em)
      #get("landlord_name", default: "[Landlord Name]")
    ],
    [
      #text(weight: "bold")[TENANT]
      #v(0.3em)
      #get("tenant_name", default: "[Tenant Name]")
    ]
  )

  #v(2em)

  #let lease_type = get("lease_type", default: "gross")

  #rect(
    width: 60%,
    inset: 12pt,
    stroke: 1pt + rgb("#059669"),
    fill: rgb("#ecfdf5"),
    radius: 4pt,
  )[
    #align(center)[
      #text(weight: "bold")[LEASE TYPE]
      #v(0.3em)
      #if lease_type == "nnn" [
        #text(size: 14pt, weight: "bold")[Triple Net (NNN)]
      ] else if lease_type == "modified_gross" [
        #text(size: 14pt, weight: "bold")[Modified Gross]
      ] else [
        #text(size: 14pt, weight: "bold")[Gross Lease]
      ]
    ]
  ]

  #v(3em)

  #text(size: 9pt, fill: rgb("#666"))[
    This commercial lease is governed by Florida Statutes Chapter 83, Part I (Non-Residential Tenancies).
    Residential tenant protections under Part II do NOT apply.
  ]
]

#pagebreak()

// ============================================================================
// TABLE OF CONTENTS
// ============================================================================

#text(size: 16pt, weight: "bold")[TABLE OF CONTENTS]
#v(1em)

#let toc_item(number, title) = [
  #box(width: 35pt)[#number]
  #title
  #v(0.3em)
]

#toc_item("1.", "PARTIES")
#toc_item("2.", "PREMISES AND PERMITTED USE")
#toc_item("3.", "TERM")
#toc_item("4.", "RENT")
#toc_item("5.", "SALES TAX ON RENT")
#toc_item("6.", "ADDITIONAL RENT / CAM CHARGES")
#toc_item("7.", "SECURITY DEPOSIT")
#toc_item("8.", "UTILITIES AND SERVICES")
#toc_item("9.", "MAINTENANCE AND REPAIRS")
#toc_item("10.", "ALTERATIONS AND IMPROVEMENTS")
#toc_item("11.", "INSURANCE")
#toc_item("12.", "INDEMNIFICATION")
#toc_item("13.", "DEFAULT AND REMEDIES")
#toc_item("14.", "TERMINATION")
#toc_item("15.", "ASSIGNMENT AND SUBLETTING")
#toc_item("16.", "COMPLIANCE WITH LAWS")
#toc_item("17.", "SIGNAGE")
#toc_item("18.", "ADDITIONAL PROVISIONS")
#toc_item("19.", "SIGNATURES")

#v(0.5em)

#text(weight: "bold")[OPTIONAL ADDENDA:]
#v(0.3em)
#toc_item("A.", "Agricultural Lien (§ 83.08, if applicable)")
#toc_item("B.", "Personal Guaranty (if applicable)")
#toc_item("C.", "Exclusivity Clause (if applicable)")

#pagebreak()

// ============================================================================
// SECTION 1: PARTIES
// ============================================================================

#text(size: 14pt, weight: "bold")[1. PARTIES]
#v(1em)

#text(size: 12pt, weight: "bold")[1.1 LANDLORD]
#v(0.5em)

#table(
  columns: (120pt, 1fr),
  stroke: none,
  inset: 5pt,
  [*Name/Entity:*], [#get("landlord_name", default: "[Landlord Legal Name]")],
  [*Address:*], [#get("landlord_address", default: "[Landlord Address]")],
  [*Phone:*], [#get("landlord_phone", default: "[Phone]")],
  [*Email:*], [#get("landlord_email", default: "[Email]")],
)

#v(1em)

#text(size: 12pt, weight: "bold")[1.2 TENANT]
#v(0.5em)

#table(
  columns: (120pt, 1fr),
  stroke: none,
  inset: 5pt,
  [*Name/Entity:*], [#get("tenant_name", default: "[Tenant Legal Name]")],
  [*Entity Type:*], [#get("tenant_entity_type", default: "[Corporation/LLC/Partnership/Individual]")],
  [*State of Formation:*], [#get("tenant_state", default: "[State]")],
  [*Address:*], [#get("tenant_address", default: "[Tenant Address]")],
  [*Phone:*], [#get("tenant_phone", default: "[Phone]")],
  [*Email:*], [#get("tenant_email", default: "[Email]")],
)

#pagebreak()

// ============================================================================
// SECTION 2: PREMISES AND PERMITTED USE
// ============================================================================

#text(size: 14pt, weight: "bold")[2. PREMISES AND PERMITTED USE]
#v(1em)

#text(size: 12pt, weight: "bold")[2.1 PREMISES]
#v(0.5em)

Landlord hereby leases to Tenant, and Tenant hereby leases from Landlord, the following described premises ("Premises"):

#table(
  columns: (120pt, 1fr),
  stroke: none,
  inset: 5pt,
  [*Address:*], [#get("property_address", default: "[Property Address]")],
  [*City, State, ZIP:*], [#get("property_city", default: "[City]"), FL #get("property_zip", default: "[ZIP]")],
  [*Suite/Unit:*], [#get("suite_number", default: "[Suite Number]")],
  [*Square Feet:*], [#get("square_feet", default: "[SF]") rentable square feet],
)

#v(1em)

#text(size: 12pt, weight: "bold")[2.2 PERMITTED USE]
#v(0.5em)

#rect(
  width: 100%,
  inset: 12pt,
  stroke: 1pt + rgb("#059669"),
  fill: rgb("#ecfdf5"),
  radius: 4pt,
)[
  #text(weight: "bold")[PERMITTED USE OF PREMISES]

  #v(0.5em)

  The Premises shall be used and occupied *only* for the following purpose(s):

  #v(0.3em)

  #get("permitted_use", default: "[Describe permitted business use - e.g., general office, retail sales, restaurant, warehouse, etc.]")

  #v(0.5em)

  Tenant shall not use or permit the Premises to be used for any other purpose without the prior written consent of Landlord.
]

#v(1em)

#text(size: 12pt, weight: "bold")[2.3 PROHIBITED USES]
#v(0.5em)

Tenant shall not use the Premises for any unlawful purpose or in any manner that:
- Violates any applicable law, ordinance, or regulation
- Creates a nuisance or interferes with other tenants
- Increases the fire hazard or insurance premiums
- Violates any exclusive use provisions granted to other tenants

#pagebreak()

// ============================================================================
// SECTION 3: TERM
// ============================================================================

#text(size: 14pt, weight: "bold")[3. TERM]
#v(1em)

#text(size: 12pt, weight: "bold")[3.1 LEASE TERM]
#v(0.5em)

#table(
  columns: (150pt, 1fr),
  stroke: 0.5pt,
  inset: 8pt,
  [*Commencement Date:*], [#get("lease_start", default: "[Start Date]")],
  [*Expiration Date:*], [#get("lease_end", default: "[End Date]")],
  [*Lease Term:*], [#get("lease_term_months", default: "[X]") months],
)

#v(1em)

#text(size: 12pt, weight: "bold")[3.2 RENEWAL OPTIONS]
#v(0.5em)

#if get_bool("has_renewal_option") [
  Tenant shall have the option to renew this Lease for #get("renewal_terms", default: "[X]") additional term(s) of #get("renewal_period", default: "[X]") months each, upon the following conditions:

  - Tenant provides written notice at least #get("renewal_notice_days", default: "180") days before expiration
  - Tenant is not in default at the time of renewal
  - Rent for renewal term shall be #get("renewal_rent_terms", default: "[at market rate / at X% increase]")
] else [
  This Lease contains no renewal options.
]

#v(1em)

#text(size: 12pt, weight: "bold")[3.3 EARLY POSSESSION]
#v(0.5em)

#if get_bool("early_possession") [
  Tenant may take early possession on #get("early_possession_date", default: "[Date]") for the purpose of #get("early_possession_purpose", default: "installing fixtures and equipment"). During early possession, all terms of this Lease shall apply except the obligation to pay Base Rent.
] else [
  Tenant shall not take possession prior to the Commencement Date without Landlord's written consent.
]

#pagebreak()

// ============================================================================
// SECTION 4: RENT
// ============================================================================

#text(size: 14pt, weight: "bold")[4. RENT]
#v(1em)

#text(size: 12pt, weight: "bold")[4.1 BASE RENT]
#v(0.5em)

#table(
  columns: (1fr, 150pt),
  stroke: 0.5pt,
  inset: 8pt,
  [*Annual Base Rent:*], [#format_money(get_num("annual_rent"))],
  [*Monthly Base Rent:*], [#format_money(get_num("monthly_rent"))],
  [*Rent Per Square Foot:*], [#format_money(get_num("rent_per_sf", default: 0))],
)

#v(1em)

#text(size: 12pt, weight: "bold")[4.2 RENT PAYMENT]
#v(0.5em)

Base Rent shall be due and payable in advance on the *first (1st) day* of each calendar month during the Lease Term, without demand, deduction, or offset.

#v(0.5em)

Rent shall be paid to:

#table(
  columns: (120pt, 1fr),
  stroke: none,
  inset: 5pt,
  [*Payee:*], [#get("rent_payee", default: "[Landlord or Management Company]")],
  [*Address:*], [#get("rent_address", default: "[Payment Address]")],
)

#v(1em)

#text(size: 12pt, weight: "bold")[4.3 RENT ESCALATION]
#v(0.5em)

#if get_bool("has_rent_escalation") [
  Base Rent shall increase as follows:

  #if get("escalation_type", default: "annual") == "annual" [
    - Annual increase of #get("escalation_percent", default: "3")% on each anniversary of the Commencement Date.
  ] else [
    - Per the rent schedule attached as Exhibit A.
  ]
] else [
  Base Rent shall remain fixed for the initial Lease Term.
]

#v(1em)

#text(size: 12pt, weight: "bold")[4.4 LATE CHARGES]
#v(0.5em)

If any installment of Rent is not received within #get("late_fee_grace_period", default: "5") days after its due date, Tenant shall pay a late charge of #get("late_fee_percent", default: "5")% of the overdue amount. This late charge is in addition to any other remedies available to Landlord.

#pagebreak()

// ============================================================================
// SECTION 5: SALES TAX ON RENT
// ============================================================================

#text(size: 14pt, weight: "bold")[5. SALES TAX ON RENT]
#v(1em)

#rect(
  width: 100%,
  inset: 15pt,
  stroke: 2pt + rgb("#b45309"),
  fill: rgb("#fffbeb"),
  radius: 4pt,
)[
  #text(weight: "bold")[FLORIDA SALES TAX ON COMMERCIAL RENT - TRANSITION PROVISION]

  #v(0.5em)

  *Historical Requirement:* Florida is the only state that has historically imposed sales tax on commercial rent payments.

  #v(0.5em)

  *Tax Repeal:* Effective *October 1, 2025*, Florida's sales tax on commercial rent is REPEALED.

  #v(0.5em)

  #text(weight: "bold")[TRANSITION CLAUSE:]

  #v(0.3em)

  Tenant shall pay, in addition to Base Rent, all applicable Florida sales tax and local discretionary surtax on commercial rent payments.

  The parties acknowledge that:

  1. For *occupancy periods through September 30, 2025*, Tenant shall pay the applicable state sales tax (2.0%) plus any local discretionary surtax on all rent payments.

  2. For *occupancy periods commencing on or after October 1, 2025*, no sales tax shall be due on rent payments, as the tax is repealed effective that date.

  3. The tax obligation is determined by the *occupancy period*, not the payment date:
     - October 2025 rent paid in September 2025 = *Tax-free*
     - September 2025 rent paid in October 2025 = *Taxable*

  Landlord shall cease collection of sales tax for occupancy periods commencing on or after the effective date of the repeal. If the Legislature delays or modifies the repeal, the obligation to pay applicable tax shall continue pursuant to law.
]

#pagebreak()

// ============================================================================
// SECTION 6: ADDITIONAL RENT / CAM CHARGES
// ============================================================================

#text(size: 14pt, weight: "bold")[6. ADDITIONAL RENT / CAM CHARGES]
#v(1em)

#let lease_type = get("lease_type", default: "gross")

#text(size: 12pt, weight: "bold")[6.1 LEASE TYPE]
#v(0.5em)

#if lease_type == "nnn" [
  #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] *TRIPLE NET (NNN) LEASE* - Tenant pays Base Rent PLUS all operating expenses.
] else [
  #box(width: 12pt, height: 12pt, stroke: 1pt)[] *TRIPLE NET (NNN) LEASE* - Tenant pays Base Rent PLUS all operating expenses.
]

#v(0.3em)

#if lease_type == "modified_gross" [
  #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] *MODIFIED GROSS LEASE* - Tenant pays Base Rent plus specified expenses.
] else [
  #box(width: 12pt, height: 12pt, stroke: 1pt)[] *MODIFIED GROSS LEASE* - Tenant pays Base Rent plus specified expenses.
]

#v(0.3em)

#if lease_type == "gross" [
  #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] *GROSS LEASE* - Tenant pays Base Rent only; Landlord pays operating expenses.
] else [
  #box(width: 12pt, height: 12pt, stroke: 1pt)[] *GROSS LEASE* - Tenant pays Base Rent only; Landlord pays operating expenses.
]

#v(1em)

#if lease_type == "nnn" or lease_type == "modified_gross" [
  #text(size: 12pt, weight: "bold")[6.2 COMMON AREA MAINTENANCE (CAM) CHARGES]
  #v(0.5em)

  #rect(
    width: 100%,
    inset: 12pt,
    stroke: 1pt + black,
    radius: 4pt,
  )[
    #text(weight: "bold")[CAM / OPERATING EXPENSES]

    #v(0.5em)

    Tenant shall pay its proportionate share of Common Area Maintenance and Operating Expenses as Additional Rent.

    #v(0.3em)

    *Tenant's Pro Rata Share:* #get("pro_rata_share", default: "[X]")% (based on #get("square_feet", default: "[X]") SF / #get("building_total_sf", default: "[X]") SF)

    #v(0.5em)

    *Operating Expenses include (but are not limited to):*

    - Property taxes and assessments
    - Property insurance
    - Common area utilities
    - Landscaping and grounds maintenance
    - Parking lot maintenance and lighting
    - Building exterior maintenance
    - Property management fees (capped at #get("management_fee_cap", default: "5")% of gross receipts)
    - Security services
    - Trash removal
  ]

  #v(1em)

  #text(size: 12pt, weight: "bold")[6.3 CAM PAYMENT AND RECONCILIATION]
  #v(0.5em)

  - Tenant shall pay *estimated* monthly CAM charges of #format_money(get_num("estimated_monthly_cam", default: 0)) per month.
  - Landlord shall provide an annual reconciliation within 90 days after each calendar year.
  - If actual expenses exceed estimates, Tenant shall pay the difference within 30 days.
  - If actual expenses are less than estimates, Landlord shall credit Tenant's account.

  #v(1em)

  #text(size: 12pt, weight: "bold")[6.4 CAM EXCLUSIONS]
  #v(0.5em)

  Operating Expenses shall NOT include:
  - Capital improvements (except if amortized over useful life)
  - Costs covered by insurance or warranties
  - Leasing commissions or legal fees
  - Landlord's income taxes
  - Expenses for other tenants' spaces
]

#pagebreak()

// ============================================================================
// SECTION 7: SECURITY DEPOSIT
// ============================================================================

#text(size: 14pt, weight: "bold")[7. SECURITY DEPOSIT]
#v(1em)

#table(
  columns: (1fr, 150pt),
  stroke: 0.5pt,
  inset: 8pt,
  [*Security Deposit Amount:*], [#format_money(get_num("security_deposit"))],
)

#v(1em)

The Security Deposit shall be held by Landlord as security for the faithful performance of Tenant's obligations under this Lease.

#v(0.5em)

*Note:* Florida Statutes Chapter 83, Part I (commercial) does NOT require:
- Interest payments on security deposits
- Specific holding requirements
- 15-day return timelines

The deposit shall be returned within #get("deposit_return_days", default: "30") days after Lease termination, less any amounts deducted for unpaid rent, damages beyond normal wear and tear, or other Tenant obligations.

#pagebreak()

// ============================================================================
// SECTION 8: UTILITIES AND SERVICES
// ============================================================================

#text(size: 14pt, weight: "bold")[8. UTILITIES AND SERVICES]
#v(1em)

Tenant shall be responsible for the following utilities and services at Tenant's sole cost:

#v(0.5em)

#grid(
  columns: (1fr, 1fr),
  gutter: 20pt,
  [
    #if get_bool("tenant_pays_electric") [#box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark]] else [#box(width: 12pt, height: 12pt, stroke: 1pt)[]] Electricity

    #if get_bool("tenant_pays_gas") [#box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark]] else [#box(width: 12pt, height: 12pt, stroke: 1pt)[]] Natural Gas

    #if get_bool("tenant_pays_water") [#box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark]] else [#box(width: 12pt, height: 12pt, stroke: 1pt)[]] Water/Sewer
  ],
  [
    #if get_bool("tenant_pays_trash") [#box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark]] else [#box(width: 12pt, height: 12pt, stroke: 1pt)[]] Trash Removal

    #if get_bool("tenant_pays_internet") [#box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark]] else [#box(width: 12pt, height: 12pt, stroke: 1pt)[]] Telephone/Internet

    #if get_bool("tenant_pays_janitorial") [#box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark]] else [#box(width: 12pt, height: 12pt, stroke: 1pt)[]] Janitorial Services
  ]
)

#v(1em)

Tenant shall establish utility accounts in Tenant's name prior to the Commencement Date.

#pagebreak()

// ============================================================================
// SECTION 9: MAINTENANCE AND REPAIRS
// ============================================================================

#text(size: 14pt, weight: "bold")[9. MAINTENANCE AND REPAIRS]
#v(1em)

#rect(
  width: 100%,
  inset: 12pt,
  stroke: 2pt + rgb("#dc2626"),
  fill: rgb("#fef2f2"),
  radius: 4pt,
)[
  #text(weight: "bold", fill: rgb("#dc2626"))[COMMERCIAL LEASE - NO IMPLIED WARRANTY OF HABITABILITY]

  #v(0.5em)

  TENANT ACKNOWLEDGES THAT THIS IS A COMMERCIAL LEASE GOVERNED BY FLORIDA STATUTES CHAPTER 83, PART I. UNLIKE RESIDENTIAL LEASES, THERE IS *NO IMPLIED WARRANTY OF HABITABILITY* OR FITNESS FOR A PARTICULAR PURPOSE.

  #v(0.3em)

  THE PREMISES ARE LEASED *AS-IS* IN THEIR PRESENT CONDITION. LANDLORD MAKES NO WARRANTIES, EXPRESS OR IMPLIED, REGARDING THE CONDITION OF THE PREMISES.
]

#v(1em)

#text(size: 12pt, weight: "bold")[9.1 TENANT'S MAINTENANCE OBLIGATIONS]
#v(0.5em)

Tenant shall, at Tenant's sole cost, maintain in good condition:

- Interior of the Premises (walls, floors, ceilings, doors, windows)
- All fixtures, equipment, and improvements installed by Tenant
- HVAC systems serving the Premises (including regular filter changes and annual service)
- Plumbing fixtures within the Premises
- Interior lighting and electrical fixtures

#v(1em)

#text(size: 12pt, weight: "bold")[9.2 LANDLORD'S MAINTENANCE OBLIGATIONS]
#v(0.5em)

Landlord shall maintain:

- Structural elements (roof, foundation, exterior walls)
- Common areas and parking lots
- Building systems serving multiple tenants
- Compliance with building codes

#v(0.5em)

*Roof Repairs:* #if get_bool("tenant_pays_roof") [Tenant's responsibility under NNN terms.] else [Landlord's responsibility (costs may be passed through as CAM).]

#pagebreak()

// ============================================================================
// SECTION 10: ALTERATIONS AND IMPROVEMENTS
// ============================================================================

#text(size: 14pt, weight: "bold")[10. ALTERATIONS AND IMPROVEMENTS]
#v(1em)

#text(size: 12pt, weight: "bold")[10.1 TENANT IMPROVEMENTS]
#v(0.5em)

Tenant shall not make any alterations, additions, or improvements to the Premises without Landlord's prior written consent, which consent shall not be unreasonably withheld for non-structural improvements.

#v(1em)

#text(size: 12pt, weight: "bold")[10.2 OWNERSHIP OF IMPROVEMENTS]
#v(0.5em)

All alterations, additions, and improvements made by Tenant shall become the property of Landlord upon installation and shall remain upon the Premises at the expiration or termination of this Lease, unless Landlord requires removal in writing.

#v(1em)

#text(size: 12pt, weight: "bold")[10.3 TRADE FIXTURES]
#v(0.5em)

Tenant may remove trade fixtures, equipment, and personal property installed by Tenant, provided:
- Tenant is not in default
- Removal is completed before Lease expiration
- Tenant repairs any damage caused by removal

#pagebreak()

// ============================================================================
// SECTION 11: INSURANCE
// ============================================================================

#text(size: 14pt, weight: "bold")[11. INSURANCE]
#v(1em)

#text(size: 12pt, weight: "bold")[11.1 TENANT'S INSURANCE]
#v(0.5em)

Tenant shall maintain, at Tenant's expense:

#table(
  columns: (1fr, 150pt),
  stroke: 0.5pt,
  inset: 8pt,
  [*Commercial General Liability:*], [#format_money(get_num("liability_coverage", default: 1000000)) per occurrence],
  [*Property Insurance (Contents):*], [Replacement cost of Tenant's property],
  [*Workers' Compensation:*], [Statutory limits],
)

#v(0.5em)

Tenant's insurance shall:
- Name Landlord as additional insured
- Provide 30 days' written notice of cancellation
- Be issued by insurers rated A- or better by A.M. Best

#v(1em)

#text(size: 12pt, weight: "bold")[11.2 LANDLORD'S INSURANCE]
#v(0.5em)

Landlord shall maintain property insurance covering the Building and common areas. Cost may be passed through to Tenant as Operating Expenses under NNN or Modified Gross leases.

#v(1em)

#text(size: 12pt, weight: "bold")[11.3 WAIVER OF SUBROGATION]
#v(0.5em)

Each party waives any right of recovery against the other party for any loss covered by the waiving party's insurance policies.

#pagebreak()

// ============================================================================
// SECTION 12: INDEMNIFICATION
// ============================================================================

#text(size: 14pt, weight: "bold")[12. INDEMNIFICATION]
#v(1em)

Tenant shall indemnify, defend, and hold harmless Landlord from and against all claims, damages, losses, and expenses arising from:

- Tenant's use and occupancy of the Premises
- Any act or omission of Tenant, its employees, agents, or invitees
- Any breach of this Lease by Tenant

Landlord shall indemnify, defend, and hold harmless Tenant from and against all claims arising from Landlord's negligence or willful misconduct.

#pagebreak()

// ============================================================================
// SECTION 13: DEFAULT AND REMEDIES
// ============================================================================

#text(size: 14pt, weight: "bold")[13. DEFAULT AND REMEDIES]
#v(1em)

#text(size: 12pt, weight: "bold")[13.1 EVENTS OF DEFAULT]
#v(0.5em)

The following shall constitute an Event of Default:

- Failure to pay Rent within #get("rent_default_days", default: "10") days after written notice
- Failure to perform any other obligation within #get("other_default_days", default: "30") days after written notice
- Abandonment of the Premises
- Filing of bankruptcy or insolvency proceedings
- Assignment for the benefit of creditors

#v(1em)

#text(size: 12pt, weight: "bold")[13.2 LANDLORD'S REMEDIES]
#v(0.5em)

Upon an Event of Default, Landlord may:

- Terminate this Lease upon written notice
- Re-enter and take possession of the Premises
- Sue for all Rent due through the end of the Term
- Relet the Premises and hold Tenant liable for any deficiency
- Exercise any other remedies available at law or in equity

#v(1em)

#text(size: 12pt, weight: "bold")[13.3 ACCELERATION]
#v(0.5em)

Upon default, Landlord may declare all remaining Rent for the Lease Term immediately due and payable.

#pagebreak()

// ============================================================================
// SECTION 14: TERMINATION
// ============================================================================

#text(size: 14pt, weight: "bold")[14. TERMINATION]
#v(1em)

#text(size: 12pt, weight: "bold")[14.1 TERMINATION NOTICE]
#v(0.5em)

#rect(
  width: 100%,
  inset: 12pt,
  stroke: 1pt + rgb("#2563eb"),
  fill: rgb("#eff6ff"),
  radius: 4pt,
)[
  #text(weight: "bold")[COMMERCIAL LEASE TERMINATION - FLEXIBLE TERMS]

  #v(0.5em)

  Unlike residential leases, Florida Statutes Chapter 83, Part I allows commercial landlords and tenants significant flexibility in termination notice requirements.

  #v(0.3em)

  *Termination Notice Required:* #get("termination_notice_days", default: "60") days written notice before the end of the Lease Term.

  #v(0.3em)

  *Month-to-Month Holdover:* If Tenant remains after expiration without a new agreement, tenancy shall be month-to-month, terminable upon #get("holdover_notice_days", default: "30") days' written notice by either party.
]

#v(1em)

#text(size: 12pt, weight: "bold")[14.2 EARLY TERMINATION]
#v(0.5em)

#if get_bool("has_early_termination") [
  Tenant may terminate this Lease early upon the following conditions:

  - Payment of early termination fee: #format_money(get_num("early_termination_fee"))
  - Provision of #get("early_termination_notice", default: "90") days' written notice
  - Tenant not in default at time of notice

  This provision shall not become effective until after the #get("early_termination_lockout", default: "12")th month of the Lease Term.
] else [
  This Lease does not contain an early termination option. Tenant is obligated to pay Rent through the full Lease Term.
]

#v(1em)

#text(size: 12pt, weight: "bold")[14.3 SURRENDER OF PREMISES]
#v(0.5em)

Upon termination, Tenant shall:
- Remove all personal property and trade fixtures
- Return all keys and access devices
- Leave Premises in broom-clean condition
- Repair any damage beyond normal wear and tear

#pagebreak()

// ============================================================================
// SECTION 15: ASSIGNMENT AND SUBLETTING
// ============================================================================

#text(size: 14pt, weight: "bold")[15. ASSIGNMENT AND SUBLETTING]
#v(1em)

Tenant shall not assign this Lease or sublet all or any portion of the Premises without Landlord's prior written consent, which:

#if get_bool("assignment_unrestricted") [
  - Shall not be unreasonably withheld, conditioned, or delayed
] else [
  - May be withheld in Landlord's sole discretion
]

#v(0.5em)

Any assignment or sublease shall not release Tenant from liability under this Lease unless Landlord expressly agrees in writing.

#pagebreak()

// ============================================================================
// SECTION 16: COMPLIANCE WITH LAWS
// ============================================================================

#text(size: 14pt, weight: "bold")[16. COMPLIANCE WITH LAWS]
#v(1em)

Tenant shall comply with all federal, state, and local laws, ordinances, and regulations applicable to Tenant's use of the Premises, including but not limited to:

- Americans with Disabilities Act (ADA)
- Fire and safety codes
- Environmental regulations
- Zoning ordinances
- Health department requirements

Tenant shall be responsible for obtaining all permits and licenses required for Tenant's business operations.

#pagebreak()

// ============================================================================
// SECTION 17: SIGNAGE
// ============================================================================

#text(size: 14pt, weight: "bold")[17. SIGNAGE]
#v(1em)

#if get_bool("signage_allowed") [
  Tenant may install signage on the Premises subject to:

  - Landlord's prior written approval of size, design, and location
  - Compliance with all applicable sign ordinances
  - Tenant's cost for installation, maintenance, and removal
  - Removal and restoration upon Lease termination

  *Allocated Signage:* #get("signage_description", default: "[Describe allocated signage rights]")
] else [
  Tenant shall not install any exterior signage without Landlord's prior written consent.
]

#pagebreak()

// ============================================================================
// SECTION 18: ADDITIONAL PROVISIONS
// ============================================================================

#text(size: 14pt, weight: "bold")[18. ADDITIONAL PROVISIONS]
#v(1em)

#text(size: 12pt, weight: "bold")[18.1 ENTIRE AGREEMENT]
#v(0.5em)

This Lease, together with all exhibits and addenda, constitutes the entire agreement between the parties. No prior negotiations, representations, or agreements shall be binding unless incorporated herein.

#v(1em)

#text(size: 12pt, weight: "bold")[18.2 GOVERNING LAW]
#v(0.5em)

This Lease shall be governed by the laws of the State of Florida, specifically Chapter 83, Part I (Non-Residential Tenancies).

#v(1em)

#text(size: 12pt, weight: "bold")[18.3 ATTORNEY'S FEES]
#v(0.5em)

In any action to enforce this Lease, the prevailing party shall be entitled to recover reasonable attorney's fees and costs.

#v(1em)

#text(size: 12pt, weight: "bold")[18.4 NOTICES]
#v(0.5em)

All notices shall be in writing and delivered by:
- Certified mail, return receipt requested
- Hand delivery with signed receipt
- Recognized overnight courier

Notices shall be sent to the addresses set forth in Section 1.

#v(1em)

#text(size: 12pt, weight: "bold")[18.5 ADDITIONAL TERMS]
#v(0.5em)

#if get("additional_terms", default: "") != "" [
  #get("additional_terms")
] else [
  [None]
]

#pagebreak()

// ============================================================================
// SECTION 19: SIGNATURES
// ============================================================================

#text(size: 14pt, weight: "bold")[19. SIGNATURES]
#v(1em)

BY SIGNING BELOW, THE PARTIES ACKNOWLEDGE THAT THEY HAVE READ THIS COMMERCIAL LEASE IN ITS ENTIRETY, UNDERSTAND ITS TERMS, AND AGREE TO BE BOUND THEREBY.

#v(2em)

#grid(
  columns: (1fr, 1fr),
  gutter: 40pt,
  [
    #text(weight: "bold")[LANDLORD]
    #v(0.5em)
    #get("landlord_name", default: "[Landlord Name]")
    #v(2em)
    Signature: #box(width: 180pt, repeat[\_])
    #v(0.5em)
    Print Name: #box(width: 180pt, repeat[\_])
    #v(0.5em)
    Title: #box(width: 180pt, repeat[\_])
    #v(0.5em)
    Date: #box(width: 120pt, repeat[\_])
  ],
  [
    #text(weight: "bold")[TENANT]
    #v(0.5em)
    #get("tenant_name", default: "[Tenant Name]")
    #v(2em)
    Signature: #box(width: 180pt, repeat[\_])
    #v(0.5em)
    Print Name: #box(width: 180pt, repeat[\_])
    #v(0.5em)
    Title: #box(width: 180pt, repeat[\_])
    #v(0.5em)
    Date: #box(width: 120pt, repeat[\_])
  ]
)

#pagebreak()

// ============================================================================
// ADDENDUM A: AGRICULTURAL LIEN (§ 83.08) - Optional
// ============================================================================

#if get_bool("is_agricultural") [
  #text(size: 14pt, weight: "bold")[ADDENDUM A: AGRICULTURAL LIEN]
  #v(0.5em)
  #text(size: 10pt, style: "italic")[Pursuant to Florida Statutes § 83.08]
  #v(1em)

  #rect(
    width: 100%,
    inset: 15pt,
    stroke: 2pt + rgb("#059669"),
    fill: rgb("#ecfdf5"),
    radius: 4pt,
  )[
    #text(weight: "bold")[LANDLORD'S AGRICULTURAL LIEN]

    #v(0.5em)

    This Addendum applies to the lease of land for agricultural purposes.

    #v(0.5em)

    Pursuant to Florida Statutes § 83.08, Landlord retains a *statutory lien* on:

    - All agricultural products and crops grown, raised, or produced on the Premises
    - All property of Tenant kept on the Premises

    #v(0.5em)

    This lien is *superior to all other liens* (except liens of record prior to the commencement of this Lease) and secures payment of:

    - Rent due under this Lease
    - Any other amounts owed by Tenant to Landlord under this Lease

    #v(0.5em)

    #text(size: 10pt, style: "italic")[
      Note: This statutory lien provides priority over other creditors (including equipment financiers) in the event of Tenant's default or bankruptcy.
    ]
  ]

  #v(2em)

  #grid(
    columns: (1fr, 1fr),
    gutter: 20pt,
    [
      Landlord Initials: #box(width: 60pt, repeat[\_])
    ],
    [
      Tenant Initials: #box(width: 60pt, repeat[\_])
    ]
  )

  #pagebreak()
]

// ============================================================================
// ADDENDUM B: PERSONAL GUARANTY - Optional
// ============================================================================

#if get_bool("requires_guaranty") [
  #text(size: 14pt, weight: "bold")[ADDENDUM B: PERSONAL GUARANTY]
  #v(1em)

  #rect(
    width: 100%,
    inset: 15pt,
    stroke: 2pt + rgb("#dc2626"),
    fill: rgb("#fef2f2"),
    radius: 4pt,
  )[
    #text(weight: "bold")[PERSONAL GUARANTY OF LEASE]

    #v(0.5em)

    The undersigned Guarantor(s), in consideration of Landlord entering into the foregoing Commercial Lease Agreement with Tenant, hereby absolutely and unconditionally guarantees the full and timely payment and performance of all of Tenant's obligations under the Lease.

    #v(0.5em)

    This Guaranty is:
    - Unconditional and continuing
    - Not subject to any defenses available to Tenant
    - Binding upon Guarantor's heirs, successors, and assigns
    - Enforceable without first pursuing remedies against Tenant
  ]

  #v(2em)

  *Guarantor:*

  #v(1.5em)

  Signature: #box(width: 250pt, repeat[\_])

  #v(0.5em)

  Print Name: #get("guarantor_name", default: "[Guarantor Name]")

  #v(0.5em)

  Address: #get("guarantor_address", default: "[Guarantor Address]")

  #v(0.5em)

  Date: #box(width: 120pt, repeat[\_])

  #pagebreak()
]

// ============================================================================
// ADDENDUM C: EXCLUSIVITY CLAUSE - Optional
// ============================================================================

#if get_bool("has_exclusivity") [
  #text(size: 14pt, weight: "bold")[ADDENDUM C: EXCLUSIVITY CLAUSE]
  #v(1em)

  #rect(
    width: 100%,
    inset: 15pt,
    stroke: 1pt + rgb("#7c3aed"),
    fill: rgb("#f5f3ff"),
    radius: 4pt,
  )[
    #text(weight: "bold")[EXCLUSIVE USE PROVISION]

    #v(0.5em)

    Landlord agrees that during the Lease Term, Landlord shall not lease any other space in the Building or Property to any tenant whose primary business is:

    #v(0.3em)

    #get("exclusivity_description", default: "[Describe protected business category]")

    #v(0.5em)

    *Exceptions:* This exclusivity shall not apply to:
    - Existing tenants as of the Commencement Date
    - Incidental sales by other tenants (less than #get("exclusivity_threshold", default: "10")% of gross revenue)
    - Tenants in separate buildings on the Property

    #v(0.5em)

    *Remedy:* If Landlord breaches this exclusivity, Tenant's sole remedy shall be:
    - Rent abatement of #get("exclusivity_abatement", default: "50")% until the breach is cured; OR
    - Termination of this Lease upon 30 days' written notice
  ]

  #v(2em)

  #grid(
    columns: (1fr, 1fr),
    gutter: 20pt,
    [
      Landlord Initials: #box(width: 60pt, repeat[\_])
    ],
    [
      Tenant Initials: #box(width: 60pt, repeat[\_])
    ]
  )

  #pagebreak()
]

// ============================================================================
// FOOTER
// ============================================================================

#v(2em)
#line(length: 100%, stroke: 0.5pt)
#v(0.5em)

#text(size: 8pt, fill: rgb("#666"))[
  This Commercial Lease Agreement is governed by Florida Statutes Chapter 83, Part I (Non-Residential Tenancies). Residential tenant protections under Chapter 83, Part II do NOT apply to this commercial lease. The parties should consult with a licensed attorney before signing. Florida's sales tax on commercial rent is scheduled to be repealed effective October 1, 2025. Time is of the essence for all provisions of this Lease.
]
