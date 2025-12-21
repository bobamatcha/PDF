// Texas Residential Lease Agreement Template
// Comprehensive lease compliant with Texas Property Code Chapter 92
// All values are dynamic via sys.inputs

#let data = sys.inputs

// Helper functions
#let get(key, default: "") = data.at(key, default: default)
#let get_bool(key) = {
  let val = data.at(key, default: false)
  if type(val) == str { val == "true" } else { val == true }
}
#let get_num(key, default: 0) = {
  let val = data.at(key, default: default)
  if type(val) == str { float(val) } else { float(val) }
}
#let format_money(amount) = {
  let num = if type(amount) == str { float(amount) } else { float(amount) }
  "$" + str(calc.round(num, digits: 2))
}

// Page setup
#set page(
  paper: "us-letter",
  margin: (top: 1in, bottom: 1in, left: 1in, right: 1in),
  numbering: "1",
  number-align: center,
)
#set text(font: "Liberation Sans", size: 10pt)
#set par(justify: true, leading: 0.65em)

// ============================================================================
// COVER PAGE
// ============================================================================

#align(center)[
  #v(2in)

  #text(size: 24pt, weight: "bold")[RESIDENTIAL LEASE AGREEMENT]

  #v(0.5em)

  #text(size: 14pt)[State of Texas]

  #v(2em)

  #rect(
    width: 80%,
    inset: 20pt,
    stroke: 2pt + black,
    radius: 4pt,
  )[
    #align(center)[
      #text(size: 12pt, weight: "bold")[PROPERTY ADDRESS]
      #v(0.5em)
      #text(size: 14pt)[#get("property_address", default: "[Property Address]")]
      #v(0.3em)
      #text(size: 11pt)[#get("property_city", default: "[City]"), Texas #get("property_zip", default: "[ZIP]")]
    ]
  ]

  #v(3em)

  #grid(
    columns: (1fr, 1fr),
    gutter: 40pt,
    [
      #text(weight: "bold")[LANDLORD]
      #v(0.3em)
      #get("landlord_name", default: "[Landlord Name]")
    ],
    [
      #text(weight: "bold")[TENANT(S)]
      #v(0.3em)
      #get("tenant_name", default: "[Tenant Name]")
    ]
  )

  #v(2em)

  #text(size: 11pt)[
    *Lease Term:* #get("lease_start", default: "[Start Date]") through #get("lease_end", default: "[End Date]")
  ]

  #v(4em)

  #text(size: 9pt, fill: rgb("#666"))[
    This lease agreement is governed by Texas Property Code Chapter 92 (Residential Tenancies)
  ]
]

#pagebreak()

// ============================================================================
// TABLE OF CONTENTS
// ============================================================================

#text(size: 16pt, weight: "bold")[TABLE OF CONTENTS]
#v(1em)

#let toc_item(number, title) = [
  #box(width: 30pt)[#number]
  #title
  #v(0.3em)
]

#toc_item("1.", "BASIC LEASE TERMS")
#toc_item("2.", "RENT AND PAYMENTS")
#toc_item("3.", "SECURITY DEPOSIT (§ 92.103-109)")
#toc_item("4.", "LANDLORD DISCLOSURE (§ 92.201)")
#toc_item("5.", "MAINTENANCE AND REPAIRS (§ 92.056)")
#toc_item("6.", "LOCKOUT POLICY (§ 92.0081)")
#toc_item("7.", "PARKING AND VEHICLES")
#toc_item("8.", "TENANT OBLIGATIONS")
#toc_item("9.", "LANDLORD OBLIGATIONS")
#toc_item("10.", "NOTICES AND TERMINATION")
#toc_item("11.", "DEFAULT AND REMEDIES")
#toc_item("12.", "LEAD-BASED PAINT DISCLOSURE")
#toc_item("13.", "FLOOD DISCLOSURE")
#toc_item("14.", "GENERAL PROVISIONS")
#toc_item("", "SIGNATURE PAGE")
#toc_item("", "ADDENDUM A: PARKING RULES")

#pagebreak()

// ============================================================================
// SECTION 1: BASIC LEASE TERMS
// ============================================================================

#text(size: 14pt, weight: "bold")[1. BASIC LEASE TERMS]
#v(1em)

