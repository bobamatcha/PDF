// Florida Personal Watercraft (Jet Ski/PWC) Bill of Sale Template
// Per Florida Statutes Chapter 328 (Vessel Registration)
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
  #text(size: 18pt, weight: "bold")[PERSONAL WATERCRAFT BILL OF SALE]
  #v(0.2em)
  #text(size: 11pt)[Jet Ski / PWC / WaveRunner]
  #v(0.2em)
  #text(size: 11pt)[State of Florida]
  #v(0.2em)
  #text(size: 9pt, style: "italic")[Pursuant to Florida Statutes Chapter 328]
]

#v(1em)

// ============================================================================
// TRANSACTION DATE
// ============================================================================

#text(weight: "bold")[Date of Sale:] #get("sale_date", default: "[Date]")

#v(1em)

// ============================================================================
// PWC INFORMATION
// ============================================================================

#text(size: 12pt, weight: "bold")[PERSONAL WATERCRAFT INFORMATION]
#v(0.3em)

#table(
  columns: (1fr, 2fr),
  inset: 8pt,
  stroke: 0.5pt,
  [*Year*], [#get("pwc_year", default: "[Year]")],
  [*Make*], [#get("pwc_make", default: "[Yamaha/Sea-Doo/Kawasaki/etc.]")],
  [*Model*], [#get("pwc_model", default: "[Model]")],
  [*Color*], [#get("pwc_color", default: "[Color]")],
  [*Hull ID Number (HIN)*], [#get("hin", default: "[Hull ID Number]")],
  [*Engine Size*], [#get("engine_size", default: "[CC]") cc],
  [*Horsepower*], [#get("horsepower", default: "[HP]") HP],
  [*FL Registration #*], [#get("registration_number", default: "[FL #]")],
  [*Title Number*], [#get("title_number", default: "[Title #]")],
  [*Engine Hours (if known)*], [#get("engine_hours", default: "[Hours]")],
)

#v(1em)

// ============================================================================
// TRAILER INFORMATION (if included)
// ============================================================================

#if get_bool("includes_trailer") [
  #text(size: 12pt, weight: "bold")[TRAILER INFORMATION (Included in Sale)]
  #v(0.3em)

  #table(
    columns: (1fr, 2fr),
    inset: 8pt,
    stroke: 0.5pt,
    [*Year*], [#get("trailer_year", default: "[Year]")],
    [*Make*], [#get("trailer_make", default: "[Make]")],
    [*VIN*], [#get("trailer_vin", default: "[VIN]")],
    [*License Plate #*], [#get("trailer_plate", default: "[Plate #]")],
    [*Title Number*], [#get("trailer_title", default: "[Title #]")],
  )

  #v(1em)
]

// ============================================================================
// SELLER INFORMATION
// ============================================================================

#text(size: 12pt, weight: "bold")[SELLER INFORMATION]
#v(0.3em)

#table(
  columns: (1fr, 2fr),
  inset: 8pt,
  stroke: 0.5pt,
  [*Name*], [#get("seller_name", default: "[Seller Name]")],
  [*Address*], [#get("seller_address", default: "[Address]")],
  [*City, State, ZIP*], [#get("seller_city", default: "[City]"), #get("seller_state", default: "FL") #get("seller_zip", default: "[ZIP]")],
  [*Phone*], [#get("seller_phone", default: "[Phone]")],
  [*Driver's License #*], [#get("seller_dl", default: "[DL Number]")],
)

#v(1em)

// ============================================================================
// BUYER INFORMATION
// ============================================================================

#text(size: 12pt, weight: "bold")[BUYER INFORMATION]
#v(0.3em)

#table(
  columns: (1fr, 2fr),
  inset: 8pt,
  stroke: 0.5pt,
  [*Name*], [#get("buyer_name", default: "[Buyer Name]")],
  [*Address*], [#get("buyer_address", default: "[Address]")],
  [*City, State, ZIP*], [#get("buyer_city", default: "[City]"), #get("buyer_state", default: "FL") #get("buyer_zip", default: "[ZIP]")],
  [*Phone*], [#get("buyer_phone", default: "[Phone]")],
  [*Driver's License #*], [#get("buyer_dl", default: "[DL Number]")],
)

#v(1em)

// ============================================================================
// SALE TERMS
// ============================================================================

#text(size: 12pt, weight: "bold")[TERMS OF SALE]
#v(0.3em)

#table(
  columns: (1fr, 1fr),
  inset: 8pt,
  stroke: 0.5pt,
  [*PWC Purchase Price*], [$ #get("pwc_price", default: "[Amount]")],
  #if get_bool("includes_trailer") [
    [*Trailer Value*], [$ #get("trailer_value", default: "[Amount]")]
  ],
  #if get("accessories_value") != "" [
    [*Accessories/Equipment*], [$ #get("accessories_value")]
  ],
  table.hline(stroke: 1pt),
  [*TOTAL PURCHASE PRICE*], [*$ #get("total_price", default: "[Total]")*],
)

#v(0.5em)

*Payment Method:*
#let payment_method = get("payment_method", default: "cash")
#if payment_method == "cash" [Cash]
else if payment_method == "check" [Check #: #get("check_number", default: "")]
else if payment_method == "certified_check" [Certified Check/Cashier's Check]
else [#payment_method]

#v(1em)

// ============================================================================
// INCLUDED EQUIPMENT
// ============================================================================

#if get("included_items") != "" [
  #text(size: 12pt, weight: "bold")[INCLUDED ITEMS AND ACCESSORIES]
  #v(0.3em)

  #rect(
    width: 100%,
    inset: 8pt,
    stroke: 0.5pt,
  )[
    #get("included_items", default: "[Cover, life jackets, ropes, dock lines, etc.]")
  ]

  #v(1em)
]

// ============================================================================
// CONDITION AND WARRANTY
// ============================================================================

#text(size: 12pt, weight: "bold")[PWC CONDITION]
#v(0.3em)

#let sale_type = get("sale_type", default: "as_is")

#if sale_type == "as_is" [
  #rect(
    width: 100%,
    inset: 10pt,
    stroke: 2pt + rgb("#ff6600"),
    fill: rgb("#fff8f0"),
    radius: 4pt,
  )[
    #text(weight: "bold")[SOLD "AS-IS, WHERE-IS" - NO WARRANTY]
    #v(0.3em)
    This personal watercraft is sold "AS-IS, WHERE-IS" with no warranties expressed or implied. The Seller makes no guarantees as to the condition, performance, or fitness for any particular purpose. The Buyer accepts all risks and has had the opportunity to inspect and test the PWC.
  ]
] else [
  #rect(
    width: 100%,
    inset: 10pt,
    stroke: 1pt + rgb("#00cc00"),
    fill: rgb("#f0fff0"),
    radius: 4pt,
  )[
    #text(weight: "bold")[WARRANTY INCLUDED]
    #v(0.3em)
    #get("warranty_terms", default: "[Description of warranty terms]")
  ]
]

#v(0.5em)

#if get("known_defects") != "" [
  *Known Defects/Issues:*
  #v(0.2em)
  #get("known_defects")
  #v(0.5em)
]

// ============================================================================
// SELLER'S CERTIFICATION
// ============================================================================

#rect(
  width: 100%,
  inset: 10pt,
  stroke: 1pt,
  fill: rgb("#f5f5f5"),
  radius: 4pt,
)[
  #text(weight: "bold")[SELLER'S CERTIFICATION]
  #v(0.3em)
  The Seller certifies that:
  #list(
    tight: true,
    [The Seller is the legal owner of the PWC (and trailer if included) or is authorized to sell it],
    [The PWC is free and clear of all liens and encumbrances #if get_bool("has_lien") [(EXCEPT: #get("lien_holder", default: ""))]],
    [The Certificate of Title will be properly signed and delivered to the Buyer],
    [All information provided in this Bill of Sale is true and accurate],
  )
]

#v(1em)

// ============================================================================
// SIGNATURES
// ============================================================================

#text(size: 12pt, weight: "bold")[SIGNATURES]
#v(0.8em)

#grid(
  columns: (1fr, 1fr),
  gutter: 2em,
  [
    #text(weight: "bold")[SELLER]
    #v(0.8em)
    #line(length: 100%, stroke: 0.5pt)
    Signature
    #v(0.6em)
    #get("seller_name", default: "[Seller Name]")
    #linebreak()
    Printed Name
    #v(0.6em)
    Date: #box(width: 1.5in)[#line(length: 100%, stroke: 0.5pt)]
  ],
  [
    #text(weight: "bold")[BUYER]
    #v(0.8em)
    #line(length: 100%, stroke: 0.5pt)
    Signature
    #v(0.6em)
    #get("buyer_name", default: "[Buyer Name]")
    #linebreak()
    Printed Name
    #v(0.6em)
    Date: #box(width: 1.5in)[#line(length: 100%, stroke: 0.5pt)]
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

  STATE OF FLORIDA, COUNTY OF #get("notary_county", default: "[County]")

  Acknowledged before me this #box(width: 0.4in)[#line(length: 100%, stroke: 0.5pt)] day of #box(width: 1in)[#line(length: 100%, stroke: 0.5pt)], 20#box(width: 0.3in)[#line(length: 100%, stroke: 0.5pt)], by the person(s) named above.

  #v(0.8em)

  #grid(
    columns: (1fr, 1fr),
    gutter: 2em,
    [
      #line(length: 100%, stroke: 0.5pt)
      Notary Public Signature
    ],
    [
      Commission Expires: #box(width: 1in)[#line(length: 100%, stroke: 0.5pt)]
    ]
  )
]

#v(1em)

// ============================================================================
// IMPORTANT REMINDERS
// ============================================================================

#rect(
  width: 100%,
  inset: 8pt,
  stroke: 1pt + rgb("#cc0000"),
  fill: rgb("#fff5f5"),
  radius: 4pt,
)[
  #text(size: 8pt, weight: "bold")[IMPORTANT REMINDERS FOR FLORIDA PWC SALES]
  #v(0.2em)
  #text(size: 8pt)[
    *BUYER:*
    - You have *30 days* to transfer the title and register the PWC
    - All personal watercraft require Florida registration
    - Sales tax (6% + county surtax) is due at time of title transfer
    - Florida requires a Boating Safety Education ID card for PWC operators born after Jan. 1, 1988

    *SELLER:*
    - Sign the title over to the buyer
    - File a Notice of Sale with the Florida DHSMV
    - Remove your registration decals
  ]
]

#v(1em)

// ============================================================================
// DISCLAIMER
// ============================================================================

#align(center)[
  #text(size: 7pt, fill: rgb("#666"))[
    DISCLAIMER: This Bill of Sale was prepared using agentPDF.org. This is not legal advice. Verify HIN and registration information before signing. Consider a professional inspection before purchase.
  ]
]
