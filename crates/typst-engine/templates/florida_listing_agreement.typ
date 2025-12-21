// Florida Exclusive Listing Agreement Template
// Compliant with F.S. Chapter 475 - Real Estate Brokers and Salespersons
// Includes required brokerage relationship disclosures per § 475.278
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
#let format_percent(amount) = {
  let num = if type(amount) == str { float(amount) } else { float(amount) }
  str(calc.round(num, digits: 2)) + "%"
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

  #text(size: 24pt, weight: "bold")[EXCLUSIVE LISTING AGREEMENT]

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
    ]
  ]

  #v(3em)

  #grid(
    columns: (1fr, 1fr),
    gutter: 40pt,
    [
      #text(weight: "bold")[SELLER/OWNER]
      #v(0.3em)
      #get("seller_name", default: "[Seller Name]")
    ],
    [
      #text(weight: "bold")[LISTING BROKER]
      #v(0.3em)
      #get("broker_name", default: "[Broker Name]")
      #v(0.2em)
      License \#: #get("broker_license", default: "[License]")
    ]
  )

  #v(2em)

  #text(size: 14pt, weight: "bold")[
    Listing Price: #format_money(get_num("listing_price"))
  ]

  #v(4em)

  #text(size: 9pt, fill: rgb("#666"))[
    This agreement is governed by Florida Statutes Chapter 475 and the rules of the Florida Real Estate Commission (FREC).
  ]
]

#pagebreak()

// ============================================================================
// BROKERAGE RELATIONSHIP DISCLOSURE (MANDATORY - § 475.278)
// ============================================================================

#text(size: 14pt, weight: "bold")[BROKERAGE RELATIONSHIP DISCLOSURE]
#v(0.5em)
#text(size: 10pt, style: "italic")[Required by Florida Statutes § 475.278]
#v(1em)

#let relationship_type = get("brokerage_relationship", default: "single_agent")

#if relationship_type == "single_agent" [
  // SINGLE AGENT DISCLOSURE - Per § 475.278(3)
  #rect(
    width: 100%,
    inset: 15pt,
    stroke: 2pt + rgb("#0066cc"),
    fill: rgb("#f0f8ff"),
    radius: 4pt,
  )[
    #text(weight: "bold", size: 14pt)[SINGLE AGENT NOTICE]

    #v(1em)

    #text(weight: "bold", size: 11pt)[FLORIDA LAW REQUIRES THAT REAL ESTATE LICENSEES OPERATING AS SINGLE AGENTS DISCLOSE TO BUYERS AND SELLERS THEIR DUTIES.]

    #v(1em)

    As a single agent, #get("broker_name", default: "[Broker Name]") and its associates owe to you the following duties:

    #v(0.5em)

    #text(weight: "bold")[1. Dealing honestly and fairly;]

    #text(weight: "bold")[2. Loyalty;]

    #text(weight: "bold")[3. Confidentiality;]

    #text(weight: "bold")[4. Obedience;]

    #text(weight: "bold")[5. Full disclosure;]

    #text(weight: "bold")[6. Accounting for all funds;]

    #text(weight: "bold")[7. Skill, care, and diligence in the transaction;]

    #text(weight: "bold")[8. Presenting all offers and counteroffers in a timely manner, unless a party has previously directed the licensee otherwise in writing; and]

    #text(weight: "bold")[9. Disclosing all known facts that materially affect the value of residential real property and are not readily observable.]

    #v(1em)

    #text(size: 9pt, style: "italic")[
      This disclosure is required by Florida Statutes § 475.278(3)(b). The duties above must be fully described and disclosed in writing before, or at the time of, entering into a listing agreement.
    ]
  ]
] else [
  // TRANSACTION BROKER DISCLOSURE - Per § 475.278(2)
  #rect(
    width: 100%,
    inset: 15pt,
    stroke: 2pt + rgb("#0066cc"),
    fill: rgb("#f0f8ff"),
    radius: 4pt,
  )[
    #text(weight: "bold", size: 14pt)[TRANSACTION BROKER NOTICE]

    #v(1em)

    #text(weight: "bold", size: 11pt)[FLORIDA LAW REQUIRES THAT REAL ESTATE LICENSEES OPERATING AS TRANSACTION BROKERS DISCLOSE TO BUYERS AND SELLERS THEIR ROLE AND DUTIES IN PROVIDING A LIMITED FORM OF REPRESENTATION.]

    #v(1em)

    As a transaction broker, #get("broker_name", default: "[Broker Name]") and its associates provide to you a limited form of representation that includes the following duties:

    #v(0.5em)

    #text(weight: "bold")[1. Dealing honestly and fairly;]

    #text(weight: "bold")[2. Accounting for all funds;]

    #text(weight: "bold")[3. Using skill, care, and diligence in the transaction;]

    #text(weight: "bold")[4. Disclosing all known facts that materially affect the value of residential real property and are not readily observable to the buyer;]

    #text(weight: "bold")[5. Presenting all offers and counteroffers in a timely manner, unless a party has previously directed the licensee otherwise in writing;]

    #text(weight: "bold")[6. Limited confidentiality, unless waived in writing by a party. This limited confidentiality will prevent disclosure that the seller will accept a price less than the asking or listed price, that the buyer will pay a price greater than the price submitted in a written offer, of the motivation of any party for selling or buying property, that a seller or buyer will agree to financing terms other than those offered, or of any other information requested by a party to remain confidential; and]

    #text(weight: "bold")[7. Any additional duties that are mutually agreed to with a party.]

    #v(1em)

    #text(size: 9pt, style: "italic")[
      This disclosure is required by Florida Statutes § 475.278(2). A transaction broker does NOT represent either party in a fiduciary capacity.
    ]
  ]
]

