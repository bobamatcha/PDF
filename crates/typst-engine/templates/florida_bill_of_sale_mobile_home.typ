// Florida Mobile Home Bill of Sale Template
// Per Florida Statutes Chapters 319 (Title) and 723 (Mobile Home Act)
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
  #text(size: 18pt, weight: "bold")[MOBILE HOME BILL OF SALE]
  #v(0.2em)
  #text(size: 11pt)[Manufactured / Mobile Home]
  #v(0.2em)
  #text(size: 11pt)[State of Florida]
  #v(0.2em)
  #text(size: 9pt, style: "italic")[Pursuant to Florida Statutes Chapters 319 & 723]
]

#v(1em)

#rect(
  width: 100%,
  inset: 8pt,
  stroke: 1pt + rgb("#0066cc"),
  fill: rgb("#f0f8ff"),
  radius: 4pt,
)[
  #text(size: 9pt)[
    *IMPORTANT:* Mobile/manufactured homes may be titled as personal property (like a vehicle) or as real property if permanently affixed to land owned by the homeowner. This Bill of Sale is for mobile homes titled as *personal property*.
  ]
]

#v(1em)

// ============================================================================
// TRANSACTION DATE
// ============================================================================

#text(weight: "bold")[Date of Sale:] #get("sale_date", default: "[Date]")

#v(1em)

// ============================================================================
// MOBILE HOME INFORMATION
// ============================================================================

#text(size: 12pt, weight: "bold")[MOBILE HOME INFORMATION]
#v(0.3em)

