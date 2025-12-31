// Florida Eviction Notice Template
// Per Florida Statutes § 83.56 (Termination of Rental Agreement)
// This is the formal notice required BEFORE filing eviction lawsuit
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
  #text(size: 18pt, weight: "bold")[NOTICE TO VACATE / EVICTION NOTICE]
  #v(0.3em)
  #text(size: 12pt)[State of Florida]
  #v(0.3em)
  #text(size: 10pt, style: "italic")[Pursuant to Florida Statutes Chapter 83, Part II]
]

#v(1.5em)

// ============================================================================
// DATE AND PROPERTY INFORMATION
// ============================================================================

#text(weight: "bold")[Date:] #get("notice_date", default: "[Date]")

#v(1em)

#text(weight: "bold")[Property Address:]
#v(0.3em)
#get("property_address", default: "[Property Address]")
#if get("property_unit") != "" [, Unit #get("property_unit")]
#v(0.3em)
#get("property_city", default: "[City]"), Florida #get("property_zip", default: "[ZIP]")

#v(1.5em)

// ============================================================================
// TENANT INFORMATION
// ============================================================================

#text(weight: "bold")[TO:]
#v(0.3em)
#get("tenant_name", default: "[Tenant Name(s)]")
#if get("additional_tenants") != "" [
  #linebreak()
  #get("additional_tenants")
]
#linebreak()
and all other occupants

#v(1.5em)

// ============================================================================
// NOTICE TYPE SELECTION
// ============================================================================

#let notice_type = get("notice_type", default: "3_day_pay")

#if notice_type == "3_day_pay" [
  // 3-Day Notice to Pay Rent or Vacate (§ 83.56(3))
  #rect(
    width: 100%,
    inset: 12pt,
    stroke: 2pt + rgb("#cc0000"),
    fill: rgb("#fff5f5"),
    radius: 4pt,
  )[
    #text(weight: "bold", size: 14pt, fill: rgb("#cc0000"))[
      THREE (3) DAY NOTICE TO PAY RENT OR VACATE
    ]
    #v(0.8em)

    Pursuant to *Florida Statutes § 83.56(3)*, you are hereby notified that you owe the following rent:

    #v(0.5em)

    #table(
      columns: (1fr, 1fr),
      inset: 8pt,
      stroke: 0.5pt,
      [*Rent Period*], [#get("rent_period", default: "[Month/Year]")],
      [*Monthly Rent*], [#get("monthly_rent", default: "$[Amount]")],
      [*Amount Past Due*], [#get("amount_due", default: "$[Amount]")],
      [*Late Fees (if applicable)*], [#get("late_fees", default: "$0.00")],
      [*TOTAL AMOUNT DUE*], [*#get("total_due", default: "$[Total]")*],
    )

    #v(0.8em)

    *YOU MUST PAY THIS AMOUNT WITHIN THREE (3) DAYS* (excluding Saturday, Sunday, and legal holidays) of receipt of this notice, or you must vacate the premises.

    #v(0.5em)

    If you do not pay the full amount due or vacate within three (3) days, your landlord will begin legal proceedings against you to recover possession of the premises, past due rent, court costs, and attorney's fees (if permitted by your lease).
  ]
] else if notice_type == "7_day_cure" [
  // 7-Day Notice to Cure (§ 83.56(2)(a))
  #rect(
    width: 100%,
    inset: 12pt,
    stroke: 2pt + rgb("#ff6600"),
    fill: rgb("#fff8f0"),
    radius: 4pt,
  )[
    #text(weight: "bold", size: 14pt, fill: rgb("#ff6600"))[
      SEVEN (7) DAY NOTICE TO CURE LEASE VIOLATION
    ]
    #v(0.8em)

    Pursuant to *Florida Statutes § 83.56(2)(a)*, you are hereby notified that you are in violation of your rental agreement as follows:

    #v(0.5em)

    *NATURE OF VIOLATION:*
    #v(0.3em)
    #rect(
      width: 100%,
      inset: 8pt,
      stroke: 0.5pt,
    )[
      #get("violation_description", default: "[Description of the specific lease violation]")
    ]

    #v(0.5em)

    *LEASE PROVISION VIOLATED:*
    #v(0.3em)
    #get("lease_provision", default: "[Section/paragraph of lease that was violated]")

    #v(0.8em)

    *YOU MUST CURE THIS VIOLATION WITHIN SEVEN (7) DAYS* of receipt of this notice. If you do not cure this violation within seven (7) days, your tenancy will terminate and you must vacate the premises.

    #v(0.5em)

    *WARNING:* If this same violation occurs within 12 months of this notice, your landlord may terminate your tenancy without giving you an opportunity to cure.
  ]
] else if notice_type == "7_day_incurable" [
  // 7-Day Notice - Incurable (§ 83.56(2)(b))
  #rect(
    width: 100%,
    inset: 12pt,
    stroke: 2pt + rgb("#cc0000"),
    fill: rgb("#fff5f5"),
    radius: 4pt,
  )[
    #text(weight: "bold", size: 14pt, fill: rgb("#cc0000"))[
      SEVEN (7) DAY UNCONDITIONAL NOTICE TO VACATE
    ]
    #v(0.8em)

    Pursuant to *Florida Statutes § 83.56(2)(b)*, you are hereby notified that you have committed a material violation of your rental agreement that cannot be cured.

    #v(0.5em)

    *NATURE OF VIOLATION:*
    #v(0.3em)
    #rect(
      width: 100%,
      inset: 8pt,
      stroke: 0.5pt,
    )[
      #get("violation_description", default: "[Description of the incurable violation]")
    ]

    #v(0.8em)

    The violation you have committed is of such a serious nature that it cannot be remedied. *Your tenancy is hereby terminated.*

    #v(0.5em)

    *YOU MUST VACATE THE PREMISES WITHIN SEVEN (7) DAYS* of receipt of this notice.

    #v(0.5em)

    If you do not vacate within seven (7) days, the landlord will begin legal proceedings to recover possession of the premises.
  ]
] else if notice_type == "15_day" [
  // 15-Day Notice - Month-to-Month (§ 83.57(3))
  #rect(
    width: 100%,
    inset: 12pt,
    stroke: 2pt + rgb("#0066cc"),
    fill: rgb("#f0f8ff"),
    radius: 4pt,
  )[
    #text(weight: "bold", size: 14pt, fill: rgb("#0066cc"))[
      FIFTEEN (15) DAY NOTICE TO VACATE
    ]
    #v(0.8em)

    Pursuant to *Florida Statutes § 83.57(3)*, you are hereby notified that your month-to-month tenancy is being terminated.

    #v(0.5em)

    *TERMINATION DATE:* #get("termination_date", default: "[Date]")

    #v(0.5em)

    You must vacate the premises and return all keys on or before the termination date. This notice is being provided at least fifteen (15) days before the end of any monthly period.
  ]
]