#v(1em)

*Brokerage Relationship Selected:*

#if relationship_type == "single_agent" [
  #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] *Single Agent* (Fiduciary relationship with full duties)

  #box(width: 12pt, height: 12pt, stroke: 1pt)[] Transaction Broker (Limited representation)
] else [
  #box(width: 12pt, height: 12pt, stroke: 1pt)[] Single Agent (Fiduciary relationship with full duties)

  #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] *Transaction Broker* (Limited representation)
]

#v(2em)

#text(size: 11pt, weight: "bold")[ACKNOWLEDGMENT OF BROKERAGE RELATIONSHIP]

By signing below, Seller acknowledges receipt and understanding of this Brokerage Relationship Disclosure as required by Florida law.

#v(1.5em)

Seller Signature: #box(width: 200pt, repeat[\_]) Date: #box(width: 100pt, repeat[\_])

#v(0.5em)

Print Name: #get("seller_name", default: "[Seller Name]")

#pagebreak()

// ============================================================================
// LISTING AGREEMENT TERMS
// ============================================================================

#text(size: 16pt, weight: "bold")[EXCLUSIVE LISTING AGREEMENT]
#v(1em)

This Exclusive Listing Agreement ("Agreement") is entered into this #get("agreement_date", default: "[Date]") by and between:

#v(0.5em)

