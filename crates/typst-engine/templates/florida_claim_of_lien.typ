// Florida Claim of Lien Template
// Per Florida Statutes § 713.08 (Construction Lien Law)
// Must be recorded in Official Records of the county
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
// RECORDING HEADER
// ============================================================================

#rect(
  width: 100%,
  height: 2.5in,
  inset: 10pt,
  stroke: 1pt,
)[
  #text(size: 9pt, weight: "bold")[THIS SPACE RESERVED FOR RECORDING DATA]

  #v(0.3em)

  #grid(
    columns: (1fr, 1fr),
    gutter: 1em,
    [
      *Return to:*
      #v(0.2em)
      #get("return_name", default: "[Lienor Name]")
      #linebreak()
      #get("return_address", default: "[Address]")
      #linebreak()
      #get("return_city", default: "[City]"), #get("return_state", default: "FL") #get("return_zip", default: "[ZIP]")
    ],
    [
      *Property Appraiser's Parcel ID:*
      #v(0.2em)
      #get("parcel_id", default: "[Parcel ID / Folio Number]")
    ]
  )
]

#v(1em)

// ============================================================================
// HEADER
// ============================================================================

#align(center)[
  #text(size: 18pt, weight: "bold")[CLAIM OF LIEN]
  #v(0.2em)
  #text(size: 11pt)[State of Florida]
  #v(0.2em)
  #text(size: 9pt, style: "italic")[Pursuant to Florida Statutes § 713.08]
]

#v(1em)

// ============================================================================
// LIENOR INFORMATION
// ============================================================================

#text(size: 11pt, weight: "bold")[1. LIENOR (Claimant)]
#v(0.3em)

