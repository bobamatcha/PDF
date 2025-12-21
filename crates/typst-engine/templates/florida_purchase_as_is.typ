// ============================================================================
// FLORIDA "AS-IS" RESIDENTIAL PURCHASE CONTRACT
// ============================================================================
// Based on FAR/BAR "As-Is" Residential Contract for Sale and Purchase
// Key Feature: Buyer's sole discretion termination during inspection period
// Compliant with: F.S. Chapter 475, Chapter 689, § 404.056, § 692.204 (SB 264)
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

  #text(size: 24pt, weight: "bold")[RESIDENTIAL REAL ESTATE]
  #v(0.2em)
  #text(size: 24pt, weight: "bold")[PURCHASE CONTRACT]
  #v(0.3em)
  #text(size: 18pt, weight: "bold", fill: rgb("#dc2626"))["AS IS"]

  #v(0.5em)

  #text(size: 12pt)[State of Florida]

  #v(2em)

  #rect(
    width: 80%,
    inset: 20pt,
    stroke: 2pt + black,
    radius: 4pt,
  )[
    #align(center)[
      #text(size: 12pt, weight: "bold")[PROPERTY ADDRESS]
      #v(0.5em)
      #text(size: 14pt)[#get("property_address", default: "[Property Address]")]
      #v(0.3em)
      #text(size: 11pt)[#get("property_city", default: "[City]"), FL #get("property_zip", default: "[ZIP]")]
      #v(0.3em)
      #text(size: 10pt)[County: #get("property_county", default: "[County]")]
      #if get("parcel_id", default: "") != "" [
        #v(0.2em)
        #text(size: 10pt)[Parcel ID: #get("parcel_id")]
      ]
    ]
  ]

  #v(2em)

  #grid(
    columns: (1fr, 1fr),
    gutter: 40pt,
    [
      #text(weight: "bold")[SELLER(S)]
      #v(0.3em)
      #get("seller_name", default: "[Seller Name]")
    ],
    [
      #text(weight: "bold")[BUYER(S)]
      #v(0.3em)
      #get("buyer_name", default: "[Buyer Name]")
    ]
  )

  #v(2em)

  #text(size: 14pt, weight: "bold")[
    Purchase Price: #format_money(get_num("purchase_price"))
  ]

  #v(1em)

  #rect(
    width: 80%,
    inset: 12pt,
    stroke: 2pt + rgb("#dc2626"),
    fill: rgb("#fef2f2"),
    radius: 4pt,
  )[
    #align(center)[
      #text(size: 11pt, weight: "bold", fill: rgb("#dc2626"))[
        "AS IS" CONTRACT - NO SELLER REPAIR OBLIGATIONS
      ]
      #v(0.3em)
      #text(size: 9pt)[
        Buyer acknowledges property is being sold in its present condition. Buyer's sole remedy during Inspection Period is to cancel this Contract.
      ]
    ]
  ]

  #v(3em)

  #text(size: 9pt, fill: rgb("#666"))[
    This contract is governed by Florida law, including F.S. Chapter 475, Chapter 689, § 404.056, and applicable federal regulations.
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

#toc_item("1.", "PARTIES AND PROPERTY")
#toc_item("2.", "PURCHASE PRICE AND DEPOSITS")
#toc_item("3.", "FINANCING")
#toc_item("4.", "INSPECTION PERIOD (AS-IS)")
#toc_item("5.", "TITLE AND SURVEY")
#toc_item("6.", "CLOSING")
#toc_item("7.", "PROPERTY CONDITION")
#toc_item("8.", "RISK OF LOSS")
#toc_item("9.", "DEFAULT AND REMEDIES")
#toc_item("10.", "DISPUTE RESOLUTION")
#toc_item("11.", "ADDITIONAL TERMS")
#toc_item("12.", "SIGNATURES")

#v(0.5em)

#text(weight: "bold")[MANDATORY DISCLOSURES:]
#v(0.3em)
#toc_item("A.", "Radon Gas Notification (§ 404.056)")
#toc_item("B.", "Property Tax Disclosure (§ 689.261)")
#toc_item("C.", "Flood Disclosure (§ 689.302)")
#toc_item("D.", "Energy Efficiency Disclosure (§ 553.996)")
#toc_item("E.", "Lead-Based Paint Disclosure (if pre-1978)")
#toc_item("F.", "HOA/Community Disclosure (§ 720.401)")
#toc_item("G.", "Foreign Ownership Disclosure (SB 264 / § 692.204)")

#v(0.5em)

#text(weight: "bold")[OPTIONAL ADDENDA:]
#v(0.3em)
#toc_item("H.", "Appraisal Gap Guarantee (if applicable)")
#toc_item("I.", "Condo/HOA Rider (if applicable)")
#toc_item("J.", "CDD Disclosure (§ 190.048, if applicable)")

#pagebreak()

// ============================================================================
// SECTION 1: PARTIES AND PROPERTY
// ============================================================================

#text(size: 14pt, weight: "bold")[1. PARTIES AND PROPERTY]
#v(1em)

#text(size: 12pt, weight: "bold")[1.1 SELLER]
#v(0.5em)

