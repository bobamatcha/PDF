// Florida Financing Contingency Addendum Template
// Standard financing/mortgage contingency for Florida real estate transactions
// All values are dynamic via sys.inputs

#let data = sys.inputs

// Helper functions
#let get(key, default: "") = data.at(key, default: default)
#let get_bool(key) = {
  let val = data.at(key, default: false)
  if type(val) == str { val == "true" } else { val == true }
}

// Page setup
#set page(
  paper: "us-letter",
  margin: (top: 1in, bottom: 1in, left: 1in, right: 1in),
)
#set text(font: "Liberation Sans", size: 11pt)
#set par(justify: true, leading: 0.65em)

// ============================================================================
// HEADER
// ============================================================================

#align(center)[
  #text(size: 16pt, weight: "bold")[FINANCING CONTINGENCY ADDENDUM]
  #v(0.3em)
  #text(size: 11pt)[To Purchase and Sale Agreement]
  #v(0.3em)
  #text(size: 10pt, style: "italic")[State of Florida]
]

#v(1em)

// ============================================================================
// CONTRACT REFERENCE
// ============================================================================

#rect(
  width: 100%,
  inset: 10pt,
  stroke: 1pt + rgb("#0066cc"),
  fill: rgb("#f0f8ff"),
  radius: 4pt,
)[
  This Financing Contingency Addendum is attached to and made part of the Purchase and Sale Agreement dated *#get("contract_date", default: "[Contract Date]")* between:

  #v(0.5em)
  *BUYER(S):* #get("buyer_name", default: "[Buyer Name(s)]")

  *SELLER(S):* #get("seller_name", default: "[Seller Name(s)]")

  *PROPERTY:* #get("property_address", default: "[Property Address]"), #get("property_city", default: "[City]"), Florida #get("property_zip", default: "[ZIP]")

  *PURCHASE PRICE:* $#get("purchase_price", default: "[Purchase Price]")
]

#v(1.5em)

// ============================================================================
// 1. FINANCING TERMS
// ============================================================================

#text(size: 12pt, weight: "bold")[1. FINANCING TERMS]
#v(0.5em)

This Contract is contingent upon Buyer obtaining financing as follows:

#v(0.5em)

#table(
  columns: (1fr, 1fr),
  inset: 8pt,
  stroke: 0.5pt,
  [*Loan Amount*], [$#get("loan_amount", default: "[Loan Amount]")],
  [*Loan Type*], [#get("loan_type", default: "Conventional")],
  [*Maximum Interest Rate*], [#get("max_interest_rate", default: "[Rate]")%],
  [*Loan Term*], [#get("loan_term", default: "30") years],
  [*Down Payment*], [$#get("down_payment", default: "[Amount]") (#get("down_payment_percent", default: "[%]")%)],
)

#v(0.5em)

#let loan_type = get("loan_type", default: "Conventional")

*Loan Type:*
#if loan_type == "Conventional" [
  #sym.ballot.x Conventional
] else [
  #sym.ballot Conventional
]
#h(1em)
#if loan_type == "FHA" [
  #sym.ballot.x FHA
] else [
  #sym.ballot FHA
]
#h(1em)
#if loan_type == "VA" [
  #sym.ballot.x VA
] else [
  #sym.ballot VA
]
#h(1em)
#if loan_type == "USDA" [
  #sym.ballot.x USDA
] else [
  #sym.ballot USDA
]
#h(1em)
#if loan_type == "Other" [
  #sym.ballot.x Other: #get("loan_type_other", default: "")
] else [
  #sym.ballot Other: #box(width: 1in)[#line(length: 100%, stroke: 0.5pt)]
]

#v(1em)

// ============================================================================
// 2. FINANCING PERIOD
// ============================================================================

#text(size: 12pt, weight: "bold")[2. FINANCING PERIOD]
#v(0.5em)

Buyer shall have until *#get("financing_deadline", default: "[Date]")* (the "Financing Deadline") to:

#list(
  [Submit a complete loan application within *#get("application_days", default: "5")* days of the Effective Date],
  [Provide Seller with a loan commitment or denial by the Financing Deadline],
  [Obtain final loan approval (clear to close) prior to closing],
)

