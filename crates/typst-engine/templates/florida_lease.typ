// Florida Residential Lease Agreement Template
// Comprehensive lease compliant with F.S. Chapter 83, § 404.056, and 24 CFR Part 35
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

  #text(size: 14pt)[State of Florida]

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
    This lease agreement is governed by Florida Statutes Chapter 83 (Florida Residential Landlord and Tenant Act)
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

#toc_item("1.", "BASIC TERMS")
#toc_item("  1.1", "Amounts Due at Signing")
#toc_item("  1.2", "Property")
#toc_item("  1.3", "Lease Term")
#toc_item("  1.4", "Rent")
#toc_item("  1.5", "Late Fees")
#toc_item("  1.6", "Utilities")
#toc_item("  1.7", "Parking")
#toc_item("  1.8", "Pets")
#toc_item("  1.9", "Smoking")
#toc_item("  1.10", "Occupants")
#toc_item("  1.11", "Emergency Contact")
#toc_item("  1.12", "Appliances & Furnishings")

#v(0.5em)

#toc_item("2.", "ADDITIONAL TERMS")
#toc_item("  2.1", "Property Condition")
#toc_item("  2.2", "Possession")
#toc_item("  2.3", "Rent Payment")
#toc_item("  2.4", "Security Deposit")
#toc_item("  2.5", "Tenant Obligations")
#toc_item("  2.6", "Landlord Obligations")
#toc_item("  2.7", "Access")
#toc_item("  2.8", "Alterations")
#toc_item("  2.9", "Subletting")
#toc_item("  2.10", "Insurance")
#toc_item("  2.11", "Surrender of Premises")
#toc_item("  2.12", "Default")
#toc_item("  2.13", "Notices")
#toc_item("  2.14", "Governing Law")
#toc_item("  2.15", "Additional Provisions")

#v(0.5em)

#toc_item("3.", "CONTACT INFORMATION")

#v(0.5em)

#toc_item("4.", "SIGNATURES")

#v(0.5em)

#text(weight: "bold")[ADDENDA:]
#v(0.3em)
#toc_item("A.", "Pet Addendum (if applicable)")
#toc_item("B.", "Parking Addendum (if applicable)")
#toc_item("C.", "Rules and Regulations (if applicable)")
#toc_item("D.", "Lead-Based Paint Disclosure (pre-1978 properties)")
#toc_item("E.", "Radon Gas Notification (Required)")
#toc_item("F.", "Security Deposit Disclosure (Required)")
#toc_item("G.", "Electronic Notice Consent (HB 615)")
#toc_item("H.", "Flood Disclosure (§ 83.512)")
#toc_item("I.", "HOA/Condo Association Addendum (if applicable)")
#toc_item("J.", "CDD Disclosure (§ 190.048, if applicable)")
#toc_item("K.", "Liquidated Damages / Early Termination (§ 83.595, optional)")
#toc_item("L.", "Mold Prevention Addendum (optional)")

#pagebreak()

// ============================================================================
// SECTION 1: BASIC TERMS
// ============================================================================

#text(size: 14pt, weight: "bold")[1. BASIC TERMS]
#v(1em)

// 1.1 Amounts Due at Signing
#text(size: 12pt, weight: "bold")[1.1 AMOUNTS DUE AT SIGNING]
#v(0.5em)

#let rent = get_num("monthly_rent")
#let deposit = get_num("security_deposit")
#let pet_dep = get_num("pet_deposit", default: 0)
#let total = rent + deposit + pet_dep

