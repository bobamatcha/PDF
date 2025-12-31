// Florida Contractor Invoice Template
// Professional invoice for construction/contracting services
// Compliant with Florida contractor regulations
// All values are dynamic via sys.inputs

#let data = sys.inputs

// Helper functions
#let get(key, default: "") = data.at(key, default: default)
#let get_bool(key) = {
  let val = data.at(key, default: false)
  if type(val) == str { val == "true" } else { val == true }
}
#let get_float(key, default: 0.0) = {
  let val = data.at(key, default: default)
  if type(val) == str { float(val) } else { float(val) }
}

// Page setup
#set page(
  paper: "us-letter",
  margin: (top: 0.75in, bottom: 0.75in, left: 0.75in, right: 0.75in),
)
#set text(font: "Liberation Sans", size: 10pt)
#set par(justify: false, leading: 0.65em)

// ============================================================================
// HEADER
// ============================================================================

#grid(
  columns: (1fr, 1fr),
  gutter: 2em,
  [
    // Company Info
    #text(size: 16pt, weight: "bold")[#get("company_name", default: "[Company Name]")]
    #v(0.3em)
    #get("company_address", default: "[Address]")
    #linebreak()
    #get("company_city", default: "[City]"), #get("company_state", default: "FL") #get("company_zip", default: "[ZIP]")
    #linebreak()
    Phone: #get("company_phone", default: "[Phone]")
    #if get("company_email") != "" [
      #linebreak()
      Email: #get("company_email")
    ]
    #v(0.3em)
    #text(size: 9pt)[
      License #: #get("license_number", default: "[License Number]")
      #if get("insurance_info") != "" [
        #linebreak()
        #get("insurance_info")
      ]
    ]
  ],
  [
    #align(right)[
      #text(size: 24pt, weight: "bold", fill: rgb("#0066cc"))[INVOICE]
      #v(0.5em)
      #table(
        columns: (auto, auto),
        inset: 6pt,
        stroke: 0.5pt,
        align: (left, right),
        [*Invoice #*], [#get("invoice_number", default: "[Invoice #]")],
        [*Date*], [#get("invoice_date", default: "[Date]")],
        [*Due Date*], [#get("due_date", default: "[Due Date]")],
      )
    ]
  ]
)

#v(1em)
#line(length: 100%, stroke: 2pt + rgb("#0066cc"))
#v(1em)

// ============================================================================
// BILLING INFORMATION
// ============================================================================

#grid(
  columns: (1fr, 1fr),
  gutter: 2em,
  [
    #text(weight: "bold", size: 11pt)[BILL TO:]
    #v(0.3em)
    #get("client_name", default: "[Client Name]")
    #linebreak()
    #get("client_address", default: "[Client Address]")
    #linebreak()
    #get("client_city", default: "[City]"), #get("client_state", default: "FL") #get("client_zip", default: "[ZIP]")
    #if get("client_phone") != "" [
      #linebreak()
      Phone: #get("client_phone")
    ]
    #if get("client_email") != "" [
      #linebreak()
      Email: #get("client_email")
    ]
  ],
  [
    #text(weight: "bold", size: 11pt)[PROJECT LOCATION:]
    #v(0.3em)
    #get("project_address", default: "[Project Address]")
    #linebreak()
    #get("project_city", default: "[City]"), Florida #get("project_zip", default: "[ZIP]")
    #if get("project_name") != "" [
      #v(0.3em)
      Project: #get("project_name")
    ]
  ]
)

#v(1em)

// ============================================================================
// INVOICE TYPE
// ============================================================================

#let invoice_type = get("invoice_type", default: "standard")

#if invoice_type == "progress" [
  #rect(
    width: 100%,
    inset: 8pt,
    stroke: 1pt + rgb("#0066cc"),
    fill: rgb("#f0f8ff"),
    radius: 4pt,
  )[
    #text(weight: "bold")[PROGRESS INVOICE]
    #h(2em)
    Application #: #get("application_number", default: "[#]")
    #h(2em)
    Period: #get("period_start", default: "[Start]") to #get("period_end", default: "[End]")
  ]
  #v(0.5em)
]

// ============================================================================
// LINE ITEMS TABLE
// ============================================================================

#text(weight: "bold", size: 11pt)[DESCRIPTION OF WORK / MATERIALS]
#v(0.3em)

// Parse line items from JSON array or use defaults
#let line_items = data.at("line_items", default: ())

