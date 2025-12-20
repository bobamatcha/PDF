// Florida Residential Real Estate Purchase Contract Template
// Compliant with F.S. Chapter 475, Chapter 689, § 404.056, § 553.996, § 720.401
// Based on Johnson v. Davis (1985) material defect disclosure requirements
// All values are dynamic via sys.inputs

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
  margin: (top: 1in, bottom: 1in, left: 1in, right: 1in),
  numbering: "1",
  number-align: center,
)
#set text(font: "Liberation Sans", size: 10pt)
#set par(justify: true, leading: 0.65em)

// ============================================================================
// COVER PAGE
// ============================================================================

#align(center)[
  #v(2in)

  #text(size: 24pt, weight: "bold")[RESIDENTIAL REAL ESTATE]
  #v(0.2em)
  #text(size: 24pt, weight: "bold")[PURCHASE CONTRACT]

  #v(0.5em)

  #text(size: 14pt)[State of Florida]

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
    ]
  ]

  #v(3em)

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

  #v(4em)

  #text(size: 9pt, fill: rgb("#666"))[
    This contract is governed by Florida law, including F.S. Chapter 475, Chapter 689, and applicable federal regulations.
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
#toc_item("2.", "PURCHASE PRICE AND TERMS")
#toc_item("3.", "FINANCING")
#toc_item("4.", "CLOSING")
#toc_item("5.", "EARNEST MONEY DEPOSIT")
#toc_item("6.", "TITLE AND SURVEY")
#toc_item("7.", "INSPECTIONS")
#toc_item("8.", "PROPERTY CONDITION")
#toc_item("9.", "RISK OF LOSS")
#toc_item("10.", "DEFAULT AND REMEDIES")
#toc_item("11.", "DISPUTE RESOLUTION")
#toc_item("12.", "ADDITIONAL TERMS")
#toc_item("13.", "SIGNATURES")

#v(0.5em)

#text(weight: "bold")[MANDATORY DISCLOSURES:]
#v(0.3em)
#toc_item("A.", "Radon Gas Notification (§ 404.056)")
#toc_item("B.", "Property Tax Disclosure (§ 689.261)")
#toc_item("C.", "Flood Disclosure (§ 689.302)")
#toc_item("D.", "Energy Efficiency Disclosure (§ 553.996)")
#toc_item("E.", "Lead-Based Paint Disclosure (if pre-1978)")
#toc_item("F.", "HOA/Community Disclosure (§ 720.401)")
#toc_item("G.", "Seller's Property Disclosure")

#pagebreak()

// ============================================================================
// SECTION 1: PARTIES AND PROPERTY
// ============================================================================

#text(size: 14pt, weight: "bold")[1. PARTIES AND PROPERTY]
#v(1em)

#text(size: 12pt, weight: "bold")[1.1 SELLER]
#v(0.5em)