#table(
  columns: (1fr, auto),
  stroke: 0.5pt + rgb("#ccc"),
  inset: 8pt,
  [First Month's Rent], [#format_money(rent)],
  [Security Deposit], [#format_money(deposit)],
  [Pet Deposit], [#format_money(pet_dep)],
  table.hline(stroke: 1pt),
  [*Total Due at Signing*], [*#format_money(total)*],
)

#v(1em)

// 1.2 Property
#text(size: 12pt, weight: "bold")[1.2 PROPERTY]
#v(0.5em)

The Landlord agrees to lease to Tenant(s) the property located at:

*#get("property_address", default: "[Property Address]")*

Property Type: #get("property_type", default: "Residential")

Bedrooms: #get("bedrooms", default: "N/A") #h(2em) Bathrooms: #get("bathrooms", default: "N/A")

#v(1em)

// 1.3 Lease Term
#text(size: 12pt, weight: "bold")[1.3 LEASE TERM]
#v(0.5em)

The lease term begins on *#get("lease_start", default: "[Start Date]")* and ends on *#get("lease_end", default: "[End Date]")*.

This is a fixed-term lease. Upon expiration, the lease will convert to a month-to-month tenancy unless either party provides written notice of non-renewal at least *30 days* before the lease end date, as required by Florida Statutes § 83.57 (amended by HB 1417, 2023).

*Month-to-Month Termination:* Either party may terminate a month-to-month tenancy by providing at least *30 days* written notice prior to the next rent due date, per § 83.57.

#v(1em)

// 1.4 Rent
#text(size: 12pt, weight: "bold")[1.4 RENT]
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

// 1.5 Late Fees
#text(size: 12pt, weight: "bold")[1.5 LATE FEES]
#v(0.5em)

If rent is not received by the *#get("grace_period_days", default: "5th")* day of the month, a late fee of *#format_money(get_num("late_fee", default: 50))* will be charged. Additional late fees of *#format_money(get_num("daily_late_fee", default: 0))* per day may apply thereafter, up to a maximum of #format_money(get_num("max_late_fee", default: 100)).

A fee of #format_money(get_num("nsf_fee", default: 35)) will be charged for any returned check or failed electronic payment.

#v(1em)

// 1.6 Utilities
#text(size: 12pt, weight: "bold")[1.6 UTILITIES]
#v(0.5em)

#let check_landlord(util) = if get(util, default: "tenant") == "landlord" { sym.checkmark } else { "" }
#let check_tenant(util) = if get(util, default: "tenant") == "tenant" { sym.checkmark } else { "" }

#table(
  columns: (1fr, auto, auto),
  stroke: 0.5pt + rgb("#ccc"),
  inset: 6pt,
  align: (left, center, center),
  [*Utility*], [*Landlord Pays*], [*Tenant Pays*],
  [Electricity], [#check_landlord("utility_electric")], [#check_tenant("utility_electric")],
  [Gas], [#check_landlord("utility_gas")], [#check_tenant("utility_gas")],
  [Water/Sewer], [#if get("utility_water", default: "landlord") == "landlord" { sym.checkmark } else { "" }], [#if get("utility_water", default: "landlord") == "tenant" { sym.checkmark } else { "" }],
  [Trash], [#if get("utility_trash", default: "landlord") == "landlord" { sym.checkmark } else { "" }], [#if get("utility_trash", default: "landlord") == "tenant" { sym.checkmark } else { "" }],
  [Internet/Cable], [#check_landlord("utility_internet")], [#check_tenant("utility_internet")],
)

#v(1em)

// 1.7 Parking
#text(size: 12pt, weight: "bold")[1.7 PARKING]
#v(0.5em)

#if get_bool("parking_included") [
  Parking is included with this lease. #get("parking_spaces", default: "1") parking space(s) is/are assigned to this unit.
  Parking location: #get("parking_location", default: "As assigned")
] else [
  Parking is not included with this lease. Street parking may be available.
]

#v(1em)

// 1.8 Pets
#text(size: 12pt, weight: "bold")[1.8 PETS]
#v(0.5em)

#if get_bool("pets_allowed") [
  Pets are allowed with the following restrictions:
  - Maximum number of pets: #get("max_pets", default: "2")
  - Pet types allowed: #get("pet_types_allowed", default: "Dogs and cats")
  - Weight limit: #get("pet_weight_limit", default: "No limit") lbs
  - Pet deposit: #format_money(get_num("pet_deposit", default: 0))
  - Monthly pet rent: #format_money(get_num("pet_rent", default: 0))

  See Pet Addendum for additional terms and conditions.
] else [
  *No pets are allowed* on the premises without prior written consent from the Landlord.
]

#v(1em)

// 1.9 Smoking
#text(size: 12pt, weight: "bold")[1.9 SMOKING]
#v(0.5em)

#if get_bool("smoking_allowed") [
  Smoking is permitted #get("smoking_location", default: "in designated outdoor areas only").
] else [
  *Smoking is strictly prohibited* on the premises, including all indoor and outdoor areas.
]

#v(1em)

// 1.10 Occupants
#text(size: 12pt, weight: "bold")[1.10 OCCUPANTS]
#v(0.5em)

The following persons are authorized to occupy the premises:

#table(
  columns: (1fr, auto),
  stroke: 0.5pt + rgb("#ccc"),
  inset: 6pt,
  [*Name*], [*Relationship*],
  [#get("tenant_name", default: "[Tenant Name]")], [Tenant],
)

No other persons may occupy the premises without prior written consent from the Landlord.

#v(1em)

// 1.11 Emergency Contact
#text(size: 12pt, weight: "bold")[1.11 EMERGENCY CONTACT]
#v(0.5em)

Tenant's emergency contact:

Name: #get("emergency_contact_name", default: "[Emergency Contact]")

Phone: #get("emergency_contact_phone", default: "[Phone Number]")

Relationship: #get("emergency_contact_relationship", default: "[Relationship]")

#v(1em)

// 1.12 Appliances & Furnishings
#text(size: 12pt, weight: "bold")[1.12 APPLIANCES & FURNISHINGS]
#v(0.5em)

The following appliances and furnishings are included with the rental:

- Refrigerator
- Stove/Oven
- Dishwasher
- Microwave
- Washer/Dryer (if applicable)

#pagebreak()

// ============================================================================
// SECTION 2: ADDITIONAL TERMS
// ============================================================================

#text(size: 14pt, weight: "bold")[2. ADDITIONAL TERMS]
#v(1em)

// 2.1 Property Condition
#text(size: 12pt, weight: "bold")[2.1 PROPERTY CONDITION]
#v(0.5em)

Tenant acknowledges receiving the premises in good condition except as noted in the move-in inspection checklist. Tenant agrees to maintain the premises in good condition throughout the lease term and return it in the same condition, normal wear and tear excepted.

#v(1em)

// 2.2 Possession
#text(size: 12pt, weight: "bold")[2.2 POSSESSION]
#v(0.5em)

If Landlord cannot deliver possession on the lease start date due to circumstances beyond Landlord's reasonable control, rent shall be abated on a daily basis until possession is delivered. If possession is not delivered within #get("possession_delay_days", default: "7") days of the start date, Tenant may terminate this lease and receive a full refund of all deposits and prepaid rent.

#v(1em)

// 2.3 Rent Payment
#text(size: 12pt, weight: "bold")[2.3 RENT PAYMENT]
#v(0.5em)

Rent shall be paid to:

*#get("landlord_name", default: "[Landlord Name]")*

#get("payment_address", default: get("landlord_address", default: "[Payment Address]"))

Acceptable payment methods: #get("payment_method", default: "Check, money order, certified check, or electronic transfer")

#v(1em)

// 2.4 Security Deposit
#text(size: 12pt, weight: "bold")[2.4 SECURITY DEPOSIT]
#v(0.5em)

Tenant has deposited *#format_money(get_num("security_deposit"))* as a security deposit. This deposit shall be held in accordance with Florida Statutes § 83.49 and returned as provided therein. See Security Deposit Disclosure Addendum for full details.

The security deposit may be used for:
- Unpaid rent
- Damage beyond normal wear and tear
- Cleaning costs if premises are not left in clean condition
- Other charges as permitted by law

#v(1em)

// 2.5 Tenant Obligations
#text(size: 12pt, weight: "bold")[2.5 TENANT OBLIGATIONS]
#v(0.5em)

Tenant agrees to:
+ Keep the premises clean and sanitary
+ Use all appliances, fixtures, and facilities in a reasonable manner
+ Not destroy, deface, damage, or remove any part of the premises
+ Not disturb neighbors' peaceful enjoyment of their premises
+ Comply with all applicable building, housing, and health codes
+ Dispose of garbage and waste in a clean and sanitary manner
+ Keep plumbing fixtures clean and sanitary
+ Use reasonable efforts to maintain heating, ventilation, and air conditioning
+ Not make alterations without prior written consent
+ Notify Landlord promptly of any conditions requiring repair

#v(1em)

// 2.6 Landlord Obligations
#text(size: 12pt, weight: "bold")[2.6 LANDLORD OBLIGATIONS]
#v(0.5em)

Landlord agrees to:
+ Comply with requirements of applicable building, housing, and health codes
+ Maintain the roof, windows, doors, floors, steps, porches, exterior walls, and foundations in good repair
+ Maintain plumbing in reasonable working condition
+ Provide running water and reasonable amounts of hot water
+ Maintain heating facilities and/or air conditioning in good working order
+ Provide extermination services as required
+ Maintain locks and keys
+ Remove garbage from common areas

#v(1em)

// 2.7 Access
#text(size: 12pt, weight: "bold")[2.7 ACCESS]
#v(0.5em)

Landlord may enter the premises for inspection, repairs, or to show the property to prospective tenants or buyers, with at least #get("access_notice_hours", default: "12") hours advance notice, except in case of emergency. Entry shall be at reasonable times.

#v(1em)

// 2.8 Alterations
#text(size: 12pt, weight: "bold")[2.8 ALTERATIONS]
#v(0.5em)

Tenant shall not make any alterations, additions, or improvements to the premises without prior written consent from Landlord. Any approved alterations become the property of Landlord unless otherwise agreed in writing.

#v(1em)

// 2.9 Subletting
#text(size: 12pt, weight: "bold")[2.9 SUBLETTING]
#v(0.5em)

Subletting or assignment of this lease is not permitted without prior written consent from Landlord.

#v(1em)

// 2.10 Insurance
#text(size: 12pt, weight: "bold")[2.10 INSURANCE]
#v(0.5em)

#if get_bool("renters_insurance_required") [
  Tenant is required to maintain renter's insurance with a minimum coverage of #format_money(get_num("insurance_minimum", default: 100000)) in liability coverage. Proof of insurance must be provided to Landlord before move-in and upon renewal.
] else [
  Landlord's insurance does not cover Tenant's personal property. Tenant is strongly encouraged to obtain renter's insurance to protect personal belongings.
]

#v(1em)

// 2.11 Surrender of Premises
#text(size: 12pt, weight: "bold")[2.11 SURRENDER OF PREMISES]
#v(0.5em)

Upon termination of this lease, Tenant shall:
+ Remove all personal property
+ Return all keys, garage remotes, and access devices
+ Leave the premises in clean condition
+ Provide forwarding address for return of security deposit
+ Allow final inspection by Landlord

#v(1em)

// 2.12 Default
#text(size: 12pt, weight: "bold")[2.12 DEFAULT]
#v(0.5em)

*Tenant Default:* If Tenant fails to pay rent or violates any other term of this lease, Landlord may, after providing notice as required by Florida law, pursue any remedies available under Florida Statutes Chapter 83, including termination of tenancy and eviction proceedings.

*Landlord Default:* If Landlord fails to comply with obligations under this lease or Florida law, Tenant may pursue remedies available under Florida Statutes § 83.56, including rent withholding after proper notice.

#v(1em)

// 2.13 Notices
#text(size: 12pt, weight: "bold")[2.13 NOTICES]
#v(0.5em)

All notices shall be in writing and delivered by:
- Personal delivery
- Certified mail, return receipt requested
- Email (if agreed by both parties)

Notices to Landlord: #get("landlord_address", default: "[Landlord Address]")

Notices to Tenant: #get("property_address", default: "[Property Address]")

#v(1em)

// 2.14 Governing Law
#text(size: 12pt, weight: "bold")[2.14 GOVERNING LAW]
#v(0.5em)

This lease shall be governed by the laws of the State of Florida, specifically Florida Statutes Chapter 83 (Florida Residential Landlord and Tenant Act).

#v(1em)

// 2.15 Additional Provisions
#text(size: 12pt, weight: "bold")[2.15 ADDITIONAL PROVISIONS]
#v(0.5em)

#get("additional_provisions", default: "None.")

#v(1em)

// 2.16 Jury Trial Waiver
#text(size: 12pt, weight: "bold")[2.16 JURY TRIAL WAIVER]
#v(0.5em)

#rect(
  width: 100%,
  inset: 12pt,
  stroke: 2pt + black,
  fill: rgb("#fff3cd"),
  radius: 4pt,
)[
  #text(weight: "bold", size: 11pt)[
    LANDLORD AND TENANT HEREBY KNOWINGLY AND VOLUNTARILY WAIVE THE RIGHT TO A JURY TRIAL FOR ANY ACTION, PROCEEDING, OR COUNTERCLAIM ARISING OUT OF THIS LEASE OR THE RELATIONSHIP BETWEEN THE PARTIES HERETO.
  ]

  #v(0.5em)

  #text(size: 9pt)[
    Both parties acknowledge that this waiver is made voluntarily and with full understanding of its implications. This waiver applies to all disputes, including but not limited to: claims for unpaid rent, security deposit disputes, property damage claims, eviction proceedings, and any other matters arising under this lease.
  ]
]

#v(1em)