#table(
  columns: (120pt, 1fr),
  stroke: none,
  inset: 5pt,
  [*Name(s):*], [#get("seller_name", default: "[Seller Legal Name]")],
  [*Address:*], [#get("seller_address", default: "[Seller Address]")],
  [*Phone:*], [#get("seller_phone", default: "[Phone]")],
  [*Email:*], [#get("seller_email", default: "[Email]")],
)

#v(1em)

#text(size: 12pt, weight: "bold")[1.2 BUYER]
#v(0.5em)

#table(
  columns: (120pt, 1fr),
  stroke: none,
  inset: 5pt,
  [*Name(s):*], [#get("buyer_name", default: "[Buyer Legal Name]")],
  [*Address:*], [#get("buyer_address", default: "[Buyer Address]")],
  [*Phone:*], [#get("buyer_phone", default: "[Phone]")],
  [*Email:*], [#get("buyer_email", default: "[Email]")],
)

#v(1em)

#text(size: 12pt, weight: "bold")[1.3 PROPERTY]
#v(0.5em)

#table(
  columns: (120pt, 1fr),
  stroke: none,
  inset: 5pt,
  [*Street Address:*], [#get("property_address", default: "[Property Address]")],
  [*City:*], [#get("property_city", default: "[City]")],
  [*County:*], [#get("property_county", default: "[County]")],
  [*ZIP Code:*], [#get("property_zip", default: "[ZIP]")],
  [*Parcel ID:*], [#get("parcel_id", default: "[Parcel ID Number]")],
)

#v(0.5em)

#text(weight: "bold")[Legal Description:]

#get("legal_description", default: "[Legal description per deed or as attached Exhibit A]")

#v(1em)

The Property includes all fixtures and improvements, unless specifically excluded, and is subject to easements, restrictions, and reservations of record.

#pagebreak()

// ============================================================================
// SECTION 2: PURCHASE PRICE AND DEPOSITS
// ============================================================================

#text(size: 14pt, weight: "bold")[2. PURCHASE PRICE AND DEPOSITS]
#v(1em)

#text(size: 12pt, weight: "bold")[2.1 PURCHASE PRICE]
#v(0.5em)

#table(
  columns: (1fr, 150pt),
  stroke: 0.5pt,
  inset: 8pt,
  [*Purchase Price:*], [#format_money(get_num("purchase_price"))],
)

#v(1em)

#text(size: 12pt, weight: "bold")[2.2 EARNEST MONEY DEPOSIT]
#v(0.5em)

#table(
  columns: (1fr, 150pt),
  stroke: 0.5pt,
  inset: 8pt,
  [*Initial Deposit (due within 3 days):*], [#format_money(get_num("initial_deposit", default: 0))],
  [*Additional Deposit (due after Inspection Period):*], [#format_money(get_num("additional_deposit", default: 0))],
  [*Total Deposit:*], [#format_money(get_num("initial_deposit", default: 0) + get_num("additional_deposit", default: 0))],
)

#v(1em)

#text(size: 12pt, weight: "bold")[2.3 ESCROW AGENT]
#v(0.5em)

Deposits shall be held in escrow by:

#table(
  columns: (120pt, 1fr),
  stroke: none,
  inset: 5pt,
  [*Escrow Agent:*], [#get("escrow_agent", default: "[Escrow Agent/Title Company]")],
  [*Address:*], [#get("escrow_address", default: "[Address]")],
  [*Phone:*], [#get("escrow_phone", default: "[Phone]")],
)

#v(0.5em)

Escrow Agent shall hold deposits in a non-interest-bearing account in accordance with Chapter 475, Florida Statutes.

#pagebreak()

// ============================================================================
// SECTION 3: FINANCING
// ============================================================================

#text(size: 14pt, weight: "bold")[3. FINANCING]
#v(1em)

#let financing_type = get("financing_type", default: "conventional")

#text(weight: "bold")[This Contract is:]
#v(0.5em)

#if financing_type == "cash" [
  #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] *CASH* - No financing contingency. Buyer has funds available to close.
] else [
  #box(width: 12pt, height: 12pt, stroke: 1pt)[] *CASH* - No financing contingency. Buyer has funds available to close.
]

#v(0.3em)

#if financing_type == "conventional" [
  #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] *CONVENTIONAL FINANCING* - Subject to obtaining a conventional mortgage loan.
] else [
  #box(width: 12pt, height: 12pt, stroke: 1pt)[] *CONVENTIONAL FINANCING* - Subject to obtaining a conventional mortgage loan.
]

#v(0.3em)

#if financing_type == "fha" [
  #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] *FHA FINANCING* - Subject to obtaining an FHA-insured loan.
] else [
  #box(width: 12pt, height: 12pt, stroke: 1pt)[] *FHA FINANCING* - Subject to obtaining an FHA-insured loan.
]

#v(0.3em)

#if financing_type == "va" [
  #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] *VA FINANCING* - Subject to obtaining a VA-guaranteed loan.
] else [
  #box(width: 12pt, height: 12pt, stroke: 1pt)[] *VA FINANCING* - Subject to obtaining a VA-guaranteed loan.
]

#v(1em)

#if financing_type != "cash" [
  #text(size: 12pt, weight: "bold")[3.1 LOAN TERMS]
  #v(0.5em)

  #table(
    columns: (1fr, 150pt),
    stroke: 0.5pt,
    inset: 8pt,
    [*Loan Amount:*], [#format_money(get_num("loan_amount", default: 0))],
    [*Maximum Interest Rate:*], [#get("max_interest_rate", default: "[Rate]")%],
    [*Loan Term:*], [#get("loan_term", default: "30") years],
  )

  #v(0.5em)

  Buyer shall apply for financing within #get("financing_application_days", default: "5") days and use good faith efforts to obtain loan approval within #get("financing_approval_days", default: "30") days after Effective Date.

  #v(1em)

  #text(size: 12pt, weight: "bold")[3.2 APPRAISAL CONTINGENCY]
  #v(0.5em)

  #if get_bool("has_appraisal_contingency") [
    This Contract IS contingent upon the Property appraising at no less than the Purchase Price.

    #if get_bool("has_appraisal_gap") [
      #v(0.5em)
      #rect(
        width: 100%,
        inset: 10pt,
        stroke: 1pt + rgb("#059669"),
        fill: rgb("#ecfdf5"),
        radius: 4pt,
      )[
        #text(weight: "bold")[APPRAISAL GAP GUARANTEE]

        #v(0.3em)

        Buyer agrees to pay up to #format_money(get_num("appraisal_gap_amount")) above the appraised value, not to exceed the Purchase Price. If the appraisal gap exceeds this amount, Buyer may terminate this Contract and receive a refund of the Deposit.
      ]
    ]
  ] else [
    This Contract is NOT contingent upon appraisal value. Buyer waives appraisal contingency.
  ]
]

#pagebreak()

// ============================================================================
// SECTION 4: INSPECTION PERIOD (AS-IS) - KEY SECTION
// ============================================================================

#text(size: 14pt, weight: "bold")[4. INSPECTION PERIOD (AS-IS)]
#v(1em)

#rect(
  width: 100%,
  inset: 15pt,
  stroke: 2pt + rgb("#dc2626"),
  fill: rgb("#fef2f2"),
  radius: 4pt,
)[
  #text(size: 12pt, weight: "bold", fill: rgb("#dc2626"))[
    CRITICAL "AS IS" PROVISION - BUYER'S SOLE DISCRETION TERMINATION RIGHT
  ]

  #v(0.5em)

  #text(size: 11pt)[
    *BUYER'S RIGHT TO CANCEL:* Buyer shall have #get("inspection_period_days", default: "15") calendar days from the Effective Date ("Inspection Period") to have the Property inspected and to determine, *in Buyer's sole and absolute discretion*, whether the Property is acceptable to Buyer.

    #v(0.5em)

    *SOLE DISCRETION:* Buyer may cancel this Contract for *any reason or no reason* during the Inspection Period by delivering written notice to Seller before the Inspection Period expires. Upon such cancellation, Buyer's Deposit shall be returned and the parties shall be released from all obligations under this Contract.

    #v(0.5em)

    *NO SELLER REPAIR OBLIGATION:* Seller has NO obligation to make any repairs or improvements to the Property. The Property is being sold "AS IS" in its present condition, with all faults.
  ]
]

#v(1em)

#text(size: 12pt, weight: "bold")[4.1 INSPECTION PERIOD DATES]
#v(0.5em)

#table(
  columns: (1fr, 150pt),
  stroke: 0.5pt,
  inset: 8pt,
  [*Inspection Period:*], [#get("inspection_period_days", default: "15") calendar days],
  [*Inspection Period Expires:*], [#get("inspection_end_date", default: "[Date]")],
)

#v(1em)

#text(size: 12pt, weight: "bold")[4.2 INSPECTIONS AND ACCESS]
#v(0.5em)

Buyer, at Buyer's expense, may conduct inspections, tests, surveys, and investigations of the Property, including but not limited to:

- Structural and mechanical inspections
- Roof, plumbing, electrical, and HVAC systems
- Termite and wood-destroying organism (WDO) inspections
- Environmental assessments (mold, radon, lead paint, asbestos)
- Survey and boundary verification
- Septic/well inspections (if applicable)
- Pool/spa inspections (if applicable)

Seller shall provide reasonable access for inspections. Buyer shall restore Property to its pre-inspection condition and indemnify Seller against inspection-related claims.

#v(1em)

#text(size: 12pt, weight: "bold")[4.3 WALK-THROUGH INSPECTION]
#v(0.5em)

Buyer may conduct a walk-through inspection within 24 hours before Closing to verify that the Property is in substantially the same condition as of the Effective Date, ordinary wear and tear excepted.

#pagebreak()

// ============================================================================
// SECTION 5: TITLE AND SURVEY
// ============================================================================

#text(size: 14pt, weight: "bold")[5. TITLE AND SURVEY]
#v(1em)

#text(size: 12pt, weight: "bold")[5.1 TITLE EVIDENCE]
#v(0.5em)

Seller shall, at Seller's expense, deliver to Buyer a title insurance commitment issued by a Florida-licensed title insurer within #get("title_commitment_days", default: "15") days after Effective Date.

#v(0.5em)

Buyer shall have #get("title_objection_days", default: "5") days after receipt of the title commitment to examine same and notify Seller in writing of any defects. Seller shall have #get("title_cure_days", default: "30") days to cure title defects.

#v(1em)

#text(size: 12pt, weight: "bold")[5.2 MARKETABLE TITLE]
#v(0.5em)

Seller shall convey marketable title to the Property by statutory warranty deed, subject only to:

- Matters contained in this Contract
- Recorded easements, covenants, and restrictions that do not prevent reasonable use
- Zoning and governmental regulations
- Matters that would be disclosed by an accurate survey

#v(1em)

#text(size: 12pt, weight: "bold")[5.3 TITLE INSURANCE]
#v(0.5em)

At Closing, Seller shall pay for owner's title insurance policy in the amount of the Purchase Price, issued by #get("title_company", default: "[Title Company]").

#v(1em)

#text(size: 12pt, weight: "bold")[5.4 SURVEY]
#v(0.5em)

#if get_bool("survey_required") [
  #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] Buyer requires a current survey. Cost to be paid by: #get("survey_paid_by", default: "Buyer").
] else [
  #box(width: 12pt, height: 12pt, stroke: 1pt)[] Buyer requires a current survey.
]

#pagebreak()

// ============================================================================
// SECTION 6: CLOSING
// ============================================================================

#text(size: 14pt, weight: "bold")[6. CLOSING]
#v(1em)

#text(size: 12pt, weight: "bold")[6.1 CLOSING DATE AND LOCATION]
#v(0.5em)

#table(
  columns: (120pt, 1fr),
  stroke: none,
  inset: 5pt,
  [*Closing Date:*], [#get("closing_date", default: "[Closing Date]")],
  [*Closing Location:*], [#get("closing_location", default: "[Title Company/Attorney Office]")],
)

#v(1em)

#text(size: 12pt, weight: "bold")[6.2 CLOSING COSTS]
#v(0.5em)

#table(
  columns: (1fr, 100pt, 100pt),
  stroke: 0.5pt,
  inset: 8pt,
  align: (left, center, center),
  [*Item*], [*Seller*], [*Buyer*],
  [Documentary Stamp Tax on Deed], [X], [],
  [Owner's Title Insurance], [X], [],
  [Title Search], [X], [],
  [Survey (if required)], [], [X],
  [Lender's Title Insurance], [], [X],
  [Recording Fees - Deed], [X], [],
  [Recording Fees - Mortgage], [], [X],
  [Intangible Tax on Mortgage], [], [X],
)

#v(1em)

#text(size: 12pt, weight: "bold")[6.3 PRORATIONS]
#v(0.5em)

The following shall be prorated as of Closing Date:

- Real property taxes (based on current year's tax or, if unavailable, prior year)
- Homeowner's association fees
- Rent (if applicable)
- CDD assessments (if applicable)

Seller shall pay any delinquent taxes, HOA assessments, or other liens at or before Closing.

#v(1em)

#text(size: 12pt, weight: "bold")[6.4 POSSESSION]
#v(0.5em)

Seller shall deliver possession and keys to Buyer at Closing unless otherwise agreed in writing.

#pagebreak()

// ============================================================================
// SECTION 7: PROPERTY CONDITION
// ============================================================================

#text(size: 14pt, weight: "bold")[7. PROPERTY CONDITION]
#v(1em)

#rect(
  width: 100%,
  inset: 15pt,
  stroke: 2pt + rgb("#dc2626"),
  fill: rgb("#fef2f2"),
  radius: 4pt,
)[
  #text(size: 12pt, weight: "bold", fill: rgb("#dc2626"))[AS IS - NO WARRANTIES]

  #v(0.5em)

  BUYER ACKNOWLEDGES AND AGREES THAT SELLER IS SELLING AND BUYER IS PURCHASING THE PROPERTY IN ITS "AS IS" CONDITION AS OF THE DATE OF THIS CONTRACT, WITH ALL FAULTS AND DEFECTS, WHETHER LATENT OR PATENT, WHETHER KNOWN OR UNKNOWN.

  #v(0.5em)

  SELLER MAKES NO WARRANTIES, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE, OR HABITABILITY.
]

#v(1em)

#text(size: 12pt, weight: "bold")[7.1 SELLER'S DISCLOSURE OBLIGATIONS]
#v(0.5em)

Notwithstanding the "as is" nature of this transaction, Seller is required by Florida law (Johnson v. Davis) to disclose any known material defects that are not readily observable and that materially affect the value of the Property.

#v(0.5em)

Seller represents that, to Seller's knowledge:

- There are no latent defects that would materially affect the value of the Property that have not been disclosed
- All major systems (roof, HVAC, plumbing, electrical) are in working order unless otherwise disclosed
- There are no pending or threatened legal actions affecting the Property

#v(1em)

#text(size: 12pt, weight: "bold")[7.2 MAINTENANCE]
#v(0.5em)

From Effective Date through Closing, Seller shall:

- Maintain the Property in its present condition
- Continue to maintain lawn, landscaping, and pool (if applicable)
- Promptly notify Buyer of any material changes to the Property

#pagebreak()

// ============================================================================
// SECTION 8: RISK OF LOSS
// ============================================================================

#text(size: 14pt, weight: "bold")[8. RISK OF LOSS]
#v(1em)

Risk of loss shall remain with Seller until Closing. If the Property is damaged or destroyed prior to Closing:

*Damage 1.5% or less of Purchase Price:* Seller shall repair the Property to its pre-damage condition before Closing.

*Damage exceeding 1.5% of Purchase Price:* Buyer may elect to:
- Accept the Property "as is" with an assignment of insurance proceeds; or
- Cancel this Contract and receive a full refund of the Deposit.

#pagebreak()

// ============================================================================
// SECTION 9: DEFAULT AND REMEDIES
// ============================================================================

#text(size: 14pt, weight: "bold")[9. DEFAULT AND REMEDIES]
#v(1em)

#text(size: 12pt, weight: "bold")[9.1 BUYER DEFAULT]
#v(0.5em)

If Buyer fails to perform under this Contract, Seller's sole remedy shall be to retain the Deposit as liquidated damages, which the parties agree is a reasonable estimate of Seller's damages and not a penalty.

#v(1em)

#text(size: 12pt, weight: "bold")[9.2 SELLER DEFAULT]
#v(0.5em)

If Seller fails to perform under this Contract, Buyer may:
- Seek specific performance to compel Seller to close; or
- Cancel this Contract and receive a full refund of the Deposit, plus reasonable expenses incurred in reliance on this Contract.

#pagebreak()

// ============================================================================
// SECTION 10: DISPUTE RESOLUTION
// ============================================================================

#text(size: 14pt, weight: "bold")[10. DISPUTE RESOLUTION]
#v(1em)

#text(size: 12pt, weight: "bold")[10.1 MEDIATION]
#v(0.5em)

Any dispute arising out of this Contract shall first be submitted to mediation before a mediator certified by the Florida Supreme Court. Costs of mediation shall be shared equally.

#v(1em)

#text(size: 12pt, weight: "bold")[10.2 ATTORNEY'S FEES]
#v(0.5em)

In any litigation arising out of this Contract, the prevailing party shall be entitled to recover reasonable attorney's fees and costs from the non-prevailing party.

#pagebreak()

// ============================================================================
// SECTION 11: ADDITIONAL TERMS
// ============================================================================

#text(size: 14pt, weight: "bold")[11. ADDITIONAL TERMS]
#v(1em)

#text(size: 12pt, weight: "bold")[11.1 ENTIRE AGREEMENT]
#v(0.5em)

This Contract, together with all addenda and exhibits, constitutes the entire agreement between the parties. No prior agreements or representations shall be binding unless incorporated herein.

#v(1em)

#text(size: 12pt, weight: "bold")[11.2 AMENDMENTS]
#v(0.5em)

This Contract may only be amended by written agreement signed by both parties.

#v(1em)

#text(size: 12pt, weight: "bold")[11.3 EFFECTIVE DATE]
#v(0.5em)

The "Effective Date" of this Contract is the date on which the last party signs or initials this Contract or any counter-offer.

#v(1em)

#text(size: 12pt, weight: "bold")[11.4 TIME]
#v(0.5em)

Time is of the essence for all provisions of this Contract. Any deadline that falls on a Saturday, Sunday, or legal holiday shall be extended to the next business day.

#v(1em)

#text(size: 12pt, weight: "bold")[11.5 COUNTERPARTS AND ELECTRONIC SIGNATURES]
#v(0.5em)

This Contract may be executed in counterparts and transmitted by electronic means, all of which together shall constitute one agreement.

#v(1em)

#text(size: 12pt, weight: "bold")[11.6 ADDITIONAL PROVISIONS]
#v(0.5em)

#if get("additional_terms", default: "") != "" [
  #get("additional_terms")
] else [
  [None]
]

#pagebreak()

// ============================================================================
// SECTION 12: SIGNATURES
// ============================================================================

#text(size: 14pt, weight: "bold")[12. SIGNATURES]
#v(1em)

#rect(
  width: 100%,
  inset: 12pt,
  stroke: 1pt + rgb("#666"),
  fill: rgb("#f9fafb"),
  radius: 4pt,
)[
  #text(size: 10pt)[
    BY SIGNING BELOW, THE PARTIES ACKNOWLEDGE THAT THEY HAVE READ THIS CONTRACT IN ITS ENTIRETY, UNDERSTAND ITS TERMS, AND AGREE TO BE BOUND THEREBY.

    #v(0.3em)

    *BUYER SPECIFICALLY ACKNOWLEDGES:*
    - The Property is being purchased "AS IS" with no warranties
    - Buyer has the right to inspect and cancel during the Inspection Period
    - All disclosures have been received and reviewed
  ]
]

#v(2em)

#grid(
  columns: (1fr, 1fr),
  gutter: 40pt,
  [
    #text(weight: "bold")[SELLER]
    #v(2em)
    Signature: #box(width: 180pt, repeat[\_])
    #v(0.5em)
    Print Name: #get("seller_name", default: "[Seller Name]")
    #v(0.5em)
    Date: #box(width: 120pt, repeat[\_])
  ],
  [
    #text(weight: "bold")[BUYER]
    #v(2em)
    Signature: #box(width: 180pt, repeat[\_])
    #v(0.5em)
    Print Name: #get("buyer_name", default: "[Buyer Name]")
    #v(0.5em)
    Date: #box(width: 120pt, repeat[\_])
  ]
)

#v(2em)

#if get("additional_seller_name", default: "") != "" or get("additional_buyer_name", default: "") != "" [
  #grid(
    columns: (1fr, 1fr),
    gutter: 40pt,
    [
      #if get("additional_seller_name", default: "") != "" [
        #text(weight: "bold")[ADDITIONAL SELLER]
        #v(2em)
        Signature: #box(width: 180pt, repeat[\_])
        #v(0.5em)
        Print Name: #get("additional_seller_name")
        #v(0.5em)
        Date: #box(width: 120pt, repeat[\_])
      ]
    ],
    [
      #if get("additional_buyer_name", default: "") != "" [
        #text(weight: "bold")[ADDITIONAL BUYER]
        #v(2em)
        Signature: #box(width: 180pt, repeat[\_])
        #v(0.5em)
        Print Name: #get("additional_buyer_name")
        #v(0.5em)
        Date: #box(width: 120pt, repeat[\_])
      ]
    ]
  )
]