#table(
  columns: (auto, 1fr),
  stroke: 0.5pt + rgb("#ccc"),
  inset: 8pt,
  [*Name*], [#get("seller_name", default: "[Seller Name]")],
  [*Address*], [#get("seller_address", default: "[Seller Address]")],
  [*Phone*], [#get("seller_phone", default: "[Phone]")],
  [*Email*], [#get("seller_email", default: "[Email]")],
)

#v(1em)

#text(size: 12pt, weight: "bold")[1.2 BUYER]
#v(0.5em)

#table(
  columns: (auto, 1fr),
  stroke: 0.5pt + rgb("#ccc"),
  inset: 8pt,
  [*Name*], [#get("buyer_name", default: "[Buyer Name]")],
  [*Address*], [#get("buyer_address", default: "[Buyer Address]")],
  [*Phone*], [#get("buyer_phone", default: "[Phone]")],
  [*Email*], [#get("buyer_email", default: "[Email]")],
)

#v(1em)

#text(size: 12pt, weight: "bold")[1.3 PROPERTY DESCRIPTION]
#v(0.5em)

#table(
  columns: (auto, 1fr),
  stroke: 0.5pt + rgb("#ccc"),
  inset: 8pt,
  [*Street Address*], [#get("property_address", default: "[Property Address]")],
  [*City*], [#get("property_city", default: "[City]")],
  [*County*], [#get("property_county", default: "[County]")],
  [*ZIP Code*], [#get("property_zip", default: "[ZIP]")],
  [*Parcel ID/Folio*], [#get("parcel_id", default: "[Parcel ID]")],
)

#v(0.5em)

#text(weight: "bold")[Legal Description:]
#v(0.3em)
#get("legal_description", default: "[Legal description as recorded in public records]")

#v(0.5em)

#text(weight: "bold")[Property Type:]
#get("property_type", default: "Single Family Residence")

#v(0.5em)

#text(weight: "bold")[Year Built:] #get("year_built", default: "[Year]")

#v(1em)

// ============================================================================
// SECTION 2: PURCHASE PRICE AND TERMS
// ============================================================================

#text(size: 14pt, weight: "bold")[2. PURCHASE PRICE AND TERMS]
#v(1em)

#let purchase_price = get_num("purchase_price")
#let earnest_money = get_num("earnest_money")
#let additional_deposit = get_num("additional_deposit", default: 0)
#let balance_due = purchase_price - earnest_money - additional_deposit

#table(
  columns: (1fr, auto),
  stroke: 0.5pt + rgb("#ccc"),
  inset: 10pt,
  [*Purchase Price*], [*#format_money(purchase_price)*],
  table.hline(stroke: 0.5pt),
  [Initial Earnest Money Deposit], [#format_money(earnest_money)],
  [Additional Deposit (if applicable)], [#format_money(additional_deposit)],
  [Balance Due at Closing], [#format_money(balance_due)],
)

#v(0.5em)

#if get_bool("includes_personal_property") [
  *Personal Property Included:* #get("personal_property_list", default: "[List of included items]")
  #v(0.5em)
]

*Excluded Items:* #get("excluded_items", default: "None")

#v(1em)

// ============================================================================
// SECTION 3: FINANCING
// ============================================================================

#text(size: 14pt, weight: "bold")[3. FINANCING]
#v(1em)

#let financing_type = get("financing_type", default: "conventional")

#if financing_type == "cash" [
  #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] *CASH:* This is an all-cash transaction. No financing contingency applies.

  Proof of funds must be provided within #get("proof_of_funds_days", default: "3") days of contract execution.
] else [
  #if financing_type == "conventional" [
    #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] *CONVENTIONAL FINANCING*
  ] else if financing_type == "fha" [
    #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] *FHA FINANCING*
  ] else if financing_type == "va" [
    #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] *VA FINANCING*
  ] else [
    #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] *OTHER FINANCING:* #get("financing_type")
  ]

  #v(0.5em)

  #table(
    columns: (1fr, 1fr),
    stroke: 0.5pt + rgb("#ccc"),
    inset: 8pt,
    [*Loan Amount*], [#format_money(get_num("loan_amount"))],
    [*Interest Rate (max)*], [#get("max_interest_rate", default: "Market rate")%],
    [*Loan Term*], [#get("loan_term", default: "30") years],
    [*Loan Application Deadline*], [#get("loan_application_deadline", default: "[Date]")],
    [*Loan Approval Deadline*], [#get("loan_approval_deadline", default: "[Date]")],
  )

  #v(0.5em)

  *Financing Contingency:* This contract is contingent upon Buyer obtaining loan approval by the Loan Approval Deadline. If Buyer fails to obtain loan approval despite good faith efforts, Buyer may terminate this contract and receive a refund of the earnest money deposit, less any costs as specified herein.
]

#v(1em)

// ============================================================================
// SECTION 4: CLOSING
// ============================================================================

#text(size: 14pt, weight: "bold")[4. CLOSING]
#v(1em)

#table(
  columns: (auto, 1fr),
  stroke: 0.5pt + rgb("#ccc"),
  inset: 8pt,
  [*Closing Date*], [#get("closing_date", default: "[Date]")],
  [*Closing Location*], [#get("closing_location", default: "Title company to be selected by Buyer")],
  [*Title Company*], [#get("title_company", default: "[Title Company Name]")],
)

#v(0.5em)

*Closing Costs:*

- *Title Insurance (Owner's Policy):* Paid by #get("title_insurance_paid_by", default: "Seller")
- *Title Insurance (Lender's Policy):* Paid by Buyer
- *Documentary Stamps on Deed:* Paid by #get("doc_stamps_paid_by", default: "Seller")
- *Recording Fees (Deed):* Paid by #get("recording_deed_paid_by", default: "Buyer")
- *Recording Fees (Mortgage):* Paid by Buyer
- *Survey:* Paid by #get("survey_paid_by", default: "Buyer")

#v(0.5em)

*Prorations:* Property taxes, HOA assessments, and other prorated items shall be prorated as of the closing date.

#v(1em)

// ============================================================================
// SECTION 5: EARNEST MONEY DEPOSIT
// ============================================================================

#text(size: 14pt, weight: "bold")[5. EARNEST MONEY DEPOSIT]
#v(1em)

#rect(
  width: 100%,
  inset: 12pt,
  stroke: 1pt + rgb("#0066cc"),
  fill: rgb("#f0f8ff"),
  radius: 4pt,
)[
  #text(weight: "bold")[Pursuant to Florida Statutes Chapter 475:]

  The escrow agent shall deposit all earnest money into an escrow account *within three (3) business days* of receipt as required by Florida law.
]

#v(0.5em)

#table(
  columns: (auto, 1fr),
  stroke: 0.5pt + rgb("#ccc"),
  inset: 8pt,
  [*Initial Deposit Amount*], [#format_money(get_num("earnest_money"))],
  [*Deposit Due Date*], [#get("earnest_money_due_date", default: "Within 3 days of contract execution")],
  [*Additional Deposit Amount*], [#format_money(get_num("additional_deposit", default: 0))],
  [*Additional Deposit Due Date*], [#get("additional_deposit_due_date", default: "N/A")],
)

#v(0.5em)

*Escrow Agent:*
#v(0.3em)
#get("escrow_agent_name", default: "[Escrow Agent Name]")
#v(0.2em)
#get("escrow_agent_address", default: "[Escrow Agent Address]")

#v(0.5em)

*Deposit Form:* #get("deposit_form", default: "Check, wire transfer, or other form acceptable to escrow agent")

#v(1em)

// ============================================================================
// SECTION 6: TITLE AND SURVEY
// ============================================================================

#text(size: 14pt, weight: "bold")[6. TITLE AND SURVEY]
#v(1em)

#text(size: 12pt, weight: "bold")[6.1 TITLE EVIDENCE]
#v(0.5em)

Seller shall provide, at Seller's expense, title evidence in the form of:

#if get("title_evidence_type", default: "commitment") == "commitment" [
  #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] Title insurance commitment from a Florida-licensed title insurer
] else [
  #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] Abstract of title
]

Title evidence shall be delivered within #get("title_evidence_days", default: "15") days of contract execution.

#v(0.5em)

#text(size: 12pt, weight: "bold")[6.2 TITLE DEFECTS]
#v(0.5em)

Buyer shall have #get("title_objection_days", default: "5") days after receipt of title evidence to examine and notify Seller in writing of any defects. Seller shall have #get("title_cure_days", default: "30") days to cure any defects. If Seller cannot cure defects, Buyer may:
+ Accept title as is, or
+ Terminate this contract and receive a refund of earnest money

#v(0.5em)

#text(size: 12pt, weight: "bold")[6.3 SURVEY]
#v(0.5em)

#if get_bool("survey_required") [
  Buyer #if get("survey_paid_by", default: "Buyer") == "Buyer" [shall] else [may] obtain a current survey at Buyer's expense.

  Survey objections must be made within #get("survey_objection_days", default: "5") days of receipt.
] else [
  No survey is required for this transaction unless otherwise agreed.
]

#v(1em)

// ============================================================================
// SECTION 7: INSPECTIONS
// ============================================================================

#text(size: 14pt, weight: "bold")[7. INSPECTIONS]
#v(1em)

*Inspection Period:* Buyer shall have #get("inspection_period_days", default: "15") days from the Effective Date to conduct inspections at Buyer's expense.

#v(0.5em)

*Permitted Inspections:*
+ General home inspection
+ Wood-destroying organism (WDO/termite) inspection
+ Roof inspection
+ HVAC inspection
+ Plumbing inspection
+ Electrical inspection
+ Mold inspection
+ Pool/spa inspection (if applicable)
+ Septic/well inspection (if applicable)
+ Environmental inspections

#v(0.5em)

*Inspection Contingency:*

#if get("inspection_contingency_type", default: "standard") == "as_is" [
  *AS-IS:* Property is being sold in its present condition. Buyer has the right to conduct inspections but Seller is not obligated to make repairs. Buyer may terminate within the inspection period for any reason and receive a refund of earnest money.
] else [
  *STANDARD:* If inspections reveal defects, Buyer may:
  + Accept the property as is
  + Request Seller to make repairs (Seller may agree, decline, or counter)
  + Terminate this contract within the inspection period and receive a refund of earnest money

  If Buyer and Seller cannot agree on repairs, either party may terminate within #get("repair_resolution_days", default: "5") days of impasse.
]

#v(1em)

// ============================================================================
// SECTION 8: PROPERTY CONDITION
// ============================================================================

#text(size: 14pt, weight: "bold")[8. PROPERTY CONDITION]
#v(1em)

#text(size: 12pt, weight: "bold")[8.1 SELLER'S OBLIGATIONS]
#v(0.5em)

Seller agrees to:
+ Maintain the property in its present condition until closing
+ Keep all utilities on through closing
+ Provide access for inspections and appraisal
+ Remove all debris and personal property not included in the sale
+ Deliver the property in broom-clean condition

#v(0.5em)

#text(size: 12pt, weight: "bold")[8.2 WALK-THROUGH INSPECTION]
#v(0.5em)

Buyer shall have the right to conduct a final walk-through inspection within #get("walkthrough_days", default: "3") days prior to closing to verify:
+ Property is in the agreed-upon condition
+ All repairs have been completed (if applicable)
+ All included items remain on the property

#v(1em)

// ============================================================================
// SECTION 9: RISK OF LOSS
// ============================================================================

#text(size: 14pt, weight: "bold")[9. RISK OF LOSS]
#v(1em)

If the property is damaged by fire, casualty, or other cause prior to closing:

*Minor Damage (cost to repair less than 1.5% of purchase price):* Seller shall repair before closing, or credit Buyer at closing for the cost of repairs.

*Major Damage (cost to repair 1.5% or more of purchase price):* Buyer may:
+ Accept the property as damaged, with an assignment of insurance proceeds, or
+ Terminate this contract and receive a refund of earnest money

#v(1em)

// ============================================================================
// SECTION 10: DEFAULT AND REMEDIES
// ============================================================================

#text(size: 14pt, weight: "bold")[10. DEFAULT AND REMEDIES]
#v(1em)

#text(size: 12pt, weight: "bold")[10.1 BUYER DEFAULT]
#v(0.5em)

If Buyer fails to perform under this contract, after notice and opportunity to cure:
+ Seller may retain the earnest money deposit as liquidated damages, or
+ Seller may seek specific performance

#v(0.5em)

#text(size: 12pt, weight: "bold")[10.2 SELLER DEFAULT]
#v(0.5em)

If Seller fails to perform under this contract, after notice and opportunity to cure:
+ Buyer may receive a refund of the earnest money deposit, or
+ Buyer may seek specific performance, or
+ Buyer may pursue any other remedy available at law or equity

#v(0.5em)

#text(size: 12pt, weight: "bold")[10.3 ATTORNEY'S FEES]
#v(0.5em)

In any litigation arising out of this contract, the prevailing party shall be entitled to recover reasonable attorney's fees and costs from the non-prevailing party.

#v(1em)

// ============================================================================
// SECTION 11: DISPUTE RESOLUTION
// ============================================================================

#text(size: 14pt, weight: "bold")[11. DISPUTE RESOLUTION]
#v(1em)

#if get_bool("mediation_required") [
  *Mediation Required:* Before filing any lawsuit, the parties agree to submit any dispute to mediation. The cost of mediation shall be shared equally.

  #v(0.5em)
]

*Governing Law:* This contract shall be governed by the laws of the State of Florida.

*Venue:* Any legal action shall be filed in #get("property_county", default: "[County]") County, Florida.

#v(1em)

// ============================================================================
// SECTION 12: ADDITIONAL TERMS
// ============================================================================

#text(size: 14pt, weight: "bold")[12. ADDITIONAL TERMS]
#v(1em)

#text(size: 12pt, weight: "bold")[12.1 EFFECTIVE DATE]
#v(0.5em)

The Effective Date of this contract is the date when the last party signs.

#v(0.5em)

#text(size: 12pt, weight: "bold")[12.2 TIME IS OF THE ESSENCE]
#v(0.5em)

Time is of the essence for all dates and deadlines in this contract.

#v(0.5em)

#text(size: 12pt, weight: "bold")[12.3 COUNTERPARTS AND ELECTRONIC SIGNATURES]
#v(0.5em)

This contract may be executed in counterparts and delivered electronically. Electronic signatures shall be deemed original signatures.

#v(0.5em)

#text(size: 12pt, weight: "bold")[12.4 ENTIRE AGREEMENT]
#v(0.5em)

This contract, together with all attached addenda and disclosures, constitutes the entire agreement between the parties. No verbal agreements shall be binding.

#v(0.5em)

#text(size: 12pt, weight: "bold")[12.5 ADDITIONAL PROVISIONS]
#v(0.5em)

#get("additional_provisions", default: "None.")

#pagebreak()

// ============================================================================
// SECTION 13: SIGNATURES
// ============================================================================

#text(size: 14pt, weight: "bold")[13. SIGNATURES]
#v(1em)

By signing below, the parties agree to all terms and conditions of this Residential Real Estate Purchase Contract and all attached addenda and disclosures.

#v(2em)

#grid(
  columns: (1fr, 1fr),
  gutter: 40pt,
  [
    #text(weight: "bold")[SELLER]

    #v(2em)

    Signature: #box(width: 180pt, repeat[\_])

    #v(0.8em)

    Print Name: #get("seller_name", default: "[Seller Name]")

    #v(0.8em)

    Date: #box(width: 120pt, repeat[\_])
  ],
  [
    #text(weight: "bold")[BUYER]

    #v(2em)

    Signature: #box(width: 180pt, repeat[\_])

    #v(0.8em)

    Print Name: #get("buyer_name", default: "[Buyer Name]")

    #v(0.8em)

    Date: #box(width: 120pt, repeat[\_])
  ]
)

#v(3em)

#if get_bool("has_additional_seller") [
  #grid(
    columns: (1fr, 1fr),
    gutter: 40pt,
    [
      #text(weight: "bold")[ADDITIONAL SELLER]

      #v(2em)

      Signature: #box(width: 180pt, repeat[\_])

      #v(0.8em)

      Print Name: #get("additional_seller_name", default: "[Additional Seller]")

      #v(0.8em)

      Date: #box(width: 120pt, repeat[\_])
    ],
    [
      #text(weight: "bold")[ADDITIONAL BUYER]

      #v(2em)

      Signature: #box(width: 180pt, repeat[\_])

      #v(0.8em)

      Print Name: #get("additional_buyer_name", default: "[Additional Buyer]")

      #v(0.8em)

      Date: #box(width: 120pt, repeat[\_])
    ]
  )
]

#pagebreak()

// ============================================================================
// ADDENDUM A: RADON GAS NOTIFICATION (MANDATORY - § 404.056)
// ============================================================================

#text(size: 14pt, weight: "bold")[ADDENDUM A: RADON GAS NOTIFICATION]
#v(0.5em)
#text(size: 10pt, style: "italic")[Required by Florida Statutes § 404.056(5)]
#v(1em)

#rect(
  width: 100%,
  inset: 15pt,
  stroke: 2pt + rgb("#0066cc"),
  fill: rgb("#f0f8ff"),
  radius: 4pt,
)[
  #text(weight: "bold", size: 12pt)[RADON GAS]

  #v(1em)

  Radon is a naturally occurring radioactive gas that, when it has accumulated in a building in sufficient quantities, may present health risks to persons who are exposed to it over time. Levels of radon that exceed federal and state guidelines have been found in buildings in Florida. Additional information regarding radon and radon testing may be obtained from your county health department.

  #v(1em)

  #text(size: 9pt, style: "italic")[
    This notification is required by Florida Statutes § 404.056(5) to be included in all real estate contracts in Florida for properties with buildings.
  ]
]

#v(2em)

#text(size: 11pt, weight: "bold")[ACKNOWLEDGMENT]

By signing below, Buyer acknowledges receipt of the above Radon Gas Notification as required by Florida law.

#v(2em)

#grid(
  columns: (1fr, 1fr),
  gutter: 40pt,
  [
    Buyer Signature: #box(width: 150pt, repeat[\_])
    #v(0.5em)
    Date: #box(width: 100pt, repeat[\_])
  ],
  [
    Buyer Signature: #box(width: 150pt, repeat[\_])
    #v(0.5em)
    Date: #box(width: 100pt, repeat[\_])
  ]
)

#pagebreak()

// ============================================================================
// ADDENDUM B: PROPERTY TAX DISCLOSURE (MANDATORY - § 689.261)
// ============================================================================

#text(size: 14pt, weight: "bold")[ADDENDUM B: PROPERTY TAX DISCLOSURE SUMMARY]
#v(0.5em)
#text(size: 10pt, style: "italic")[Required by Florida Statutes § 689.261]
#v(1em)

#rect(
  width: 100%,
  inset: 15pt,
  stroke: 2pt + rgb("#dc2626"),
  fill: rgb("#fef2f2"),
  radius: 4pt,
)[
  #text(weight: "bold", size: 12pt)[PROPERTY TAX DISCLOSURE SUMMARY]

  #v(1em)

  #text(weight: "bold")[BUYER SHOULD NOT RELY ON THE SELLER'S CURRENT PROPERTY TAXES AS THE AMOUNT OF PROPERTY TAXES THAT THE BUYER MAY BE OBLIGATED TO PAY IN THE YEAR SUBSEQUENT TO PURCHASE.]

  #v(1em)

  A CHANGE OF OWNERSHIP OR PROPERTY IMPROVEMENTS TRIGGERS REASSESSMENTS OF THE PROPERTY THAT COULD RESULT IN HIGHER PROPERTY TAXES.

  #v(1em)

  IF YOU HAVE ANY QUESTIONS CONCERNING VALUATION, CONTACT THE COUNTY PROPERTY APPRAISER'S OFFICE FOR INFORMATION.

  #v(1em)

  #text(size: 9pt, style: "italic")[
    This disclosure is required by Florida Statutes § 689.261 for the sale of residential property.
  ]
]

#v(1em)

*Current Property Tax Information (for reference only):*

#table(
  columns: (1fr, 1fr),
  stroke: 0.5pt + rgb("#ccc"),
  inset: 8pt,
  [*Current Annual Property Tax*], [#format_money(get_num("current_property_tax", default: 0))],
  [*County Property Appraiser*], [#get("property_county", default: "[County]") County],
)

#v(2em)

#text(size: 11pt, weight: "bold")[ACKNOWLEDGMENT]

By signing below, Buyer acknowledges receipt of this Property Tax Disclosure Summary as required by Florida law.

#v(2em)

#grid(
  columns: (1fr, 1fr),
  gutter: 40pt,
  [
    Buyer Signature: #box(width: 150pt, repeat[\_])
    #v(0.5em)
    Date: #box(width: 100pt, repeat[\_])
  ],
  [
    Buyer Signature: #box(width: 150pt, repeat[\_])
    #v(0.5em)
    Date: #box(width: 100pt, repeat[\_])
  ]
)

#pagebreak()

// ============================================================================
// ADDENDUM C: FLOOD DISCLOSURE (MANDATORY - § 689.302)
// ============================================================================

#text(size: 14pt, weight: "bold")[ADDENDUM C: FLOOD DISCLOSURE]
#v(0.5em)
#text(size: 10pt, style: "italic")[Required by Florida Statutes § 689.302 (Expanded effective October 1, 2025)]
#v(1em)

#rect(
  width: 100%,
  inset: 15pt,
  stroke: 2pt + rgb("#dc2626"),
  fill: rgb("#fef2f2"),
  radius: 4pt,
)[
  #text(weight: "bold", size: 12pt)[MANDATORY FLOOD DISCLOSURE]

  #v(1em)

  Pursuant to Florida Statutes § 689.302, the Seller is required to disclose the following information regarding flood history for the property located at:

  #v(0.5em)

  #text(weight: "bold")[#get("property_address", default: "[Property Address]")]
]

#v(1em)

#text(size: 12pt, weight: "bold")[SELLER'S DISCLOSURE]
#v(0.5em)

#text(weight: "bold")[1. KNOWLEDGE OF PRIOR FLOODING]
#v(0.3em)
#text(size: 10pt, style: "italic")[(Expanded requirement effective October 1, 2025)]

#let has_prior_flooding = get_bool("has_prior_flooding")

#if has_prior_flooding [
  #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] Seller HAS knowledge of flooding that damaged the property during Seller's ownership.

  #v(0.5em)

  #box(width: 12pt, height: 12pt, stroke: 1pt)[] Seller has NO knowledge of flooding that damaged the property.

  #v(0.5em)

  Description of flooding: #get("flooding_description", default: "[Describe flooding events]")
] else [
  #box(width: 12pt, height: 12pt, stroke: 1pt)[] Seller HAS knowledge of flooding that damaged the property during Seller's ownership.

  #v(0.5em)

  #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] Seller has NO knowledge of flooding that damaged the property.
]

#v(1em)

#text(weight: "bold")[2. FLOOD INSURANCE CLAIMS]

#let has_flood_claims = get_bool("has_flood_claims")

#if has_flood_claims [
  #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] Flood insurance claims HAVE been filed for this property.

  #v(0.5em)

  Details: #get("flood_claims_details", default: "[Describe claims]")
] else [
  #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] No flood insurance claims have been filed for this property.
]

#v(1em)

#text(weight: "bold")[3. FLOOD ASSISTANCE RECEIVED]
#v(0.3em)
#text(size: 10pt, style: "italic")[(Expanded to include federal, state, local, and private assistance effective October 1, 2025)]

#let has_flood_assistance = get_bool("has_flood_assistance")

#if has_flood_assistance [
  #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] Assistance HAS been received for flood damage to this property.

  #v(0.5em)

  Source of assistance: #get("flood_assistance_source", default: "[Federal/State/Local/Private]")

  Details: #get("flood_assistance_details", default: "[Describe assistance received]")
] else [
  #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] No assistance has been received for flood damage to this property.
]

#v(1em)

#rect(
  width: 100%,
  inset: 12pt,
  stroke: 1pt + rgb("#666"),
  fill: rgb("#fffbeb"),
  radius: 4pt,
)[
  #text(weight: "bold")[DEFINITION OF "FLOODING" (§ 689.302)]

  #v(0.5em)

  For purposes of this disclosure, "flooding" means a general or temporary condition of partial or complete inundation of the property caused by:
  - The unusual and rapid accumulation of runoff or surface waters from any established water source (river, stream, drainage ditch), or
  - Sustained periods of standing water resulting from rainfall
]

#v(2em)

#grid(
  columns: (1fr, 1fr),
  gutter: 40pt,
  [
    #text(weight: "bold")[SELLER]
    #v(1.5em)
    Signature: #box(width: 150pt, repeat[\_])
    #v(0.5em)
    Date: #box(width: 100pt, repeat[\_])
  ],
  [
    #text(weight: "bold")[BUYER ACKNOWLEDGMENT]
    #v(1.5em)
    Signature: #box(width: 150pt, repeat[\_])
    #v(0.5em)
    Date: #box(width: 100pt, repeat[\_])
  ]
)

#pagebreak()

// ============================================================================
// ADDENDUM D: ENERGY EFFICIENCY DISCLOSURE (MANDATORY - § 553.996)
// ============================================================================

#text(size: 14pt, weight: "bold")[ADDENDUM D: ENERGY EFFICIENCY DISCLOSURE]
#v(0.5em)
#text(size: 10pt, style: "italic")[Required by Florida Statutes § 553.996]
#v(1em)

#rect(
  width: 100%,
  inset: 15pt,
  stroke: 2pt + rgb("#0066cc"),
  fill: rgb("#f0f8ff"),
  radius: 4pt,
)[
  #text(weight: "bold", size: 12pt)[ENERGY EFFICIENCY RATING DISCLOSURE]

  #v(1em)

  In accordance with Section 553.996, Florida Statutes, Buyer is hereby notified that Buyer has the option to have an energy-efficiency rating conducted on this property.

  #v(1em)

  #text(weight: "bold")[Information about energy-efficiency ratings:]

  #v(0.5em)

  + How to analyze the building's energy-efficiency rating
  + Comparisons to statewide averages for new and existing construction
  + Information concerning methods to improve the building's energy-efficiency rating
  + An energy-efficiency rating may qualify the purchaser for an energy-efficient mortgage from lending institutions

  #v(1em)

  For more information, contact a certified energy rater or your local utility company.
]

#v(2em)

#text(size: 11pt, weight: "bold")[ACKNOWLEDGMENT]

By signing below, Buyer acknowledges receipt of this Energy Efficiency Disclosure as required by Florida law.

#v(2em)

#grid(
  columns: (1fr, 1fr),
  gutter: 40pt,
  [
    Buyer Signature: #box(width: 150pt, repeat[\_])
    #v(0.5em)
    Date: #box(width: 100pt, repeat[\_])
  ],
  [
    Buyer Signature: #box(width: 150pt, repeat[\_])
    #v(0.5em)
    Date: #box(width: 100pt, repeat[\_])
  ]
)