// 2.17 Unauthorized Occupants (HB 621)
#text(size: 12pt, weight: "bold")[2.17 UNAUTHORIZED OCCUPANTS (HB 621)]
#v(0.5em)

Only the individuals named in this lease as Tenant(s) are authorized to occupy the premises. No other person may reside at the property without prior written consent from Landlord.

#v(0.5em)

#rect(
  width: 100%,
  inset: 12pt,
  stroke: 2pt + rgb("#dc2626"),
  fill: rgb("#fef2f2"),
  radius: 4pt,
)[
  #text(weight: "bold")[NOTICE PURSUANT TO HB 621 (2024)]

  #v(0.3em)

  Any person occupying the premises without Landlord's prior written consent is hereby declared a *transient occupant* and/or *trespasser* under Florida law (HB 621). Such unauthorized occupants are subject to immediate removal by law enforcement without the need for formal eviction proceedings.

  #v(0.3em)

  Tenant agrees to immediately notify Landlord of any unauthorized persons residing at or frequently staying at the premises. Failure to report unauthorized occupants constitutes a material breach of this lease.
]

#v(1em)

// 2.18 Service Member Rights (§ 83.682)
#text(size: 12pt, weight: "bold")[2.18 SERVICE MEMBER RIGHTS (§ 83.682)]
#v(0.5em)

#rect(
  width: 100%,
  inset: 12pt,
  stroke: 1pt + rgb("#1e40af"),
  fill: rgb("#eff6ff"),
  radius: 4pt,
)[
  #text(weight: "bold")[DISCLOSURE FOR MILITARY TENANTS]

  #v(0.3em)

  Pursuant to Florida Statutes § 83.682, if Tenant is a service member (as defined in § 250.01) and receives orders requiring a permanent change of station to a location more than *35 miles* from the premises, or orders to deploy with a military unit for a period of at least 90 days, Tenant may terminate this lease upon providing:

  #v(0.3em)

  1. Written notice of termination to Landlord;
  2. A copy of the official military orders; and
  3. Payment of rent for 30 days following the next rent due date after notice is delivered.

  #v(0.3em)

  Upon proper termination, Tenant shall not be liable for rent beyond the termination date, and Landlord shall return the security deposit in accordance with § 83.49.

  #v(0.3em)

  #text(size: 9pt, style: "italic")[
    This disclosure is provided for informational purposes as required by Florida law. Service members should consult the full text of § 83.682 and may also have additional rights under the federal Servicemembers Civil Relief Act (SCRA).
  ]
]

#v(1em)

// 2.19 Emotional Support Animal / Assistance Animal Policy (SB 1084)
#text(size: 12pt, weight: "bold")[2.19 EMOTIONAL SUPPORT ANIMAL (ESA) POLICY (SB 1084)]
#v(0.5em)

Florida law (SB 1084, codified at Florida Statute § 817.265) provides protections for both housing providers and individuals with disabilities regarding emotional support animals and assistance animals.

#v(0.5em)

#rect(
  width: 100%,
  inset: 12pt,
  stroke: 2pt + rgb("#7c3aed"),
  fill: rgb("#f5f3ff"),
  radius: 4pt,
)[
  #text(weight: "bold")[DOCUMENTATION REQUIREMENTS]

  #v(0.3em)

  If Tenant requests a reasonable accommodation for an emotional support animal (ESA) or other assistance animal, Landlord may require documentation from a licensed healthcare provider who has *personal knowledge* of the Tenant's disability-related need for the animal.

  #v(0.3em)

  Acceptable documentation must:

  1. Be from a healthcare provider licensed in Florida (or the state where the provider practices);
  2. Be from a provider who has an established therapeutic relationship with the Tenant;
  3. Confirm the provider has *personal knowledge* of the Tenant's disability; and
  4. Verify the disability-related need for the specific animal.

  #v(0.3em)

  #text(size: 9pt, style: "italic")[
    Documentation from online-only services that do not establish an ongoing therapeutic relationship may not satisfy these requirements.
  ]
]

#v(0.5em)

#rect(
  width: 100%,
  inset: 12pt,
  stroke: 2pt + rgb("#dc2626"),
  fill: rgb("#fef2f2"),
  radius: 4pt,
)[
  #text(weight: "bold")[WARNING: FLORIDA STATUTE § 817.265 - FRAUD PENALTIES]

  #v(0.3em)

  Under Florida Statute § 817.265, it is a *second-degree misdemeanor* to:

  - Knowingly provide false information or documentation for the purpose of obtaining an emotional support animal accommodation;
  - Falsely represent that an animal is an emotional support animal or assistance animal;
  - Knowingly provide fraudulent documentation regarding an emotional support animal.

  #v(0.3em)

  Violations may result in:
  - Criminal penalties including fines and potential jail time
  - Mandatory community service hours
  - Liability for damages caused by the animal
]

#v(0.5em)

#text(size: 10pt)[
  *Tenant Acknowledgment:* By signing this lease, Tenant acknowledges understanding of Florida's ESA fraud prevention laws and agrees to provide truthful documentation if requesting an ESA accommodation.
]

#pagebreak()

// ============================================================================
// SECTION 3: CONTACT INFORMATION
// ============================================================================

#text(size: 14pt, weight: "bold")[3. CONTACT INFORMATION]
#v(1em)

#text(size: 12pt, weight: "bold")[LANDLORD / PROPERTY MANAGER]
#v(0.5em)