// 1.1 Parties
#text(size: 12pt, weight: "bold")[1.1 PARTIES TO THIS AGREEMENT]
#v(0.5em)

This Residential Lease Agreement ("Lease") is entered into between:

*LANDLORD:* #get("landlord_name", default: "[Landlord Name]")
#if get("landlord_address") != "" [
  \ Address: #get("landlord_address")
]
#if get("landlord_phone") != "" [
  \ Phone: #get("landlord_phone")
]
#if get("landlord_email") != "" [
  \ Email: #get("landlord_email")
]

*TENANT(S):* #get("tenant_name", default: "[Tenant Name]")
#if get("tenant_phone") != "" [
  \ Phone: #get("tenant_phone")
]
#if get("tenant_email") != "" [
  \ Email: #get("tenant_email")
]

#v(1em)

// 1.2 Property Description
#text(size: 12pt, weight: "bold")[1.2 PROPERTY DESCRIPTION]
#v(0.5em)

The Landlord agrees to lease to the Tenant the following residential property ("Premises"):

*Address:* #get("property_address", default: "[Property Address]")
*City:* #get("property_city", default: "[City]"), Texas #get("property_zip", default: "[ZIP]")
#if get("property_county") != "" [
  *County:* #get("property_county")
]

*Property Type:* #get("property_type", default: "Single Family Residence")
#if get("bedrooms") != "" [
  *Bedrooms:* #get("bedrooms") #h(2em) *Bathrooms:* #get("bathrooms", default: "N/A")
]

#v(1em)

// 1.3 Lease Term
#text(size: 12pt, weight: "bold")[1.3 LEASE TERM]
#v(0.5em)

The lease term begins on *#get("lease_start", default: "[Start Date]")* and ends on *#get("lease_end", default: "[End Date]")*.

This is a fixed-term lease. Upon expiration, the lease will convert to a month-to-month tenancy unless either party provides written notice of non-renewal at least *30 days* before the lease end date or before the next monthly rent due date.

*Month-to-Month Termination:* Either party may terminate a month-to-month tenancy by providing at least *30 days* written notice prior to the next rent due date.

#v(1em)

// ============================================================================
// SECTION 2: RENT AND PAYMENTS
// ============================================================================

#text(size: 14pt, weight: "bold")[2. RENT AND PAYMENTS]
#v(1em)

#text(size: 12pt, weight: "bold")[2.1 RENT]
#v(0.5em)

Monthly rent is *#format_money(get_num("monthly_rent"))*, due on the *#get("rent_due_day", default: "1st")* day of each month.