#pagebreak()

// ============================================================================
// ADDENDUM A: RADON GAS NOTIFICATION (§ 404.056)
// ============================================================================

#text(size: 14pt, weight: "bold")[ADDENDUM A: RADON GAS NOTIFICATION]
#v(0.5em)
#text(size: 10pt, style: "italic")[Required by Florida Statutes § 404.056(5)]
#v(1em)

#rect(
  width: 100%,
  inset: 15pt,
  stroke: 2pt + rgb("#b45309"),
  fill: rgb("#fffbeb"),
  radius: 4pt,
)[
  #text(weight: "bold")[RADON GAS DISCLOSURE]

  #v(0.5em)

  RADON GAS: Radon is a naturally occurring radioactive gas that, when it has accumulated in a building in sufficient quantities, may present health risks to persons who are exposed to it over time. Levels of radon that exceed federal and state guidelines have been found in buildings in Florida. Additional information regarding radon and radon testing may be obtained from your county health department.

  #v(0.3em)

  #text(size: 9pt, style: "italic")[
    This disclosure is required by Florida Statutes § 404.056(5) for all residential real estate transactions.
  ]
]

#v(2em)

#grid(
  columns: (1fr, 1fr),
  gutter: 20pt,
  [
    Buyer Initials: #box(width: 60pt, repeat[\_])
  ],
  [
    Date: #box(width: 100pt, repeat[\_])
  ]
)

