// Florida Notice to Owner Template
// Per Florida Statutes § 713.06 (Preliminary Notice - Construction Lien Law)
// Required for subcontractors/suppliers to preserve lien rights
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
#set text(font: "Liberation Sans", size: 10pt)
#set par(justify: true, leading: 0.65em)

// ============================================================================
// HEADER
// ============================================================================

#align(center)[
  #text(size: 16pt, weight: "bold")[NOTICE TO OWNER]
  #v(0.2em)
  #text(size: 11pt)[Preliminary Notice Under Florida Construction Lien Law]
  #v(0.2em)
  #text(size: 9pt, style: "italic")[Pursuant to Florida Statutes § 713.06]
]

#v(1em)

// ============================================================================
// STATUTORY WARNING TO OWNER
// ============================================================================

#rect(
  width: 100%,
  inset: 10pt,
  stroke: 2pt + rgb("#cc0000"),
  fill: rgb("#fff5f5"),
  radius: 4pt,
)[
  #text(size: 10pt, weight: "bold")[WARNING TO OWNER]
  #v(0.3em)

  #text(size: 9pt)[
    This is a preliminary notice required under Florida law. It notifies the property owner that the undersigned lienor has furnished or will furnish labor, services, or materials for improvements to the owner's property.

    *BEFORE MAKING FINAL PAYMENT*, the owner should:

    #list(
      [Obtain a Contractor's Final Payment Affidavit listing all lienors who have served a Notice to Owner],
      [Obtain lien releases or waivers from all lienors who have served a Notice to Owner],
      [Retain sufficient funds to pay all lienors who have served a Notice to Owner],
    )

    Failure to do so may result in the owner paying twice for the same improvements.
  ]
]

#v(1em)

// ============================================================================
// NOTICE DATE
// ============================================================================

#text(weight: "bold")[Date of Notice:] #get("notice_date", default: "[Date]")

#v(1em)

// ============================================================================
// TO: PROPERTY OWNER
// ============================================================================

#text(size: 11pt, weight: "bold")[TO: PROPERTY OWNER]
#v(0.3em)