#table(
  columns: (1fr, 1fr),
  stroke: 0.5pt + rgb("#ccc"),
  inset: 8pt,
  [*Monthly Rent*], [#format_money(get_num("monthly_rent"))],
  [*Due Date*], [#get("rent_due_day", default: "1st") of each month],
  [*Payment Method*], [#get("payment_method", default: "Check, money order, or electronic transfer")],
  [*Payable To*], [#get("landlord_name", default: "[Landlord Name]")],
)

#v(1em)

#text(size: 12pt, weight: "bold")[2.2 LATE FEES]
#v(0.5em)

#let late_fee_pct = get("late_fee_percent", default: "10")
#let grace_days = get("grace_period_days", default: "3")

If rent is not received by the *#grace_days* day of the month, a late fee of *#late_fee_pct%* of the monthly rent (#format_money(get_num("monthly_rent") * float(late_fee_pct) / 100)) will be charged. Texas law requires late fees to be "reasonable" (typically 10-12% is accepted by courts).

#v(1em)

// ============================================================================
// SECTION 3: SECURITY DEPOSIT (§ 92.103-109)
// ============================================================================

#text(size: 14pt, weight: "bold")[3. SECURITY DEPOSIT]
#v(0.5em)
#text(size: 10pt, fill: rgb("#444"))[Texas Property Code §§ 92.103-92.109]
#v(1em)

#text(size: 12pt, weight: "bold")[3.1 DEPOSIT AMOUNT]
#v(0.5em)

Tenant shall pay a security deposit of *#format_money(get_num("security_deposit"))* upon execution of this Lease.

#table(
  columns: (1fr, 1fr),
  stroke: 0.5pt + rgb("#ccc"),
  inset: 8pt,
  [*Security Deposit*], [#format_money(get_num("security_deposit"))],
  [*Due Date*], [Upon lease signing],
  [*Held By*], [#get("deposit_holder", default: "[Landlord/Property Manager]")],
)

#v(1em)

#text(size: 12pt, weight: "bold")[3.2 DEPOSIT RETURN (§ 92.104)]
#v(0.5em)

*The security deposit, less any lawful deductions, will be returned within 30 days of lease termination and Tenant vacating the Premises.*

Lawful deductions may include:
- Unpaid rent
- Damage to the Premises beyond normal wear and tear
- Cleaning costs to restore the Premises to its original condition
- Other breaches of this Lease

*Written Itemization:* If any portion of the deposit is retained, Landlord will provide a written itemization of deductions within 30 days.

#v(1em)

#text(size: 12pt, weight: "bold")[3.3 FORWARDING ADDRESS (§ 92.107)]
#v(0.5em)

*Tenant must provide a forwarding address in writing upon move-out.* Failure to provide a forwarding address may affect the timing and method of deposit return.

#v(1em)

// ============================================================================
// SECTION 4: LANDLORD DISCLOSURE (§ 92.201)
// ============================================================================

#text(size: 14pt, weight: "bold")[4. LANDLORD DISCLOSURE]
#v(0.5em)
#text(size: 10pt, fill: rgb("#444"))[Texas Property Code § 92.201 - Disclosure of Ownership and Management]
#v(1em)

As required by Texas law, the following information is provided:

#table(
  columns: (1fr, 2fr),
  stroke: 0.5pt + rgb("#ccc"),
  inset: 8pt,
  [*Property Owner*], [#get("owner_name", default: get("landlord_name", default: "[Owner Name]"))],
  [*Owner Address*], [#get("owner_address", default: get("landlord_address", default: "[Owner Address]"))],
  [*Property Manager*], [#get("manager_name", default: "Same as Owner")],
  [*Manager Address*], [#get("manager_address", default: "Same as Owner")],
  [*Emergency Contact*], [#get("emergency_phone", default: get("landlord_phone", default: "[Phone Number]"))],
)

#v(1em)

// ============================================================================
// SECTION 5: MAINTENANCE AND REPAIRS (§ 92.056)
// ============================================================================

#text(size: 14pt, weight: "bold")[5. MAINTENANCE AND REPAIRS]
#v(0.5em)
#text(size: 10pt, fill: rgb("#444"))[Texas Property Code § 92.056]
#v(1em)

#text(size: 12pt, weight: "bold")[5.1 REPAIR REQUEST PROCEDURE]
#v(0.5em)

*All repair requests must be submitted in writing* (mail, email, or written notice delivered to Landlord) to:

#get("repair_contact", default: get("landlord_name", default: "[Landlord/Property Manager]"))
#if get("repair_email") != "" [
  \ Email: #get("repair_email")
]
#if get("repair_address") != "" [
  \ Address: #get("repair_address")
]

Landlord will respond within a *reasonable time*, typically:
- Emergency repairs (no water, heat, or security issue): 24 hours
- Non-emergency repairs: 7-14 days

#v(1em)

#text(size: 12pt, weight: "bold")[5.2 LANDLORD'S DUTY TO REPAIR (§ 92.006)]
#v(0.5em)

Landlord is obligated to:
- Maintain the Premises in a habitable condition
- Keep all common areas safe and in good repair
- Repair conditions that materially affect health and safety

*This duty cannot be waived under Texas law.*

#v(1em)

#text(size: 12pt, weight: "bold")[5.3 TENANT'S MAINTENANCE DUTIES]
#v(0.5em)

Tenant shall:
- Keep the Premises clean and sanitary
- Dispose of garbage properly
- Use appliances and fixtures in a reasonable manner
- Not damage or allow damage to the Premises
- Promptly notify Landlord of needed repairs

#v(1em)

// ============================================================================
// SECTION 6: LOCKOUT POLICY (§ 92.0081)
// ============================================================================

#text(size: 14pt, weight: "bold")[6. LOCKOUT POLICY]
#v(0.5em)
#text(size: 10pt, fill: rgb("#444"))[Texas Property Code § 92.0081]
#v(1em)

#rect(
  width: 100%,
  inset: 12pt,
  stroke: 2pt + rgb("#c00"),
  fill: rgb("#fff0f0"),
)[
  #text(weight: "bold", size: 11pt)[
    *IMPORTANT NOTICE REGARDING LOCKOUT FOR NONPAYMENT OF RENT*

    *IF TENANT FAILS TO PAY RENT, LANDLORD MAY CHANGE THE DOOR LOCKS AND DENY ACCESS TO THE PREMISES. Landlord must post a notice on the front door stating the name and address or telephone number of the person to contact regarding reletting the Premises.*

    *Tenant has the right to pay all outstanding rent and fees and receive a key at any time, day or night.*
  ]
]

#v(1em)

// ============================================================================
// SECTION 7: PARKING AND VEHICLES
// ============================================================================

#text(size: 14pt, weight: "bold")[7. PARKING AND VEHICLES]
#v(1em)

#if get_bool("parking_included") [
  Parking is included with this Lease. See *Addendum A: Parking Rules* for detailed parking policies and towing provisions.
] else [
  Parking is not included with this Lease. Tenant is responsible for arranging their own parking.
]

#if get("parking_spaces") != "" [
  *Assigned Parking Spaces:* #get("parking_spaces")
]

*Per Texas Property Code § 92.0131, if vehicles may be towed from the property, specific parking rules must be provided in a separate addendum.* See Addendum A attached to this Lease.

#v(1em)

// ============================================================================
// SECTION 8: TENANT OBLIGATIONS
// ============================================================================

#text(size: 14pt, weight: "bold")[8. TENANT OBLIGATIONS]
#v(1em)

Tenant agrees to:

1. *Pay rent* on time as specified in Section 2
2. *Use the Premises* only as a private residence
3. *Comply with all laws* and property rules
4. *Not engage in criminal activity* at the Premises
5. *Not disturb neighbors* or other tenants
6. *Not alter the Premises* without written consent
7. *Allow Landlord access* for repairs and inspections with reasonable notice (typically 24 hours)
8. *Maintain utilities* as specified in Section 14
9. *Not assign or sublet* without written consent

#v(1em)

// ============================================================================
// SECTION 9: LANDLORD OBLIGATIONS
// ============================================================================

#text(size: 14pt, weight: "bold")[9. LANDLORD OBLIGATIONS]
#v(1em)

Landlord agrees to:

1. *Maintain habitability* of the Premises (§ 92.006)
2. *Make repairs* within a reasonable time after written request (§ 92.056)
3. *Provide essential services* (if applicable)
4. *Respect Tenant's privacy* and provide reasonable notice before entry
5. *Comply with all building codes* and health and safety laws
6. *Return security deposit* within 30 days of lease termination (§ 92.104)
7. *Provide required disclosures* (ownership, lead paint, flood history)

#v(1em)

// ============================================================================
// SECTION 10: NOTICES AND TERMINATION
// ============================================================================

#text(size: 14pt, weight: "bold")[10. NOTICES AND TERMINATION]
#v(1em)

#text(size: 12pt, weight: "bold")[10.1 NOTICE REQUIREMENTS]
#v(0.5em)

All formal notices shall be in writing and delivered to:

*To Landlord:*
#get("landlord_name", default: "[Landlord Name]")
#if get("landlord_address") != "" [
  \ #get("landlord_address")
]

*To Tenant:*
At the Premises address, or forwarding address if provided

#v(1em)

#text(size: 12pt, weight: "bold")[10.2 TERMINATION]
#v(0.5em)

Either party may terminate this Lease:
- *At end of fixed term:* 30 days written notice before lease end date
- *Month-to-month:* 30 days written notice before next rent due date
- *For cause:* As permitted by Texas Property Code Chapter 92

#v(1em)

// ============================================================================
// SECTION 11: DEFAULT AND REMEDIES
// ============================================================================

#text(size: 14pt, weight: "bold")[11. DEFAULT AND REMEDIES]
#v(1em)

#text(size: 12pt, weight: "bold")[11.1 TENANT DEFAULT]
#v(0.5em)

Tenant shall be in default if:
- Rent is not paid when due
- Tenant violates any term of this Lease
- Tenant engages in criminal activity at the Premises

*Notice to Vacate (§ 92.008):* If Tenant is in default, Landlord must provide written notice to vacate. For nonpayment of rent, Landlord must provide at least *3 days* notice to vacate before filing an eviction action.

#v(1em)

#text(size: 12pt, weight: "bold")[11.2 LANDLORD REMEDIES]
#v(0.5em)

Upon Tenant default, Landlord may:
- Terminate the Lease and seek possession
- Sue for unpaid rent and damages
- Change door locks (see Section 6 - Lockout Policy)
- Exercise other remedies allowed by Texas law

#v(1em)

// ============================================================================
// SECTION 12: LEAD-BASED PAINT DISCLOSURE
// ============================================================================

#text(size: 14pt, weight: "bold")[12. LEAD-BASED PAINT DISCLOSURE]
#v(0.5em)
#text(size: 10pt, fill: rgb("#444"))[42 U.S.C. § 4852d - Required for properties built before 1978]
#v(1em)

#let year_built = get_num("year_built", default: 0)
#let is_pre_1978 = year_built > 0 and year_built < 1978

#if is_pre_1978 [
  #rect(
    width: 100%,
    inset: 12pt,
    stroke: 1pt + rgb("#666"),
    fill: rgb("#f9f9f9"),
  )[
    *LEAD-BASED PAINT DISCLOSURE*

    This property was built in *#get("year_built")*, before 1978.

    *Landlord's Disclosure:*
    #if get_bool("lead_paint_known") [
      Lead-based paint and/or lead-based paint hazards are known to be present. Details: #get("lead_paint_details", default: "[Details provided]")
    ] else [
      Landlord has no knowledge of lead-based paint and/or lead-based paint hazards in the housing.
    ]

    *Records and Reports:*
    #if get_bool("lead_reports_available") [
      Landlord has provided the Tenant with all available records and reports pertaining to lead-based paint and/or lead-based paint hazards in the housing.
    ] else [
      Landlord has no reports or records pertaining to lead-based paint and/or lead-based paint hazards in the housing.
    ]

    *Tenant's Acknowledgment:*
    - Tenant has received the EPA pamphlet "Protect Your Family From Lead in Your Home."
    - Tenant has received a 10-day opportunity to conduct a risk assessment or inspection for lead-based paint hazards.
    #if get_bool("lead_inspection_waived") [
      - Tenant has waived the opportunity to conduct a risk assessment or inspection.
    ]
  ]
] else [
  This property was built in #if year_built > 0 [#get("year_built")] else [[year not specified]], which is after 1978. Lead-based paint disclosure is not required for properties built after 1978.
]