#table(
  columns: (auto, 1fr),
  stroke: 0.5pt + rgb("#ccc"),
  inset: 8pt,
  [*SELLER*], [#get("seller_name", default: "[Seller Name]")],
  [*Address*], [#get("seller_address", default: "[Seller Address]")],
  [*Phone*], [#get("seller_phone", default: "[Phone]")],
  [*Email*], [#get("seller_email", default: "[Email]")],
)

#v(0.5em)

#table(
  columns: (auto, 1fr),
  stroke: 0.5pt + rgb("#ccc"),
  inset: 8pt,
  [*BROKER*], [#get("broker_name", default: "[Broker Name]")],
  [*Broker License \#*], [#get("broker_license", default: "[License]")],
  [*Brokerage Firm*], [#get("brokerage_firm", default: "[Firm Name]")],
  [*Address*], [#get("broker_address", default: "[Broker Address]")],
  [*Phone*], [#get("broker_phone", default: "[Phone]")],
  [*Email*], [#get("broker_email", default: "[Email]")],
)

#v(1em)

// ============================================================================
// SECTION 1: PROPERTY
// ============================================================================

#text(size: 14pt, weight: "bold")[1. PROPERTY]
#v(1em)

Seller hereby grants Broker the exclusive right to sell the following property:

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
  [*Property Type*], [#get("property_type", default: "Single Family Residence")],
)

#v(0.5em)

*Legal Description:*
#get("legal_description", default: "[Legal description as recorded in public records]")

#v(1em)

// ============================================================================
// SECTION 2: LISTING PERIOD
// ============================================================================

#text(size: 14pt, weight: "bold")[2. LISTING PERIOD]
#v(1em)

#rect(
  width: 100%,
  inset: 12pt,
  stroke: 1pt + rgb("#dc2626"),
  fill: rgb("#fef2f2"),
  radius: 4pt,
)[
  #text(weight: "bold")[DEFINITE EXPIRATION DATE REQUIRED (§ 475.25)]

  #v(0.5em)

  Pursuant to Florida Statutes § 475.25, this Agreement must contain a definite expiration date. This Agreement shall NOT automatically renew.
]

#v(0.5em)

#table(
  columns: (1fr, 1fr),
  stroke: 0.5pt + rgb("#ccc"),
  inset: 10pt,
  [*Listing Start Date*], [#get("listing_start_date", default: "[Start Date]")],
  [*Listing Expiration Date*], [#get("listing_expiration_date", default: "[Expiration Date]")],
)

#v(0.5em)

This Agreement shall expire at 11:59 PM on the Listing Expiration Date. Seller is NOT required to provide notice to cancel after expiration.

#v(1em)

// ============================================================================
// SECTION 3: LISTING PRICE AND TERMS
// ============================================================================

#text(size: 14pt, weight: "bold")[3. LISTING PRICE AND TERMS]
#v(1em)

#table(
  columns: (auto, 1fr),
  stroke: 0.5pt + rgb("#ccc"),
  inset: 10pt,
  [*Listing Price*], [*#format_money(get_num("listing_price"))*],
  [*Minimum Acceptable Price*], [#format_money(get_num("minimum_price", default: 0)) (for Broker reference only)],
)

#v(0.5em)

*Acceptable Financing Terms:*

#let cash_ok = get_bool("accept_cash")
#let conventional_ok = get_bool("accept_conventional")
#let fha_ok = get_bool("accept_fha")
#let va_ok = get_bool("accept_va")

#if cash_ok [#box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark]] else [#box(width: 12pt, height: 12pt, stroke: 1pt)[]] Cash
#h(1em)
#if conventional_ok [#box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark]] else [#box(width: 12pt, height: 12pt, stroke: 1pt)[]] Conventional
#h(1em)
#if fha_ok [#box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark]] else [#box(width: 12pt, height: 12pt, stroke: 1pt)[]] FHA
#h(1em)
#if va_ok [#box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark]] else [#box(width: 12pt, height: 12pt, stroke: 1pt)[]] VA

#v(0.5em)

*Personal Property Included:*
#get("included_items", default: "Standard fixtures and appliances as typically conveyed with the property.")

*Excluded Items:*
#get("excluded_items", default: "None.")

#v(1em)

// ============================================================================
// SECTION 4: BROKER'S COMPENSATION
// ============================================================================

#text(size: 14pt, weight: "bold")[4. BROKER'S COMPENSATION]
#v(1em)

// NAR Settlement Compliance - Mandatory Fee Negotiability Disclosure
#rect(
  width: 100%,
  inset: 12pt,
  stroke: 2pt + rgb("#b45309"),
  fill: rgb("#fffbeb"),
  radius: 4pt,
)[
  #text(weight: "bold", size: 11pt)[NOTICE: BROKER FEES ARE FULLY NEGOTIABLE]

  #v(0.5em)

  #text(weight: "bold")[
    THE AMOUNT OR RATE OF REAL ESTATE COMMISSIONS IS NOT FIXED BY LAW. COMMISSIONS ARE SET BY EACH BROKER INDIVIDUALLY AND MAY BE NEGOTIABLE BETWEEN SELLER AND BROKER.
  ]

  #v(0.5em)

  #text(size: 10pt)[
    This disclosure is required pursuant to the 2024 NAR Settlement Agreement. Seller acknowledges that broker compensation is not set by any law, regulation, or real estate board, and that Seller has had the opportunity to negotiate the commission rate or fee.
  ]

  #v(0.5em)

  #grid(
    columns: (1fr, 1fr),
    gutter: 20pt,
    [
      Seller Initials: #box(width: 60pt, repeat[\_])
    ],
    [
      Date: #box(width: 80pt, repeat[\_])
    ]
  )
]

#v(1em)

#text(size: 12pt, weight: "bold")[4.1 COMMISSION]
#v(0.5em)

Seller agrees to pay Broker a commission calculated as follows:

#let commission_type = get("commission_type", default: "percentage")

#if commission_type == "percentage" [
  #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] *#format_percent(get_num("commission_rate"))* of the gross sales price

  #box(width: 12pt, height: 12pt, stroke: 1pt)[] Flat fee of \$\_\_\_\_\_\_
] else [
  #box(width: 12pt, height: 12pt, stroke: 1pt)[] \_\_\_% of the gross sales price

  #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] *Flat fee of #format_money(get_num("flat_fee"))*
]

#v(0.5em)

#text(size: 12pt, weight: "bold")[4.2 COOPERATING BROKER COMPENSATION]
#v(0.5em)

Broker may offer compensation to cooperating brokers who procure a buyer:

#format_percent(get_num("coop_commission_rate", default: 0)) of the gross sales price, or #format_money(get_num("coop_flat_fee", default: 0)) flat fee

#v(0.5em)

#text(size: 12pt, weight: "bold")[4.3 WHEN COMMISSION IS EARNED]
#v(0.5em)

Commission shall be due and payable upon the earlier of:
+ Closing of the sale
+ Seller's default or refusal to close on a ready, willing, and able buyer
+ Seller's withdrawal of the property from the market during the listing period

#v(0.5em)

#text(size: 12pt, weight: "bold")[4.4 PROTECTION PERIOD]
#v(0.5em)

If, within #get("protection_period_days", default: "90") days after expiration of this Agreement, the Property is sold to any buyer who was introduced to the Property during the listing period, Seller shall pay Broker the full commission, unless the Property is re-listed with another broker.

#v(1em)

// ============================================================================
// SECTION 5: BROKER'S DUTIES AND SERVICES
// ============================================================================

#text(size: 14pt, weight: "bold")[5. BROKER'S DUTIES AND SERVICES]
#v(1em)

Broker agrees to:

+ Use diligent efforts to find a buyer for the Property
+ Market the Property through appropriate channels
+ Present all offers to Seller in a timely manner
+ Assist in negotiating terms acceptable to Seller
+ Coordinate with title company, lender, and other parties
+ Provide guidance through the closing process

#v(0.5em)

*Marketing Services:*

#let mls_ok = get_bool("list_on_mls")
#let photos_ok = get_bool("professional_photos")
#let virtual_ok = get_bool("virtual_tour")
#let open_house_ok = get_bool("open_houses")

#if mls_ok [#box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark]] else [#box(width: 12pt, height: 12pt, stroke: 1pt)[]] MLS Listing
#h(1em)
#if photos_ok [#box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark]] else [#box(width: 12pt, height: 12pt, stroke: 1pt)[]] Professional Photography
#h(1em)
#if virtual_ok [#box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark]] else [#box(width: 12pt, height: 12pt, stroke: 1pt)[]] Virtual Tour

#if open_house_ok [#box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark]] else [#box(width: 12pt, height: 12pt, stroke: 1pt)[]] Open Houses
#h(1em)
#box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] Online Marketing
#h(1em)
#box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] Yard Sign

#v(1em)

// ============================================================================
// SECTION 6: SELLER'S DUTIES AND REPRESENTATIONS
// ============================================================================

#text(size: 14pt, weight: "bold")[6. SELLER'S DUTIES AND REPRESENTATIONS]
#v(1em)

Seller agrees to:

+ Cooperate with Broker in marketing and showing the Property
+ Make the Property available for showings at reasonable times
+ Refer all inquiries to Broker
+ Provide accurate information about the Property
+ Complete all required disclosures honestly
+ Maintain the Property in showable condition
+ Notify Broker of any material changes to the Property

#v(0.5em)

*Seller represents and warrants:*

+ Seller has the legal authority to sell the Property
+ The Property is not subject to any undisclosed liens or encumbrances
+ #{ if get_bool("property_occupied") { [The Property is currently occupied by: #get("occupant_type", default: "Owner")] } else { [The Property is vacant] } }
+ #{ if get_bool("has_hoa") { [The Property is subject to a homeowners' association] } else { [The Property is NOT subject to a homeowners' association] } }

#v(1em)

// ============================================================================
// SECTION 7: DISCLOSURE REQUIREMENTS
// ============================================================================

#text(size: 14pt, weight: "bold")[7. DISCLOSURE REQUIREMENTS]
#v(1em)

Seller acknowledges the duty to disclose all known material facts affecting the value of the Property (Johnson v. Davis, 480 So.2d 625). Seller agrees to complete all required disclosures, including:

+ Seller's Property Disclosure Statement
+ Radon Gas Notification (§ 404.056)
+ Lead-Based Paint Disclosure (if built before 1978)
+ Flood Disclosure (§ 689.302)
+ HOA Disclosure (§ 720.401, if applicable)
+ Any other disclosures required by law

#v(1em)

// ============================================================================
// SECTION 8: LOCKBOX AND ACCESS
// ============================================================================

#text(size: 14pt, weight: "bold")[8. LOCKBOX AND ACCESS]
#v(1em)

#if get_bool("lockbox_authorized") [
  #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] *Lockbox Authorized:* Seller authorizes Broker to install a lockbox for showing access.
] else [
  #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] *No Lockbox:* Showings by appointment only through Broker.
]

#v(0.5em)

*Showing Instructions:*
#get("showing_instructions", default: "Contact listing agent to schedule all showings.")

#v(1em)

// ============================================================================
// SECTION 9: ADDITIONAL TERMS
// ============================================================================

#text(size: 14pt, weight: "bold")[9. ADDITIONAL TERMS]
#v(1em)

#text(size: 12pt, weight: "bold")[9.1 FAIR HOUSING]
#v(0.5em)

Broker and Seller agree to comply with all federal, state, and local fair housing laws. The Property will be marketed and shown without regard to race, color, religion, sex, handicap, familial status, national origin, or any other protected class.

#v(0.5em)

#text(size: 12pt, weight: "bold")[9.2 WIRE FRAUD WARNING]
#v(0.5em)

#rect(
  width: 100%,
  inset: 12pt,
  stroke: 2pt + rgb("#dc2626"),
  fill: rgb("#fef2f2"),
  radius: 4pt,
)[
  #text(weight: "bold")[WIRE FRAUD ALERT]

  Real estate transactions are targets for wire fraud. NEVER wire funds based on email instructions alone. Always verify wiring instructions by calling a known, trusted phone number.
]

#v(0.5em)

#text(size: 12pt, weight: "bold")[9.3 DISPUTE RESOLUTION]
#v(0.5em)

#if get_bool("mediation_required") [
  Any dispute arising under this Agreement shall first be submitted to mediation before litigation.
]

This Agreement shall be governed by Florida law. Venue for any legal action shall be in #get("property_county", default: "[County]") County, Florida.

#v(0.5em)

#text(size: 12pt, weight: "bold")[9.4 ENTIRE AGREEMENT]
#v(0.5em)

This Agreement, including any attached addenda, constitutes the entire agreement between the parties. Amendments must be in writing and signed by both parties.

#v(0.5em)

#text(size: 12pt, weight: "bold")[9.5 ADDITIONAL PROVISIONS]
#v(0.5em)

#get("additional_provisions", default: "None.")

#pagebreak()

// ============================================================================
// SECTION 10: SIGNATURES
// ============================================================================

#text(size: 14pt, weight: "bold")[10. SIGNATURES]
#v(1em)

By signing below, the parties agree to all terms and conditions of this Exclusive Listing Agreement.

#v(0.5em)

#rect(
  width: 100%,
  inset: 12pt,
  stroke: 1pt + rgb("#666"),
  fill: rgb("#f9f9f9"),
  radius: 4pt,
)[
  #text(weight: "bold")[SELLER ACKNOWLEDGMENTS]

  #v(0.5em)

  By signing, Seller acknowledges:
  + Receipt and understanding of the Brokerage Relationship Disclosure
  + This Agreement has a definite expiration date and does not auto-renew
  + Seller's duty to disclose material facts about the Property
  + Seller has had the opportunity to seek legal counsel
]

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
    #text(weight: "bold")[BROKER/AGENT]

    #v(2em)

    Signature: #box(width: 180pt, repeat[\_])

    #v(0.8em)

    Print Name: #get("agent_name", default: "[Agent Name]")

    #v(0.8em)

    License \#: #get("agent_license", default: "[License]")

    #v(0.8em)

    Date: #box(width: 120pt, repeat[\_])
  ]
)

#v(2em)

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
      #text(weight: "bold")[SUPERVISING BROKER]

      #v(2em)

      Signature: #box(width: 180pt, repeat[\_])

      #v(0.8em)

      Print Name: #get("broker_name", default: "[Broker Name]")

      #v(0.8em)

      License \#: #get("broker_license", default: "[License]")

      #v(0.8em)

      Date: #box(width: 120pt, repeat[\_])
    ]
  )
]

#v(2em)

#align(center)[
  #text(size: 9pt, fill: rgb("#666"))[
    #get("brokerage_firm", default: "[Brokerage Firm Name]") | #get("broker_address", default: "[Address]") | #get("broker_phone", default: "[Phone]")
  ]
]