#table(
  columns: (1fr, 2fr),
  inset: 6pt,
  stroke: 0.5pt,
  [*Owner Name(s)*], [#get("owner_name", default: "[Owner Name(s)]")],
  [*Owner Address*], [#get("owner_address", default: "[Owner Address]")],
  [*City, State, ZIP*], [#get("owner_city", default: "[City]"), #get("owner_state", default: "FL") #get("owner_zip", default: "[ZIP]")],
)

#v(0.8em)

// ============================================================================
// TO: CONTRACTOR (if different from owner)
// ============================================================================

#if get("contractor_name") != "" [
  #text(size: 11pt, weight: "bold")[TO: GENERAL CONTRACTOR]
  #v(0.3em)

  #table(
    columns: (1fr, 2fr),
    inset: 6pt,
    stroke: 0.5pt,
    [*Contractor Name*], [#get("contractor_name")],
    [*Company*], [#get("contractor_company", default: "")],
    [*Address*], [#get("contractor_address", default: "[Contractor Address]")],
    [*City, State, ZIP*], [#get("contractor_city", default: "[City]"), #get("contractor_state", default: "FL") #get("contractor_zip", default: "[ZIP]")],
  )

  #v(0.8em)
]

// ============================================================================
// PROPERTY INFORMATION
// ============================================================================

#text(size: 11pt, weight: "bold")[PROPERTY DESCRIPTION]
#v(0.3em)

#table(
  columns: (1fr, 2fr),
  inset: 6pt,
  stroke: 0.5pt,
  [*Property Address*], [#get("property_address", default: "[Property Address]")],
  [*City, State, ZIP*], [#get("property_city", default: "[City]"), Florida #get("property_zip", default: "[ZIP]")],
  [*County*], [#get("property_county", default: "[County]")],
  [*Legal Description*], [#get("legal_description", default: "[Legal Description if known]")],
)

#v(1em)

// ============================================================================
// FROM: LIENOR INFORMATION
// ============================================================================

#text(size: 11pt, weight: "bold")[FROM: LIENOR (Person Providing Labor, Services, or Materials)]
#v(0.3em)

#table(
  columns: (1fr, 2fr),
  inset: 6pt,
  stroke: 0.5pt,
  [*Lienor Name*], [#get("lienor_name", default: "[Lienor Name]")],
  [*Company Name*], [#get("lienor_company", default: "[Company Name]")],
  [*Address*], [#get("lienor_address", default: "[Lienor Address]")],
  [*City, State, ZIP*], [#get("lienor_city", default: "[City]"), #get("lienor_state", default: "FL") #get("lienor_zip", default: "[ZIP]")],
  [*Phone*], [#get("lienor_phone", default: "[Phone]")],
  [*Email*], [#get("lienor_email", default: "[Email]")],
  [*License Number*], [#get("lienor_license", default: "[License # if applicable]")],
)

#v(1em)

// ============================================================================
// DESCRIPTION OF SERVICES/MATERIALS
// ============================================================================

#text(size: 11pt, weight: "bold")[DESCRIPTION OF LABOR, SERVICES, OR MATERIALS]
#v(0.3em)

#rect(
  width: 100%,
  inset: 8pt,
  stroke: 0.5pt,
)[
  *General Description:*
  #v(0.2em)
  #get("services_description", default: "[Description of labor, services, or materials to be furnished]")

  #v(0.5em)

  *Furnished Under Contract With:*
  #v(0.2em)
  #get("contracted_with", default: "[Name of person/entity with whom lienor has a contract]")

  #v(0.5em)

  *First Furnishing Date (or anticipated date):*
  #v(0.2em)
  #get("first_furnishing_date", default: "[Date]")
]

#v(1em)

// ============================================================================
// LIENOR TYPE
// ============================================================================

#text(size: 11pt, weight: "bold")[LIENOR CLASSIFICATION]
#v(0.3em)

#let lienor_type = get("lienor_type", default: "subcontractor")

#grid(
  columns: (1fr, 1fr),
  gutter: 1em,
  [
    #if lienor_type == "subcontractor" [#sym.ballot.x] else [#sym.ballot] Subcontractor
    #linebreak()
    #if lienor_type == "sub_subcontractor" [#sym.ballot.x] else [#sym.ballot] Sub-subcontractor
    #linebreak()
    #if lienor_type == "materialman" [#sym.ballot.x] else [#sym.ballot] Materialman / Supplier
  ],
  [
    #if lienor_type == "laborer" [#sym.ballot.x] else [#sym.ballot] Laborer
    #linebreak()
    #if lienor_type == "professional" [#sym.ballot.x] else [#sym.ballot] Design Professional
    #linebreak()
    #if lienor_type == "other" [#sym.ballot.x] else [#sym.ballot] Other: #get("lienor_type_other", default: "")
  ]
)

#v(1em)

// ============================================================================
// AMOUNT INFORMATION
// ============================================================================

#text(size: 11pt, weight: "bold")[CONTRACT/ESTIMATED AMOUNT]
#v(0.3em)

#table(
  columns: (1fr, 1fr),
  inset: 6pt,
  stroke: 0.5pt,
  [*Total Contract Amount*], [$#get("contract_amount", default: "[Amount]")],
  [*Amount Already Paid*], [$#get("amount_paid", default: "0.00")],
  [*Balance Remaining*], [$#get("balance_remaining", default: "[Balance]")],
)

#v(0.5em)

#text(size: 9pt, style: "italic")[
  Note: The amounts stated above are good faith estimates. The actual amount may vary based on the scope of work performed or materials furnished.
]

#v(1em)

// ============================================================================
// NOTICE OF COMMENCEMENT REFERENCE
// ============================================================================

#text(size: 11pt, weight: "bold")[NOTICE OF COMMENCEMENT INFORMATION]
#v(0.3em)

#if get("noc_recording_info") != "" [
  Notice of Commencement recorded in:
  #v(0.2em)
  #get("noc_recording_info", default: "[Book/Page or Instrument Number]")
  #v(0.2em)
  Recording Date: #get("noc_recording_date", default: "[Date]")
] else [
  #rect(
    width: 100%,
    inset: 6pt,
    stroke: 0.5pt,
  )[
    #sym.ballot.x Notice of Commencement recording information not available at time of this notice.
  ]
]

#v(1em)

// ============================================================================
// BOND INFORMATION
// ============================================================================

#if get_bool("has_bond") [
  #text(size: 11pt, weight: "bold")[PAYMENT BOND INFORMATION]
  #v(0.3em)

  #table(
    columns: (1fr, 2fr),
    inset: 6pt,
    stroke: 0.5pt,
    [*Surety Name*], [#get("surety_name", default: "[Surety Company]")],
    [*Bond Amount*], [$#get("bond_amount", default: "[Amount]")],
    [*Bond Number*], [#get("bond_number", default: "[Bond Number]")],
  )

  #v(1em)
]

// ============================================================================
// STATUTORY NOTICE TEXT
// ============================================================================

#rect(
  width: 100%,
  inset: 10pt,
  stroke: 1pt,
  fill: rgb("#f5f5f5"),
  radius: 4pt,
)[
  #text(size: 9pt)[
    *STATUTORY NOTICE:* This is a Notice to Owner pursuant to Section 713.06, Florida Statutes. This notice is given to protect the lienor's rights under the Florida Construction Lien Law.

    Under Florida law, those who furnish labor, services, or materials for construction improvements have lien rights on the real property improved. This notice is not a lien but rather a preliminary step in preserving the lienor's rights to file a claim of lien if payment is not made.

    The property owner is advised to:
    #list(
      [Determine whether the amounts stated are correct],
      [Verify that proper payment is being made to those who have served a Notice to Owner],
      [Consult with an attorney if there are any questions about construction lien law],
    )
  ]
]

#v(1.5em)

// ============================================================================
// SIGNATURE
// ============================================================================

#text(size: 11pt, weight: "bold")[LIENOR SIGNATURE]
#v(0.8em)

#grid(
  columns: (1fr, 1fr),
  gutter: 2em,
  [
    #line(length: 100%, stroke: 0.5pt)
    Signature
    #v(0.6em)
    #get("lienor_name", default: "[Lienor Name]")
    #linebreak()
    Printed Name
    #v(0.6em)
    Title: #get("lienor_title", default: "[Title]")
  ],
  [
    #line(length: 100%, stroke: 0.5pt)
    Date
    #v(0.6em)
    #get("notice_date", default: "[Date]")
  ]
)

#v(1.5em)

// ============================================================================
// CERTIFICATE OF SERVICE
// ============================================================================

#text(size: 11pt, weight: "bold")[CERTIFICATE OF SERVICE]
#v(0.5em)

I hereby certify that a copy of this Notice to Owner was served on #get("service_date", default: "[Date]") by:

#v(0.3em)

#let service_method = get("service_method", default: "certified_mail")

#if service_method == "certified_mail" [
  #sym.ballot.x Certified mail, return receipt requested
] else [
  #sym.ballot Certified mail, return receipt requested
]

#if service_method == "personal" [
  #sym.ballot.x Personal delivery
] else [
  #sym.ballot Personal delivery
]

#if service_method == "registered_mail" [
  #sym.ballot.x Registered mail
] else [
  #sym.ballot Registered mail
]

#v(0.5em)

*Sent to:*
#list(
  [Owner at the address shown above],
  [#if get("contractor_name") != "" [General Contractor at the address shown above] else [N/A - No general contractor]],
)

#if get("service_tracking") != "" [
  #v(0.3em)
  *Tracking/Receipt Number:* #get("service_tracking")
]

#v(1em)

#grid(
  columns: (1fr, 1fr),
  gutter: 2em,
  [
    #line(length: 100%, stroke: 0.5pt)
    Signature of Person Serving Notice
  ],
  [
    #line(length: 100%, stroke: 0.5pt)
    Date
  ]
)

#v(1.5em)

// ============================================================================
// TIMING REMINDER
// ============================================================================

#rect(
  width: 100%,
  inset: 8pt,
  stroke: 1pt + rgb("#ff6600"),
  fill: rgb("#fff8f0"),
  radius: 4pt,
)[
  #text(size: 9pt, weight: "bold")[IMPORTANT TIMING REQUIREMENTS]
  #v(0.3em)
  #text(size: 9pt)[
    Under Florida Statutes § 713.06, this Notice to Owner must be served:
    #list(
      [*Before* commencing to furnish labor, services, or materials, OR],
      [*Within 45 days* after commencing to furnish labor, services, or materials],
    )
    Failure to serve this notice within the required timeframe may result in loss of lien rights for amounts accrued more than 45 days before service of notice.
  ]
]

#v(1em)

// ============================================================================
// DISCLAIMER
// ============================================================================

#align(center)[
  #text(size: 7pt, fill: rgb("#666"))[
    DISCLAIMER: This document was prepared using agentPDF.org. This is not legal advice. Construction lien law is complex. Consult a Florida attorney for specific legal guidance and to ensure proper compliance with all statutory requirements.
  ]
]