#pagebreak()

// ============================================================================
// ADDENDUM B: PROPERTY TAX DISCLOSURE (§ 689.261)
// ============================================================================

#text(size: 14pt, weight: "bold")[ADDENDUM B: PROPERTY TAX DISCLOSURE]
#v(0.5em)
#text(size: 10pt, style: "italic")[Required by Florida Statutes § 689.261]
#v(1em)

#rect(
  width: 100%,
  inset: 15pt,
  stroke: 1pt + black,
  radius: 4pt,
)[
  #text(weight: "bold")[PROPERTY TAX DISCLOSURE SUMMARY]

  #v(0.5em)

  BUYER SHOULD NOT RELY ON THE SELLER'S CURRENT PROPERTY TAXES AS THE AMOUNT OF PROPERTY TAXES THAT THE BUYER MAY BE OBLIGATED TO PAY IN THE YEAR SUBSEQUENT TO PURCHASE.

  A CHANGE OF OWNERSHIP OR PROPERTY IMPROVEMENTS TRIGGERS REASSESSMENTS OF THE PROPERTY THAT COULD RESULT IN HIGHER PROPERTY TAXES.

  IF YOU HAVE ANY QUESTIONS CONCERNING VALUATION, CONTACT THE COUNTY PROPERTY APPRAISER'S OFFICE FOR INFORMATION.
]

