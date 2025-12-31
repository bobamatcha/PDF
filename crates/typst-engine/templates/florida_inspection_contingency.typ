// Florida Inspection Contingency Addendum Template
// Standard real estate inspection contingency for Florida transactions
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
  #text(size: 16pt, weight: "bold")[INSPECTION CONTINGENCY ADDENDUM]
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
  This Inspection Contingency Addendum is attached to and made part of the Purchase and Sale Agreement dated *#get("contract_date", default: "[Contract Date]")* between:

  #v(0.5em)
  *BUYER(S):* #get("buyer_name", default: "[Buyer Name(s)]")

  *SELLER(S):* #get("seller_name", default: "[Seller Name(s)]")

  *PROPERTY:* #get("property_address", default: "[Property Address]"), #get("property_city", default: "[City]"), Florida #get("property_zip", default: "[ZIP]")

  #if get("legal_description") != "" [
    #v(0.3em)
    *Legal Description:* #get("legal_description")
  ]
]

#v(1.5em)

// ============================================================================
// 1. INSPECTION PERIOD
// ============================================================================

#text(size: 12pt, weight: "bold")[1. INSPECTION PERIOD]
#v(0.5em)

Buyer shall have *#get("inspection_days", default: "15")* calendar days from the Effective Date of the Purchase Agreement (the "Inspection Period") to conduct inspections of the Property at Buyer's expense.

#v(0.3em)

*Inspection Period Start Date:* #get("inspection_start_date", default: "[Effective Date]")

*Inspection Period End Date:* #get("inspection_end_date", default: "[End Date]") at 11:59 PM

#v(1em)

// ============================================================================
// 2. TYPES OF INSPECTIONS
// ============================================================================

#text(size: 12pt, weight: "bold")[2. INSPECTIONS AUTHORIZED]
#v(0.5em)

Buyer may conduct, at Buyer's sole expense, the following inspections by licensed professionals:

#v(0.3em)

#let inspection_types = (
  ("general_inspection", "General Home Inspection"),
  ("roof_inspection", "Roof Inspection"),
  ("termite_inspection", "Wood-Destroying Organism (WDO/Termite) Inspection"),
  ("mold_inspection", "Mold Inspection"),
  ("pool_inspection", "Pool/Spa Inspection"),
  ("septic_inspection", "Septic System Inspection"),
  ("well_inspection", "Well Water Testing"),
  ("hvac_inspection", "HVAC Inspection"),
  ("electrical_inspection", "Electrical System Inspection"),
  ("plumbing_inspection", "Plumbing Inspection"),
  ("structural_inspection", "Structural/Foundation Inspection"),
  ("radon_inspection", "Radon Testing"),
  ("survey", "Property Survey"),
  ("environmental", "Environmental Assessment"),
)

#for (key, label) in inspection_types [
  #if get_bool(key) [#sym.ballot.x] else [#sym.ballot] #label
  #linebreak()
]

#if get("other_inspections") != "" [
  #sym.ballot.x Other: #get("other_inspections")
]

#v(1em)

// ============================================================================
// 3. ACCESS AND SCHEDULING
// ============================================================================

#text(size: 12pt, weight: "bold")[3. ACCESS AND SCHEDULING]
#v(0.5em)

#list(
  [Seller shall provide reasonable access to the Property for all inspections upon *24 hours* advance notice.],
  [Inspections shall be scheduled at mutually agreeable times during normal business hours unless otherwise agreed.],
  [Buyer and Buyer's inspectors shall not damage the Property during inspections.],
  [Buyer shall restore the Property to its pre-inspection condition.],
  [Utilities shall remain on during the Inspection Period at Seller's expense.],
)

#v(1em)

// ============================================================================
// 4. INSPECTION CONTINGENCY TYPE
// ============================================================================

#text(size: 12pt, weight: "bold")[4. INSPECTION CONTINGENCY TYPE]
#v(0.5em)

#let contingency_type = get("contingency_type", default: "right_to_cancel")

#if contingency_type == "right_to_cancel" [
  #rect(
    width: 100%,
    inset: 10pt,
    stroke: 1pt,
    radius: 4pt,
  )[
    #sym.ballot.x *OPTION A - RIGHT TO CANCEL ("AS-IS" WITH INSPECTION)*

    Buyer may cancel this contract for any reason related to the inspections by delivering written notice to Seller before the Inspection Period expires. If Buyer fails to deliver written notice of cancellation, Buyer shall be deemed to have accepted the Property in its present "AS-IS" condition and this contingency shall be deemed satisfied.
  ]
] else [
  #rect(
    width: 100%,
    inset: 10pt,
    stroke: 1pt,
    radius: 4pt,
  )[
    #sym.ballot *OPTION A - RIGHT TO CANCEL ("AS-IS" WITH INSPECTION)*

    Buyer may cancel this contract for any reason related to the inspections by delivering written notice to Seller before the Inspection Period expires. If Buyer fails to deliver written notice of cancellation, Buyer shall be deemed to have accepted the Property in its present "AS-IS" condition and this contingency shall be deemed satisfied.
  ]
]