#pagebreak()

// ============================================================================
// ADDENDUM E: LEAD-BASED PAINT DISCLOSURE (Pre-1978 Properties)
// ============================================================================

#let year_built = get_num("year_built", default: 2000)

#if year_built < 1978 [
  #text(size: 14pt, weight: "bold")[ADDENDUM E: LEAD-BASED PAINT DISCLOSURE]
  #v(0.5em)
  #text(size: 10pt, style: "italic")[Required by 24 CFR Part 35 for housing built before 1978]
  #v(1em)

  #rect(
    width: 100%,
    inset: 12pt,
    stroke: 2pt + rgb("#cc0000"),
    fill: rgb("#fff5f5"),
    radius: 4pt,
  )[
    #text(weight: "bold", size: 11pt)[IMPORTANT NOTICE]

    Housing built before 1978 may contain lead-based paint. Lead from paint, paint chips, and dust can pose health hazards if not managed properly. Lead exposure is especially harmful to young children and pregnant women.
  ]

  #v(1em)

  #text(size: 12pt, weight: "bold")[SELLER'S DISCLOSURE]
  #v(0.5em)

  (a) Presence of lead-based paint and/or lead-based paint hazards:

  #if get_bool("lead_paint_known") [
    #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] Known lead-based paint and/or lead-based paint hazards are present

    Location/Condition: #get("lead_paint_details", default: "[Details]")

    #box(width: 12pt, height: 12pt, stroke: 1pt)[] Seller has no knowledge of lead-based paint
  ] else [
    #box(width: 12pt, height: 12pt, stroke: 1pt)[] Known lead-based paint and/or lead-based paint hazards are present

    #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] Seller has no knowledge of lead-based paint and/or lead-based paint hazards
  ]

  #v(1em)

  (b) Records and reports available to the Seller:

  #if get_bool("lead_reports_available") [
    #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] Seller has provided Buyer with all available records and reports
  ] else [
    #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] Seller has no reports or records pertaining to lead-based paint
  ]

  #v(1em)

  #text(size: 12pt, weight: "bold")[BUYER'S ACKNOWLEDGMENT]
  #v(0.5em)

  (c) Buyer has received the following:

  #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] The pamphlet "Protect Your Family From Lead in Your Home"

  #v(0.5em)

  (d) Buyer has received a 10-day opportunity to conduct a risk assessment or inspection:

  #if get_bool("lead_inspection_waived") [
    #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] Buyer has waived the opportunity to conduct a lead-based paint inspection
  ] else [
    #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] Buyer has received the 10-day opportunity
  ]

  #v(2em)

  #grid(
    columns: (1fr, 1fr),
    gutter: 20pt,
    [
      #text(weight: "bold")[SELLER]
      #v(1.5em)
      Signature: #box(width: 150pt, repeat[\_])
      #v(0.5em)
      Date: #box(width: 100pt, repeat[\_])
    ],
    [
      #text(weight: "bold")[BUYER]
      #v(1.5em)
      Signature: #box(width: 150pt, repeat[\_])
      #v(0.5em)
      Date: #box(width: 100pt, repeat[\_])
    ]
  )

  #pagebreak()
]