#v(1em)

#table(
  columns: (1fr, 150pt),
  stroke: 0.5pt,
  inset: 8pt,
  [*Current Property Taxes (Annual):*], [#if get("current_taxes", default: "") != "" [#format_money(get_num("current_taxes"))] else [[Amount]]],
  [*Homestead Exemption in Place:*], [#if get_bool("has_homestead") [Yes] else [No / Unknown]],
)

#v(1em)

#text(size: 10pt, style: "italic")[
  Note: If Seller currently has a Homestead Exemption, Buyer's taxes after purchase will likely increase significantly due to removal of exemption and reassessment to market value.
]

#v(2em)

#grid(
  columns: (1fr, 1fr),
  gutter: 20pt,
  [
    Buyer Initials: #box(width: 60pt, repeat[\_])
  ],
  [
    Date: #box(width: 100pt, repeat[\_])
  ]
)

#pagebreak()

// ============================================================================
// ADDENDUM C: FLOOD DISCLOSURE (§ 689.302)
// ============================================================================

#text(size: 14pt, weight: "bold")[ADDENDUM C: FLOOD DISCLOSURE]
#v(0.5em)
#text(size: 10pt, style: "italic")[Required by Florida Statutes § 689.302]
#v(1em)

#rect(
  width: 100%,
  inset: 15pt,
  stroke: 2pt + rgb("#2563eb"),
  fill: rgb("#eff6ff"),
  radius: 4pt,
)[
  #text(weight: "bold")[FLOOD ZONE DISCLOSURE]

  #v(0.5em)

  #text(weight: "bold")[1. FLOOD ZONE STATUS]

  #v(0.3em)

  #let flood_zone = get("flood_zone", default: "unknown")

  #if flood_zone == "sfha" [
    #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] The Property IS located in a Special Flood Hazard Area (SFHA).
  ] else [
    #box(width: 12pt, height: 12pt, stroke: 1pt)[] The Property IS located in a Special Flood Hazard Area (SFHA).
  ]

  #v(0.2em)

  #if flood_zone == "non-sfha" [
    #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] The Property is NOT located in a Special Flood Hazard Area.
  ] else [
    #box(width: 12pt, height: 12pt, stroke: 1pt)[] The Property is NOT located in a Special Flood Hazard Area.
  ]

  #v(0.2em)

  #if flood_zone == "unknown" [
    #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] Flood zone status is unknown. Buyer should verify.
  ] else [
    #box(width: 12pt, height: 12pt, stroke: 1pt)[] Flood zone status is unknown. Buyer should verify.
  ]

  #v(0.5em)

  FEMA Flood Zone: #get("fema_flood_zone", default: "[Zone Designation]")

  #v(0.5em)

  #text(weight: "bold")[2. FLOOD HISTORY]

  #v(0.3em)

  Has the Property flooded in the past? #get("flood_history", default: "[Yes/No/Unknown]")

  #if get("flood_history_details", default: "") != "" [
    #v(0.2em)
    Details: #get("flood_history_details")
  ]

  #v(0.5em)

  #text(weight: "bold")[3. FLOOD INSURANCE]

  #v(0.3em)

  If the Property is in a Special Flood Hazard Area and Buyer obtains a federally-backed mortgage, flood insurance will be REQUIRED.

  Buyer is advised to contact the National Flood Insurance Program (NFIP) or a private insurer for flood insurance information.
]