#table(
  columns: (auto, 1fr),
  stroke: 0.5pt + rgb("#ccc"),
  inset: 8pt,
  [*Name*], [#get("landlord_name", default: "[Landlord Name]")],
  [*Address*], [#get("landlord_address", default: "[Landlord Address]")],
  [*Phone*], [#get("landlord_phone", default: "[Phone Number]")],
  [*Email*], [#get("landlord_email", default: "[Email Address]")],
  [*Emergency Contact*], [#get("landlord_emergency_phone", default: "[Phone Number]")],
)

#v(1em)

#text(size: 12pt, weight: "bold")[TENANT(S)]
#v(0.5em)

#table(
  columns: (auto, 1fr),
  stroke: 0.5pt + rgb("#ccc"),
  inset: 8pt,
  [*Name*], [#get("tenant_name", default: "[Tenant Name]")],
  [*Current Address*], [#get("tenant_address", default: "[Tenant Current Address]")],
  [*Phone*], [#get("tenant_phone", default: "[Phone Number]")],
  [*Email*], [#get("tenant_email", default: "[Email Address]")],
)

#pagebreak()

// ============================================================================
// SECTION 4: SIGNATURES
// ============================================================================

#text(size: 14pt, weight: "bold")[4. SIGNATURES]
#v(1em)

By signing below, the parties agree to all terms and conditions of this Residential Lease Agreement.

#v(2em)

#grid(
  columns: (1fr, 1fr),
  gutter: 40pt,
  [
    #text(weight: "bold")[LANDLORD]

    #v(2em)

    Signature: #box(width: 180pt, repeat[\_])

    #v(0.8em)

    Print Name: #get("landlord_name", default: "[Landlord Name]")

    #v(0.8em)

    Date: #box(width: 120pt, repeat[\_])
  ],
  [
    #text(weight: "bold")[TENANT]

    #v(2em)

    Signature: #box(width: 180pt, repeat[\_])

    #v(0.8em)

    Print Name: #get("tenant_name", default: "[Tenant Name]")

    #v(0.8em)

    Date: #box(width: 120pt, repeat[\_])
  ]
)

#pagebreak()

// ============================================================================
// ADDENDUM A: PET ADDENDUM (Optional)
// ============================================================================

#if get_bool("include_pet_addendum") [
  #text(size: 14pt, weight: "bold")[ADDENDUM A: PET ADDENDUM]
  #v(1em)

  This Pet Addendum is attached to and made part of the Residential Lease Agreement dated #get("lease_start", default: "[Start Date]").

  #v(1em)

  #text(size: 12pt, weight: "bold")[PET INFORMATION]
  #v(0.5em)

  #table(
    columns: (auto, 1fr),
    stroke: 0.5pt + rgb("#ccc"),
    inset: 8pt,
    [*Pet Type*], [#get("pet_1_type", default: "[Type]")],
    [*Breed*], [#get("pet_1_breed", default: "[Breed]")],
    [*Name*], [#get("pet_1_name", default: "[Name]")],
    [*Weight*], [#get("pet_1_weight", default: "[Weight]") lbs],
    [*Age*], [#get("pet_1_age", default: "[Age]")],
    [*Color*], [#get("pet_1_color", default: "[Color]")],
    [*License/Tag \#*], [#get("pet_1_license", default: "[License #]")],
    [*Vaccination Status*], [#get("pet_1_vaccination", default: "Current")],
  )

  #v(1em)

  #text(size: 12pt, weight: "bold")[PET FEES]
  #v(0.5em)

  #table(
    columns: (1fr, auto),
    stroke: 0.5pt + rgb("#ccc"),
    inset: 8pt,
    [Non-refundable Pet Fee], [#format_money(get_num("pet_fee", default: 0))],
    [Refundable Pet Deposit], [#format_money(get_num("pet_deposit", default: 0))],
    [Monthly Pet Rent], [#format_money(get_num("pet_rent", default: 0))],
  )

  #v(1em)

  #text(size: 12pt, weight: "bold")[PET RULES]
  #v(0.5em)

  + Tenant is responsible for all damage caused by the pet(s)
  + Pet(s) must be kept under control at all times
  + Tenant shall immediately clean up after pet(s) in all common areas
  + Pet(s) shall not create a nuisance or disturb neighbors
  + All pets must be properly licensed and vaccinated as required by law
  + Tenant shall not keep any pet offspring beyond #get("pet_offspring_days", default: "90") days
  + Landlord may require removal of any pet that creates a nuisance or danger
  + Only the pets listed above are authorized; no additional pets without prior written consent

  #v(2em)

  #grid(
    columns: (1fr, 1fr),
    gutter: 40pt,
    [
      Landlord Signature: #box(width: 150pt, repeat[\_])
      #v(0.5em)
      Date: #box(width: 100pt, repeat[\_])
    ],
    [
      Tenant Signature: #box(width: 150pt, repeat[\_])
      #v(0.5em)
      Date: #box(width: 100pt, repeat[\_])
    ]
  )

  #pagebreak()
]

// ============================================================================
// ADDENDUM B: PARKING ADDENDUM (Optional)
// ============================================================================

#if get_bool("include_parking_addendum") [
  #text(size: 14pt, weight: "bold")[ADDENDUM B: PARKING ADDENDUM]
  #v(1em)

  This Parking Addendum is attached to and made part of the Residential Lease Agreement dated #get("lease_start", default: "[Start Date]").

  #v(1em)

  #text(size: 12pt, weight: "bold")[ASSIGNED PARKING]
  #v(0.5em)

  #table(
    columns: (auto, 1fr),
    stroke: 0.5pt + rgb("#ccc"),
    inset: 8pt,
    [*Space Number(s)*], [#get("parking_space_numbers", default: "[Space #]")],
    [*Location*], [#get("parking_location", default: "[Location]")],
    [*Type*], [#get("parking_type", default: "Uncovered")],
    [*Monthly Fee*], [#format_money(get_num("parking_fee", default: 0))],
  )

  #v(1em)

  #text(size: 12pt, weight: "bold")[VEHICLE INFORMATION]
  #v(0.5em)

  #table(
    columns: (auto, 1fr),
    stroke: 0.5pt + rgb("#ccc"),
    inset: 8pt,
    [*Vehicle 1 Make/Model*], [#get("vehicle_1_make_model", default: "[Make/Model]")],
    [*Year*], [#get("vehicle_1_year", default: "[Year]")],
    [*Color*], [#get("vehicle_1_color", default: "[Color]")],
    [*License Plate*], [#get("vehicle_1_plate", default: "[Plate #]")],
  )

  #v(1em)

  #text(size: 12pt, weight: "bold")[PARKING RULES]
  #v(0.5em)

  + Only vehicles registered above may use the assigned parking space(s)
  + All vehicles must be currently registered and operable
  + No vehicle repairs, maintenance, or car washing in parking areas
  + No storage of boats, trailers, or recreational vehicles without prior consent
  + Vehicles parked in violation may be towed at owner's expense
  + Speed limit in parking areas is 5 MPH

  #v(2em)

  #grid(
    columns: (1fr, 1fr),
    gutter: 40pt,
    [
      Landlord Signature: #box(width: 150pt, repeat[\_])
      #v(0.5em)
      Date: #box(width: 100pt, repeat[\_])
    ],
    [
      Tenant Signature: #box(width: 150pt, repeat[\_])
      #v(0.5em)
      Date: #box(width: 100pt, repeat[\_])
    ]
  )

  #pagebreak()
]

// ============================================================================
// ADDENDUM C: RULES AND REGULATIONS (Optional)
// ============================================================================

#if get_bool("include_rules_addendum") [
  #text(size: 14pt, weight: "bold")[ADDENDUM C: RULES AND REGULATIONS]
  #v(1em)

  This Rules and Regulations Addendum is attached to and made part of the Residential Lease Agreement dated #get("lease_start", default: "[Start Date]").

  #v(1em)

  #text(size: 12pt, weight: "bold")[GENERAL RULES]
  #v(0.5em)

  + *Quiet Hours:* 10:00 PM to 8:00 AM daily
  + *Common Areas:* Keep hallways, stairways, and common areas clear of personal items
  + *Trash:* All garbage must be placed in designated containers
  + *Grills:* Charcoal grills are prohibited on balconies; gas grills may be used in designated areas only
  + *Window Treatments:* Only white or neutral-colored window coverings visible from outside
  + *Balconies/Patios:* No storage of items other than patio furniture
  + *Laundry:* Laundry may not be hung outside
  + *Guests:* Guests staying more than 7 consecutive days or 14 days in any 30-day period require prior approval
  + *Keys:* Do not make copies of keys; request additional keys from management
  + *Locks:* Do not change locks without prior written consent
  + *Signs:* No signs may be displayed from windows or in common areas
  + *Waterbeds:* Not permitted without prior written consent and proof of insurance

  #v(1em)

  #text(size: 12pt, weight: "bold")[MOVE-IN/MOVE-OUT]
  #v(0.5em)

  + Reserve the elevator/moving areas 48 hours in advance
  + Moving permitted only between 8:00 AM and 6:00 PM
  + Protect floors and walls during move
  + Remove all boxes and packing materials promptly

  #v(2em)

  #grid(
    columns: (1fr, 1fr),
    gutter: 40pt,
    [
      Landlord Signature: #box(width: 150pt, repeat[\_])
      #v(0.5em)
      Date: #box(width: 100pt, repeat[\_])
    ],
    [
      Tenant Signature: #box(width: 150pt, repeat[\_])
      #v(0.5em)
      Date: #box(width: 100pt, repeat[\_])
    ]
  )

  #pagebreak()
]

// ============================================================================
// ADDENDUM D: LEAD-BASED PAINT DISCLOSURE (Pre-1978 Properties)
// ============================================================================

#if get_bool("property_built_before_1978") [
  #text(size: 14pt, weight: "bold")[ADDENDUM D: LEAD-BASED PAINT DISCLOSURE]
  #v(0.5em)
  #text(size: 10pt, style: "italic")[Required for housing built before 1978 (24 CFR Part 35)]
  #v(1em)

  #rect(
    width: 100%,
    inset: 12pt,
    stroke: 2pt + rgb("#cc0000"),
    fill: rgb("#fff5f5"),
    radius: 4pt,
  )[
    #text(weight: "bold", size: 11pt)[IMPORTANT NOTICE]

    Housing built before 1978 may contain lead-based paint. Lead from paint, paint chips, and dust can pose health hazards if not managed properly. Lead exposure is especially harmful to young children and pregnant women.
  ]

  #v(1em)

  #text(size: 12pt, weight: "bold")[LANDLORD'S DISCLOSURE]
  #v(0.5em)

  (a) Presence of lead-based paint and/or lead-based paint hazards:

  #box(width: 12pt, height: 12pt, stroke: 1pt)[] Known lead-based paint and/or lead-based paint hazards are present

  #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] Landlord has no knowledge of lead-based paint and/or lead-based paint hazards

  #v(1em)

  (b) Records and reports available to the Landlord:

  #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] Landlord has no reports or records pertaining to lead-based paint

  #v(1em)

  #text(size: 12pt, weight: "bold")[TENANT'S ACKNOWLEDGMENT]
  #v(0.5em)

  (c) Tenant has received the following:

  #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] The pamphlet "Protect Your Family From Lead in Your Home"

  #v(0.5em)

  (d) Tenant has received a 10-day opportunity (or mutually agreed upon period) to conduct a risk assessment or inspection for lead-based paint:

  #if get_bool("lead_inspection_waived") [
    #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] Tenant has waived the opportunity to conduct a risk assessment or inspection
  ] else [
    #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] Tenant has received the 10-day opportunity
  ]

  #v(1em)

  #text(size: 12pt, weight: "bold")[CERTIFICATION]
  #v(0.5em)

  The parties certify that the information provided is true and accurate to the best of their knowledge.

  #v(2em)

  #grid(
    columns: (1fr, 1fr),
    gutter: 20pt,
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

  #pagebreak()
]

// ============================================================================
// ADDENDUM E: RADON GAS NOTIFICATION (MANDATORY - F.S. § 404.056)
// ============================================================================

#text(size: 14pt, weight: "bold")[ADDENDUM E: RADON GAS NOTIFICATION]
#v(0.5em)
#text(size: 10pt, style: "italic")[Required by Florida Statutes § 404.056]
#v(1em)

#rect(
  width: 100%,
  inset: 15pt,
  stroke: 2pt + rgb("#0066cc"),
  fill: rgb("#f0f8ff"),
  radius: 4pt,
)[
  #text(weight: "bold", size: 12pt)[RADON GAS NOTIFICATION]

  #v(1em)

  Radon is a naturally occurring radioactive gas that, when it has accumulated in a building in sufficient quantities, may present health risks to persons who are exposed to it over time. Levels of radon that exceed federal and state guidelines have been found in buildings in Florida. Additional information regarding radon and radon testing may be obtained from your county health department.

  #v(1em)

  #text(size: 9pt, style: "italic")[
    This notification is required by Florida Statutes § 404.056(5) to be included in all residential lease agreements in Florida.
  ]
]

#v(2em)

#text(size: 11pt, weight: "bold")[ACKNOWLEDGMENT]

By signing below, Tenant acknowledges receipt of the above Radon Gas Notification as required by Florida law.

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

#pagebreak()

// ============================================================================
// ADDENDUM F: SECURITY DEPOSIT DISCLOSURE (MANDATORY - F.S. § 83.49)
// ============================================================================

#text(size: 14pt, weight: "bold")[ADDENDUM F: SECURITY DEPOSIT DISCLOSURE]
#v(0.5em)
#text(size: 10pt, style: "italic")[Required by Florida Statutes § 83.49]
#v(1em)

#text(size: 12pt, weight: "bold")[SECURITY DEPOSIT AMOUNT]
#v(0.5em)

#table(
  columns: (1fr, auto),
  stroke: 0.5pt + rgb("#ccc"),
  inset: 10pt,
  [Security Deposit], [*#format_money(get_num("security_deposit"))*],
  [Pet Deposit], [*#format_money(get_num("pet_deposit", default: 0))*],
)

#v(1em)

#text(size: 12pt, weight: "bold")[METHOD OF HOLDING DEPOSIT]

#v(0.5em)

Pursuant to Florida Statutes § 83.49(1), the Landlord holds the security deposit in the following manner:

#v(0.5em)

#let deposit_method = get("deposit_method", default: "separate")

#if deposit_method == "separate" [
  #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] In a separate non-interest-bearing account in a Florida banking institution

  #box(width: 12pt, height: 12pt, stroke: 1pt)[] In a separate interest-bearing account in a Florida banking institution

  #box(width: 12pt, height: 12pt, stroke: 1pt)[] Posted in the form of a surety bond
] else if deposit_method == "interest" [
  #box(width: 12pt, height: 12pt, stroke: 1pt)[] In a separate non-interest-bearing account in a Florida banking institution

  #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] In a separate interest-bearing account in a Florida banking institution

  #box(width: 12pt, height: 12pt, stroke: 1pt)[] Posted in the form of a surety bond
] else [
  #box(width: 12pt, height: 12pt, stroke: 1pt)[] In a separate non-interest-bearing account in a Florida banking institution

  #box(width: 12pt, height: 12pt, stroke: 1pt)[] In a separate interest-bearing account in a Florida banking institution

  #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] Posted in the form of a surety bond
]

#v(0.5em)

*Depository Name:* #get("deposit_bank_name", default: "[Bank Name]")

*Depository Address:* #get("deposit_bank_address", default: "[Bank Address]")

#v(1em)

#text(size: 12pt, weight: "bold")[STATUTORY RIGHTS AND OBLIGATIONS]
#v(0.5em)

#rect(
  width: 100%,
  inset: 12pt,
  stroke: 1pt + rgb("#666"),
  fill: rgb("#f9f9f9"),
  radius: 4pt,
)[
  Pursuant to Florida Statutes § 83.49:

  #v(0.5em)

  *Return of Deposit:* Upon termination of the lease and vacation of the premises, the Landlord shall return the security deposit together with interest if required, or shall provide written notice of intention to impose a claim on the deposit:

  - *Within 15 days* if no claim is made against the deposit
  - *Within 30 days* if a claim is made, with itemized written notice sent by certified mail

  #v(0.5em)

  *Tenant's Right to Object:* If the Landlord sends notice of a claim, the Tenant has 15 days from receipt to object. If Tenant does not object within 15 days, the Landlord may deduct the claimed amount.

  #v(0.5em)

  *Permissible Deductions:* The security deposit may be used for unpaid rent, damage to the premises beyond normal wear and tear, and other charges properly due under the lease.
]

#v(1em)

#text(size: 12pt, weight: "bold")[FORWARDING ADDRESS]
#v(0.5em)

Tenant must provide a forwarding address in writing within 10 days after vacating the premises to receive return of the security deposit.

#v(2em)

#text(size: 11pt, weight: "bold")[ACKNOWLEDGMENT]

By signing below, Tenant acknowledges receipt of this Security Deposit Disclosure as required by Florida Statutes § 83.49.

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

#pagebreak()

// ============================================================================
// ADDENDUM G: ELECTRONIC NOTICE CONSENT (HB 615 - § 83.56)
// ============================================================================

#text(size: 14pt, weight: "bold")[ADDENDUM G: ELECTRONIC NOTICE CONSENT]
#v(0.5em)
#text(size: 10pt, style: "italic")[Pursuant to Florida Statutes § 83.56 as amended by HB 615]
#v(1em)

#rect(
  width: 100%,
  inset: 15pt,
  stroke: 2pt + rgb("#0066cc"),
  fill: rgb("#f0f8ff"),
  radius: 4pt,
)[
  #text(weight: "bold", size: 12pt)[CONSENT TO ELECTRONIC DELIVERY OF NOTICES]

  #v(1em)

  Florida law (HB 615, amending F.S. § 83.56) permits landlords and tenants to agree to the electronic delivery of legally required notices, including but not limited to:

  #v(0.5em)

  - Three (3) day notices for nonpayment of rent
  - Seven (7) day notices for lease violations
  - Notices of termination or non-renewal
  - Notices regarding security deposit claims
  - Any other notices required under Florida Statutes Chapter 83

  #v(1em)

  #text(weight: "bold")[This consent is OPTIONAL.] If Tenant declines, all notices will be delivered via personal delivery or certified mail as required by law.
]

#v(1em)

// NOTE: Email consent is signed by TENANT during signature ceremony,
// not pre-filled by landlord. Per HB 615, tenant must actively consent.
// Both checkboxes left unchecked - tenant checks one during signing.

#text(size: 12pt, weight: "bold")[TENANT'S ELECTION]
#v(0.5em)

#text(style: "italic", size: 10pt)[
  (Tenant: Please check ONE option below during signing)
]
#v(0.5em)

#box(width: 12pt, height: 12pt, stroke: 1pt)[] *I CONSENT* to receive all legally required notices via electronic mail.

#v(0.5em)

#box(width: 12pt, height: 12pt, stroke: 1pt)[] *I DECLINE* and require notices via personal delivery or certified mail.

#v(1em)

#text(size: 12pt, weight: "bold")[TENANT EMAIL ADDRESS FOR NOTICES]
#v(0.5em)

Email: #get("tenant_email", default: "[Tenant Email Address]")

#v(0.5em)

#text(size: 9pt, style: "italic")[
  By providing an email address above and consenting to electronic notices, Tenant agrees that delivery of any legally required notice to this email address constitutes valid delivery under Florida law. Tenant is responsible for maintaining access to this email address and promptly notifying Landlord of any changes.
]

#v(1em)

#text(size: 12pt, weight: "bold")[LANDLORD EMAIL ADDRESS FOR NOTICES]
#v(0.5em)

Email: #get("landlord_email", default: "[Landlord Email Address]")

#v(2em)

#text(size: 11pt, weight: "bold")[ACKNOWLEDGMENT]

By signing below, both parties acknowledge and agree to the electronic notice provisions selected above pursuant to Florida Statutes § 83.56 as amended by HB 615.

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

#pagebreak()

// ============================================================================
// ADDENDUM H: FLOOD DISCLOSURE (SB 948 - § 83.512)
// ============================================================================

#text(size: 14pt, weight: "bold")[ADDENDUM H: FLOOD DISCLOSURE]
#v(0.5em)
#text(size: 10pt, style: "italic")[Required by Florida Statutes § 83.512 (SB 948)]
#v(1em)

#rect(
  width: 100%,
  inset: 15pt,
  stroke: 2pt + rgb("#dc2626"),
  fill: rgb("#fef2f2"),
  radius: 4pt,
)[
  #text(weight: "bold", size: 12pt)[MANDATORY FLOOD DISCLOSURE]

  #v(1em)

  Pursuant to Florida Statutes § 83.512, the Landlord is required to disclose to the Tenant, prior to execution of a residential lease for a term of one year or longer, the following information regarding the property located at:

  #v(0.5em)

  #text(weight: "bold")[#get("property_address", default: "[Property Address]")]
]

#v(1em)

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

#v(1em)

#rect(
  width: 100%,
  inset: 12pt,
  stroke: 1pt + rgb("#666"),
  fill: rgb("#fffbeb"),
  radius: 4pt,
)[
  #text(weight: "bold")[TENANT'S RIGHTS UNDER § 83.512]

  #v(0.5em)

  If the Landlord fails to provide this disclosure and the Tenant suffers a loss due to flooding, the Tenant may have the right to:

  - Terminate the lease immediately
  - Seek a full refund of rent paid
  - Pursue damages as provided by law

  #v(0.5em)

  #text(size: 9pt, style: "italic")[
    Tenants are encouraged to independently verify flood zone status at FEMA's Flood Map Service Center (msc.fema.gov) and consider obtaining renter's flood insurance.
  ]
]

#v(2em)

#text(size: 11pt, weight: "bold")[ACKNOWLEDGMENT]

By signing below, both parties certify that this Flood Disclosure has been provided and received prior to execution of the Residential Lease Agreement, as required by Florida Statutes § 83.512.

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

// ============================================================================
// ADDENDUM I: HOA/CONDO ASSOCIATION ADDENDUM (Optional - Properties in HOA/Condo)
// Per FL_LEASE.md §3.1: "Association Supremacy Clause", indemnity, approval contingency
// ============================================================================

#if get_bool("has_hoa") or get_bool("has_association") [
  #pagebreak()

  #text(size: 14pt, weight: "bold")[ADDENDUM I: HOA/CONDO ASSOCIATION ADDENDUM]

  #v(0.5em)

  This HOA/Condominium Association Addendum is attached to and made part of the Residential Lease Agreement dated #get("lease_start", default: "[Start Date]") for the property located at #get("property_address", default: "[Property Address]").

  #v(1em)

  #text(weight: "bold")[1. ASSOCIATION IDENTIFICATION]

  #v(0.5em)

  The Property is located within the following Homeowners Association or Condominium Association:

  #v(0.3em)

  *Association Name:* #get("hoa_name", default: "[Association Name]")

  *Management Company (if any):* #get("hoa_management", default: "[Management Company]")

  *Contact Information:* #get("hoa_contact", default: "[Contact Phone/Email]")

  #v(1em)

  // ========================================================================
  // ASSOCIATION SUPREMACY CLAUSE
  // Per FL_LEASE.md: Lease must explicitly subordinate itself to Association
  // ========================================================================

  #rect(
    width: 100%,
    inset: 12pt,
    stroke: 2pt + black,
    radius: 4pt,
  )[
    #text(weight: "bold", size: 11pt)[2. ASSOCIATION SUPREMACY CLAUSE]

    #v(0.5em)

    *SUBORDINATION TO GOVERNING DOCUMENTS:* This Lease Agreement is subordinate to the Declaration of Covenants, Conditions, and Restrictions (CC&Rs), Bylaws, Rules and Regulations, and all other governing documents of the Association (collectively, the "Governing Documents").

    #v(0.5em)

    *TENANT COMPLIANCE REQUIRED:* Tenant agrees to abide by all Governing Documents of the Association. *Any violation of Association rules by Tenant constitutes a material breach of this Lease* and grounds for termination.

    #v(0.5em)

    *ASSOCIATION AS THIRD-PARTY BENEFICIARY:* The Association is a third-party beneficiary of this Lease with respect to Tenant's compliance with the Governing Documents.
  ]

  #v(1em)

  // ========================================================================
  // EVICTION BY ASSOCIATION INDEMNITY
  // Per FL_LEASE.md: Under Ch 718, Associations can evict tenant for rule violations
  // ========================================================================

  #text(weight: "bold")[3. INDEMNIFICATION FOR ASSOCIATION ACTIONS]

  #v(0.5em)

  Under Florida Statutes Chapter 718 (Condominiums) and Chapter 720 (Homeowners Associations), the Association has independent authority to pursue eviction or levy fines against Tenant for violations of the Governing Documents.

  #v(0.5em)

  *TENANT INDEMNITY:* Tenant agrees to indemnify and hold harmless Landlord from and against any and all costs, expenses, fines, assessments, legal fees, and other damages incurred by Landlord as a result of:

  - Tenant's violation of any Governing Document
  - Any Association-initiated eviction action against Tenant
  - Any fines or penalties assessed by the Association due to Tenant's conduct

  #v(1em)

  // ========================================================================
  // APPROVAL CONTINGENCY
  // Per FL_LEASE.md: Many Associations require tenant approval
  // ========================================================================

  #text(weight: "bold")[4. ASSOCIATION APPROVAL CONTINGENCY]

  #v(0.5em)

  #if get_bool("hoa_approval_required") [
    This Lease is *contingent upon approval of Tenant by the Association*.

    #v(0.3em)

    - Landlord shall submit Tenant's application to the Association within *5 business days* of lease execution
    - If the Association denies Tenant's application, this Lease shall automatically terminate
    - Upon termination due to Association denial, all deposits shall be returned to Tenant within 10 days
    - Neither party shall have any further liability to the other
  ] else [
    #box(width: 12pt, height: 12pt, stroke: 1pt)[] Association approval is required for this tenancy.

    #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] Association approval is NOT required, or Tenant has already been approved.
  ]

  #v(1em)

  #text(weight: "bold")[5. ASSOCIATION FEES AND ASSESSMENTS]

  #v(0.5em)

  *Monthly HOA/Condo Fees:* #if get("hoa_monthly_fee", default: "") != "" [#format_money(get_num("hoa_monthly_fee"))] else [[Amount]]

  *Responsibility:* #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] Landlord #box(width: 12pt, height: 12pt, stroke: 1pt)[] Tenant

  *Special Assessments:* Any special assessments levied by the Association during the Lease term shall be the responsibility of the *Landlord*, unless caused by Tenant's conduct.

  #v(1em)

  #text(weight: "bold")[6. ACCESS FOR INSPECTIONS]

  #v(0.5em)

  Tenant grants Landlord and the Association reasonable access to the Premises for inspections required by:
  - The Governing Documents
  - Florida Statutes (including structural inspections under HB 1021)
  - Local building codes

  Landlord shall provide at least 24 hours' notice except in emergencies.

  #v(1em)

  #text(weight: "bold")[7. RECEIPT OF GOVERNING DOCUMENTS]

  #v(0.5em)

  #box(width: 12pt, height: 12pt, stroke: 1pt)[] Tenant acknowledges receipt of a copy of the Association's Rules and Regulations.

  #box(width: 12pt, height: 12pt, stroke: 1pt)[] Tenant acknowledges that Rules and Regulations are available at: #get("hoa_rules_url", default: "[Association Website or Office]")

  #v(2em)

  #grid(
    columns: (1fr, 1fr),
    gutter: 40pt,
    [
      #text(weight: "bold")[LANDLORD]
      #v(1.5em)
      Signature: #box(width: 150pt, repeat[\_])
      #v(0.5em)
      Date: #box(width: 100pt, repeat[\_])
    ],
    [
      #text(weight: "bold")[TENANT]
      #v(1.5em)
      Signature: #box(width: 150pt, repeat[\_])
      #v(0.5em)
      Date: #box(width: 100pt, repeat[\_])
    ]
  )
]

