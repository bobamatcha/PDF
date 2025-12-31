// Florida Release/Waiver of Lien Template
// Per Florida Statutes § 713.20 and § 713.21 (Construction Lien Law)
// Releases lien rights upon payment
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
// RECORDING HEADER (if release is for recorded lien)
// ============================================================================

#if get_bool("for_recorded_lien") [
  #rect(
    width: 100%,
    height: 2in,
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
        #get("return_name", default: "[Name]")
        #linebreak()
        #get("return_address", default: "[Address]")
        #linebreak()
        #get("return_city", default: "[City]"), #get("return_state", default: "FL") #get("return_zip", default: "[ZIP]")
      ],
      [
        *Parcel ID:* #get("parcel_id", default: "[Parcel ID]")
        #v(0.5em)
        *Original Lien Recorded:*
        #linebreak()
        Book: #get("lien_book", default: "") Page: #get("lien_page", default: "")
        #linebreak()
        Instrument #: #get("lien_instrument", default: "")
      ]
    )
  ]

  #v(1em)
]

// ============================================================================
// HEADER
// ============================================================================

#let release_type = get("release_type", default: "final_unconditional")

#align(center)[
  #if release_type == "partial_conditional" [
    #text(size: 16pt, weight: "bold")[PARTIAL CONDITIONAL WAIVER AND RELEASE OF LIEN]
  ] else if release_type == "partial_unconditional" [
    #text(size: 16pt, weight: "bold")[PARTIAL UNCONDITIONAL WAIVER AND RELEASE OF LIEN]
  ] else if release_type == "final_conditional" [
    #text(size: 16pt, weight: "bold")[FINAL CONDITIONAL WAIVER AND RELEASE OF LIEN]
  ] else [
    #text(size: 16pt, weight: "bold")[FINAL UNCONDITIONAL WAIVER AND RELEASE OF LIEN]
  ]
  #v(0.2em)
  #text(size: 11pt)[State of Florida]
  #v(0.2em)
  #text(size: 9pt, style: "italic")[Pursuant to Florida Statutes § 713.20]
]

#v(1em)

// ============================================================================
// PROPERTY INFORMATION
// ============================================================================

#text(size: 11pt, weight: "bold")[PROPERTY INFORMATION]
#v(0.3em)