#v(1em)

// ============================================================================
// 3. BUYER'S OBLIGATIONS
// ============================================================================

#text(size: 12pt, weight: "bold")[3. BUYER'S OBLIGATIONS]
#v(0.5em)

Buyer agrees to:

#list(
  [Apply for financing within *#get("application_days", default: "5")* days of the Effective Date],
  [Provide complete and accurate information to the lender],
  [Promptly provide all documentation requested by the lender],
  [Not make any changes to financial status that would adversely affect loan approval (e.g., changing jobs, making large purchases, opening new credit accounts)],
  [Maintain employment and creditworthiness through closing],
  [Pay all loan application fees, appraisal fees, and related costs],
  [Keep Seller informed of the loan status],
]

#v(1em)

// ============================================================================
// 4. LOAN COMMITMENT
// ============================================================================

#text(size: 12pt, weight: "bold")[4. LOAN COMMITMENT]
#v(0.5em)

A loan commitment acceptable under this Addendum must:

#list(
  [Be in writing from an institutional lender],
  [Commit to provide financing on terms equal to or better than those specified in Section 1],
  [Contain only conditions within Buyer's reasonable control or standard conditions related to title, appraisal, or survey],
)

#v(0.5em)

Upon receipt of an acceptable loan commitment, Buyer shall provide written notice to Seller within *#get("commitment_notice_days", default: "3")* days.

#v(1em)

// ============================================================================
// 5. APPRAISAL CONTINGENCY
// ============================================================================

#if get_bool("include_appraisal_contingency") [
  #text(size: 12pt, weight: "bold")[5. APPRAISAL CONTINGENCY]
  #v(0.5em)

  #rect(
    width: 100%,
    inset: 10pt,
    stroke: 1pt,
    radius: 4pt,
  )[
    This Contract is contingent upon the Property appraising at no less than the Purchase Price.

    #v(0.5em)

    If the appraisal is less than the Purchase Price:
    #list(
      [Seller may elect to reduce the Purchase Price to the appraised value],
      [Buyer may elect to pay the difference in cash],
      [The parties may negotiate a new Purchase Price],
      [Either party may cancel the Contract and Buyer's deposit shall be returned],
    )

    #v(0.5em)

    *Appraisal Gap Coverage:* Buyer agrees to pay up to $#get("appraisal_gap", default: "0") above the appraised value.
  ]

  #v(1em)
]

// ============================================================================
// 6. FAILURE TO OBTAIN FINANCING
// ============================================================================

#text(size: 12pt, weight: "bold")[#if get_bool("include_appraisal_contingency") [6.] else [5.] FAILURE TO OBTAIN FINANCING]
#v(0.5em)

If Buyer is unable to obtain financing on the terms specified:

#v(0.3em)

*Option A - Denial of Financing:*
If Buyer's loan application is denied and Buyer provides written notice to Seller with documentation of the denial on or before the Financing Deadline:
#list(
  [This Contract shall be terminated],
  [Buyer's earnest money deposit shall be returned in full],
  [Neither party shall have further liability under this Contract],
)

#v(0.5em)

*Option B - Failure to Provide Notice:*
If Buyer fails to provide notice of financing denial or commitment by the Financing Deadline:

#let default_action = get("default_action", default: "waive")

#if default_action == "waive" [
  #sym.ballot.x Buyer waives this contingency and agrees to proceed with purchase
] else [
  #sym.ballot Buyer waives this contingency and agrees to proceed with purchase
]

#if default_action == "terminate" [
  #sym.ballot.x Seller may terminate this Contract by providing written notice to Buyer
] else [
  #sym.ballot Seller may terminate this Contract by providing written notice to Buyer
]

#v(1em)

// ============================================================================
// 7. SELLER'S RIGHT TO CONTINUE MARKETING
// ============================================================================

#if get_bool("seller_backup_rights") [
  #text(size: 12pt, weight: "bold")[#if get_bool("include_appraisal_contingency") [7.] else [6.] SELLER'S RIGHT TO CONTINUE MARKETING]
  #v(0.5em)

  Seller reserves the right to continue marketing the Property and accept backup offers. If Seller accepts a backup offer, Seller shall provide written notice to Buyer. Buyer shall then have *#get("backup_response_days", default: "48")* hours to:

  #list(
    [Waive this Financing Contingency and proceed with the purchase, OR],
    [Terminate this Contract, in which case Buyer's deposit shall be returned],
  )

  #v(1em)
]

