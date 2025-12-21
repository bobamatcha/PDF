// Florida Escalation Addendum Template
// Addendum to Residential Real Estate Purchase Contract
// Based on best practices and industry standards for competitive offer scenarios
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
// HEADER
// ============================================================================

#align(center)[
  #text(size: 18pt, weight: "bold")[ESCALATION ADDENDUM]
  #v(0.3em)
  #text(size: 12pt)[TO RESIDENTIAL REAL ESTATE PURCHASE CONTRACT]
  #v(0.5em)
  #text(size: 11pt, style: "italic")[State of Florida]
]

#v(1em)

#rect(
  width: 100%,
  inset: 12pt,
  stroke: 1pt + rgb("#666"),
  fill: rgb("#f9f9f9"),
  radius: 4pt,
)[
  This Escalation Addendum ("Addendum") is attached to and made part of that certain Residential Real Estate Purchase Contract ("Contract") dated #get("contract_date", default: "[Date]") between:

  #v(0.5em)

  *Seller:* #get("seller_name", default: "[Seller Name]")

  *Buyer:* #get("buyer_name", default: "[Buyer Name]")

  *Property:* #get("property_address", default: "[Property Address]")
]

#v(1em)

// ============================================================================
// SECTION 1: ESCALATION TERMS
// ============================================================================

#text(size: 14pt, weight: "bold")[1. ESCALATION TERMS]
#v(1em)

#let base_price = get_num("base_purchase_price")
#let escalation_increment = get_num("escalation_increment")
#let max_price = get_num("maximum_purchase_price")

#text(size: 12pt, weight: "bold")[1.1 BASE PURCHASE PRICE]
#v(0.5em)

The Base Purchase Price offered by Buyer in the Contract is: *#format_money(base_price)*

#v(1em)

#text(size: 12pt, weight: "bold")[1.2 ESCALATION CLAUSE]
#v(0.5em)

If Seller receives one or more Bona Fide Competing Offers (as defined below), Buyer agrees to increase the Purchase Price as follows:

#v(0.5em)

#rect(
  width: 100%,
  inset: 15pt,
  stroke: 2pt + rgb("#0066cc"),
  fill: rgb("#f0f8ff"),
  radius: 4pt,
)[
  Buyer will pay *#format_money(escalation_increment)* more than the highest Bona Fide Competing Offer, up to a *MAXIMUM PURCHASE PRICE* of:

  #v(0.5em)

  #align(center)[
    #text(size: 16pt, weight: "bold")[#format_money(max_price)]
  ]
]

#v(1em)

#text(size: 12pt, weight: "bold")[1.3 ESCALATION INCREMENT]
#v(0.5em)

The escalation increment is: *#format_money(escalation_increment)*

This means Buyer will automatically outbid any qualifying competing offer by this amount, up to the Maximum Purchase Price.

#v(1em)

#text(size: 12pt, weight: "bold")[1.4 MAXIMUM PURCHASE PRICE CAP]
#v(0.5em)

#rect(
  width: 100%,
  inset: 12pt,
  stroke: 2pt + rgb("#dc2626"),
  fill: rgb("#fef2f2"),
  radius: 4pt,
)[
  #text(weight: "bold")[IMPORTANT: MAXIMUM PRICE LIMITATION]

  #v(0.5em)

  Under no circumstances shall the Purchase Price exceed *#format_money(max_price)* (the "Maximum Purchase Price"). If the highest Bona Fide Competing Offer plus the escalation increment exceeds the Maximum Purchase Price, the Purchase Price shall be capped at the Maximum Purchase Price.

  #v(0.5em)

  #text(size: 10pt, style: "italic")[
    Buyer represents that Buyer has the financial ability to pay up to the Maximum Purchase Price.
  ]
]

#v(1em)

// ============================================================================
// SECTION 2: BONA FIDE COMPETING OFFER
// ============================================================================

#text(size: 14pt, weight: "bold")[2. BONA FIDE COMPETING OFFER]
#v(1em)

#text(size: 12pt, weight: "bold")[2.1 DEFINITION]
#v(0.5em)

