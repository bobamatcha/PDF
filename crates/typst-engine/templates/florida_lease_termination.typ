// Florida Lease Termination Notice Template
// Per Florida Statutes § 83.57 (Month-to-Month) and § 83.56 (Termination)
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
  #text(size: 18pt, weight: "bold")[NOTICE OF TERMINATION OF TENANCY]
  #v(0.3em)
  #text(size: 12pt)[State of Florida]
  #v(0.3em)
  #text(size: 10pt, style: "italic")[Pursuant to Florida Statutes Chapter 83]
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
#if get("additional_tenant_name") != "" [
  #linebreak()
  #get("additional_tenant_name")
]

#v(1.5em)

// ============================================================================
// NOTICE TYPE SELECTION
// ============================================================================

#let notice_type = get("notice_type", default: "30_day")
#let termination_date = get("termination_date", default: "[Termination Date]")

#if notice_type == "7_day_nonpayment" [
  // 7-Day Notice - Non-Payment of Rent (§ 83.56(3))
  #rect(
    width: 100%,
    inset: 12pt,
    stroke: 2pt + rgb("#cc0000"),
    fill: rgb("#fff5f5"),
    radius: 4pt,
  )[
    #text(weight: "bold", size: 12pt)[SEVEN (7) DAY NOTICE TO PAY RENT OR VACATE]
    #v(0.5em)
    Pursuant to *Florida Statutes § 83.56(3)*, you are hereby notified that you are in default of your rental agreement for failure to pay rent.

    #v(0.5em)
    *Amount Due:* #get("amount_due", default: "$[Amount]")

    *Rent Period:* #get("rent_period", default: "[Month/Year]")

    #v(0.5em)
    You must pay the full amount due within *SEVEN (7) DAYS* of receipt of this notice, or vacate the premises. If you do not pay the rent due or vacate within seven (7) days, your landlord may begin legal proceedings to terminate your tenancy and recover possession of the premises.
  ]
] else if notice_type == "7_day_noncompliance" [
  // 7-Day Notice - Non-Compliance (§ 83.56(2)(b))
  #rect(
    width: 100%,
    inset: 12pt,
    stroke: 2pt + rgb("#cc0000"),
    fill: rgb("#fff5f5"),
    radius: 4pt,
  )[
    #text(weight: "bold", size: 12pt)[SEVEN (7) DAY NOTICE OF NONCOMPLIANCE - INCURABLE]
    #v(0.5em)
    Pursuant to *Florida Statutes § 83.56(2)(b)*, you are hereby notified that you have materially violated the terms of your rental agreement in a manner that cannot be cured.

    #v(0.5em)
    *Nature of Violation:*
    #v(0.3em)
    #get("violation_description", default: "[Description of violation]")

    #v(0.5em)
    This violation is of such a nature that it cannot be remedied. Therefore, your tenancy will terminate *SEVEN (7) DAYS* from the date of this notice, on *#termination_date*.

    You must vacate the premises by that date.
  ]
] else if notice_type == "7_day_curable" [
  // 7-Day Notice - Curable Non-Compliance (§ 83.56(2)(a))
  #rect(
    width: 100%,
    inset: 12pt,
    stroke: 2pt + rgb("#ff6600"),
    fill: rgb("#fff8f0"),
    radius: 4pt,
  )[
    #text(weight: "bold", size: 12pt)[SEVEN (7) DAY NOTICE OF NONCOMPLIANCE - CURABLE]
    #v(0.5em)
    Pursuant to *Florida Statutes § 83.56(2)(a)*, you are hereby notified that you have violated the terms of your rental agreement.

    #v(0.5em)
    *Nature of Violation:*
    #v(0.3em)
    #get("violation_description", default: "[Description of violation]")

    #v(0.5em)
    You have *SEVEN (7) DAYS* from the date of this notice to cure this violation. If you do not cure the violation within seven (7) days, your tenancy will terminate and you must vacate the premises.

    #v(0.5em)
    *Note:* If this same violation occurs within 12 months of this notice, the landlord may terminate the rental agreement without providing you an opportunity to cure the violation.
  ]
] else if notice_type == "15_day" [
  // 15-Day Notice - Month-to-Month Termination (§ 83.57(3))
  #rect(
    width: 100%,
    inset: 12pt,
    stroke: 2pt + rgb("#0066cc"),
    fill: rgb("#f0f8ff"),
    radius: 4pt,
  )[
    #text(weight: "bold", size: 12pt)[FIFTEEN (15) DAY NOTICE OF TERMINATION]
    #v(0.5em)
    Pursuant to *Florida Statutes § 83.57(3)*, this notice is to inform you that your month-to-month tenancy will be terminated.

    #v(0.5em)
    Your tenancy will end on *#termination_date*.

    #v(0.5em)
    You must vacate the premises and return all keys by 11:59 PM on the termination date. Please ensure the property is left in clean condition, normal wear and tear excepted.
  ]
] else [
  // 30-Day Notice - Standard Termination (§ 83.57)
  #rect(
    width: 100%,
    inset: 12pt,
    stroke: 2pt + rgb("#0066cc"),
    fill: rgb("#f0f8ff"),
    radius: 4pt,
  )[
    #text(weight: "bold", size: 12pt)[THIRTY (30) DAY NOTICE OF TERMINATION]
    #v(0.5em)
    Pursuant to *Florida Statutes § 83.57* (as amended by HB 1417), this notice is to inform you that your tenancy will be terminated.

    #v(0.5em)
    Your tenancy will end on *#termination_date*.

    #v(0.5em)
    You must vacate the premises and return all keys by 11:59 PM on the termination date. Please ensure the property is left in clean condition, normal wear and tear excepted.
  ]
]