#v(2em)

#grid(
  columns: (1fr, 1fr),
  gutter: 20pt,
  [
    Buyer Initials: #box(width: 60pt, repeat[\_])
  ],
  [
    Date: #box(width: 100pt, repeat[\_])
  ]
)

#pagebreak()

// ============================================================================
// ADDENDUM D: ENERGY EFFICIENCY DISCLOSURE (§ 553.996)
// ============================================================================

#text(size: 14pt, weight: "bold")[ADDENDUM D: ENERGY EFFICIENCY DISCLOSURE]
#v(0.5em)
#text(size: 10pt, style: "italic")[Required by Florida Statutes § 553.996]
#v(1em)

#rect(
  width: 100%,
  inset: 15pt,
  stroke: 1pt + rgb("#059669"),
  fill: rgb("#ecfdf5"),
  radius: 4pt,
)[
  #text(weight: "bold")[ENERGY EFFICIENCY RATING DISCLOSURE]

  #v(0.5em)

  The Buyer may have this building's energy efficiency rated by a state-certified building energy rater. If the building is rated, the rating shall be disclosed on the Energy Efficiency Rating Disclosure Form.

  #v(0.3em)

  Buyer acknowledges that an energy efficiency rating has not been provided by Seller unless attached hereto.
]

#v(2em)