// ============================================================================
// ADDENDUM J: CDD DISCLOSURE (§ 190.048)
// Community Development District Disclosure - Required for properties in CDD
// Boldfaced text required per Florida Statutes
// ============================================================================

#if get_bool("in_cdd") [
  #pagebreak()

  #text(size: 14pt, weight: "bold")[ADDENDUM J: COMMUNITY DEVELOPMENT DISTRICT (CDD) DISCLOSURE]

  #v(0.5em)

  This Community Development District Disclosure is attached to and made part of the Residential Lease Agreement dated #get("lease_start", default: "[Start Date]") for the property located at #get("property_address", default: "[Property Address]").

  #v(1em)

  #text(weight: "bold")[PURSUANT TO FLORIDA STATUTES § 190.048]

  #v(1em)

  // ========================================================================
  // MANDATORY CDD DISCLOSURE - BOLDFACED AS REQUIRED BY STATUTE
  // ========================================================================

  #rect(
    width: 100%,
    inset: 15pt,
    stroke: 3pt + black,
    fill: rgb("#fff3cd"),
    radius: 4pt,
  )[
    #text(weight: "bold", size: 12pt)[
      THE #upper(get("cdd_name", default: "[CDD NAME]")) COMMUNITY DEVELOPMENT DISTRICT MAY IMPOSE AND LEVY TAXES OR ASSESSMENTS, OR BOTH TAXES AND ASSESSMENTS, ON THIS PROPERTY.

      #v(0.5em)

      THESE TAXES AND ASSESSMENTS PAY THE CONSTRUCTION, OPERATION, AND MAINTENANCE COSTS OF CERTAIN PUBLIC FACILITIES AND SERVICES OF THE DISTRICT AND ARE SET ANNUALLY BY THE GOVERNING BOARD OF THE DISTRICT.

      #v(0.5em)

      THESE ASSESSMENTS ARE IN ADDITION TO COUNTY AND OTHER LOCAL GOVERNMENTAL TAXES AND ASSESSMENTS AND ALL OTHER TAXES AND ASSESSMENTS PROVIDED FOR BY LAW.
    ]
  ]

  #v(1em)

  #text(weight: "bold")[1. CDD IDENTIFICATION]

  #v(0.5em)

  *Community Development District Name:* #get("cdd_name", default: "[CDD Name]")

  *CDD Contact/Management:* #get("cdd_contact", default: "[CDD Manager Contact]")

  #v(1em)

  #text(weight: "bold")[2. CURRENT CDD ASSESSMENT AMOUNTS]

  #v(0.5em)

  *Annual CDD Assessment:* #if get("cdd_assessment", default: "") != "" [#format_money(get_num("cdd_assessment"))] else [[Current Annual Amount]]

  *Monthly Equivalent:* #if get("cdd_assessment", default: "") != "" [#format_money(get_num("cdd_assessment") / 12)] else [[Monthly Amount]]

  *Assessment Responsibility:*
  - Debt Service (Bond Payments): #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] Landlord
  - Operations & Maintenance: #box(width: 12pt, height: 12pt, stroke: 1pt, inset: 2pt)[#sym.checkmark] Landlord

  #v(0.5em)

  #text(size: 9pt, style: "italic")[
    Note: CDD assessments typically appear on the annual property tax bill. The Landlord is responsible for these assessments unless otherwise agreed in writing.
  ]

  #v(1em)

  #text(weight: "bold")[3. NATURE OF CDD ASSESSMENTS]

  #v(0.5em)

  CDD assessments fund:
  - *Infrastructure:* Roads, water management, utilities
  - *Amenities:* Parks, pools, clubhouses, landscaping
  - *Maintenance:* Common area upkeep, security

  #v(0.5em)

  #rect(
    width: 100%,
    inset: 10pt,
    stroke: 1pt + rgb("#666"),
    radius: 4pt,
  )[
    #text(weight: "bold")[IMPORTANT NOTICE:]

    CDD assessments are *NOT rent*. They are governmental assessments that run with the property. Unlike HOA fees, CDD assessments:
    - Are collected through the County Tax Collector
    - Are secured by a lien on the property
    - May increase or decrease based on District needs
  ]

  #v(1em)

  #text(weight: "bold")[4. TENANT ACKNOWLEDGMENT]

  #v(0.5em)

  Tenant acknowledges and understands:

  #box(width: 12pt, height: 12pt, stroke: 1pt)[] The Property is located within a Community Development District

  #box(width: 12pt, height: 12pt, stroke: 1pt)[] CDD assessments are in addition to other property taxes

  #box(width: 12pt, height: 12pt, stroke: 1pt)[] CDD assessment amounts may change annually

  #box(width: 12pt, height: 12pt, stroke: 1pt)[] Tenant has been informed of the current CDD assessment amount

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
]