#v(1em)

// ============================================================================
// SECTION 13: FLOOD DISCLOSURE
// ============================================================================

#text(size: 14pt, weight: "bold")[13. FLOOD DISCLOSURE]
#v(1em)

#text(size: 12pt, weight: "bold")[13.1 FLOOD HISTORY]
#v(0.5em)

#let has_flooding = get("flood_history_status", default: "unknown")

#if has_flooding == "yes" [
  *Landlord discloses:* The property has experienced flooding that damaged any portion of the improvements during Landlord's ownership.

  *Description:* #get("flooding_description", default: "[Flooding details]")
] else if has_flooding == "no" [
  *Landlord discloses:* To Landlord's knowledge, the property has NOT experienced flooding that damaged any portion of the improvements during Landlord's ownership.
] else [
  *Landlord discloses:* Landlord does not have knowledge of whether the property has experienced flooding. (Property may have been recently acquired.)
]

#v(1em)

#text(size: 12pt, weight: "bold")[13.2 FLOOD INSURANCE CLAIMS]
#v(0.5em)

#let has_claims = get("flood_claims_status", default: "unknown")

#if has_claims == "yes" [
  Flood insurance claims have been filed for this property. Details: #get("flood_claims_details", default: "[Claim details]")
] else if has_claims == "no" [
  No flood insurance claims have been filed for this property during Landlord's ownership.
] else [
  Landlord does not have knowledge of whether flood insurance claims have been filed.
]

