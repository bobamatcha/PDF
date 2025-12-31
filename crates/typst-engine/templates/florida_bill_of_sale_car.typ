// Florida Motor Vehicle Bill of Sale Template
// Per Florida Statutes Chapter 319 (Motor Vehicles)
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
  #text(size: 18pt, weight: "bold")[MOTOR VEHICLE BILL OF SALE]
  #v(0.2em)
  #text(size: 11pt)[State of Florida]
  #v(0.2em)
  #text(size: 9pt, style: "italic")[Pursuant to Florida Statutes Chapter 319]
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
    *IMPORTANT:* This Bill of Sale is a supplemental document. Florida law requires the seller to sign over the Certificate of Title to transfer ownership. This Bill of Sale serves as proof of the transaction.
  ]
]

#v(1em)

// ============================================================================
// TRANSACTION DATE
// ============================================================================

#text(weight: "bold")[Date of Sale:] #get("sale_date", default: "[Date]")

#v(1em)

// ============================================================================
// VEHICLE INFORMATION
// ============================================================================

#text(size: 12pt, weight: "bold")[VEHICLE INFORMATION]
#v(0.3em)

#table(
  columns: (1fr, 2fr),
  inset: 8pt,
  stroke: 0.5pt,
  [*Year*], [#get("vehicle_year", default: "[Year]")],
  [*Make*], [#get("vehicle_make", default: "[Make]")],
  [*Model*], [#get("vehicle_model", default: "[Model]")],
  [*Body Style*], [#get("body_style", default: "[Sedan/SUV/Truck/etc.]")],
  [*Color*], [#get("vehicle_color", default: "[Color]")],
  [*VIN (Vehicle Identification Number)*], [#get("vin", default: "[17-character VIN]")],
  [*Current Odometer Reading*], [#get("odometer", default: "[Miles]") miles],
  [*License Plate # (if applicable)*], [#get("plate_number", default: "[Plate #]")],
  [*Title Number*], [#get("title_number", default: "[Title #]")],
)

#v(1em)

// ============================================================================
// ODOMETER DISCLOSURE
// ============================================================================

#rect(
  width: 100%,
  inset: 10pt,
  stroke: 1pt,
  radius: 4pt,
)[
  #text(weight: "bold")[ODOMETER DISCLOSURE STATEMENT (Federal and State Requirement)]
  #v(0.5em)

  I, #get("seller_name", default: "[Seller Name]"), state that the odometer now reads *#get("odometer", default: "[Miles]")* miles and to the best of my knowledge:

  #v(0.3em)

  #let odometer_status = get("odometer_status", default: "actual")

  #if odometer_status == "actual" [
    #sym.ballot.x The odometer reading reflects the *actual mileage* of the vehicle.
  ] else [
    #sym.ballot The odometer reading reflects the *actual mileage* of the vehicle.
  ]

  #if odometer_status == "exceeds" [
    #sym.ballot.x The odometer reading is *in excess of its mechanical limits* (vehicle has more than 99,999 miles).
  ] else [
    #sym.ballot The odometer reading is *in excess of its mechanical limits* (vehicle has more than 99,999 miles).
  ]

  #if odometer_status == "not_actual" [
    #sym.ballot.x The odometer reading is *NOT the actual mileage* — WARNING: ODOMETER DISCREPANCY.
  ] else [
    #sym.ballot The odometer reading is *NOT the actual mileage* — WARNING: ODOMETER DISCREPANCY.
  ]
]

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
  #if get("trade_in_value") != "" [
    [*Trade-In Value*], [$ #get("trade_in_value")]
  ],
  #if get("down_payment") != "" [
    [*Down Payment*], [$ #get("down_payment")]
  ],
  [*Total Amount Paid*], [*$ #get("total_paid", default: "[Amount]")*],
)

#v(0.5em)

*Payment Method:*
#let payment_method = get("payment_method", default: "cash")
#if payment_method == "cash" [Cash]
else if payment_method == "check" [Check #: #get("check_number", default: "")]
else if payment_method == "certified_check" [Certified Check/Cashier's Check #: #get("check_number", default: "")]
else if payment_method == "financing" [Financed through: #get("lender_name", default: "")]
else [#payment_method]

#v(1em)

// ============================================================================
// CONDITION AND WARRANTY
// ============================================================================

#text(size: 12pt, weight: "bold")[VEHICLE CONDITION]
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
    #text(weight: "bold")[SOLD "AS-IS" - NO WARRANTY]
    #v(0.3em)
    This vehicle is sold "AS-IS" with no warranties expressed or implied. The Seller makes no guarantees as to the condition, performance, or fitness for any particular purpose. The Buyer accepts all risks associated with this purchase.
  ]
] else if sale_type == "warranty" [
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
    [The Seller is the legal owner of the vehicle or is authorized to sell it],
    [The vehicle is free and clear of all liens and encumbrances #if get_bool("has_lien") [(EXCEPT: #get("lien_holder", default: ""))]],
    [The Title will be properly signed and delivered to the Buyer],
    [The information provided in this Bill of Sale is true and accurate],
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
// NOTARY (Optional)
// ============================================================================

#if get_bool("include_notary") [
  #rect(
    width: 100%,
    inset: 10pt,
    stroke: 1pt,
    fill: rgb("#f9f9f9"),
    radius: 4pt,
  )[
    #text(size: 10pt, weight: "bold")[NOTARY ACKNOWLEDGMENT (Optional)]
    #v(0.5em)

    STATE OF FLORIDA, COUNTY OF #get("notary_county", default: "[County]")

    Acknowledged before me this #box(width: 0.4in)[#line(length: 100%, stroke: 0.5pt)] day of #box(width: 1in)[#line(length: 100%, stroke: 0.5pt)], 20#box(width: 0.3in)[#line(length: 100%, stroke: 0.5pt)].

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
]

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
  #text(size: 8pt, weight: "bold")[IMPORTANT REMINDERS FOR FLORIDA]
  #v(0.2em)
  #text(size: 8pt)[
    *BUYER:*
    - You have *30 days* to transfer the title into your name at a Florida DMV/Tax Collector office
    - You must obtain Florida insurance before driving the vehicle
    - Sales tax (currently 6% + county discretionary tax) is due at time of title transfer

    *SELLER:*
    - Sign the title over to the buyer in the designated area
    - Complete the Notice of Sale on the title or file separately with the DMV
    - Remove your license plate (plates belong to the owner, not the vehicle)
  ]
]

#v(1em)

// ============================================================================
// DISCLAIMER
// ============================================================================

#align(center)[
  #text(size: 7pt, fill: rgb("#666"))[
    DISCLAIMER: This Bill of Sale was prepared using agentPDF.org. This is not legal advice. For significant purchases or complex situations, consult with an attorney. Verify all information before signing.
  ]
]