A "Bona Fide Competing Offer" means a written offer from a different prospective buyer that:

+ Is made in good faith and without collusion with Seller or any other party
+ Is a legitimate offer from a competing buyer who is not affiliated with Seller
+ Has not been created, formulated, or solicited by Seller solely to trigger this escalation clause
+ Includes terms that Seller would reasonably consider for acceptance absent this escalation clause
+ Is submitted on or before: #get("escalation_deadline", default: "[Date/Time]")

#v(0.5em)

#text(size: 12pt, weight: "bold")[2.2 EXCLUDED OFFERS]
#v(0.5em)

The following shall NOT be considered Bona Fide Competing Offers:

+ Offers from entities in which Seller has an ownership interest
+ Offers from family members of Seller (unless arm's length transaction)
+ Offers that Seller knows or should know are not genuine
+ Offers submitted after the escalation deadline
+ Offers with contingencies substantially more burdensome than Buyer's offer

#v(1em)

// ============================================================================
// SECTION 3: PROOF OF COMPETING OFFER
// ============================================================================

#text(size: 14pt, weight: "bold")[3. PROOF OF COMPETING OFFER]
#v(1em)

#text(size: 12pt, weight: "bold")[3.1 SELLER'S OBLIGATION TO PROVIDE PROOF]
#v(0.5em)

If Seller invokes this escalation clause, Seller shall provide Buyer with:

#if get_bool("require_full_offer_copy") [
  #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] A complete copy of the Bona Fide Competing Offer (with buyer's personal information redacted as reasonably requested)
] else [
  #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] The first page of the Bona Fide Competing Offer showing the purchase price (with buyer's personal information redacted)
]

#v(0.5em)

*Timing:* Seller shall provide such proof within #get("proof_deadline_hours", default: "24") hours of invoking the escalation clause or acceptance of Buyer's escalated offer, whichever is sooner.

#v(0.5em)

#text(size: 12pt, weight: "bold")[3.2 BUYER'S RIGHT TO VERIFY]
#v(0.5em)

Buyer shall have the right to review the competing offer proof to confirm:
+ The offer is bona fide
+ The purchase price stated
+ The offer was received before the escalation deadline

If Buyer reasonably disputes the validity of the competing offer, the parties shall negotiate in good faith. If no resolution is reached, Buyer may proceed at the Base Purchase Price or terminate the Contract with a full refund of earnest money.

#v(1em)

// ============================================================================
// SECTION 4: CALCULATION OF ESCALATED PRICE
// ============================================================================

#text(size: 14pt, weight: "bold")[4. CALCULATION OF ESCALATED PRICE]
#v(1em)

#text(size: 12pt, weight: "bold")[4.1 ESCALATION FORMULA]
#v(0.5em)

