// Business Letter Template
// Required inputs: sender_name, recipient_name, body
// Optional inputs: sender_address, recipient_address, date, subject, closing

#let data = sys.inputs

#set page(margin: 2.5cm)
#set text(font: "Liberation Sans", size: 11pt)
#set par(justify: true, leading: 0.65em)

// Sender Information
#text(weight: "bold", size: 12pt)[#data.at("sender_name", default: "Sender Name")]
#if data.at("sender_address", default: none) != none [
  #linebreak()
  #text(size: 10pt)[#data.at("sender_address")]
]

#v(1.5em)

// Date
#text(size: 10pt)[#data.at("date", default: datetime.today().display("[month repr:long] [day], [year]"))]

#v(1.5em)

// Recipient Information
#text(weight: "bold")[#data.at("recipient_name", default: "Recipient Name")]
#if data.at("recipient_address", default: none) != none [
  #linebreak()
  #text(size: 10pt)[#data.at("recipient_address")]
]

#v(1.5em)

// Subject Line (if provided)
#if data.at("subject", default: none) != none [
  #text(weight: "bold")[Re: #data.at("subject")]
  #v(1em)
]

// Body
#data.at("body", default: "Letter body text goes here.")

#v(1.5em)

// Closing
#text[#data.at("closing", default: "Sincerely,")]

#v(2em)

// Signature Line
#text(weight: "bold")[#data.at("sender_name", default: "Sender Name")]