// ============================================================================
// ADDENDUM F: HOA/COMMUNITY DISCLOSURE (§ 720.401)
// ============================================================================

#if get_bool("has_hoa") [
  #text(size: 14pt, weight: "bold")[ADDENDUM F: HOMEOWNERS' ASSOCIATION DISCLOSURE]
  #v(0.5em)
  #text(size: 10pt, style: "italic")[Required by Florida Statutes § 720.401]
  #v(1em)

  #rect(
    width: 100%,
    inset: 15pt,
    stroke: 2pt + rgb("#dc2626"),
    fill: rgb("#fef2f2"),
    radius: 4pt,
  )[
    #text(weight: "bold", size: 12pt)[DISCLOSURE SUMMARY FOR (Name of Community)]

    #v(1em)

    #text(weight: "bold")[#get("hoa_name", default: "[HOA Name]")]

    #v(1em)

    #text(weight: "bold")[1.] AS A PURCHASER OF PROPERTY IN THIS COMMUNITY, YOU WILL BE OBLIGATED TO BE A MEMBER OF A HOMEOWNERS' ASSOCIATION.

    #v(0.5em)

    #text(weight: "bold")[2.] THERE HAVE BEEN OR WILL BE RECORDED RESTRICTIVE COVENANTS GOVERNING THE USE AND OCCUPANCY OF PROPERTIES IN THIS COMMUNITY.

    #v(0.5em)

    #text(weight: "bold")[3.] YOU WILL BE OBLIGATED TO PAY ASSESSMENTS TO THE ASSOCIATION. ASSESSMENTS MAY BE SUBJECT TO PERIODIC CHANGE. IF APPLICABLE, THE CURRENT AMOUNT IS #format_money(get_num("hoa_assessment")) PER #get("hoa_assessment_frequency", default: "MONTH"). HOWEVER, YOU SHOULD NOT RELY ON THE CURRENT ASSESSMENT AMOUNT.

    #v(0.5em)

    #text(weight: "bold")[4.] YOU MAY BE OBLIGATED TO PAY SPECIAL ASSESSMENTS TO THE RESPECTIVE ASSOCIATION. SUCH SPECIAL ASSESSMENTS MAY BE SUBJECT TO CHANGE. IF APPLICABLE, THE CURRENT AMOUNT IS #format_money(get_num("hoa_special_assessment", default: 0)).

    #v(0.5em)

    #text(weight: "bold")[5.] YOU MAY BE OBLIGATED TO PAY A FEE TO THE ASSOCIATION FOR CAPITAL IMPROVEMENTS OR OTHER SIMILAR FEES.

    #v(0.5em)

    #text(weight: "bold")[6.] YOUR FAILURE TO PAY SPECIAL ASSESSMENTS OR ASSESSMENTS LEVIED BY A MANDATORY HOMEOWNERS' ASSOCIATION COULD RESULT IN A LIEN ON YOUR PROPERTY.

    #v(0.5em)

    #text(weight: "bold")[7.] THE STATEMENTS CONTAINED IN THIS DISCLOSURE FORM ARE ONLY SUMMARY IN NATURE, AND, AS A PROSPECTIVE PURCHASER, YOU SHOULD REFER TO THE COVENANTS AND THE ASSOCIATION GOVERNING DOCUMENTS BEFORE PURCHASING PROPERTY.
  ]

  #v(1em)

  *HOA Contact Information:*
  #v(0.3em)
  #get("hoa_contact", default: "[HOA Management Company/Contact]")
  #v(0.2em)
  #get("hoa_address", default: "[HOA Address]")
  #v(0.2em)
  Phone: #get("hoa_phone", default: "[Phone]")

  #v(1em)

  #rect(
    width: 100%,
    inset: 12pt,
    stroke: 1pt + rgb("#666"),
    fill: rgb("#fffbeb"),
    radius: 4pt,
  )[
    #text(weight: "bold")[BUYER'S CANCELLATION RIGHT (§ 720.401)]

    #v(0.5em)

    If this disclosure summary is not provided before you execute the contract for sale, you may void the contract by delivering written notice to the Seller within 3 days after receiving this disclosure or prior to closing, whichever occurs first. Any purported waiver of this right is void. This right terminates at closing.
  ]

  #v(2em)

  #grid(
    columns: (1fr, 1fr),
    gutter: 40pt,
    [
      #text(weight: "bold")[SELLER]
      #v(1.5em)
      Signature: #box(width: 150pt, repeat[\_])
      #v(0.5em)
      Date: #box(width: 100pt, repeat[\_])
    ],
    [
      #text(weight: "bold")[BUYER ACKNOWLEDGMENT]
      #v(1.5em)
      Signature: #box(width: 150pt, repeat[\_])
      #v(0.5em)
      Date: #box(width: 100pt, repeat[\_])
    ]
  )

  #pagebreak()
]

