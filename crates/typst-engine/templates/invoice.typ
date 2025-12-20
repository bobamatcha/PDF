// Invoice Template
// Required inputs: company_name, client_name, items
// Optional inputs: company_address, client_address, invoice_number, date, due_date, notes

#let data = sys.inputs

#set page(margin: 2cm)
#set text(font: "Liberation Sans", size: 10pt)

// Header
#grid(
  columns: (1fr, 1fr),
  align: (left, right),
  [
    #text(size: 24pt, weight: "bold")[INVOICE]
    #v(0.5em)
    #text(size: 14pt, fill: rgb("#333"))[#data.at("company_name", default: "Company Name")]
    #if data.at("company_address", default: none) != none [
      #linebreak()
      #text(size: 9pt, fill: rgb("#666"))[#data.at("company_address")]
    ]
  ],
  [
    #if data.at("invoice_number", default: none) != none [
      *Invoice \#:* #data.at("invoice_number") #linebreak()
    ]
    *Date:* #data.at("date", default: datetime.today().display("[month repr:long] [day], [year]")) #linebreak()
    #if data.at("due_date", default: none) != none [
      *Due:* #data.at("due_date")
    ]
  ]
)

#v(1em)
#line(length: 100%, stroke: 0.5pt + rgb("#ccc"))
#v(1em)

// Bill To
#grid(
  columns: (1fr, 1fr),
  [
    #text(weight: "bold", fill: rgb("#666"))[BILL TO]
    #v(0.3em)
    #text(size: 11pt)[#data.at("client_name", default: "Client Name")]
    #if data.at("client_address", default: none) != none [
      #linebreak()
      #text(size: 9pt, fill: rgb("#666"))[#data.at("client_address")]
    ]
  ],
  []
)

#v(1.5em)

// Items Table
#let items = data.at("items", default: ())
#let total = items.fold(0, (acc, item) => acc + item.at("qty", default: 1) * item.at("price", default: 0))

#table(
  columns: (auto, 1fr, auto, auto, auto),
  stroke: none,
  inset: 8pt,
  fill: (_, row) => if row == 0 { rgb("#f5f5f5") } else { none },
  [*Qty*], [*Description*], [*Unit Price*], [*Amount*], [],
  ..items.map(item => {
    let qty = item.at("qty", default: 1)
    let price = item.at("price", default: 0)
    let amount = qty * price
    (
      str(qty),
      item.at("description", default: "Item"),
      [\$#str(price)],
      [\$#str(amount)],
      [],
    )
  }).flatten()
)

#line(length: 100%, stroke: 0.5pt + rgb("#ccc"))

// Total
#v(0.5em)
#align(right)[
  #box(width: 200pt)[
    #grid(
      columns: (1fr, auto),
      gutter: 8pt,
      [Subtotal:], [\$#str(total)],
      [Tax (0%):], [\$0],
      [*Total:*], [*\$#str(total)*],
    )
  ]
]

// Notes
#if data.at("notes", default: none) != none [
  #v(2em)
  #text(weight: "bold", fill: rgb("#666"))[NOTES]
  #v(0.3em)
  #text(size: 9pt)[#data.at("notes")]
]