#if line_items.len() > 0 [
  #table(
    columns: (3fr, 0.5fr, 1fr, 1fr),
    inset: 8pt,
    stroke: 0.5pt,
    fill: (x, y) => if y == 0 { rgb("#f0f0f0") } else { none },
    align: (left, center, right, right),
    [*Description*], [*Qty*], [*Unit Price*], [*Amount*],
    ..line_items.map(item => (
      item.at("description", default: ""),
      str(item.at("quantity", default: 1)),
      "$" + str(item.at("unit_price", default: "0.00")),
      "$" + str(item.at("amount", default: "0.00")),
    )).flatten()
  )
] else [
  // Default table with placeholder rows
  #table(
    columns: (3fr, 0.5fr, 1fr, 1fr),
    inset: 8pt,
    stroke: 0.5pt,
    fill: (x, y) => if y == 0 { rgb("#f0f0f0") } else { none },
    align: (left, center, right, right),
    [*Description*], [*Qty*], [*Unit Price*], [*Amount*],
    [#get("item1_desc", default: "[Description of work or materials]")], [#get("item1_qty", default: "1")], [$#get("item1_price", default: "0.00")], [$#get("item1_amount", default: "0.00")],
    [#get("item2_desc", default: "")], [#get("item2_qty", default: "")], [#get("item2_price", default: "")], [#get("item2_amount", default: "")],
    [#get("item3_desc", default: "")], [#get("item3_qty", default: "")], [#get("item3_price", default: "")], [#get("item3_amount", default: "")],
    [#get("item4_desc", default: "")], [#get("item4_qty", default: "")], [#get("item4_price", default: "")], [#get("item4_amount", default: "")],
    [#get("item5_desc", default: "")], [#get("item5_qty", default: "")], [#get("item5_price", default: "")], [#get("item5_amount", default: "")],
  )
]

#v(0.5em)

// ============================================================================
// TOTALS
// ============================================================================

#align(right)[
  #table(
    columns: (1.5fr, 1fr),
    inset: 8pt,
    stroke: 0.5pt,
    align: (left, right),
    [Subtotal], [$#get("subtotal", default: "0.00")],
    #if get("discount") != "" and get("discount") != "0" and get("discount") != "0.00" [
      [Discount], [-$#get("discount")]
    ],
    #if invoice_type == "progress" [
      [Previous Billings], [-$#get("previous_billings", default: "0.00")]
    ],
    #if get_bool("taxable") [
      [Sales Tax (#get("tax_rate", default: "0")%)], [$#get("tax_amount", default: "0.00")]
    ],
    table.hline(stroke: 1pt),
    [*TOTAL DUE*], [*$#get("total_due", default: "0.00")*],
  )
]

#v(1em)

// ============================================================================
// CONTRACT REFERENCE (if applicable)
// ============================================================================

#if get("contract_reference") != "" [
  #text(weight: "bold", size: 10pt)[CONTRACT REFERENCE]
  #v(0.3em)
  Contract #: #get("contract_reference")
  #if get("contract_date") != "" [
    #h(2em)
    Dated: #get("contract_date")
  ]
  #if get("contract_amount") != "" [
    #h(2em)
    Original Contract: $#get("contract_amount")
  ]
  #v(0.8em)
]

// ============================================================================
// PROGRESS BILLING SUMMARY (for progress invoices)
// ============================================================================

#if invoice_type == "progress" [
  #text(weight: "bold", size: 10pt)[PROGRESS BILLING SUMMARY]
  #v(0.3em)

  #table(
    columns: (2fr, 1fr),
    inset: 6pt,
    stroke: 0.5pt,
    align: (left, right),
    [Original Contract Amount], [$#get("original_contract", default: "0.00")],
    [Approved Change Orders], [$#get("change_orders", default: "0.00")],
    [Revised Contract Amount], [$#get("revised_contract", default: "0.00")],
    table.hline(stroke: 0.5pt),
    [Total Completed to Date], [$#get("completed_to_date", default: "0.00")],
    [Retainage (#get("retainage_percent", default: "10")%)], [-$#get("retainage_amount", default: "0.00")],
    [Less Previous Payments], [-$#get("previous_payments", default: "0.00")],
    table.hline(stroke: 1pt),
    [*Current Payment Due*], [*$#get("current_due", default: "0.00")*],
  )

  #v(0.8em)
]

// ============================================================================
// PAYMENT INFORMATION
// ============================================================================

#rect(
  width: 100%,
  inset: 10pt,
  stroke: 1pt,
  fill: rgb("#f5f5f5"),
  radius: 4pt,
)[
  #text(weight: "bold", size: 10pt)[PAYMENT INFORMATION]
  #v(0.3em)

  #grid(
    columns: (1fr, 1fr),
    gutter: 2em,
    [
      *Payment Due:* #get("due_date", default: "[Due Date]")

      *Accepted Payment Methods:*
      #list(
        tight: true,
        [Check payable to: #get("company_name", default: "[Company Name]")],
        #if get_bool("accept_credit") [
          [Credit Card (convenience fee may apply)]
        ],
        #if get("bank_info") != "" [
          [Bank Transfer / ACH]
        ],
      )
    ],
    [
      #if get("bank_info") != "" [
        *For Bank Transfers:*
        #v(0.2em)
        #text(size: 9pt)[#get("bank_info")]
      ]

      #if get("late_fee_info") != "" [
        #v(0.3em)
        *Late Payment:* #text(size: 9pt)[#get("late_fee_info")]
      ]
    ]
  )
]

#v(1em)

// ============================================================================
// NOTES / TERMS
// ============================================================================

#if get("notes") != "" [
  #text(weight: "bold", size: 10pt)[NOTES]
  #v(0.3em)
  #get("notes")
  #v(0.8em)
]

// ============================================================================
// TERMS AND CONDITIONS
// ============================================================================

#text(weight: "bold", size: 10pt)[TERMS AND CONDITIONS]
#v(0.3em)

#text(size: 8pt)[
  #list(
    tight: true,
    [Payment is due within #get("payment_terms", default: "30") days of invoice date unless otherwise specified.],
    [#if get("late_fee_percent") != "" [Late payments are subject to a #get("late_fee_percent")% monthly finance charge (#get("late_fee_apr", default: "18")% APR).] else [Late payments may be subject to finance charges.]],
    [This invoice is for work performed in accordance with the contract between the parties.],
    [All materials remain the property of the contractor until payment is received in full.],
    [Disputes must be reported in writing within 10 days of invoice date.],
  )
]

#v(0.8em)

// ============================================================================
// LIEN RIGHTS NOTICE (Florida requirement)
// ============================================================================

#rect(
  width: 100%,
  inset: 8pt,
  stroke: 1pt + rgb("#cc0000"),
  fill: rgb("#fff5f5"),
  radius: 4pt,
)[
  #text(size: 8pt, weight: "bold")[NOTICE PURSUANT TO FLORIDA CONSTRUCTION LIEN LAW]
  #v(0.2em)
  #text(size: 8pt)[
    Under Florida law, those who work on your property or provide materials and are not paid in full have a right to enforce their claim against your property. This claim is known as a construction lien. If your contractor fails to pay subcontractors or material suppliers, those who are owed money may look to your property for payment even if you have paid your contractor in full. To protect yourself, you may request that your contractor provide lien releases from all subcontractors and suppliers before making final payment.
  ]
]

#v(1em)

// ============================================================================
// SIGNATURE (for approved invoices)
// ============================================================================

#if get_bool("requires_approval") [
  #text(weight: "bold", size: 10pt)[APPROVAL]
  #v(0.5em)

  #grid(
    columns: (1fr, 1fr),
    gutter: 2em,
    [
      #line(length: 100%, stroke: 0.5pt)
      Approved By
      #v(0.5em)
      Date: #box(width: 1.5in)[#line(length: 100%, stroke: 0.5pt)]
    ],
    [
      #line(length: 100%, stroke: 0.5pt)
      Amount Approved
    ]
  )

  #v(1em)
]

// ============================================================================
// FOOTER
// ============================================================================

#align(center)[
  #text(size: 9pt, weight: "bold")[Thank you for your business!]
  #v(0.3em)
  #text(size: 8pt, fill: rgb("#666"))[
    Questions about this invoice? Contact us at #get("company_phone", default: "[Phone]") or #get("company_email", default: "[Email]")
  ]
]

#v(0.5em)

// ============================================================================
// DISCLAIMER
// ============================================================================

#align(center)[
  #text(size: 7pt, fill: rgb("#666"))[
    Generated with agentPDF.org | Invoice #get("invoice_number", default: "[#]")
  ]
]