// ============================================================================
// ADDENDUM G: SELLER'S PROPERTY DISCLOSURE (Johnson v. Davis)
// ============================================================================

#text(size: 14pt, weight: "bold")[ADDENDUM G: SELLER'S PROPERTY DISCLOSURE]
#v(0.5em)
#text(size: 10pt, style: "italic")[Based on Johnson v. Davis, 480 So.2d 625 (Fla. 1985) - Material Defect Disclosure Duty]
#v(1em)

#rect(
  width: 100%,
  inset: 12pt,
  stroke: 1pt + rgb("#666"),
  fill: rgb("#f9f9f9"),
  radius: 4pt,
)[
  #text(weight: "bold")[SELLER'S DUTY TO DISCLOSE]

  #v(0.5em)

  Under Florida law (Johnson v. Davis), the Seller is required to disclose known facts materially affecting the value of the property which are not readily observable and are not known to the Buyer. This duty exists even if the property is sold "as is."
]

#v(1em)

#text(size: 12pt, weight: "bold")[PROPERTY SYSTEMS AND COMPONENTS]
#v(0.5em)

#text(size: 10pt, style: "italic")[Mark the current condition: (✓) Working, (X) Defective, (N/A) Not Applicable, (U) Unknown]

#v(0.5em)

#table(
  columns: (1fr, auto, auto, auto, auto),
  stroke: 0.5pt + rgb("#ccc"),
  inset: 6pt,
  align: (left, center, center, center, center),
  [*Item*], [*✓*], [*X*], [*N/A*], [*U*],
  [Roof], [], [], [], [],
  [HVAC/Air Conditioning], [], [], [], [],
  [Plumbing], [], [], [], [],
  [Electrical], [], [], [], [],
  [Water Heater], [], [], [], [],
  [Pool/Spa], [], [], [], [],
  [Appliances], [], [], [], [],
  [Garage Door/Opener], [], [], [], [],
  [Septic System], [], [], [], [],
  [Well], [], [], [], [],
  [Sprinkler System], [], [], [], [],
  [Seawall/Dock], [], [], [], [],
)