#table(
  columns: (1fr, 2fr),
  inset: 8pt,
  stroke: 0.5pt,
  [*Year*], [#get("mh_year", default: "[Year]")],
  [*Make/Manufacturer*], [#get("mh_make", default: "[Manufacturer]")],
  [*Model*], [#get("mh_model", default: "[Model]")],
  [*Size (Width x Length)*], [#get("mh_width", default: "[Width]") x #get("mh_length", default: "[Length]") feet],
  [*Number of Sections*], [#get("sections", default: "[Single/Double/Triple]")],
  [*VIN/Serial Number(s)*], [#get("vin", default: "[VIN/Serial #]")],
  #if get("vin2") != "" [
    [*VIN #2 (if multi-section)*], [#get("vin2")]
  ],
  [*Title Number*], [#get("title_number", default: "[Title #]")],
  [*HUD Label/Tag Number*], [#get("hud_number", default: "[HUD #]")],
)

#v(0.8em)

// ============================================================================
// LOCATION INFORMATION
// ============================================================================

#text(size: 12pt, weight: "bold")[CURRENT LOCATION]
#v(0.3em)

#table(
  columns: (1fr, 2fr),
  inset: 8pt,
  stroke: 0.5pt,
  [*Address/Lot*], [#get("location_address", default: "[Address/Lot Number]")],
  [*Park Name (if applicable)*], [#get("park_name", default: "[Mobile Home Park Name]")],
  [*City, State, ZIP*], [#get("location_city", default: "[City]"), Florida #get("location_zip", default: "[ZIP]")],
  [*County*], [#get("location_county", default: "[County]")],
)

#v(1em)

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
  [*Purchase Price*], [$ #get("purchase_price", default: "[Amount]")],
  #if get("personal_property") != "" [
    [*Personal Property Included*], [$ #get("personal_property_value", default: "[Amount]")]
  ],
  #if get("down_payment") != "" [
    [*Down Payment*], [$ #get("down_payment")]
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
else if payment_method == "financing" [Financed through: #get("lender_name", default: "")]
else [#payment_method]

#v(1em)

// ============================================================================
// INCLUDED ITEMS
// ============================================================================

#text(size: 12pt, weight: "bold")[ITEMS INCLUDED IN SALE]
#v(0.3em)

The following items are included in this sale:

#list(
  tight: true,
  [Mobile home and all permanently attached fixtures],
  #if get_bool("include_appliances") [
    [Appliances: #get("appliances_list", default: "refrigerator, stove, dishwasher, washer, dryer")]
  ] else [
    [Appliances: None / NOT included]
  ],
  #if get_bool("include_ac") [
    [Air conditioning/heating system]
  ],
  #if get_bool("include_skirting") [
    [Skirting]
  ],
  #if get_bool("include_steps") [
    [Steps/deck/porch]
  ],
  #if get_bool("include_shed") [
    [Storage shed]
  ],
)

#if get("other_included") != "" [
  *Other items included:* #get("other_included")
]

#v(1em)

// ============================================================================
// CONDITION AND WARRANTY
// ============================================================================

#text(size: 12pt, weight: "bold")[CONDITION]
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
    This mobile home is sold "AS-IS, WHERE-IS" with no warranties expressed or implied. The Seller makes no guarantees as to the condition, habitability, or fitness for any particular purpose. The Buyer has had the opportunity to inspect the home and accepts all risks.
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
// PARK TENANCY (if applicable)
// ============================================================================

#if get("park_name") != "" [
  #text(size: 12pt, weight: "bold")[MOBILE HOME PARK TENANCY]
  #v(0.3em)

  #rect(
    width: 100%,
    inset: 10pt,
    stroke: 1pt,
    fill: rgb("#f5f5f5"),
    radius: 4pt,
  )[
    *Park Name:* #get("park_name")

    *Current Lot Rent:* $#get("lot_rent", default: "[Amount]") per month

    *IMPORTANT:* The Buyer understands that:
    #list(
      tight: true,
      [Park approval may be required before Buyer can occupy the home],
      [Buyer must apply for and be approved for tenancy by the park],
      [Buyer is responsible for reviewing the park prospectus and rules],
      [Lot rent is subject to change per Florida Statutes Chapter 723],
    )

    #if get_bool("park_approval_obtained") [
      #sym.ballot.x Park has approved Buyer's tenancy application
    ] else [
      #sym.ballot Park approval is pending / required
    ]
  ]

  #v(1em)
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
    [The Seller is the legal owner of the mobile home or is authorized to sell it],
    [The mobile home is free and clear of all liens and encumbrances #if get_bool("has_lien") [(EXCEPT: #get("lien_holder", default: ""))]],
    [All property taxes are current (or will be prorated at closing)],
    [The Certificate of Title will be properly signed and delivered to the Buyer],
    [All information provided in this Bill of Sale is true and accurate],
    [#if get("park_name") != "" [Seller is current on lot rent and there are no outstanding balances with the park]],
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

  The foregoing instrument was acknowledged before me by means of #sym.ballot physical presence or #sym.ballot online notarization this #box(width: 0.4in)[#line(length: 100%, stroke: 0.5pt)] day of #box(width: 1in)[#line(length: 100%, stroke: 0.5pt)], 20#box(width: 0.3in)[#line(length: 100%, stroke: 0.5pt)], by the person(s) named above.

  #v(0.8em)

  #grid(
    columns: (1fr, 1fr),
    gutter: 2em,
    [
      #line(length: 100%, stroke: 0.5pt)
      Notary Public Signature
      #v(0.3em)
      Printed Name: #box(width: 1.2in)[#line(length: 100%, stroke: 0.5pt)]
    ],
    [
      My Commission Expires:
      #v(0.3em)
      #box(width: 1.5in)[#line(length: 100%, stroke: 0.5pt)]
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
  #text(size: 8pt, weight: "bold")[IMPORTANT REMINDERS FOR FLORIDA MOBILE HOME SALES]
  #v(0.2em)
  #text(size: 8pt)[
    *BUYER:*
    - You have *30 days* to transfer the title at a Florida Tax Collector office
    - Sales tax (6% + county surtax) is due at time of title transfer
    - Contact the county Property Appraiser to update ownership records
    - If in a mobile home park, you must be approved by park management before moving in
    - Review Florida Statutes Chapter 723 for your rights as a mobile home owner in a park

    *SELLER:*
    - Sign the title over to the buyer
    - Notify the mobile home park of the sale
    - File a Notice of Sale with the Florida DHSMV
    - Prorate property taxes if applicable
  ]
]

#v(1em)

// ============================================================================
// DISCLAIMER
// ============================================================================

#align(center)[
  #text(size: 7pt, fill: rgb("#666"))[
    DISCLAIMER: This Bill of Sale was prepared using agentPDF.org. This is not legal advice. Mobile home transactions can be complex. Consider a professional inspection and consult with an attorney for significant purchases.
  ]
]
