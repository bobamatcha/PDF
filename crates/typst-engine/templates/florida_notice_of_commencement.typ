// Florida Notice of Commencement Template
// Per Florida Statutes § 713.13 (Construction Lien Law)
// Required before starting construction work
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
  margin: (top: 0.75in, bottom: 0.75in, left: 1in, right: 1in),
)
#set text(font: "Liberation Sans", size: 10pt)
#set par(justify: true, leading: 0.65em)

// ============================================================================
// HEADER
// ============================================================================

#align(center)[
  #text(size: 16pt, weight: "bold")[NOTICE OF COMMENCEMENT]
  #v(0.2em)
  #text(size: 11pt)[State of Florida]
  #v(0.2em)
  #text(size: 9pt, style: "italic")[Pursuant to Florida Statutes § 713.13]
]

#v(0.8em)

#rect(
  width: 100%,
  inset: 8pt,
  stroke: 1pt + rgb("#cc0000"),
  fill: rgb("#fff5f5"),
  radius: 4pt,
)[
  #text(size: 9pt, weight: "bold")[
    WARNING TO OWNER: ANY PAYMENTS MADE BY THE OWNER AFTER THE RECORDING OF THIS NOTICE OF COMMENCEMENT ARE CONSIDERED IMPROPER PAYMENTS UNDER CHAPTER 713, PART I, FLORIDA STATUTES, AND CAN RESULT IN YOUR PAYING TWICE FOR IMPROVEMENTS TO YOUR PROPERTY. A COPY OF THIS NOTICE WITH ALL STATEMENTS COMPLETED MUST BE POSTED AT THE JOB SITE BEFORE THE FIRST INSPECTION.
  ]
]

#v(1em)

// ============================================================================
// PROPERTY INFORMATION
// ============================================================================

#text(size: 11pt, weight: "bold")[1. PROPERTY DESCRIPTION]
#v(0.3em)