#v(1em)

#text(size: 12pt, weight: "bold")[13.3 FLOOD ZONE INFORMATION]
#v(0.5em)

#if get("flood_zone") != "" [
  This property is located in FEMA flood zone: *#get("flood_zone")*

  #if get_bool("flood_insurance_required") [
    *Flood insurance may be required by your lender.*
  ]
]

Tenant is encouraged to verify flood zone status at FEMA's Flood Map Service Center (msc.fema.gov).

#v(1em)

// ============================================================================
// SECTION 14: GENERAL PROVISIONS
// ============================================================================

#text(size: 14pt, weight: "bold")[14. GENERAL PROVISIONS]
#v(1em)

#text(size: 12pt, weight: "bold")[14.1 UTILITIES]
#v(0.5em)

#table(
  columns: (2fr, 1fr, 1fr),
  stroke: 0.5pt + rgb("#ccc"),
  inset: 6pt,
  [*Utility*], [*Landlord*], [*Tenant*],
  [Electric], [#if get_bool("landlord_pays_electric") [X] else []], [#if not get_bool("landlord_pays_electric") [X] else []],
  [Gas], [#if get_bool("landlord_pays_gas") [X] else []], [#if not get_bool("landlord_pays_gas") [X] else []],
  [Water/Sewer], [#if get_bool("landlord_pays_water") [X] else []], [#if not get_bool("landlord_pays_water") [X] else []],
  [Trash], [#if get_bool("landlord_pays_trash") [X] else []], [#if not get_bool("landlord_pays_trash") [X] else []],
  [Internet/Cable], [#if get_bool("landlord_pays_internet") [X] else []], [#if not get_bool("landlord_pays_internet") [X] else []],
)

#v(1em)

#text(size: 12pt, weight: "bold")[14.2 PETS]
#v(0.5em)

#if get_bool("pets_allowed") [
  Pets are allowed with the following conditions:
  - Pet types: #get("pet_types", default: "[Specify allowed pets]")
  - Pet deposit: #format_money(get_num("pet_deposit", default: 0))
  - Monthly pet rent: #format_money(get_num("pet_rent", default: 0))
] else [
  *No pets are allowed* without prior written consent from Landlord.
]

#v(1em)

#text(size: 12pt, weight: "bold")[14.3 SMOKING]
#v(0.5em)

#if get_bool("smoking_allowed") [
  Smoking is permitted only in designated areas.
] else [
  *Smoking is prohibited* on the Premises and in common areas.
]

#v(1em)

#text(size: 12pt, weight: "bold")[14.4 GOVERNING LAW]
#v(0.5em)

This Lease shall be governed by Texas Property Code Chapter 92 and other applicable Texas laws.

#v(1em)

#text(size: 12pt, weight: "bold")[14.5 ENTIRE AGREEMENT]
#v(0.5em)

This Lease, including all attached addenda, constitutes the entire agreement between the parties. Any modifications must be in writing and signed by both parties.

#v(1em)

#text(size: 12pt, weight: "bold")[14.6 SEVERABILITY]
#v(0.5em)

If any provision of this Lease is found to be invalid or unenforceable, the remaining provisions shall continue in full force and effect.

#v(1em)

#text(size: 12pt, weight: "bold")[14.7 VOID CLAUSES NOTICE]
#v(0.5em)

*The following provisions are void under Texas law and cannot be enforced:*
- Any waiver of Landlord's duty to repair (§ 92.006)
- Any waiver of Tenant's right to jury trial (§ 92.0062)
- Any waiver of Tenant's remedies under Chapter 92

#pagebreak()

// ============================================================================
// SIGNATURE PAGE
// ============================================================================

#align(center)[
  #text(size: 16pt, weight: "bold")[SIGNATURE PAGE]
]
#v(1em)

By signing below, the parties agree to all terms and conditions of this Residential Lease Agreement.

#v(2em)

#text(size: 12pt, weight: "bold")[LANDLORD]
#v(1em)

#grid(
  columns: (1fr, 1fr),
  gutter: 20pt,
  [
    Signature: #box(width: 200pt, stroke: (bottom: 0.5pt))
    #v(0.5em)
    Print Name: #get("landlord_name", default: "[Landlord Name]")
  ],
  [
    Date: #box(width: 150pt, stroke: (bottom: 0.5pt))
  ]
)