#table(
  columns: (1fr, 2fr),
  inset: 6pt,
  stroke: 0.5pt,
  [*Property Address*], [#get("property_address", default: "[Property Address]")],
  [*City, State, ZIP*], [#get("property_city", default: "[City]"), Florida #get("property_zip", default: "[ZIP]")],
  [*County*], [#get("property_county", default: "[County]")],
  [*Legal Description*], [#get("legal_description", default: "[Legal Description]")],
)

#v(0.8em)

// ============================================================================
// OWNER INFORMATION
// ============================================================================

#text(size: 11pt, weight: "bold")[PROPERTY OWNER]
#v(0.3em)

#table(
  columns: (1fr, 2fr),
  inset: 6pt,
  stroke: 0.5pt,
  [*Owner Name(s)*], [#get("owner_name", default: "[Owner Name(s)]")],
  [*Owner Address*], [#get("owner_address", default: "[Owner Address]")],
)

#v(0.8em)

// ============================================================================
// LIENOR/CLAIMANT INFORMATION
// ============================================================================

#text(size: 11pt, weight: "bold")[LIENOR / CLAIMANT]
#v(0.3em)

#table(
  columns: (1fr, 2fr),
  inset: 6pt,
  stroke: 0.5pt,
  [*Lienor Name*], [#get("lienor_name", default: "[Lienor Name]")],
  [*Company Name*], [#get("lienor_company", default: "[Company Name if applicable]")],
  [*Address*], [#get("lienor_address", default: "[Lienor Address]")],
  [*City, State, ZIP*], [#get("lienor_city", default: "[City]"), #get("lienor_state", default: "FL") #get("lienor_zip", default: "[ZIP]")],
)

#v(1em)

// ============================================================================
// RELEASE TYPE-SPECIFIC CONTENT
// ============================================================================

#if release_type == "partial_conditional" [
  // PARTIAL CONDITIONAL
  #rect(
    width: 100%,
    inset: 12pt,
    stroke: 2pt + rgb("#ff6600"),
    fill: rgb("#fff8f0"),
    radius: 4pt,
  )[
    #text(size: 11pt, weight: "bold")[PARTIAL CONDITIONAL WAIVER AND RELEASE]
    #v(0.5em)

    Upon receipt of the payment described below, the undersigned waives and releases lien and bond rights *only to the extent of the payment received*.

    #v(0.5em)

    #table(
      columns: (1fr, 1fr),
      inset: 8pt,
      stroke: 0.5pt,
      [*Payment Amount*], [$ #get("payment_amount", default: "[Amount]")],
      [*For Work Through Date*], [#get("through_date", default: "[Date]")],
      [*Check/Payment Number*], [#get("payment_number", default: "[Check #]")],
    )

    #v(0.5em)

    #text(weight: "bold")[CONDITIONAL UPON:]
    This waiver is conditioned upon actual receipt of payment. If payment is not received, or if a check is dishonored, this release shall be void and of no effect.

    #v(0.5em)

    *EXCEPTIONS:* The following items are specifically excluded from this release:

    #rect(
      width: 100%,
      inset: 6pt,
      stroke: 0.5pt,
    )[
      #get("exceptions", default: "[List any exceptions or write 'None']")
    ]
  ]

] else if release_type == "partial_unconditional" [
  // PARTIAL UNCONDITIONAL
  #rect(
    width: 100%,
    inset: 12pt,
    stroke: 2pt + rgb("#0066cc"),
    fill: rgb("#f0f8ff"),
    radius: 4pt,
  )[
    #text(size: 11pt, weight: "bold")[PARTIAL UNCONDITIONAL WAIVER AND RELEASE]
    #v(0.5em)

    The undersigned has received payment and hereby waives and releases lien and bond rights *to the extent of the payment received*.

    #v(0.5em)

    #table(
      columns: (1fr, 1fr),
      inset: 8pt,
      stroke: 0.5pt,
      [*Payment Received*], [$ #get("payment_amount", default: "[Amount]")],
      [*For Work Through Date*], [#get("through_date", default: "[Date]")],
      [*Date Payment Received*], [#get("payment_date", default: "[Date]")],
    )

    #v(0.5em)

    This release covers all labor, services, equipment, and materials furnished through the date shown above.

    #v(0.5em)

    *EXCEPTIONS:* The following items are specifically excluded from this release:

    #rect(
      width: 100%,
      inset: 6pt,
      stroke: 0.5pt,
    )[
      #get("exceptions", default: "[List any exceptions or write 'None']")
    ]
  ]

] else if release_type == "final_conditional" [
  // FINAL CONDITIONAL
  #rect(
    width: 100%,
    inset: 12pt,
    stroke: 2pt + rgb("#ff6600"),
    fill: rgb("#fff8f0"),
    radius: 4pt,
  )[
    #text(size: 11pt, weight: "bold")[FINAL CONDITIONAL WAIVER AND RELEASE]
    #v(0.5em)

    Upon receipt of the final payment described below, the undersigned waives and releases *ALL* lien and bond rights related to the property.

    #v(0.5em)

    #table(
      columns: (1fr, 1fr),
      inset: 8pt,
      stroke: 0.5pt,
      [*Final Payment Amount*], [$ #get("payment_amount", default: "[Amount]")],
      [*Total Contract Amount*], [$ #get("contract_amount", default: "[Amount]")],
      [*Check/Payment Number*], [#get("payment_number", default: "[Check #]")],
    )

    #v(0.5em)

    #text(weight: "bold")[CONDITIONAL UPON:]
    This waiver is conditioned upon actual receipt of the final payment. If payment is not received, or if a check is dishonored, this release shall be void and of no effect, and all lien rights shall remain in full force.
  ]

] else [
  // FINAL UNCONDITIONAL (default)
  #rect(
    width: 100%,
    inset: 12pt,
    stroke: 2pt + rgb("#00cc00"),
    fill: rgb("#f0fff0"),
    radius: 4pt,
  )[
    #text(size: 11pt, weight: "bold")[FINAL UNCONDITIONAL WAIVER AND RELEASE]
    #v(0.5em)

    The undersigned has been paid in full and hereby *unconditionally and irrevocably* waives and releases *ALL* lien and bond rights related to the property described above.

    #v(0.5em)

    #table(
      columns: (1fr, 1fr),
      inset: 8pt,
      stroke: 0.5pt,
      [*Final Payment Received*], [$ #get("payment_amount", default: "[Amount]")],
      [*Total Contract Amount*], [$ #get("contract_amount", default: "[Amount]")],
      [*Date of Final Payment*], [#get("payment_date", default: "[Date]")],
    )

    #v(0.5em)

    This release covers *all* labor, services, equipment, and materials furnished to the above-described property. The undersigned certifies that all laborers, subcontractors, and material suppliers have been paid in full.
  ]
]

#v(1em)

// ============================================================================
// RELEASE OF RECORDED LIEN (if applicable)
// ============================================================================

#if get_bool("for_recorded_lien") [
  #text(size: 11pt, weight: "bold")[RELEASE OF RECORDED CLAIM OF LIEN]
  #v(0.3em)

  The undersigned hereby releases the Claim of Lien recorded in the Official Records of #get("property_county", default: "[County]") County, Florida:

  #v(0.3em)

  #table(
    columns: (1fr, 1fr, 1fr),
    inset: 6pt,
    stroke: 0.5pt,
    [*Recording Date*], [*Book/Page*], [*Instrument Number*],
    [#get("lien_recording_date", default: "[Date]")], [#get("lien_book", default: "") / #get("lien_page", default: "")], [#get("lien_instrument", default: "[Instrument #]")],
  )

  #v(0.3em)

  The lien claimed therein is hereby *fully satisfied and released*.

  #v(1em)
]

// ============================================================================
// CERTIFICATIONS
// ============================================================================

#text(size: 11pt, weight: "bold")[CERTIFICATIONS]
#v(0.3em)

The undersigned certifies that:

#list(
  [The undersigned is authorized to execute this release on behalf of the lienor],
  [All statements made herein are true and correct],
  #if release_type == "final_unconditional" or release_type == "final_conditional" [
    [All laborers, subcontractors, and material suppliers have been or will be paid from the funds received]
  ] else [
    [The work covered by this release has been satisfactorily completed]
  ],
)

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
    #get("release_date", default: "[Date]")
  ]
)

#v(1.5em)

// ============================================================================
// NOTARY ACKNOWLEDGMENT (required for release of recorded lien)
// ============================================================================

#if get_bool("for_recorded_lien") or get_bool("include_notary") [
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
]

// ============================================================================
// IMPORTANT NOTICE
// ============================================================================

#rect(
  width: 100%,
  inset: 8pt,
  stroke: 1pt,
  fill: rgb("#f5f5f5"),
  radius: 4pt,
)[
  #text(size: 8pt)[
    *NOTICE:* This document affects legal rights. Under Florida law (§ 713.20), a waiver and release of lien form must substantially follow the statutory form. This release is binding and may not be rescinded.

    #if release_type == "final_conditional" or release_type == "partial_conditional" [
      *CONDITIONAL RELEASES:* This is a CONDITIONAL release. It is effective *only* upon actual receipt of payment in good funds. Dishonored checks do not constitute payment.
    ]
  ]
]

#v(1em)

// ============================================================================
// DISCLAIMER
// ============================================================================

#align(center)[
  #text(size: 7pt, fill: rgb("#666"))[
    DISCLAIMER: This document was prepared using agentPDF.org. This is not legal advice. Lien releases have significant legal consequences. Consult a Florida attorney if you have questions about your rights.
  ]
]