#table(
  columns: (1fr, 2fr),
  inset: 6pt,
  stroke: 0.5pt,
  [*Street Address*], [#get("property_address", default: "[Property Address]")],
  [*City, State, ZIP*], [#get("property_city", default: "[City]"), Florida #get("property_zip", default: "[ZIP]")],
  [*County*], [#get("property_county", default: "[County]")],
  [*Legal Description*], [#get("legal_description", default: "[Legal Description - Lot, Block, Subdivision, etc.]")],
  [*Parcel ID / Folio #*], [#get("parcel_id", default: "[Parcel ID]")],
)

#v(0.8em)

// ============================================================================
// OWNER INFORMATION
// ============================================================================

#text(size: 11pt, weight: "bold")[2. OWNER INFORMATION]
#v(0.3em)

#table(
  columns: (1fr, 2fr),
  inset: 6pt,
  stroke: 0.5pt,
  [*Owner Name(s)*], [#get("owner_name", default: "[Owner Name(s)]")],
  [*Owner Address*], [#get("owner_address", default: "[Owner Address]")],
  [*City, State, ZIP*], [#get("owner_city", default: "[City]"), #get("owner_state", default: "FL") #get("owner_zip", default: "[ZIP]")],
  [*Phone*], [#get("owner_phone", default: "[Phone]")],
  [*Email*], [#get("owner_email", default: "[Email]")],
)

#v(0.5em)

*Interest in Property:*
#let owner_interest = get("owner_interest", default: "fee_simple")
#if owner_interest == "fee_simple" [Fee Simple Owner]
else if owner_interest == "lessee" [Lessee (Tenant)]
else if owner_interest == "contract_buyer" [Contract Buyer]
else [#owner_interest]

#v(0.8em)

// ============================================================================
// GENERAL CONTRACTOR INFORMATION
// ============================================================================

#text(size: 11pt, weight: "bold")[3. GENERAL CONTRACTOR / DIRECT CONTRACTOR]
#v(0.3em)

#table(
  columns: (1fr, 2fr),
  inset: 6pt,
  stroke: 0.5pt,
  [*Contractor Name*], [#get("contractor_name", default: "[Contractor Name]")],
  [*Company Name*], [#get("contractor_company", default: "[Company Name]")],
  [*Address*], [#get("contractor_address", default: "[Contractor Address]")],
  [*City, State, ZIP*], [#get("contractor_city", default: "[City]"), #get("contractor_state", default: "FL") #get("contractor_zip", default: "[ZIP]")],
  [*Phone*], [#get("contractor_phone", default: "[Phone]")],
  [*Email*], [#get("contractor_email", default: "[Email]")],
  [*License Number*], [#get("contractor_license", default: "[License #]")],
)

#v(0.8em)

// ============================================================================
// SURETY/BOND INFORMATION
// ============================================================================

#text(size: 11pt, weight: "bold")[4. SURETY / PAYMENT BOND INFORMATION (if applicable)]
#v(0.3em)

#if get_bool("has_bond") [
  #table(
    columns: (1fr, 2fr),
    inset: 6pt,
    stroke: 0.5pt,
    [*Surety Company*], [#get("surety_name", default: "[Surety Company Name]")],
    [*Surety Address*], [#get("surety_address", default: "[Surety Address]")],
    [*City, State, ZIP*], [#get("surety_city", default: "[City]"), #get("surety_state", default: "[State]") #get("surety_zip", default: "[ZIP]")],
    [*Bond Amount*], [$#get("bond_amount", default: "[Amount]")],
    [*Bond Number*], [#get("bond_number", default: "[Bond Number]")],
  )
] else [
  #rect(
    width: 100%,
    inset: 6pt,
    stroke: 0.5pt,
  )[
    #sym.ballot.x No payment bond has been recorded for this project.
  ]
]

#v(0.8em)

// ============================================================================
// LENDER INFORMATION
// ============================================================================

#text(size: 11pt, weight: "bold")[5. CONSTRUCTION LENDER (if any)]
#v(0.3em)

#if get("lender_name") != "" [
  #table(
    columns: (1fr, 2fr),
    inset: 6pt,
    stroke: 0.5pt,
    [*Lender Name*], [#get("lender_name")],
    [*Lender Address*], [#get("lender_address", default: "[Lender Address]")],
    [*City, State, ZIP*], [#get("lender_city", default: "[City]"), #get("lender_state", default: "[State]") #get("lender_zip", default: "[ZIP]")],
  )
] else [
  #rect(
    width: 100%,
    inset: 6pt,
    stroke: 0.5pt,
  )[
    #sym.ballot.x No construction lender for this project.
  ]
]

#v(0.8em)

// ============================================================================
// DESIGNATED AGENT FOR SERVICE (if any)
// ============================================================================

#text(size: 11pt, weight: "bold")[6. PERSON DESIGNATED TO RECEIVE NOTICES (other than owner)]
#v(0.3em)

#if get("agent_name") != "" [
  #table(
    columns: (1fr, 2fr),
    inset: 6pt,
    stroke: 0.5pt,
    [*Name*], [#get("agent_name")],
    [*Address*], [#get("agent_address", default: "[Agent Address]")],
    [*City, State, ZIP*], [#get("agent_city", default: "[City]"), #get("agent_state", default: "FL") #get("agent_zip", default: "[ZIP]")],
    [*Phone*], [#get("agent_phone", default: "[Phone]")],
  )
] else [
  #rect(
    width: 100%,
    inset: 6pt,
    stroke: 0.5pt,
  )[
    #sym.ballot.x No designated agent. Owner shall receive all notices.
  ]
]

#v(0.8em)

// ============================================================================
// PROJECT INFORMATION
// ============================================================================

#text(size: 11pt, weight: "bold")[7. PROJECT INFORMATION]
#v(0.3em)

#table(
  columns: (1fr, 2fr),
  inset: 6pt,
  stroke: 0.5pt,
  [*Description of Work*], [#get("project_description", default: "[Description of improvements to be made]")],
  [*Commencement Date*], [#get("commencement_date", default: "[Date work will begin]")],
  [*Estimated Completion*], [#get("estimated_completion", default: "[Estimated completion date]")],
  [*Contract Amount*], [$#get("contract_amount", default: "[Contract Amount]")],
)

#v(0.8em)

// ============================================================================
// EXPIRATION
// ============================================================================

#text(size: 11pt, weight: "bold")[8. EXPIRATION OF NOTICE]
#v(0.3em)

This Notice of Commencement expires *#get("expiration_date", default: "[One year from recording date or specified date]")*, unless an amended notice is recorded extending the expiration date.

#v(0.5em)

#text(size: 9pt, style: "italic")[
  Note: Under § 713.13(6), a Notice of Commencement is effective for one year from the date of recording, or until the date stated in this notice, whichever is earlier.
]

#v(1em)

// ============================================================================
// OWNER'S CERTIFICATION
// ============================================================================

#rect(
  width: 100%,
  inset: 10pt,
  stroke: 1pt,
  radius: 4pt,
)[
  #text(size: 10pt, weight: "bold")[OWNER'S CERTIFICATION]
  #v(0.5em)

  I hereby certify that the foregoing is a true and accurate Notice of Commencement for the improvement described above, and I understand that:

  #list(
    [I should not make any payments until I receive lien waivers or releases from all lienors who have served a Notice to Owner],
    [I may make proper payments to the contractor for work performed if no Notice to Owner has been received],
    [I must post a copy of this Notice of Commencement at the job site before the first inspection],
  )
]

#v(1em)

// ============================================================================
// SIGNATURES
// ============================================================================

#text(size: 11pt, weight: "bold")[OWNER SIGNATURE]
#v(0.8em)

#grid(
  columns: (1fr, 1fr),
  gutter: 2em,
  [
    #line(length: 100%, stroke: 0.5pt)
    Owner Signature
    #v(0.6em)
    #get("owner_name", default: "[Owner Name]")
    #linebreak()
    Printed Name
    #v(0.6em)
    Date: #box(width: 1.2in)[#line(length: 100%, stroke: 0.5pt)]
  ],
  [
    #if get("owner2_name") != "" [
      #line(length: 100%, stroke: 0.5pt)
      Owner Signature
      #v(0.6em)
      #get("owner2_name")
      #linebreak()
      Printed Name
      #v(0.6em)
      Date: #box(width: 1.2in)[#line(length: 100%, stroke: 0.5pt)]
    ]
  ]
)

#v(1.2em)

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
  by #get("owner_name", default: "[Owner Name(s)]"),
  who is/are personally known to me or who has/have produced #box(width: 1.5in)[#line(length: 100%, stroke: 0.5pt)] as identification.

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
// RECORDING INFORMATION
// ============================================================================

#text(size: 10pt, weight: "bold")[FOR CLERK'S USE ONLY - RECORDING INFORMATION]
#v(0.3em)

#table(
  columns: (1fr, 1fr, 1fr),
  inset: 6pt,
  stroke: 0.5pt,
  [*Book:*], [*Page:*], [*Instrument #:*],
  [], [], [],
)

#v(1em)

// ============================================================================
// DISCLAIMER
// ============================================================================

#align(center)[
  #text(size: 7pt, fill: rgb("#666"))[
    DISCLAIMER: This document was prepared using agentPDF.org. This is not legal advice. The Notice of Commencement must be recorded in the official records of the county where the property is located BEFORE construction work begins. Consult a Florida attorney for specific legal guidance.
  ]
]
