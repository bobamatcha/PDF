// ============================================================================
// FLORIDA STANDALONE FLOOD DISCLOSURE (SB 948 / HB 1015 - § 83.512)
// ============================================================================
// Effective: October 1, 2025
// Purpose: Quick-generate just the flood disclosure form for landlords
// who need to provide this as a separate document.
// ============================================================================

#set page(
  paper: "us-letter",
  margin: (top: 0.75in, bottom: 0.75in, left: 1in, right: 1in),
)

#set text(
  font: "New Computer Modern",
  size: 11pt,
)

#set par(
  justify: true,
  leading: 0.65em,
)

// Helper function to get input with default fallback
#let get(key, default: "") = {
  if key in sys.inputs {
    sys.inputs.at(key)
  } else {
    default
  }
}

// ============================================================================
// HEADER
// ============================================================================

#align(center)[
  #text(size: 16pt, weight: "bold")[FLORIDA FLOOD DISCLOSURE]
  #v(0.3em)
  #text(size: 11pt)[Pursuant to Florida Statutes § 83.512]
  #v(0.3em)
  #text(size: 10pt, style: "italic")[(SB 948 / HB 1015 - Effective October 1, 2025)]
]

#v(1em)

// ============================================================================
// MANDATORY DISCLOSURE NOTICE
// ============================================================================

#rect(
  width: 100%,
  inset: 15pt,
  stroke: 2pt + rgb("#dc2626"),
  fill: rgb("#fef2f2"),
  radius: 4pt,
)[
  #text(weight: "bold", size: 12pt)[MANDATORY FLOOD DISCLOSURE]

  #v(0.5em)

  Pursuant to Florida Statutes § 83.512, the Landlord is required to disclose to the Tenant, prior to execution of a residential lease for a term of one year or longer, the following information regarding the property located at:

  #v(0.5em)

  #text(weight: "bold", size: 11pt)[#get("property_address", default: "[Property Address]")]

  #if get("property_city", default: "") != "" or get("property_zip", default: "") != "" [
    #v(0.3em)
    #get("property_city", default: "")#if get("property_city", default: "") != "" and get("property_zip", default: "") != "" [, Florida ] else [ Florida ]#get("property_zip", default: "")
  ]
]

#v(1em)

// ============================================================================
// PARTIES
// ============================================================================

#text(size: 12pt, weight: "bold")[PARTIES]
#v(0.5em)

#grid(
  columns: (1fr, 1fr),
  gutter: 20pt,
  [
    *Landlord:* #get("landlord_name", default: "[Landlord Name]")
    #if get("landlord_address", default: "") != "" [
      #v(0.3em)
      Address: #get("landlord_address", default: "")
    ]
  ],
  [
    *Tenant:* #get("tenant_name", default: "[Tenant Name]")
  ]
)

#v(1em)

// ============================================================================
// LANDLORD'S DISCLOSURE
// ============================================================================

#text(size: 12pt, weight: "bold")[LANDLORD'S DISCLOSURE]
#v(0.5em)

// Flood History Status - Tristate: "yes", "no", "unknown"
// Per scrivener adherence, must offer neutral "unknown" option
#let flood_history = get("flood_history_status", default: "unknown")

#text(weight: "bold")[1. KNOWLEDGE OF PRIOR FLOODING]

#v(0.5em)

#if flood_history == "yes" [
  #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] Landlord HAS knowledge of prior flooding at this property.
] else [
  #box(width: 12pt, height: 12pt, stroke: 1pt)[] Landlord HAS knowledge of prior flooding at this property.
]

#v(0.3em)

#if flood_history == "no" [
  #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] Landlord has NO knowledge of prior flooding at this property.
] else [
  #box(width: 12pt, height: 12pt, stroke: 1pt)[] Landlord has NO knowledge of prior flooding at this property.
]

#v(0.3em)

#if flood_history == "unknown" [
  #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] Landlord does not know / Property recently acquired.
] else [
  #box(width: 12pt, height: 12pt, stroke: 1pt)[] Landlord does not know / Property recently acquired.
]

#if flood_history == "yes" [
  #v(0.5em)
  Description of flooding: #get("flooding_description", default: "[Describe flooding events]")
]

#v(1em)

// Flood Claims Status - Tristate: "yes", "no", "unknown"
#let flood_claims = get("flood_claims_status", default: "unknown")

#text(weight: "bold")[2. FLOOD INSURANCE CLAIMS]

#v(0.5em)

#if flood_claims == "yes" [
  #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] Flood insurance claims HAVE been filed for this property.
] else [
  #box(width: 12pt, height: 12pt, stroke: 1pt)[] Flood insurance claims HAVE been filed for this property.
]

#v(0.3em)

#if flood_claims == "no" [
  #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] No flood insurance claims have been filed for this property.
] else [
  #box(width: 12pt, height: 12pt, stroke: 1pt)[] No flood insurance claims have been filed for this property.
]

#v(0.3em)

#if flood_claims == "unknown" [
  #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] Landlord does not know / Property recently acquired.
] else [
  #box(width: 12pt, height: 12pt, stroke: 1pt)[] Landlord does not know / Property recently acquired.
]

#if flood_claims == "yes" [
  #v(0.5em)
  Details: #get("flood_claims_details", default: "[Describe claims]")
]

#v(1em)