#v(0.5em)

#if contingency_type == "right_to_negotiate" [
  #rect(
    width: 100%,
    inset: 10pt,
    stroke: 1pt,
    radius: 4pt,
  )[
    #sym.ballot.x *OPTION B - RIGHT TO NEGOTIATE REPAIRS*

    Buyer shall deliver to Seller, within the Inspection Period, a written list of repair requests. Seller shall have *#get("seller_response_days", default: "5")* days to respond in writing to accept, reject, or propose alternative terms. If Seller rejects or proposes alternative terms, Buyer shall have *#get("buyer_response_days", default: "3")* days to accept or cancel.
  ]
] else [
  #rect(
    width: 100%,
    inset: 10pt,
    stroke: 1pt,
    radius: 4pt,
  )[
    #sym.ballot *OPTION B - RIGHT TO NEGOTIATE REPAIRS*

    Buyer shall deliver to Seller, within the Inspection Period, a written list of repair requests. Seller shall have *#get("seller_response_days", default: "5")* days to respond in writing to accept, reject, or propose alternative terms. If Seller rejects or proposes alternative terms, Buyer shall have *#get("buyer_response_days", default: "3")* days to accept or cancel.
  ]
]

#v(1em)

// ============================================================================
// 5. REPAIR LIMITATIONS (if applicable)
// ============================================================================

#if contingency_type == "right_to_negotiate" [
  #text(size: 12pt, weight: "bold")[5. REPAIR LIMITATIONS]
  #v(0.5em)

  #if get("repair_cap") != "" [
    Seller's obligation to make repairs shall not exceed *$#get("repair_cap")*.

    If the cost of agreed repairs exceeds this amount:
    #list(
      [Buyer may elect to pay the excess amount, or],
      [Either party may cancel the contract and Buyer's deposit shall be returned.],
    )

    #v(0.5em)
  ]

  Repairs shall be:
  #list(
    [Completed by licensed and insured contractors where required by Florida law],
    [Completed in a workmanlike manner and in compliance with applicable codes],
    [Completed before closing unless otherwise agreed in writing],
    [Accompanied by paid receipts and/or lien waivers at closing],
  )

  #v(1em)
]

// ============================================================================
// 6. WDO (TERMITE) INSPECTION REQUIREMENTS
// ============================================================================

#if get_bool("termite_inspection") [
  #text(size: 12pt, weight: "bold")[#if contingency_type == "right_to_negotiate" [6.] else [5.] WDO (TERMITE) INSPECTION]
  #v(0.5em)

  The Wood-Destroying Organism inspection shall be conducted by a licensed pest control operator.

  #v(0.3em)

  #let wdo_responsibility = get("wdo_responsibility", default: "seller")

  *Treatment Responsibility:*
  #if wdo_responsibility == "seller" [
    #list(
      [Seller shall pay for treatment of any active infestation],
      [Seller shall pay for repair of damage up to $#get("wdo_repair_cap", default: "1,500")],
    )
  ] else if wdo_responsibility == "buyer" [
    #list(
      [Buyer accepts responsibility for any WDO treatment and repair],
    )
  ] else [
    #list(
      [Treatment and repair costs shall be negotiated between the parties],
    )
  ]

  #v(1em)
]

// ============================================================================
// 7. CANCELLATION AND DEPOSIT
// ============================================================================

#text(size: 12pt, weight: "bold")[#if contingency_type == "right_to_negotiate" and get_bool("termite_inspection") [7.] else if contingency_type == "right_to_negotiate" or get_bool("termite_inspection") [6.] else [5.] CANCELLATION AND DEPOSIT]
#v(0.5em)

If Buyer properly cancels within the Inspection Period:
#list(
  [Buyer's earnest money deposit shall be returned in full],
  [All parties shall sign a mutual release],
  [Neither party shall have further liability under this contract],
)

#v(0.5em)

*Deposit Amount:* $#get("deposit_amount", default: "[Amount]")

*Escrow Agent:* #get("escrow_agent", default: "[Escrow Agent Name]")

#v(1.5em)

// ============================================================================
// 8. ADDITIONAL TERMS
// ============================================================================

#if get("additional_terms") != "" [
  #text(size: 12pt, weight: "bold")[ADDITIONAL TERMS]
  #v(0.5em)

  #get("additional_terms")

  #v(1.5em)
]

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