#v(1.5em)

// ============================================================================
// SECURITY DEPOSIT INFORMATION (§ 83.49)
// ============================================================================

#text(size: 12pt, weight: "bold")[Security Deposit]
#v(0.5em)

Pursuant to *Florida Statutes § 83.49*, your security deposit will be handled as follows:

- The landlord must return the security deposit, or provide written notice of a claim against it, within *15 days* (if no claim) or *30 days* (if claim) after you vacate.
- You must provide a forwarding address in writing to receive your deposit or claim notice.
- If you do not object to a claim within 15 days, the landlord may deduct the claimed amount.

#v(0.5em)

*Please provide your forwarding address to:*
#v(0.3em)
#get("landlord_name", default: "[Landlord Name]")
#linebreak()
#get("landlord_address", default: "[Landlord Address]")
#if get("landlord_email") != "" [
  #linebreak()
  Email: #get("landlord_email")
]

#v(1.5em)

// ============================================================================
// MOVE-OUT INSTRUCTIONS
// ============================================================================

#text(size: 12pt, weight: "bold")[Move-Out Instructions]
#v(0.5em)

#list(
  [Return all keys, access cards, and remote controls to the landlord],
  [Remove all personal belongings from the premises],
  [Clean the premises thoroughly],
  [Dispose of all trash and debris],
  [Complete a final walk-through inspection if requested],
  [Provide a forwarding address for security deposit return],
)

#if get("additional_instructions") != "" [
  #v(0.5em)
  *Additional Instructions:*
  #v(0.3em)
  #get("additional_instructions")
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
    *IMPORTANT LEGAL NOTICE:* This notice is served pursuant to Florida Statutes Chapter 83, Part II (Residential Tenancies). If you believe this notice was served improperly or you have questions about your rights, you may wish to consult with an attorney or contact your local legal aid office.

    Failure to comply with this notice may result in legal action to recover possession of the premises.
  ]
]

#v(2em)

// ============================================================================
// SIGNATURES
// ============================================================================

#text(size: 12pt, weight: "bold")[LANDLORD/AGENT]
#v(1em)

#grid(
  columns: (1fr, 1fr),
  gutter: 2em,
  [
    #line(length: 100%, stroke: 0.5pt)
    Signature
    #v(1em)
    #get("landlord_name", default: "[Landlord Name]")
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

I hereby certify that a copy of this notice was served on the tenant(s) named above on #get("service_date", default: "[Service Date]") by:

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
  #sym.ballot.x Posting on the premises (after reasonable attempts at personal delivery)
] else [
  #sym.ballot Posting on the premises (after reasonable attempts at personal delivery)
]

#if service_method == "mail" [
  #sym.ballot.x Mailing by certified mail, return receipt requested
] else [
  #sym.ballot Mailing by certified mail, return receipt requested
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
// DISCLAIMER
// ============================================================================

#align(center)[
  #text(size: 8pt, fill: rgb("#666"))[
    DISCLAIMER: This document was prepared using agentPDF.org, a document preparation service. No attorney-client relationship is created. This is not legal advice. For complex matters, consult a Florida attorney.
  ]
]