// ============================================================================
// ADDENDUM K: LIQUIDATED DAMAGES / EARLY TERMINATION (§ 83.595)
// "Safe Harbor" Early Termination Fee - Requires Separate Signature
// Per FL_LEASE.md §6.2: Max 2 months rent, specific bold language required
// ============================================================================

#if get_bool("early_termination_fee") [
  #pagebreak()

  #text(size: 14pt, weight: "bold")[ADDENDUM K: LIQUIDATED DAMAGES / EARLY TERMINATION FEE]

  #v(0.5em)

  #text(size: 10pt, style: "italic")[Pursuant to Florida Statutes § 83.595]

  #v(1em)

  This Liquidated Damages Addendum ("Addendum") is attached to and made part of the Residential Lease Agreement dated #get("lease_start", default: "[Start Date]") between:

  #v(0.5em)

  *Landlord:* #get("landlord_name", default: "[Landlord Name]")

  *Tenant:* #get("tenant_name", default: "[Tenant Name]")

  *Property:* #get("property_address", default: "[Property Address]")

  #v(1em)

  // ========================================================================
  // STATUTORY BACKGROUND
  // ========================================================================

  #rect(
    width: 100%,
    inset: 12pt,
    stroke: 1pt + rgb("#666"),
    fill: rgb("#f0f9ff"),
    radius: 4pt,
  )[
    #text(weight: "bold")[FLORIDA STATUTES § 83.595 - LIQUIDATED DAMAGES]

    #v(0.5em)

    Florida law provides a "safe harbor" for lease termination fees. Under § 83.595, if a tenant wishes to terminate the lease early, both parties may agree to a fixed fee (not exceeding two months' rent) as liquidated damages, in lieu of the landlord pursuing claims for unpaid rent through the end of the lease term.
  ]

  #v(1em)

  // ========================================================================
  // EARLY TERMINATION FEE AMOUNT
  // ========================================================================

  #text(weight: "bold")[1. EARLY TERMINATION FEE AMOUNT]

  #v(0.5em)

  The Early Termination Fee under this Addendum is:

  #v(0.5em)

  #let monthly_rent_val = get_num("monthly_rent")
  #let custom_fee = get("early_termination_amount", default: "")

  #text(size: 12pt, weight: "bold")[
    #if custom_fee != "" [
      #format_money(get_num("early_termination_amount"))
    ] else [
      #format_money(monthly_rent_val * 2)
    ]
  ]

  #v(0.5em)

  #text(size: 10pt, style: "italic")[
    (This amount does not exceed two months' rent as required by § 83.595)
  ]

  #v(1em)

  // ========================================================================
  // REQUIRED TENANT ACKNOWLEDGMENT (BOLD TEXT PER STATUTE)
  // ========================================================================

  #text(weight: "bold")[2. TENANT ACKNOWLEDGMENT]

  #v(0.5em)

  #rect(
    width: 100%,
    inset: 15pt,
    stroke: 3pt + rgb("#dc2626"),
    fill: rgb("#fef2f2"),
    radius: 4pt,
  )[
    #text(weight: "bold", size: 11pt)[
      I, THE TENANT, AGREE, AS PROVIDED IN THE RENTAL AGREEMENT, TO PAY THE EARLY TERMINATION FEE STATED ABOVE (AN AMOUNT THAT DOES NOT EXCEED TWO (2) MONTHS' RENT) AS LIQUIDATED DAMAGES IN THE EVENT I ELECT TO TERMINATE THIS LEASE BEFORE THE END OF THE LEASE TERM.

      #v(0.5em)

      I UNDERSTAND THAT:

      #v(0.3em)

      1. THIS FEE IS IN LIEU OF THE LANDLORD PURSUING CLAIMS FOR UNPAID RENT THROUGH THE END OF THE LEASE TERM;

      #v(0.2em)

      2. I MUST PROVIDE PROPER WRITTEN NOTICE OF MY INTENT TO TERMINATE;

      #v(0.2em)

      3. THE EARLY TERMINATION FEE MUST BE PAID IN FULL BEFORE OR UPON VACATING THE PREMISES;

      #v(0.2em)

      4. THIS ADDENDUM DOES NOT RELIEVE ME OF OTHER OBLIGATIONS UNDER THE LEASE, INCLUDING LEAVING THE PREMISES IN GOOD CONDITION AND PAYING ALL RENT DUE THROUGH THE TERMINATION DATE.
    ]
  ]

  #v(1em)

  // ========================================================================
  // TERMS AND CONDITIONS
  // ========================================================================

  #text(weight: "bold")[3. TERMS AND CONDITIONS FOR EARLY TERMINATION]

  #v(0.5em)

  To exercise early termination under this Addendum, Tenant must:

  #v(0.3em)

  a) Provide Landlord with written notice of intent to terminate at least *30 days* before the desired termination date;

  #v(0.2em)

  b) Pay the Early Termination Fee in full at the time of notice or as otherwise agreed in writing;

  #v(0.2em)

  c) Continue to pay rent through the termination date;

  #v(0.2em)

  d) Return possession of the premises in the condition required by the Lease (normal wear and tear excepted);

  #v(0.2em)

  e) Return all keys and access devices to Landlord.

  #v(1em)

  #text(weight: "bold")[4. EFFECT OF EARLY TERMINATION]

  #v(0.5em)

  Upon satisfaction of all conditions in Section 3:

  #v(0.3em)

  - Landlord releases Tenant from liability for rent for the remainder of the original lease term;
  - Landlord waives the right to pursue claims for unpaid rent beyond the termination date;
  - Tenant's security deposit will be handled in accordance with Florida Statutes § 83.49.

  #v(1em)

  #text(weight: "bold")[5. LANDLORD'S DUTY TO MITIGATE]

  #v(0.5em)

  #text(size: 10pt, style: "italic")[
    Note: If Tenant does not exercise early termination under this Addendum and simply vacates, Landlord retains all rights under Florida law, including the duty to mitigate damages by making reasonable efforts to re-rent the property. In such case, Tenant may be liable for rent until the property is re-rented or the lease term expires, whichever occurs first.
  ]

  #v(2em)

  // ========================================================================
  // SEPARATE SIGNATURE REQUIRED
  // ========================================================================

  #rect(
    width: 100%,
    inset: 12pt,
    stroke: 2pt + rgb("#b45309"),
    fill: rgb("#fffbeb"),
    radius: 4pt,
  )[
    #text(weight: "bold")[IMPORTANT: SEPARATE SIGNATURE REQUIRED]

    #v(0.3em)

    Florida Statutes § 83.595 requires that this Liquidated Damages provision be contained in a separate addendum and signed separately by the Tenant. This signature below is IN ADDITION TO the Tenant's signature on the main Lease Agreement.
  ]

  #v(2em)

  #text(size: 11pt, weight: "bold")[ACKNOWLEDGMENT AND AGREEMENT]

  #v(0.5em)

  By signing below, Tenant acknowledges having read and understood this Addendum and agrees to its terms.

  #v(2em)

  #grid(
    columns: (1fr, 1fr),
    gutter: 40pt,
    [
      #text(weight: "bold")[TENANT SIGNATURE (REQUIRED)]
      #v(1.5em)
      Signature: #box(width: 150pt, repeat[\_])
      #v(0.5em)
      Print Name: #get("tenant_name", default: "[Tenant Name]")
      #v(0.5em)
      Date: #box(width: 100pt, repeat[\_])
    ],
    [
      #text(weight: "bold")[LANDLORD SIGNATURE]
      #v(1.5em)
      Signature: #box(width: 150pt, repeat[\_])
      #v(0.5em)
      Print Name: #get("landlord_name", default: "[Landlord Name]")
      #v(0.5em)
      Date: #box(width: 100pt, repeat[\_])
    ]
  )
]