// ============================================================================
// 8. ADDITIONAL FINANCING TERMS
// ============================================================================

#if get("additional_terms") != "" [
  #text(size: 12pt, weight: "bold")[ADDITIONAL FINANCING TERMS]
  #v(0.5em)

  #get("additional_terms")

  #v(1.5em)
]

// ============================================================================
// LENDER INFORMATION
// ============================================================================

#text(size: 12pt, weight: "bold")[LENDER INFORMATION]
#v(0.5em)

#table(
  columns: (1fr, 2fr),
  inset: 8pt,
  stroke: 0.5pt,
  [*Lender Name*], [#get("lender_name", default: "[Lender Name]")],
  [*Loan Officer*], [#get("loan_officer", default: "[Loan Officer Name]")],
  [*Phone*], [#get("lender_phone", default: "[Phone]")],
  [*Email*], [#get("lender_email", default: "[Email]")],
  [*NMLS #*], [#get("lender_nmls", default: "[NMLS Number]")],
)

#v(1.5em)

// ============================================================================
// DEPOSIT INFORMATION
// ============================================================================

#text(size: 12pt, weight: "bold")[DEPOSIT INFORMATION]
#v(0.5em)

*Earnest Money Deposit:* $#get("deposit_amount", default: "[Amount]")

*Escrow Agent:* #get("escrow_agent", default: "[Escrow Agent Name]")

*Escrow Agent Address:* #get("escrow_address", default: "[Address]")

#v(1.5em)

// ============================================================================
// SIGNATURES
// ============================================================================

#text(size: 12pt, weight: "bold")[AGREEMENT AND SIGNATURES]
#v(0.5em)

This Addendum, upon execution by all parties, becomes part of the Purchase and Sale Agreement referenced above. All other terms of the Purchase Agreement remain in full force and effect.

#v(1.5em)

#text(weight: "bold")[BUYER(S):]
#v(1em)

#grid(
  columns: (1fr, 1fr),
  gutter: 2em,
  [
    #line(length: 100%, stroke: 0.5pt)
    Buyer Signature
    #v(0.8em)
    #get("buyer_name", default: "[Buyer Name]")
    #linebreak()
    Printed Name
    #v(0.8em)
    Date: #box(width: 1.5in)[#line(length: 100%, stroke: 0.5pt)]
  ],
  [
    #if get("buyer2_name") != "" [
      #line(length: 100%, stroke: 0.5pt)
      Buyer Signature
      #v(0.8em)
      #get("buyer2_name")
      #linebreak()
      Printed Name
      #v(0.8em)
      Date: #box(width: 1.5in)[#line(length: 100%, stroke: 0.5pt)]
    ]
  ]
)

#v(2em)

#text(weight: "bold")[SELLER(S):]
#v(1em)

#grid(
  columns: (1fr, 1fr),
  gutter: 2em,
  [
    #line(length: 100%, stroke: 0.5pt)
    Seller Signature
    #v(0.8em)
    #get("seller_name", default: "[Seller Name]")
    #linebreak()
    Printed Name
    #v(0.8em)
    Date: #box(width: 1.5in)[#line(length: 100%, stroke: 0.5pt)]
  ],
  [
    #if get("seller2_name") != "" [
      #line(length: 100%, stroke: 0.5pt)
      Seller Signature
      #v(0.8em)
      #get("seller2_name")
      #linebreak()
      Printed Name
      #v(0.8em)
      Date: #box(width: 1.5in)[#line(length: 100%, stroke: 0.5pt)]
    ]
  ]
)

#v(2em)

// ============================================================================
// DISCLAIMER
// ============================================================================

#align(center)[
  #text(size: 8pt, fill: rgb("#666"))[
    DISCLAIMER: This document was prepared using agentPDF.org, a document preparation service. This is not legal advice. No attorney-client relationship is created. For real estate transactions, consult a Florida real estate attorney or licensed real estate professional.
  ]
]