#v(2em)

#text(size: 12pt, weight: "bold")[TENANT(S)]
#v(1em)

#grid(
  columns: (1fr, 1fr),
  gutter: 20pt,
  [
    Signature: #box(width: 200pt, stroke: (bottom: 0.5pt))
    #v(0.5em)
    Print Name: #get("tenant_name", default: "[Tenant Name]")
  ],
  [
    Date: #box(width: 150pt, stroke: (bottom: 0.5pt))
  ]
)

#if get("additional_tenant_name") != "" [
  #v(1.5em)
  #grid(
    columns: (1fr, 1fr),
    gutter: 20pt,
    [
      Signature: #box(width: 200pt, stroke: (bottom: 0.5pt))
      #v(0.5em)
      Print Name: #get("additional_tenant_name")
    ],
    [
      Date: #box(width: 150pt, stroke: (bottom: 0.5pt))
    ]
  )
]

#pagebreak()

// ============================================================================
// ADDENDUM A: PARKING RULES
// ============================================================================

#align(center)[
  #text(size: 16pt, weight: "bold")[ADDENDUM A: PARKING RULES]
  #v(0.5em)
  #text(size: 10pt, fill: rgb("#444"))[Texas Property Code § 92.0131]
]
#v(1em)

This Parking Rules Addendum is attached to and made a part of the Residential Lease Agreement for the property at:

*#get("property_address", default: "[Property Address]")*

#v(1em)

#text(size: 12pt, weight: "bold")[1. PARKING SPACES]
#v(0.5em)

#if get_bool("parking_included") [
  Tenant is assigned the following parking space(s): #get("parking_spaces", default: "[Space Number(s)]")
] else [
  No parking spaces are assigned under this Lease.
]

#v(1em)

#text(size: 12pt, weight: "bold")[2. VEHICLE REQUIREMENTS]
#v(0.5em)

- Vehicles must be properly registered and insured
- Vehicles must be in operable condition
- No commercial vehicles over 1 ton may be parked on the property
- Recreational vehicles, trailers, and boats require prior written approval

#v(1em)

#text(size: 12pt, weight: "bold")[3. TOWING POLICY]
#v(0.5em)

#rect(
  width: 100%,
  inset: 12pt,
  stroke: 2pt + rgb("#c00"),
  fill: rgb("#fff0f0"),
)[
  #text(weight: "bold")[NOTICE: UNAUTHORIZED VEHICLES MAY BE TOWED]

  Vehicles may be towed at owner's expense if:
  - Parked in unauthorized areas (fire lanes, handicap spaces without permit, reserved spaces)
  - Parked in a manner that blocks other vehicles or common areas
  - Abandoned or inoperable for more than 72 hours
  - Not registered to a current tenant or authorized guest

  *Towing Company:* #get("towing_company", default: "[Towing Company Name]")
  *Phone:* #get("towing_phone", default: "[Towing Company Phone]")
  *Impound Location:* #get("towing_address", default: "[Impound Address]")
]

#v(1em)

#text(size: 12pt, weight: "bold")[4. GUEST PARKING]
#v(0.5em)

Guests may park in designated guest parking areas only. Guest vehicles parked for more than 72 consecutive hours may be towed.

#v(2em)

#text(size: 12pt, weight: "bold")[ACKNOWLEDGMENT]
#v(1em)

By signing below, Tenant acknowledges receipt of this Parking Rules Addendum and agrees to comply with all parking rules.

#v(1.5em)

#grid(
  columns: (1fr, 1fr),
  gutter: 20pt,
  [
    Tenant Signature: #box(width: 180pt, stroke: (bottom: 0.5pt))
  ],
  [
    Date: #box(width: 120pt, stroke: (bottom: 0.5pt))
  ]
)

#v(1em)

#grid(
  columns: (1fr, 1fr),
  gutter: 20pt,
  [
    Landlord Signature: #box(width: 180pt, stroke: (bottom: 0.5pt))
  ],
  [
    Date: #box(width: 120pt, stroke: (bottom: 0.5pt))
  ]
)