#table(
  columns: (1fr, auto),
  stroke: 0.5pt + rgb("#ccc"),
  inset: 10pt,
  [Highest Bona Fide Competing Offer], [\$ \[A\]],
  [Plus: Escalation Increment], [+ #format_money(escalation_increment)],
  table.hline(stroke: 1pt),
  [Calculated Escalated Price], [\$ \[A + Increment\]],
  table.hline(stroke: 0.5pt),
  [Maximum Purchase Price Cap], [#format_money(max_price)],
  table.hline(stroke: 1pt),
  [*Final Purchase Price*], [*Lesser of Calculated Price or Maximum*],
)

#v(0.5em)

#text(size: 12pt, weight: "bold")[4.2 EXAMPLE CALCULATIONS]
#v(0.5em)

#text(size: 10pt, style: "italic")[For illustration purposes based on this Addendum's terms:]

#v(0.3em)

*Example 1:* If highest competing offer is #format_money(base_price + 10000):
- Escalated price = #format_money(base_price + 10000 + escalation_increment)
- Final price = #format_money(calc.min(base_price + 10000 + escalation_increment, max_price))

*Example 2:* If highest competing offer is #format_money(max_price - 1000):
- Calculated price would be #format_money(max_price - 1000 + escalation_increment)
- Final price = #format_money(max_price) (capped at Maximum)

#v(1em)

// ============================================================================
// SECTION 5: APPRAISAL CONTINGENCY INTERACTION
// ============================================================================

#text(size: 14pt, weight: "bold")[5. APPRAISAL CONTINGENCY INTERACTION]
#v(1em)

#if get_bool("appraisal_gap_coverage") [
  #text(size: 12pt, weight: "bold")[5.1 APPRAISAL GAP COVERAGE]
  #v(0.5em)

  #let appraisal_gap = get_num("appraisal_gap_amount")

  Buyer agrees to cover any appraisal gap up to *#format_money(appraisal_gap)* if the property appraises below the Final Purchase Price.

  #v(0.5em)

  If the appraisal gap exceeds #format_money(appraisal_gap):

  #if get_bool("appraisal_waiver") [
    #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] Buyer waives appraisal contingency and agrees to pay the Final Purchase Price regardless of appraised value.
  ] else [
    #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] Buyer may renegotiate price or terminate with refund of earnest money.
  ]
] else [
  #text(size: 12pt, weight: "bold")[5.1 APPRAISAL CONTINGENCY PRESERVED]
  #v(0.5em)

  The appraisal contingency in the original Contract remains in effect. If the property appraises below the Final Purchase Price (as escalated), Buyer's rights under the appraisal contingency shall apply.
]

#v(1em)

// ============================================================================
// SECTION 6: FINANCING CONSIDERATIONS
// ============================================================================

#text(size: 14pt, weight: "bold")[6. FINANCING CONSIDERATIONS]
#v(1em)

#if get("financing_type", default: "conventional") == "cash" [
  #text(size: 12pt, weight: "bold")[6.1 CASH PURCHASE - PROOF OF FUNDS]
  #v(0.5em)

  Buyer represents that Buyer has available funds to pay up to the Maximum Purchase Price of #format_money(max_price) in cash.

  Buyer shall provide updated proof of funds within #get("updated_proof_days", default: "2") business days of the escalation being invoked if the escalated price exceeds the amount shown in Buyer's initial proof of funds.
] else [
  #text(size: 12pt, weight: "bold")[6.1 FINANCED PURCHASE - LOAN CONSIDERATIONS]
  #v(0.5em)

  Buyer acknowledges that:
  + Buyer's lender has been notified of this escalation clause
  + Buyer is pre-approved for financing up to #format_money(max_price)
  + Any increase in Purchase Price may affect loan-to-value ratios and down payment requirements

  #v(0.5em)

  #if get_bool("additional_down_payment_available") [
    Buyer represents having additional funds of at least #format_money(get_num("additional_funds")) available for increased down payment if required due to escalation.
  ]
]

#v(1em)

// ============================================================================
// SECTION 7: SELLER'S OPTIONS
// ============================================================================

#text(size: 14pt, weight: "bold")[7. SELLER'S OPTIONS AND RIGHTS]
#v(1em)

#text(size: 12pt, weight: "bold")[7.1 NO OBLIGATION TO INVOKE]
#v(0.5em)

Seller is not obligated to invoke this escalation clause. Seller retains the right to:
+ Accept Buyer's offer at the Base Purchase Price
+ Accept another offer (even if lower than Buyer's escalated price)
+ Reject all offers
+ Counter any offer

#v(0.5em)

#text(size: 12pt, weight: "bold")[7.2 MULTIPLE COMPETING OFFERS]
#v(0.5em)

If multiple Bona Fide Competing Offers are received, this escalation clause shall apply only to the highest such offer.

#v(0.5em)

#text(size: 12pt, weight: "bold")[7.3 COMPETING ESCALATION CLAUSES]
#v(0.5em)

If multiple offers contain escalation clauses, Seller may:
+ Compare final escalated prices from all escalating offers
+ This Buyer's escalated price will be determined by comparing against the highest competing offer (escalated or not)
+ Buyer understands that other escalation clauses may result in a purchase price exceeding Buyer's Maximum Purchase Price

#v(1em)

// ============================================================================
// SECTION 8: ADDITIONAL TERMS
// ============================================================================

#text(size: 14pt, weight: "bold")[8. ADDITIONAL TERMS]
#v(1em)

#text(size: 12pt, weight: "bold")[8.1 EARNEST MONEY]
#v(0.5em)

#if get_bool("increase_earnest_money") [
  If the escalation clause is invoked and the Purchase Price increases, Buyer shall deposit additional earnest money equal to #get("additional_earnest_percentage", default: "10")% of the price increase within #get("additional_earnest_days", default: "2") business days.
] else [
  The earnest money deposit stated in the Contract shall apply regardless of any price escalation.
]