#grid(
  columns: (1fr, 1fr),
  gutter: 20pt,
  [
    Buyer Initials: #box(width: 60pt, repeat[\_])
  ],
  [
    Date: #box(width: 100pt, repeat[\_])
  ]
)

#pagebreak()

// ============================================================================
// ADDENDUM E: LEAD-BASED PAINT DISCLOSURE (Pre-1978 Properties)
// ============================================================================

#let property_year = get_num("property_year_built", default: 2000)

#if property_year < 1978 [
  #text(size: 14pt, weight: "bold")[ADDENDUM E: LEAD-BASED PAINT DISCLOSURE]
  #v(0.5em)
  #text(size: 10pt, style: "italic")[Required by Federal Law (42 U.S.C. 4852d) for properties built before 1978]
  #v(1em)

  #rect(
    width: 100%,
    inset: 15pt,
    stroke: 2pt + rgb("#dc2626"),
    fill: rgb("#fef2f2"),
    radius: 4pt,
  )[
    #text(weight: "bold", fill: rgb("#dc2626"))[LEAD WARNING STATEMENT]

    #v(0.5em)

    Every purchaser of any interest in residential real property on which a residential dwelling was built prior to 1978 is notified that such property may present exposure to lead from lead-based paint that may place young children at risk of developing lead poisoning.

    Lead poisoning in young children may produce permanent neurological damage, including learning disabilities, reduced intelligence quotient, behavioral problems, and impaired memory.

    Lead poisoning also poses a particular risk to pregnant women. The seller of any interest in residential real property is required to provide the buyer with any information on lead-based paint hazards from risk assessments or inspections in the seller's possession and notify the buyer of any known lead-based paint hazards.

    A risk assessment or inspection for possible lead-based paint hazards is recommended prior to purchase.
  ]

  #v(1em)

  #text(weight: "bold")[Seller's Disclosure:]

  #v(0.5em)

  #let lead_known = get("lead_paint_known", default: "unknown")

  #if lead_known == "yes" [
    #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] Known lead-based paint and/or lead-based paint hazards are present in the housing.
  ] else [
    #box(width: 12pt, height: 12pt, stroke: 1pt)[] Known lead-based paint and/or lead-based paint hazards are present in the housing.
  ]

  #v(0.3em)

  #if lead_known == "no" [
    #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] Seller has no knowledge of lead-based paint and/or lead-based paint hazards in the housing.
  ] else [
    #box(width: 12pt, height: 12pt, stroke: 1pt)[] Seller has no knowledge of lead-based paint and/or lead-based paint hazards in the housing.
  ]

  #v(1em)

  #text(weight: "bold")[Buyer's Acknowledgment:]

  #v(0.5em)

  #box(width: 12pt, height: 12pt, stroke: 1pt)[] Buyer has received the EPA pamphlet "Protect Your Family From Lead in Your Home."

  #box(width: 12pt, height: 12pt, stroke: 1pt)[] Buyer has received a 10-day opportunity to conduct a lead-based paint inspection.

  #v(2em)

  #grid(
    columns: (1fr, 1fr),
    gutter: 20pt,
    [
      Buyer Initials: #box(width: 60pt, repeat[\_])
    ],
    [
      Date: #box(width: 100pt, repeat[\_])
    ]
  )

  #pagebreak()
]

// ============================================================================
// ADDENDUM F: HOA/COMMUNITY DISCLOSURE (§ 720.401)
// ============================================================================

#if get_bool("has_hoa") [
  #text(size: 14pt, weight: "bold")[ADDENDUM F: HOA/COMMUNITY DISCLOSURE]
  #v(0.5em)
  #text(size: 10pt, style: "italic")[Required by Florida Statutes § 720.401]
  #v(1em)

  #rect(
    width: 100%,
    inset: 15pt,
    stroke: 1pt + black,
    radius: 4pt,
  )[
    #text(weight: "bold")[HOMEOWNERS' ASSOCIATION DISCLOSURE]

    #v(0.5em)

    The Property is located within a community subject to a mandatory homeowners' association (HOA).

    #v(0.5em)

    *HOA Name:* #get("hoa_name", default: "[HOA Name]")

    *Management Company:* #get("hoa_management", default: "[Management Company]")

    *Contact:* #get("hoa_contact", default: "[Phone/Email]")

    #v(0.5em)

    *Current Monthly Assessment:* #if get("hoa_monthly_fee", default: "") != "" [#format_money(get_num("hoa_monthly_fee"))] else [[Amount]]

    *Any Pending Special Assessments:* #get("hoa_special_assessment", default: "[Yes/No/Unknown]")

    #v(0.5em)

    #text(weight: "bold")[DISCLOSURE SUMMARY:]

    1. AS A PURCHASER OF PROPERTY IN THIS COMMUNITY, YOU WILL BE OBLIGATED TO BE A MEMBER OF THE HOMEOWNERS' ASSOCIATION.

    2. THERE HAVE BEEN OR MAY BE AMENDMENTS TO THE RESTRICTIVE COVENANTS GOVERNING THE PROPERTY AND THE HOMEOWNERS' ASSOCIATION.

    3. YOU SHOULD RECEIVE A DISCLOSURE SUMMARY REQUIRED BY § 720.401 AND A COPY OF THE GOVERNING DOCUMENTS FROM THE SELLER OR HOA.
  ]

  #v(2em)

  #grid(
    columns: (1fr, 1fr),
    gutter: 20pt,
    [
      Buyer Initials: #box(width: 60pt, repeat[\_])
    ],
    [
      Date: #box(width: 100pt, repeat[\_])
    ]
  )

  #pagebreak()
]