// ============================================================================
// ADDENDUM L: MOLD PREVENTION ADDENDUM (Optional)
// Per FL_LEASE.md §6.4: AC operation, humidity control, leak reporting
// Florida's climate makes mold prevention critical
// ============================================================================

#if get_bool("mold_addendum") [
  #pagebreak()

  #text(size: 14pt, weight: "bold")[ADDENDUM L: MOLD PREVENTION ADDENDUM]

  #v(0.5em)

  #text(size: 10pt, style: "italic")[Florida Climate Mold Prevention Requirements]

  #v(1em)

  This Mold Prevention Addendum is attached to and made part of the Residential Lease Agreement dated #get("lease_start", default: "[Start Date]") for the property located at #get("property_address", default: "[Property Address]").

  #v(1em)

  // ========================================================================
  // INTRODUCTION
  // ========================================================================

  #rect(
    width: 100%,
    inset: 12pt,
    stroke: 1pt + rgb("#666"),
    fill: rgb("#f0fdf4"),
    radius: 4pt,
  )[
    #text(weight: "bold")[IMPORTANCE OF MOLD PREVENTION]

    #v(0.3em)

    Florida's humid subtropical climate creates conditions favorable to mold growth. Mold can cause property damage and may affect indoor air quality. Both Landlord and Tenant have responsibilities in preventing mold growth. This addendum outlines Tenant's specific obligations to help prevent mold.
  ]

  #v(1em)

  // ========================================================================
  // TENANT OBLIGATIONS
  // ========================================================================

  #text(weight: "bold")[1. TENANT MOLD PREVENTION OBLIGATIONS]

  #v(0.5em)

  Tenant agrees to the following mold prevention measures:

  #v(0.5em)

  #text(weight: "bold")[A. Air Conditioning Operation]

  - Tenant shall operate the air conditioning system *continuously* during warm and humid months (typically April through October)
  - Thermostat shall be set to maintain indoor temperature at or below 78°F when occupied
  - AC shall not be turned off for extended periods, even when premises are unoccupied
  - Tenant shall notify Landlord immediately if the AC system fails or is not cooling properly

  #v(0.5em)

  #text(weight: "bold")[B. Humidity Control]

  - Tenant shall maintain indoor humidity levels *below 60%*
  - Use of bathroom exhaust fans during and after bathing/showering is required
  - Use of kitchen exhaust fans during cooking is required
  - Tenant should avoid hanging wet clothes indoors to dry

  #v(0.5em)

  #text(weight: "bold")[C. Ventilation]

  - Tenant shall ensure adequate air circulation throughout the premises
  - Furniture shall not completely block air vents or returns
  - Closet doors should be left slightly open to allow air circulation

  #v(0.5em)

  #text(weight: "bold")[D. Leak Reporting]

  - Tenant shall *immediately* report any leaks, water intrusion, or moisture problems to Landlord
  - This includes: roof leaks, plumbing leaks, window leaks, condensation on windows, and standing water
  - Tenant shall not ignore or delay reporting any signs of water damage or mold

  #v(0.5em)

  #text(weight: "bold")[E. Cleaning]

  - Tenant shall regularly clean bathroom surfaces to prevent mold growth
  - Tenant shall promptly clean any visible mold with appropriate cleaning products
  - Tenant shall not allow food waste to accumulate

  #v(1em)

  // ========================================================================
  // REPORTING REQUIREMENTS
  // ========================================================================

  #text(weight: "bold")[2. REQUIRED REPORTING]

  #v(0.5em)

  Tenant shall immediately notify Landlord in writing of any of the following:

  - Visible mold or mildew growth
  - Musty or moldy odors
  - Water leaks or water damage
  - Malfunctioning HVAC equipment
  - Excessive condensation on windows or walls
  - Any flooding or water intrusion

  #v(1em)

  // ========================================================================
  // LANDLORD RESPONSIBILITIES
  // ========================================================================

  #text(weight: "bold")[3. LANDLORD RESPONSIBILITIES]

  #v(0.5em)

  Landlord agrees to:

  - Maintain the premises in a condition that prevents moisture intrusion
  - Promptly respond to reports of leaks, water damage, or mold
  - Ensure HVAC systems are in proper working condition
  - Address any structural issues that may contribute to moisture problems

  #v(1em)

  // ========================================================================
  // CONSEQUENCES
  // ========================================================================

  #rect(
    width: 100%,
    inset: 12pt,
    stroke: 2pt + rgb("#b45309"),
    fill: rgb("#fffbeb"),
    radius: 4pt,
  )[
    #text(weight: "bold")[IMPORTANT NOTICE]

    #v(0.3em)

    Failure to comply with the mold prevention obligations in this addendum may result in:

    - Tenant being held responsible for mold remediation costs caused by Tenant's negligence
    - Deduction from security deposit for mold damage
    - Potential liability for property damage

    #v(0.3em)

    Mold problems that result from Tenant's failure to operate AC, control humidity, or report leaks may be considered damage beyond normal wear and tear.
  ]

  #v(2em)

  // ========================================================================
  // SIGNATURES
  // ========================================================================

  #text(size: 11pt, weight: "bold")[ACKNOWLEDGMENT]

  #v(0.5em)

  By signing below, Tenant acknowledges having read, understood, and agreed to comply with all terms of this Mold Prevention Addendum.

  #v(2em)

  #grid(
    columns: (1fr, 1fr),
    gutter: 40pt,
    [
      #text(weight: "bold")[TENANT]
      #v(1.5em)
      Signature: #box(width: 150pt, repeat[\_])
      #v(0.5em)
      Print Name: #get("tenant_name", default: "[Tenant Name]")
      #v(0.5em)
      Date: #box(width: 100pt, repeat[\_])
    ],
    [
      #text(weight: "bold")[LANDLORD]
      #v(1.5em)
      Signature: #box(width: 150pt, repeat[\_])
      #v(0.5em)
      Print Name: #get("landlord_name", default: "[Landlord Name]")
      #v(0.5em)
      Date: #box(width: 100pt, repeat[\_])
    ]
  )
]