#v(0.5em)

#text(size: 12pt, weight: "bold")[8.2 DEADLINE FOR ESCALATION]
#v(0.5em)

This escalation clause shall only apply to Bona Fide Competing Offers received by Seller on or before:

*Escalation Deadline:* #get("escalation_deadline", default: "[Date and Time]")

Competing offers received after this deadline shall not trigger the escalation.

#v(0.5em)

#text(size: 12pt, weight: "bold")[8.3 EXPIRATION]
#v(0.5em)

If no Bona Fide Competing Offer is received by the Escalation Deadline, the Contract shall proceed at the Base Purchase Price of #format_money(base_price).

#v(0.5em)

#text(size: 12pt, weight: "bold")[8.4 CONFLICT WITH CONTRACT]
#v(0.5em)

In the event of any conflict between this Addendum and the Contract, the terms of this Addendum shall control.

#v(0.5em)

#get("additional_terms", default: "")

#v(1em)

// ============================================================================
// SECTION 9: SIGNATURES
// ============================================================================

#text(size: 14pt, weight: "bold")[9. SIGNATURES]
#v(1em)

By signing below, the parties acknowledge they have read, understand, and agree to the terms of this Escalation Addendum.

#v(0.5em)

#rect(
  width: 100%,
  inset: 12pt,
  stroke: 1pt + rgb("#dc2626"),
  fill: rgb("#fef2f2"),
  radius: 4pt,
)[
  #text(weight: "bold")[BUYER'S ACKNOWLEDGMENT]

  #v(0.5em)

  Buyer acknowledges and confirms:
  + Buyer understands the escalation mechanism and Maximum Purchase Price
  + Buyer has the financial ability to pay up to the Maximum Purchase Price
  + Buyer has consulted with Buyer's broker and/or attorney regarding this Addendum
  + Buyer understands Seller is not obligated to accept this offer even with escalation
]

#v(2em)

#grid(
  columns: (1fr, 1fr),
  gutter: 40pt,
  [
    #text(weight: "bold")[BUYER]

    #v(2em)

    Signature: #box(width: 180pt, repeat[\_])

    #v(0.8em)

    Print Name: #get("buyer_name", default: "[Buyer Name]")

    #v(0.8em)

    Date: #box(width: 120pt, repeat[\_])
  ],
  [
    #text(weight: "bold")[SELLER]

    #v(2em)

    Signature: #box(width: 180pt, repeat[\_])

    #v(0.8em)

    Print Name: #get("seller_name", default: "[Seller Name]")

    #v(0.8em)

    Date: #box(width: 120pt, repeat[\_])
  ]
)

#v(2em)

#if get_bool("has_additional_parties") [
  #grid(
    columns: (1fr, 1fr),
    gutter: 40pt,
    [
      #text(weight: "bold")[ADDITIONAL BUYER]

      #v(2em)

      Signature: #box(width: 180pt, repeat[\_])

      #v(0.8em)

      Print Name: #get("additional_buyer_name", default: "[Additional Buyer]")

      #v(0.8em)

      Date: #box(width: 120pt, repeat[\_])
    ],
    [
      #text(weight: "bold")[ADDITIONAL SELLER]

      #v(2em)

      Signature: #box(width: 180pt, repeat[\_])

      #v(0.8em)

      Print Name: #get("additional_seller_name", default: "[Additional Seller]")

      #v(0.8em)

      Date: #box(width: 120pt, repeat[\_])
    ]
  )
]