// ============================================================================
// ADDENDUM G: FOREIGN OWNERSHIP DISCLOSURE (SB 264 / § 692.204)
// ============================================================================

#text(size: 14pt, weight: "bold")[ADDENDUM G: FOREIGN OWNERSHIP DISCLOSURE]
#v(0.5em)
#text(size: 10pt, style: "italic")[Required by Florida Statutes § 692.204 (SB 264)]
#v(1em)

#rect(
  width: 100%,
  inset: 15pt,
  stroke: 2pt + rgb("#dc2626"),
  fill: rgb("#fef2f2"),
  radius: 4pt,
)[
  #text(weight: "bold", fill: rgb("#dc2626"))[FLORIDA FOREIGN OWNERSHIP DISCLOSURE]

  #v(0.5em)

  Effective July 1, 2023, Florida law (SB 264, codified at F.S. § 692.201-692.205) restricts certain foreign principals from purchasing real property in Florida, particularly:

  - Properties within 10 miles of military installations or critical infrastructure
  - Agricultural land
  - Properties near airports or other designated facilities

  #v(0.5em)

  #text(weight: "bold")[BUYER CERTIFICATION:]

  #v(0.3em)

  By signing below, Buyer certifies that:

  #box(width: 12pt, height: 12pt, stroke: 1pt)[] Buyer is NOT a "foreign principal" as defined in § 692.201, Florida Statutes.

  #box(width: 12pt, height: 12pt, stroke: 1pt)[] Buyer IS a "foreign principal" but is eligible to purchase this Property under an exception in the statute.

  #v(0.5em)

  *Note:* "Foreign principal" includes citizens of China, Russia, Iran, North Korea, Cuba, Venezuela, or Syria (and entities controlled by them), as well as any person domiciled in those countries.

  #v(0.3em)

  #text(size: 9pt, style: "italic")[
    This certification is required by Florida law. Providing false information is a crime and may result in forfeiture of the property.
  ]
]

#v(2em)

#grid(
  columns: (1fr, 1fr),
  gutter: 40pt,
  [
    #text(weight: "bold")[BUYER]
    #v(1.5em)
    Signature: #box(width: 150pt, repeat[\_])
    #v(0.5em)
    Print Name: #get("buyer_name", default: "[Buyer Name]")
    #v(0.5em)
    Date: #box(width: 100pt, repeat[\_])
  ],
  [
    #if get("additional_buyer_name", default: "") != "" [
      #text(weight: "bold")[ADDITIONAL BUYER]
      #v(1.5em)
      Signature: #box(width: 150pt, repeat[\_])
      #v(0.5em)
      Print Name: #get("additional_buyer_name")
      #v(0.5em)
      Date: #box(width: 100pt, repeat[\_])
    ]
  ]
)

#pagebreak()

// ============================================================================
// ADDENDUM H: APPRAISAL GAP GUARANTEE (Optional)
// ============================================================================

#if get_bool("has_appraisal_gap") [
  #text(size: 14pt, weight: "bold")[ADDENDUM H: APPRAISAL GAP GUARANTEE]
  #v(1em)

  #rect(
    width: 100%,
    inset: 15pt,
    stroke: 2pt + rgb("#059669"),
    fill: rgb("#ecfdf5"),
    radius: 4pt,
  )[
    #text(weight: "bold")[APPRAISAL GAP GUARANTEE ADDENDUM]

    #v(0.5em)

    This Addendum is attached to and made part of the Purchase Contract dated #get("contract_date", default: "[Date]") for the Property located at #get("property_address", default: "[Address]").

    #v(1em)

    #text(weight: "bold")[1. APPRAISAL GAP AMOUNT]

    #v(0.3em)

    If the Property appraises for less than the Purchase Price, Buyer agrees to pay the difference between the appraised value and the Purchase Price, up to a maximum of:

    #v(0.5em)

    #align(center)[
      #text(size: 14pt, weight: "bold")[#format_money(get_num("appraisal_gap_amount"))]
    ]

    #v(0.5em)

    This amount shall be paid by Buyer at Closing in addition to the down payment.

    #v(1em)

    #text(weight: "bold")[2. TERMINATION RIGHT]

    #v(0.3em)

    If the appraisal gap exceeds the amount stated above, Buyer may:

    - Elect to pay the additional difference and proceed with Closing; or
    - Terminate this Contract and receive a full refund of the Deposit.

    #v(1em)

    #text(weight: "bold")[3. EXAMPLE]

    #v(0.3em)

    Purchase Price: #format_money(get_num("purchase_price"))
    Appraisal Gap Guarantee: #format_money(get_num("appraisal_gap_amount"))

    If appraisal comes in at #format_money(get_num("purchase_price") - get_num("appraisal_gap_amount") - 5000):
    - Gap is #format_money(get_num("appraisal_gap_amount") + 5000) (exceeds guarantee)
    - Buyer may terminate OR pay #format_money(5000) extra to proceed
  ]

  #v(2em)

  #grid(
    columns: (1fr, 1fr),
    gutter: 20pt,
    [
      Buyer Initials: #box(width: 60pt, repeat[\_])
    ],
    [
      Seller Initials: #box(width: 60pt, repeat[\_])
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
  This "As-Is" Residential Purchase Contract is provided for informational purposes. The parties should consult with a licensed real estate attorney before signing. This contract is governed by Florida law, including Florida Statutes Chapter 475 (Real Estate Brokers), Chapter 689 (Conveyances of Land), § 404.056 (Radon), § 689.302 (Flood Disclosure), § 692.204 (Foreign Ownership/SB 264), and applicable federal regulations. All inspections should be completed by licensed professionals. Time is of the essence.
]