#table(
  columns: (1fr, 2fr),
  inset: 6pt,
  stroke: 0.5pt,
  [*Lienor Name*], [#get("lienor_name", default: "[Lienor Name]")],
  [*Company Name*], [#get("lienor_company", default: "[Company/Business Name if applicable]")],
  [*Address*], [#get("lienor_address", default: "[Lienor Address]")],
  [*City, State, ZIP*], [#get("lienor_city", default: "[City]"), #get("lienor_state", default: "FL") #get("lienor_zip", default: "[ZIP]")],
  [*Phone*], [#get("lienor_phone", default: "[Phone]")],
)

#v(0.8em)

// ============================================================================
// PROPERTY INFORMATION
// ============================================================================

#text(size: 11pt, weight: "bold")[2. PROPERTY DESCRIPTION]
#v(0.3em)

#table(
  columns: (1fr, 2fr),
  inset: 6pt,
  stroke: 0.5pt,
  [*Street Address*], [#get("property_address", default: "[Property Address]")],
  [*City, State, ZIP*], [#get("property_city", default: "[City]"), Florida #get("property_zip", default: "[ZIP]")],
  [*County*], [#get("property_county", default: "[County]")],
)

#v(0.5em)

*Legal Description:*
#v(0.2em)
#rect(
  width: 100%,
  inset: 8pt,
  stroke: 0.5pt,
)[
  #get("legal_description", default: "[Complete legal description of property - Lot, Block, Subdivision, Plat Book, Page, etc.]")
]

#v(0.8em)

// ============================================================================
// PROPERTY OWNER INFORMATION
// ============================================================================

#text(size: 11pt, weight: "bold")[3. PROPERTY OWNER]
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
// GENERAL CONTRACTOR / PERSON WITH WHOM CONTRACT MADE
// ============================================================================

#text(size: 11pt, weight: "bold")[4. CONTRACTOR / PERSON WITH WHOM CONTRACT WAS MADE]
#v(0.3em)

#table(
  columns: (1fr, 2fr),
  inset: 6pt,
  stroke: 0.5pt,
  [*Name*], [#get("contractor_name", default: "[General Contractor or person with whom lienor contracted]")],
  [*Company*], [#get("contractor_company", default: "[Company Name if applicable]")],
  [*Address*], [#get("contractor_address", default: "[Address]")],
  [*City, State, ZIP*], [#get("contractor_city", default: "[City]"), #get("contractor_state", default: "FL") #get("contractor_zip", default: "[ZIP]")],
)

#v(0.8em)

// ============================================================================
// DESCRIPTION OF SERVICES/MATERIALS
// ============================================================================

#text(size: 11pt, weight: "bold")[5. DESCRIPTION OF LABOR, SERVICES, OR MATERIALS FURNISHED]
#v(0.3em)

#rect(
  width: 100%,
  inset: 8pt,
  stroke: 0.5pt,
)[
  #get("services_description", default: "[Detailed description of labor, services, or materials furnished for the improvement of the real property]")
]

#v(0.8em)

// ============================================================================
// FURNISHING DATES
// ============================================================================

#text(size: 11pt, weight: "bold")[6. DATES OF FURNISHING]
#v(0.3em)

#table(
  columns: (1fr, 1fr),
  inset: 6pt,
  stroke: 0.5pt,
  [*First Date of Furnishing*], [#get("first_furnishing_date", default: "[Date]")],
  [*Last Date of Furnishing*], [#get("last_furnishing_date", default: "[Date]")],
)

#v(0.8em)

// ============================================================================
// AMOUNT CLAIMED
// ============================================================================

#text(size: 11pt, weight: "bold")[7. AMOUNT OF LIEN CLAIMED]
#v(0.3em)

#rect(
  width: 100%,
  inset: 10pt,
  stroke: 2pt + rgb("#0066cc"),
  fill: rgb("#f0f8ff"),
  radius: 4pt,
)[
  #table(
    columns: (2fr, 1fr),
    inset: 8pt,
    stroke: 0.5pt,
    [*Total Contract Price / Value of Labor, Services, or Materials*], [$ #get("contract_amount", default: "[Amount]")],
    [*Less: Payments Received*], [$ #get("payments_received", default: "0.00")],
    [*Less: Credits (if any)*], [$ #get("credits", default: "0.00")],
    table.hline(stroke: 1pt),
    [*TOTAL AMOUNT CLAIMED DUE*], [*$ #get("lien_amount", default: "[Total Lien Amount]")*],
  )

  #v(0.5em)
  #text(weight: "bold")[Amount in Words:] #get("lien_amount_words", default: "[Amount in words]") DOLLARS
]

#v(0.8em)

// ============================================================================
// NOTICE TO OWNER SERVED
// ============================================================================

#text(size: 11pt, weight: "bold")[8. NOTICE TO OWNER]
#v(0.3em)

#if get_bool("nto_served") [
  #sym.ballot.x Notice to Owner was served on: #get("nto_date", default: "[Date]")
  #v(0.2em)
  Method of service: #get("nto_method", default: "Certified mail")
] else if get_bool("nto_not_required") [
  #sym.ballot.x Notice to Owner was not required (Lienor contracted directly with Owner)
] else [
  #sym.ballot Notice to Owner status: #get("nto_status", default: "[Status]")
]

#v(0.8em)

// ============================================================================
// NOTICE OF COMMENCEMENT
// ============================================================================

#text(size: 11pt, weight: "bold")[9. NOTICE OF COMMENCEMENT]
#v(0.3em)

#if get("noc_recording_info") != "" [
  Recorded in Official Records of #get("property_county", default: "[County]") County, Florida
  #v(0.2em)
  #table(
    columns: (1fr, 1fr, 1fr),
    inset: 6pt,
    stroke: 0.5pt,
    [*Book*], [*Page*], [*Instrument #*],
    [#get("noc_book", default: "")], [#get("noc_page", default: "")], [#get("noc_instrument", default: "")],
  )
] else [
  #sym.ballot.x No Notice of Commencement was recorded, OR recording information is unknown.
]

#v(0.8em)

// ============================================================================
// BOND INFORMATION
// ============================================================================

#if get_bool("has_bond") [
  #text(size: 11pt, weight: "bold")[10. PAYMENT BOND]
  #v(0.3em)

  A payment bond was recorded for this project:
  #v(0.2em)
  #table(
    columns: (1fr, 2fr),
    inset: 6pt,
    stroke: 0.5pt,
    [*Surety*], [#get("surety_name", default: "[Surety Company]")],
    [*Bond Amount*], [$ #get("bond_amount", default: "[Amount]")],
    [*Bond Number*], [#get("bond_number", default: "[Bond Number]")],
  )

  #v(0.8em)
]

// ============================================================================
// LIEN CLAIM STATEMENT
// ============================================================================

#rect(
  width: 100%,
  inset: 10pt,
  stroke: 1pt,
  fill: rgb("#f5f5f5"),
  radius: 4pt,
)[
  #text(size: 10pt)[
    The undersigned, #get("lienor_name", default: "[Lienor Name]"), pursuant to Chapter 713, Florida Statutes, hereby claims a lien against the above-described real property for labor, services, or materials furnished for the improvement thereof.

    The undersigned affirms that the statements made herein are true and correct to the best of the lienor's knowledge and belief.

    *This claim of lien secures the amount of #get("lien_amount", default: "$[Amount]")*, plus interest, costs, and reasonable attorney's fees as may be allowed by law.
  ]
]

#v(1em)

// ============================================================================
// SIGNATURE
// ============================================================================

#text(size: 11pt, weight: "bold")[LIENOR'S SIGNATURE]
#v(0.8em)

#grid(
  columns: (1fr, 1fr),
  gutter: 2em,
  [
    #line(length: 100%, stroke: 0.5pt)
    Signature of Lienor (or authorized representative)
    #v(0.6em)
    #get("lienor_name", default: "[Lienor Name]")
    #linebreak()
    Printed Name
    #v(0.6em)
    Title: #get("lienor_title", default: "[Title if signing for company]")
  ],
  [
    #line(length: 100%, stroke: 0.5pt)
    Date
    #v(0.6em)
    #get("claim_date", default: "[Date]")
  ]
)

#v(1.5em)

// ============================================================================
// NOTARY ACKNOWLEDGMENT
// ============================================================================

#rect(
  width: 100%,
  inset: 10pt,
  stroke: 1pt,
  fill: rgb("#f9f9f9"),
  radius: 4pt,
)[
  #text(size: 10pt, weight: "bold")[NOTARY ACKNOWLEDGMENT]
  #v(0.5em)

  STATE OF FLORIDA
  #linebreak()
  COUNTY OF #get("notary_county", default: "[County]")

  #v(0.5em)

  The foregoing instrument was acknowledged before me by means of
  #sym.ballot physical presence or #sym.ballot online notarization
  this #box(width: 0.5in)[#line(length: 100%, stroke: 0.5pt)] day of #box(width: 1.2in)[#line(length: 100%, stroke: 0.5pt)], 20#box(width: 0.3in)[#line(length: 100%, stroke: 0.5pt)],
  by #get("lienor_name", default: "[Lienor Name]"),
  who is personally known to me or who has produced #box(width: 1.5in)[#line(length: 100%, stroke: 0.5pt)] as identification.

  #v(1em)

  #grid(
    columns: (1fr, 1fr),
    gutter: 2em,
    [
      #line(length: 100%, stroke: 0.5pt)
      Notary Public Signature
      #v(0.5em)
      Printed Name: #box(width: 1.5in)[#line(length: 100%, stroke: 0.5pt)]
    ],
    [
      My Commission Expires:
      #v(0.3em)
      #box(width: 1.5in)[#line(length: 100%, stroke: 0.5pt)]
      #v(0.3em)
      Commission No.: #box(width: 1in)[#line(length: 100%, stroke: 0.5pt)]
    ]
  )

  #v(0.5em)
  #align(center)[
    #text(size: 8pt)[(NOTARY SEAL)]
  ]
]

#v(1em)

// ============================================================================
// IMPORTANT NOTICES
// ============================================================================

#rect(
  width: 100%,
  inset: 10pt,
  stroke: 1pt + rgb("#cc0000"),
  fill: rgb("#fff5f5"),
  radius: 4pt,
)[
  #text(size: 9pt, weight: "bold")[IMPORTANT NOTICES AND DEADLINES]
  #v(0.3em)
  #text(size: 8pt)[
    *FILING DEADLINE:* Under § 713.08, this Claim of Lien must be recorded:
    #list(
      [Within *90 days* after the final furnishing of labor, services, or materials by the lienor, OR],
      [If a Notice of Termination has been recorded, within the time stated in the statute],
    )

    *SERVICE REQUIREMENT:* The lienor must serve a copy of this Claim of Lien on the owner within *15 days* after recording.

    *LAWSUIT DEADLINE:* Under § 713.22, an action to enforce this lien must be commenced within *1 year* after recording of the Claim of Lien.

    *FRAUDULENT LIEN WARNING:* Under § 713.31, filing a fraudulent lien is a crime and may subject the lienor to civil liability for damages including attorney's fees.
  ]
]

#v(1em)

// ============================================================================
// DISCLAIMER
// ============================================================================

#align(center)[
  #text(size: 7pt, fill: rgb("#666"))[
    DISCLAIMER: This document was prepared using agentPDF.org. This is not legal advice. Construction lien law has strict deadlines and requirements. Consult a Florida attorney before recording a Claim of Lien.
  ]
]