#v(1em)

#text(size: 12pt, weight: "bold")[KNOWN DEFECTS OR MATERIAL FACTS]
#v(0.5em)

Seller discloses the following known defects or material facts affecting the property:

#v(0.5em)

#rect(
  width: 100%,
  height: 80pt,
  stroke: 0.5pt + rgb("#ccc"),
  inset: 8pt,
)[
  #get("known_defects", default: "")
]

#v(1em)

#text(size: 12pt, weight: "bold")[PAST REPAIRS OR INSURANCE CLAIMS]
#v(0.5em)

#get("past_repairs", default: "None disclosed.")

#v(1em)

#text(size: 12pt, weight: "bold")[ENVIRONMENTAL CONCERNS]
#v(0.5em)

#let has_environmental = get_bool("has_environmental_issues")

#if has_environmental [
  #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] Yes - See details below
  #v(0.3em)
  #get("environmental_details", default: "[Details]")
] else [
  #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] No known environmental concerns (mold, asbestos, underground tanks, etc.)
]

#v(1em)

#text(size: 12pt, weight: "bold")[SELLER'S CERTIFICATION]
#v(0.5em)

Seller certifies that the information provided above is true and correct to the best of Seller's knowledge as of the date signed. Seller agrees to notify Buyer of any changes to this disclosure prior to closing.

#v(2em)

#grid(
  columns: (1fr, 1fr),
  gutter: 40pt,
  [
    #text(weight: "bold")[SELLER]
    #v(1.5em)
    Signature: #box(width: 150pt, repeat[\_])
    #v(0.5em)
    Date: #box(width: 100pt, repeat[\_])
  ],
  [
    #text(weight: "bold")[BUYER ACKNOWLEDGMENT]
    #v(1.5em)
    Signature: #box(width: 150pt, repeat[\_])
    #v(0.5em)
    Date: #box(width: 100pt, repeat[\_])
  ]
)