#v(1.5em)

// ============================================================================
// PAYMENT INSTRUCTIONS (for 3-day notice)
// ============================================================================

#if notice_type == "3_day_pay" [
  #text(size: 12pt, weight: "bold")[Payment Instructions]
  #v(0.5em)

  Payment of the full amount due must be made by:

  #v(0.3em)
  #list(
    [Cash or certified funds],
    [Money order or cashier's check],
    [Personal check (if permitted by your lease)],
  )

  #v(0.5em)

  *Payment must be delivered to:*
  #v(0.3em)
  #get("landlord_name", default: "[Landlord/Agent Name]")
  #linebreak()
  #get("payment_address", default: "[Payment Address]")
  #if get("payment_phone") != "" [
    #linebreak()
    Phone: #get("payment_phone")
  ]

  #v(1em)
]

// ============================================================================
// LANDLORD INFORMATION
// ============================================================================

#text(size: 12pt, weight: "bold")[From:]
#v(0.5em)

#get("landlord_name", default: "[Landlord Name]")
#if get("landlord_company") != "" [
  #linebreak()
  #get("landlord_company")
]
#linebreak()
#get("landlord_address", default: "[Landlord Address]")
#linebreak()
#get("landlord_city", default: "[City]"), Florida #get("landlord_zip", default: "[ZIP]")
#if get("landlord_phone") != "" [
  #linebreak()
  Phone: #get("landlord_phone")
]
#if get("landlord_email") != "" [
  #linebreak()
  Email: #get("landlord_email")
]

#v(1.5em)

// ============================================================================
// LEGAL NOTICE
// ============================================================================