// Flood FEMA Status - Tristate: "yes", "no", "unknown"
#let flood_fema = get("flood_fema_status", default: "unknown")

#text(weight: "bold")[3. FEDERAL FLOOD ASSISTANCE (FEMA)]

#v(0.5em)

#if flood_fema == "yes" [
  #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] Federal flood assistance (FEMA) HAS been received for this property.
] else [
  #box(width: 12pt, height: 12pt, stroke: 1pt)[] Federal flood assistance (FEMA) HAS been received for this property.
]

#v(0.3em)

#if flood_fema == "no" [
  #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] No federal flood assistance has been received for this property.
] else [
  #box(width: 12pt, height: 12pt, stroke: 1pt)[] No federal flood assistance has been received for this property.
]

#v(0.3em)

#if flood_fema == "unknown" [
  #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] Landlord does not know / Property recently acquired.
] else [
  #box(width: 12pt, height: 12pt, stroke: 1pt)[] Landlord does not know / Property recently acquired.
]

#if flood_fema == "yes" [
  #v(0.5em)
  Details: #get("fema_details", default: "[Describe assistance received]")
]

#v(1.5em)

// ============================================================================
// RENTER'S INSURANCE WARNING
// ============================================================================

#rect(
  width: 100%,
  inset: 12pt,
  stroke: 2pt + rgb("#b45309"),
  fill: rgb("#fffbeb"),
  radius: 4pt,
)[
  #text(weight: "bold", size: 11pt)[IMPORTANT NOTICE REGARDING RENTER'S INSURANCE]

  #v(0.5em)

  *Standard renter's insurance policies typically DO NOT cover flood damage.*

  Tenants are strongly encouraged to:

  - Verify flood zone status at FEMA's Flood Map Service Center (msc.fema.gov)
  - Consider obtaining separate flood insurance through the National Flood Insurance Program (NFIP) or a private insurer
  - Review their renter's insurance policy to understand what is and is not covered

  #v(0.3em)

  #text(size: 9pt, style: "italic")[
    Flood insurance may be purchased even if the property is not in a high-risk flood zone. Contact your insurance agent for more information.
  ]
]

#v(1.5em)

// ============================================================================
// TENANT'S RIGHTS
// ============================================================================

#rect(
  width: 100%,
  inset: 12pt,
  stroke: 1pt + rgb("#666"),
  fill: rgb("#f0f9ff"),
  radius: 4pt,
)[
  #text(weight: "bold")[TENANT'S RIGHTS UNDER § 83.512]

  #v(0.5em)

  If the Landlord fails to provide this disclosure and the Tenant suffers a loss due to flooding, the Tenant may have the right to:

  - Terminate the lease immediately
  - Seek a full refund of rent paid
  - Pursue damages as provided by law

  #v(0.3em)

  #text(size: 9pt, style: "italic")[
    This disclosure is required by law for all residential leases of one year or longer. Tenants should retain a copy of this signed disclosure for their records.
  ]
]

#v(2em)

// ============================================================================
// ACKNOWLEDGMENT AND SIGNATURES
// ============================================================================

#text(size: 11pt, weight: "bold")[ACKNOWLEDGMENT]

#v(0.5em)

By signing below, both parties certify that:

1. This Flood Disclosure has been provided by the Landlord to the Tenant;
2. The Tenant has received and reviewed this disclosure;
3. This disclosure was provided prior to execution of the Residential Lease Agreement;
4. The information provided by the Landlord is true and accurate to the best of their knowledge.

#v(0.3em)

#text(size: 9pt, style: "italic")[
  This disclosure is made in compliance with Florida Statutes § 83.512 (SB 948/HB 1015).
]

#v(2em)

#grid(
  columns: (1fr, 1fr),
  gutter: 40pt,
  [
    #text(weight: "bold")[LANDLORD]
    #v(1.5em)
    Signature: #box(width: 150pt, repeat[\_])
    #v(0.5em)
    Print Name: #get("landlord_name", default: "[Landlord Name]")
    #v(0.5em)
    Date: #box(width: 100pt, repeat[\_])
  ],
  [
    #text(weight: "bold")[TENANT]
    #v(1.5em)
    Signature: #box(width: 150pt, repeat[\_])
    #v(0.5em)
    Print Name: #get("tenant_name", default: "[Tenant Name]")
    #v(0.5em)
    Date: #box(width: 100pt, repeat[\_])
  ]
)

#v(1em)

// Additional tenant signatures if needed
#if get("additional_tenant_name", default: "") != "" [
  #v(1em)
  #text(weight: "bold")[ADDITIONAL TENANT]
  #v(1.5em)
  Signature: #box(width: 150pt, repeat[\_])
  #v(0.5em)
  Print Name: #get("additional_tenant_name", default: "")
  #v(0.5em)
  Date: #box(width: 100pt, repeat[\_])
]

#v(2em)

// ============================================================================
// FOOTER
// ============================================================================

#line(length: 100%, stroke: 0.5pt)

#v(0.5em)

#text(size: 8pt, fill: rgb("#666"))[
  This Flood Disclosure form is provided in compliance with Florida Statutes § 83.512, effective October 1, 2025. Landlords must provide this disclosure to tenants at or before the execution of any residential lease agreement for a term of one year or longer. Failure to provide this disclosure may subject the landlord to liability if the tenant suffers flood-related losses. Both parties should retain a signed copy of this disclosure.
]