#rect(
  width: 100%,
  inset: 10pt,
  stroke: 1pt + rgb("#666"),
  fill: rgb("#f5f5f5"),
  radius: 4pt,
)[
  #text(size: 9pt)[
    *IMPORTANT LEGAL NOTICE*

    This notice is served pursuant to Florida Statutes Chapter 83, Part II (Residential Tenancies). You have legal rights under Florida law.

    #list(
      marker: sym.bullet,
      [You have the right to contest this notice in court.],
      [You may be entitled to a jury trial if you demand one.],
      [If you believe this notice is improper, you should consult an attorney.],
      [Free legal assistance may be available through Legal Aid or local tenant advocacy groups.],
    )

    *FOR NON-PAYMENT NOTICES:* The three (3) day period excludes Saturday, Sunday, and legal holidays. The day you receive this notice does not count as one of the three days.

    *WARNING:* Failure to respond to this notice may result in a lawsuit being filed against you. An eviction judgment may affect your credit and ability to rent in the future.
  ]
]

#v(2em)

// ============================================================================
// SIGNATURE
// ============================================================================

#text(size: 12pt, weight: "bold")[LANDLORD/AGENT SIGNATURE]
#v(1em)

#grid(
  columns: (1fr, 1fr),
  gutter: 2em,
  [
    #line(length: 100%, stroke: 0.5pt)
    Signature
    #v(1em)
    #get("landlord_name", default: "[Landlord/Agent Name]")
    #linebreak()
    Printed Name
  ],
  [
    #line(length: 100%, stroke: 0.5pt)
    Date
    #v(1em)
    #get("notice_date", default: "[Date]")
  ]
)

#v(2em)

// ============================================================================
// CERTIFICATE OF SERVICE
// ============================================================================

#text(size: 12pt, weight: "bold")[CERTIFICATE OF SERVICE]
#v(0.5em)

I hereby certify that a copy of this notice was served on the tenant(s) named above on #get("service_date", default: "[Service Date]") by the following method:

#v(0.5em)

#let service_method = get("service_method", default: "personal")

#if service_method == "personal" [
  #sym.ballot.x Personal delivery to the tenant
] else [
  #sym.ballot Personal delivery to the tenant
]

#if service_method == "substitute" [
  #sym.ballot.x Delivery to a person of suitable age and discretion at the premises
] else [
  #sym.ballot Delivery to a person of suitable age and discretion at the premises
]

#if service_method == "posting" [
  #sym.ballot.x Posting on the premises AND mailing a copy
] else [
  #sym.ballot Posting on the premises AND mailing a copy
]

#v(0.5em)

#text(size: 9pt, style: "italic")[
  Note: Under Florida law, service may be made by: (1) personal delivery to tenant; (2) delivery to a person of suitable age and discretion at the premises; or (3) posting on the premises AND mailing a copy.
]

#v(1.5em)

#grid(
  columns: (1fr, 1fr),
  gutter: 2em,
  [
    #line(length: 100%, stroke: 0.5pt)
    Server Signature
  ],
  [
    #line(length: 100%, stroke: 0.5pt)
    Date
  ]
)

#v(2em)

// ============================================================================
// TENANT RESPONSE (Optional)
// ============================================================================

#if get_bool("include_response_section") [
  #line(length: 100%, stroke: 1pt)
  #v(1em)

  #text(size: 12pt, weight: "bold")[TENANT ACKNOWLEDGMENT OF RECEIPT]
  #v(0.5em)

  I acknowledge that I received this notice on: #box(width: 2in)[#line(length: 100%, stroke: 0.5pt)]

  #v(1em)

  #grid(
    columns: (1fr, 1fr),
    gutter: 2em,
    [
      #line(length: 100%, stroke: 0.5pt)
      Tenant Signature
    ],
    [
      #line(length: 100%, stroke: 0.5pt)
      Date
    ]
  )

  #v(1em)
]

// ============================================================================
// DISCLAIMER
// ============================================================================

#align(center)[
  #text(size: 8pt, fill: rgb("#666"))[
    DISCLAIMER: This document was prepared using agentPDF.org, a document preparation service. This is not legal advice. No attorney-client relationship is created. Eviction is a legal process—consult a Florida attorney for specific legal guidance.
  ]
]
