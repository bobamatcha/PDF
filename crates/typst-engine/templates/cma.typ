// CMA Report Template - Professional Comparative Market Analysis
// =============================================================================
// Required inputs: subject_address, site_name
// Optional inputs: site_color, subject_city, subject_zip, subject_beds, subject_baths,
//                  subject_sqft, subject_year_built, estimated_value, estimated_low,
//                  estimated_high, confidence_score, comps (array), adjustment_config,
//                  rental_yield, rent_estimate, price_per_sqft, school_score, crime_score,
//                  school_elementary, school_middle, school_high, school_avg, flood_zone,
//                  list_price, market_position, recommendation, comp_count, generated_at

#let data = sys.inputs

// Theme colors based on site
#let site_color = rgb(data.at("site_color", default: "#1e40af"))
#let site_name = data.at("site_name", default: "Tampa Deals")

// Helper functions
#let format_currency(value) = {
  if value == none or value == 0 { "---" }
  else {
    let n = int(value)
    let s = str(n)
    let len = s.len()
    let result = ""
    for i in range(len) {
      if i > 0 and calc.rem(len - i, 3) == 0 {
        result = result + ","
      }
      result = result + s.at(i)
    }
    "$" + result
  }
}

#let format_number(value) = {
  if value == none or value == 0 { "---" }
  else {
    let n = int(value)
    let s = str(n)
    let len = s.len()
    let result = ""
    for i in range(len) {
      if i > 0 and calc.rem(len - i, 3) == 0 {
        result = result + ","
      }
      result = result + s.at(i)
    }
    result
  }
}

#let rating_bar(score, max: 10) = {
  let filled = calc.min(score, max)
  let bar_color = if score >= 7 { rgb("#22c55e") } else if score >= 5 { rgb("#eab308") } else { rgb("#ef4444") }
  box(
    fill: rgb("#e5e7eb"),
    radius: 2pt,
    width: 100%,
    height: 8pt,
    box(
      fill: bar_color,
      radius: 2pt,
      width: (score / max * 100) * 1%,
      height: 8pt,
    )
  )
}

// Page setup
#set page(
  paper: "us-letter",
  margin: (top: 1in, bottom: 1in, left: 0.75in, right: 0.75in),
  header: context {
    if counter(page).get().first() > 1 [
      #set text(size: 9pt, fill: gray)
      #site_name #h(1fr) Comparative Market Analysis
    ]
  },
  footer: [
    #set text(size: 8pt, fill: gray)
    Generated #data.at("generated_at", default: datetime.today().display("[month repr:long] [day], [year]"))
    #h(1fr)
    Page #context counter(page).display("1 of 1", both: true)
  ],
)

#set text(font: "Liberation Sans", size: 10pt)

// =============================================================================
// PAGE 1: COVER & SUMMARY
// =============================================================================

// Header with logo area
#align(center)[
  #box(
    fill: site_color,
    radius: 8pt,
    inset: 12pt,
    width: 100%,
  )[
    #text(fill: white, size: 24pt, weight: "bold")[#site_name]
    #v(4pt)
    #text(fill: white.darken(10%), size: 12pt)[Comparative Market Analysis]
  ]
]

#v(1.5em)

// Subject Property Box
#block(
  fill: rgb("#f8fafc"),
  stroke: (left: 4pt + site_color),
  radius: (right: 4pt),
  inset: 16pt,
  width: 100%,
)[
  #text(size: 9pt, fill: gray, weight: "medium")[SUBJECT PROPERTY]
  #v(4pt)
  #text(size: 16pt, weight: "bold")[#data.at("subject_address", default: "Property Address")]
  #v(2pt)
  #text(size: 11pt, fill: gray)[#data.at("subject_city", default: "City"), FL #data.at("subject_zip", default: "")]

  #v(12pt)

  #grid(
    columns: (1fr, 1fr, 1fr, 1fr),
    gutter: 16pt,
    [
      #text(size: 9pt, fill: gray)[Bedrooms]
      #v(2pt)
      #text(size: 18pt, weight: "bold")[#data.at("subject_beds", default: "---")]
    ],
    [
      #text(size: 9pt, fill: gray)[Bathrooms]
      #v(2pt)
      #text(size: 18pt, weight: "bold")[#data.at("subject_baths", default: "---")]
    ],
    [
      #text(size: 9pt, fill: gray)[Living Area]
      #v(2pt)
      #text(size: 18pt, weight: "bold")[#format_number(data.at("subject_sqft", default: 0))] #text(size: 10pt)[sqft]
    ],
    [
      #text(size: 9pt, fill: gray)[Year Built]
      #v(2pt)
      #text(size: 18pt, weight: "bold")[#data.at("subject_year_built", default: "---")]
    ],
  )
]

#v(1.5em)

// Valuation Summary
#block(
  fill: rgb("#ecfdf5"),
  stroke: 1pt + rgb("#86efac"),
  radius: 8pt,
  inset: 20pt,
  width: 100%,
)[
  #align(center)[
    #text(size: 10pt, fill: gray, weight: "medium")[ESTIMATED MARKET VALUE]
    #v(8pt)
    #text(size: 36pt, weight: "bold", fill: rgb("#059669"))[
      #format_currency(data.at("estimated_value", default: 0))
    ]
    #v(8pt)
    #text(size: 12pt, fill: gray)[
      Range: #format_currency(data.at("estimated_low", default: 0)) --- #format_currency(data.at("estimated_high", default: 0))
    ]
    #v(8pt)
    #box(
      fill: rgb("#dcfce7"),
      radius: 12pt,
      inset: (x: 12pt, y: 4pt),
    )[
      #text(size: 10pt, fill: rgb("#166534"), weight: "medium")[
        Confidence: #str(int(data.at("confidence_score", default: 0)))%
      ]
    ]
  ]
]

#v(1.5em)

// Key Metrics Grid
#text(size: 11pt, weight: "bold")[KEY METRICS]
#v(8pt)

#grid(
  columns: (1fr, 1fr, 1fr, 1fr),
  gutter: 12pt,
  block(
    fill: rgb("#f8fafc"),
    radius: 6pt,
    inset: 12pt,
    width: 100%,
  )[
    #align(center)[
      #text(size: 20pt, weight: "bold", fill: site_color)[
        \$#str(int(data.at("price_per_sqft", default: 0)))
      ]
      #v(2pt)
      #text(size: 9pt, fill: gray)[per sqft]
    ]
  ],
  block(
    fill: rgb("#f8fafc"),
    radius: 6pt,
    inset: 12pt,
    width: 100%,
  )[
    #align(center)[
      #text(size: 20pt, weight: "bold", fill: site_color)[
        #str(calc.round(data.at("rental_yield", default: 0.0), digits: 1))%
      ]
      #v(2pt)
      #text(size: 9pt, fill: gray)[gross yield]
    ]
  ],
  block(
    fill: rgb("#f8fafc"),
    radius: 6pt,
    inset: 12pt,
    width: 100%,
  )[
    #align(center)[
      #text(size: 20pt, weight: "bold", fill: site_color)[
        #data.at("school_score", default: "---")/10
      ]
      #v(2pt)
      #text(size: 9pt, fill: gray)[schools]
    ]
  ],
  block(
    fill: rgb("#f8fafc"),
    radius: 6pt,
    inset: 12pt,
    width: 100%,
  )[
    #align(center)[
      #text(size: 20pt, weight: "bold", fill: site_color)[
        #data.at("crime_score", default: "---")/10
      ]
      #v(2pt)
      #text(size: 9pt, fill: gray)[safety]
    ]
  ],
)

#v(1.5em)

// Methodology Note
#block(
  fill: rgb("#fef3c7"),
  stroke: 1pt + rgb("#fcd34d"),
  radius: 6pt,
  inset: 12pt,
  width: 100%,
)[
  #text(size: 9pt, weight: "bold", fill: rgb("#92400e"))[Valuation Methodology]
  #v(4pt)
  #text(size: 9pt, fill: rgb("#78350f"))[
    This analysis uses a weighted comparable sales approach with adjustments for bedrooms,
    bathrooms, living area, and year built. #data.at("comp_count", default: 0) comparable
    properties sold within 0.5 miles in the last 180 days were analyzed.
  ]
]

// =============================================================================
// PAGE 2: ADJUSTMENT GRID
// =============================================================================
#pagebreak()

#text(size: 14pt, weight: "bold")[COMPARABLE SALES ADJUSTMENT GRID]
#v(4pt)
#text(size: 9pt, fill: gray)[Dollar adjustments bring each comparable to subject property equivalence]
#v(16pt)

#let comps = data.at("comps", default: ())
#let adjustment_config = data.at("adjustment_config", default: (:))

// Adjustment values reference
#block(
  fill: rgb("#f0f9ff"),
  radius: 4pt,
  inset: 8pt,
  width: 100%,
)[
  #text(size: 9pt, weight: "medium")[Adjustment Values: ]
  #text(size: 9pt)[
    Bedroom: \$#format_number(adjustment_config.at("bedroom_value", default: 15000)) |
    Bathroom: \$#format_number(adjustment_config.at("bathroom_value", default: 10000)) |
    Living Area: #str(int(adjustment_config.at("sqft_factor", default: 0.5) * 100))% of market \$/sqft
  ]
]

#v(12pt)

// Create adjustment grid table
#let comp_count = calc.min(5, if type(comps) == array { comps.len() } else { 0 })

#if comp_count > 0 [
  #table(
    columns: (2fr, 1fr, ..range(comp_count).map(_ => 1fr)),
    stroke: 0.5pt + rgb("#e5e7eb"),
    inset: 8pt,
    fill: (col, row) => if row == 0 { site_color } else if calc.rem(row, 2) == 0 { rgb("#f9fafb") } else { white },

    // Header row
    table.cell(fill: site_color)[#text(fill: white, weight: "bold")[Feature]],
    table.cell(fill: site_color)[#text(fill: white, weight: "bold")[Subject]],
    ..range(comp_count).map(i => {
      table.cell(fill: site_color)[#text(fill: white, weight: "bold")[Comp #str(i + 1)]]
    }),

    // Address row
    [Address],
    [#text(size: 8pt)[#data.at("subject_address", default: "").split(",").at(0, default: "")]],
    ..range(comp_count).map(i => {
      let comp = comps.at(i)
      [#text(size: 8pt)[#comp.at("address", default: "").split(",").at(0, default: "")]]
    }),

    // Sale Price row
    [Sale Price],
    [---],
    ..range(comp_count).map(i => {
      let comp = comps.at(i)
      [#format_currency(comp.at("sale_price", default: comp.at("price", default: 0)))]
    }),

    // Distance row
    [Distance (mi)],
    [---],
    ..range(comp_count).map(i => {
      let comp = comps.at(i)
      [#str(calc.round(comp.at("distance_miles", default: 0.0), digits: 2))]
    }),

    // Bedrooms row
    [Bedrooms],
    [#data.at("subject_beds", default: "---")],
    ..range(comp_count).map(i => {
      let comp = comps.at(i)
      let adjustments = comp.at("adjustments", default: ())
      let adj = if type(adjustments) == array { adjustments.find(a => a.at("category", default: "") == "Bedrooms") } else { none }
      if adj != none {
        let amt = adj.at("dollar_amount", default: 0)
        [#comp.at("beds", default: "---") #text(size: 8pt, fill: if amt > 0 { rgb("#16a34a") } else { rgb("#dc2626") })[(#if amt > 0 { "+" }#format_currency(amt))]]
      } else {
        [#comp.at("beds", default: "---")]
      }
    }),

    // Bathrooms row
    [Bathrooms],
    [#data.at("subject_baths", default: "---")],
    ..range(comp_count).map(i => {
      let comp = comps.at(i)
      let adjustments = comp.at("adjustments", default: ())
      let adj = if type(adjustments) == array { adjustments.find(a => a.at("category", default: "") == "Bathrooms") } else { none }
      if adj != none {
        let amt = adj.at("dollar_amount", default: 0)
        [#comp.at("baths", default: "---") #text(size: 8pt, fill: if amt > 0 { rgb("#16a34a") } else { rgb("#dc2626") })[(#if amt > 0 { "+" }#format_currency(amt))]]
      } else {
        [#comp.at("baths", default: "---")]
      }
    }),

    // Living Area row
    [Living Area],
    [#format_number(data.at("subject_sqft", default: 0)) sqft],
    ..range(comp_count).map(i => {
      let comp = comps.at(i)
      let adjustments = comp.at("adjustments", default: ())
      let adj = if type(adjustments) == array { adjustments.find(a => a.at("category", default: "") == "Living Area") } else { none }
      if adj != none {
        let amt = adj.at("dollar_amount", default: 0)
        [#format_number(comp.at("sqft", default: 0)) #text(size: 8pt, fill: if amt > 0 { rgb("#16a34a") } else { rgb("#dc2626") })[(#if amt > 0 { "+" }#format_currency(amt))]]
      } else {
        [#format_number(comp.at("sqft", default: 0))]
      }
    }),

    // Year Built row
    [Year Built],
    [#data.at("subject_year_built", default: "---")],
    ..range(comp_count).map(i => {
      let comp = comps.at(i)
      let adjustments = comp.at("adjustments", default: ())
      let adj = if type(adjustments) == array { adjustments.find(a => a.at("category", default: "") == "Year Built") } else { none }
      if adj != none {
        let amt = adj.at("dollar_amount", default: 0)
        [#comp.at("year_built", default: "---") #text(size: 8pt, fill: if amt > 0 { rgb("#16a34a") } else { rgb("#dc2626") })[(#if amt > 0 { "+" }#format_currency(amt))]]
      } else {
        [#comp.at("year_built", default: "---")]
      }
    }),

    // Total Adjustment row
    table.cell(fill: rgb("#f0f9ff"))[#text(weight: "bold")[Total Adjustment]],
    table.cell(fill: rgb("#f0f9ff"))[---],
    ..range(comp_count).map(i => {
      let comp = comps.at(i)
      let total = comp.at("total_adjustment", default: 0)
      table.cell(fill: rgb("#f0f9ff"))[
        #text(weight: "bold", fill: if total > 0 { rgb("#16a34a") } else if total < 0 { rgb("#dc2626") } else { black })[
          #if total > 0 { "+" }#format_currency(total)
        ]
      ]
    }),

    // Adjusted Price row
    table.cell(fill: rgb("#ecfdf5"))[#text(weight: "bold")[Adjusted Price]],
    table.cell(fill: rgb("#ecfdf5"))[---],
    ..range(comp_count).map(i => {
      let comp = comps.at(i)
      table.cell(fill: rgb("#ecfdf5"))[
        #text(weight: "bold", fill: rgb("#059669"))[#format_currency(comp.at("adjusted_price", default: 0))]
      ]
    }),

    // Weight row
    [Weight],
    [---],
    ..range(comp_count).map(i => {
      let comp = comps.at(i)
      [#str(int(comp.at("weight", default: 0.0) * 100))%]
    }),
  )
] else [
  #block(
    fill: rgb("#fef3c7"),
    radius: 4pt,
    inset: 12pt,
    width: 100%,
  )[
    #text(fill: rgb("#92400e"), size: 10pt)[
      No comparable sales data available for adjustment grid.
    ]
  ]
]

// =============================================================================
// PAGE 3: NEIGHBORHOOD ANALYSIS
// =============================================================================
#pagebreak()

#text(size: 14pt, weight: "bold")[NEIGHBORHOOD ANALYSIS]
#v(16pt)

// School Ratings
#block(
  stroke: 1pt + rgb("#e5e7eb"),
  radius: 8pt,
  inset: 16pt,
  width: 100%,
)[
  #text(size: 11pt, weight: "bold")[School Ratings]
  #text(size: 9pt, fill: gray)[ (GreatSchools 1-10)]
  #v(12pt)

  #let elem_score = data.at("school_elementary", default: 0)
  #let mid_score = data.at("school_middle", default: 0)
  #let high_score = data.at("school_high", default: 0)

  #grid(
    columns: (1fr, 2fr, 1fr),
    gutter: 8pt,
    [Elementary],
    rating_bar(if type(elem_score) == int or type(elem_score) == float { elem_score } else { 0 }),
    [#elem_score/10],

    [Middle],
    rating_bar(if type(mid_score) == int or type(mid_score) == float { mid_score } else { 0 }),
    [#mid_score/10],

    [High],
    rating_bar(if type(high_score) == int or type(high_score) == float { high_score } else { 0 }),
    [#high_score/10],
  )

  #v(8pt)
  #text(size: 9pt, fill: gray)[
    Average: #text(weight: "bold")[#str(calc.round(data.at("school_avg", default: 0.0), digits: 1))/10]
  ]
]

#v(16pt)

// Safety Score
#block(
  stroke: 1pt + rgb("#e5e7eb"),
  radius: 8pt,
  inset: 16pt,
  width: 100%,
)[
  #text(size: 11pt, weight: "bold")[Safety Score]
  #v(12pt)

  #let crime = data.at("crime_score", default: 5)
  #let crime_val = if type(crime) == int or type(crime) == float { crime } else { 5 }

  #grid(
    columns: (1fr, 2fr, 1fr),
    gutter: 8pt,
    [Crime Score],
    rating_bar(crime_val),
    [#crime/10],
  )

  #v(8pt)
  #text(size: 9pt, fill: gray)[
    Based on incidents within 0.5 mile radius over the past 6 months.
    Higher score = safer neighborhood.
  ]
]

#v(16pt)

// Flood Zone (if available)
#let flood_zone = data.at("flood_zone", default: none)
#if flood_zone != none [
  #block(
    stroke: 1pt + rgb("#e5e7eb"),
    radius: 8pt,
    inset: 16pt,
    width: 100%,
  )[
    #text(size: 11pt, weight: "bold")[Flood Zone]
    #v(12pt)

    #let zone = flood_zone
    #let zone_color = if zone == "X" or zone == "C" { rgb("#22c55e") }
      else if zone == "B" or zone == "0.2 PCT" { rgb("#eab308") }
      else { rgb("#ef4444") }

    #box(
      fill: zone_color.lighten(80%),
      stroke: 1pt + zone_color,
      radius: 4pt,
      inset: (x: 12pt, y: 6pt),
    )[
      #text(weight: "bold", fill: zone_color.darken(20%))[Zone #zone]
    ]

    #h(12pt)

    #text(size: 10pt)[
      #if zone == "X" or zone == "C" { "Minimal flood risk" }
      else if zone == "B" or zone == "0.2 PCT" { "Moderate flood risk" }
      else { "High flood risk - flood insurance required" }
    ]
  ]
]

// =============================================================================
// PAGE 4: INVESTMENT ANALYSIS
// =============================================================================
#pagebreak()

#text(size: 14pt, weight: "bold")[INVESTMENT ANALYSIS]
#v(16pt)

// Rental Yield
#block(
  stroke: 1pt + rgb("#e5e7eb"),
  radius: 8pt,
  inset: 16pt,
  width: 100%,
)[
  #text(size: 11pt, weight: "bold")[Rental Yield Analysis]
  #v(12pt)

  #let rent = data.at("rent_estimate", default: 0)

  #grid(
    columns: (1fr, 1fr),
    gutter: 24pt,
    [
      #text(size: 9pt, fill: gray)[Estimated Monthly Rent]
      #v(4pt)
      #text(size: 20pt, weight: "bold")[#format_currency(rent)]
      #text(size: 10pt)[/month]
    ],
    [
      #text(size: 9pt, fill: gray)[Annual Gross Rent]
      #v(4pt)
      #text(size: 20pt, weight: "bold")[#format_currency(rent * 12)]
      #text(size: 10pt)[/year]
    ],
  )

  #v(16pt)

  #let yield_pct = data.at("rental_yield", default: 0.0)
  #let yield_val = if type(yield_pct) == float or type(yield_pct) == int { yield_pct } else { 0.0 }
  #let yield_rating = if yield_val >= 8.0 { ("STRONG", rgb("#16a34a")) }
    else if yield_val >= 6.0 { ("GOOD", rgb("#22c55e")) }
    else if yield_val >= 4.0 { ("FAIR", rgb("#eab308")) }
    else { ("WEAK", rgb("#dc2626")) }

  #align(center)[
    #box(
      fill: yield_rating.at(1).lighten(80%),
      stroke: 1pt + yield_rating.at(1),
      radius: 8pt,
      inset: 16pt,
    )[
      #text(size: 10pt, fill: gray)[Gross Yield]
      #v(4pt)
      #text(size: 28pt, weight: "bold", fill: yield_rating.at(1).darken(20%))[
        #str(calc.round(yield_val, digits: 2))%
      ]
      #v(4pt)
      #text(size: 10pt, weight: "bold", fill: yield_rating.at(1).darken(20%))[
        #yield_rating.at(0)
      ]
    ]
  ]
]

#v(16pt)

// Market Position
#block(
  stroke: 1pt + rgb("#e5e7eb"),
  radius: 8pt,
  inset: 16pt,
  width: 100%,
)[
  #text(size: 11pt, weight: "bold")[Market Position]
  #v(12pt)

  #let list_price = data.at("list_price", default: 0)
  #let est_value = data.at("estimated_value", default: 0)
  #let list_val = if type(list_price) == int or type(list_price) == float { list_price } else { 0 }
  #let est_val = if type(est_value) == int or type(est_value) == float { est_value } else { 0 }
  #let diff = list_val - est_val
  #let diff_pct = if est_val > 0 { (diff / est_val) * 100.0 } else { 0.0 }

  #grid(
    columns: (1fr, 1fr, 1fr),
    gutter: 16pt,
    [
      #text(size: 9pt, fill: gray)[List Price]
      #v(4pt)
      #text(size: 16pt, weight: "bold")[#format_currency(list_val)]
    ],
    [
      #text(size: 9pt, fill: gray)[Estimated Value]
      #v(4pt)
      #text(size: 16pt, weight: "bold")[#format_currency(est_val)]
    ],
    [
      #text(size: 9pt, fill: gray)[Difference]
      #v(4pt)
      #text(size: 16pt, weight: "bold", fill: if diff > 0 { rgb("#dc2626") } else { rgb("#16a34a") })[
        #if diff > 0 { "+" }#format_currency(diff)
        (#if diff > 0 { "+" }#str(calc.round(diff_pct, digits: 1))%)
      ]
    ],
  )

  #v(16pt)

  #let position = data.at("market_position", default: "FAIR_PRICE")
  #let recommendation = data.at("recommendation", default: "Based on comparable sales analysis.")

  #block(
    fill: if position == "UNDERPRICED" { rgb("#dcfce7") }
      else if position == "OVERPRICED" { rgb("#fee2e2") }
      else { rgb("#fef3c7") },
    radius: 6pt,
    inset: 12pt,
    width: 100%,
  )[
    #text(weight: "bold")[#str(position).replace("_", " ")]
    #v(4pt)
    #text(size: 10pt)[#recommendation]
  ]
]

// =============================================================================
// PAGE 5: COMPARABLE DETAILS
// =============================================================================
#pagebreak()

#text(size: 14pt, weight: "bold")[COMPARABLE PROPERTY DETAILS]
#v(16pt)

#if comp_count > 0 [
  #for i in range(comp_count) [
    #let comp = comps.at(i)
    #block(
      stroke: 1pt + rgb("#e5e7eb"),
      radius: 8pt,
      inset: 16pt,
      width: 100%,
    )[
      #grid(
        columns: (auto, 1fr, auto),
        gutter: 12pt,
        [
          #box(
            fill: site_color,
            radius: 50%,
            inset: 8pt,
          )[
            #text(fill: white, weight: "bold")[#str(i + 1)]
          ]
        ],
        [
          #text(size: 12pt, weight: "bold")[#comp.at("address", default: "Address")]
          #v(4pt)
          #text(size: 10pt, fill: gray)[
            #str(int(comp.at("beds", default: 0))) bed |
            #str(calc.round(comp.at("baths", default: 0.0), digits: 1)) bath |
            #format_number(comp.at("sqft", default: 0)) sqft |
            Built #comp.at("year_built", default: "---")
          ]
        ],
        [
          #text(size: 14pt, weight: "bold")[#format_currency(comp.at("sale_price", default: comp.at("price", default: 0)))]
          #v(2pt)
          #text(size: 9pt, fill: gray)[
            \$#str(int(comp.at("price_per_sqft", default: 0)))/sqft
          ]
        ],
      )

      #v(8pt)

      #grid(
        columns: (1fr, 1fr, 1fr, 1fr),
        gutter: 8pt,
        [
          #text(size: 9pt, fill: gray)[Distance]
          #v(2pt)
          #text(weight: "medium")[#str(calc.round(comp.at("distance_miles", default: 0.0), digits: 2)) mi]
        ],
        [
          #text(size: 9pt, fill: gray)[Adjusted Price]
          #v(2pt)
          #text(weight: "medium", fill: rgb("#059669"))[#format_currency(comp.at("adjusted_price", default: 0))]
        ],
        [
          #text(size: 9pt, fill: gray)[Total Adjustment]
          #v(2pt)
          #let total = comp.at("total_adjustment", default: 0)
          #text(weight: "medium", fill: if total > 0 { rgb("#16a34a") } else if total < 0 { rgb("#dc2626") } else { black })[
            #if total > 0 { "+" }#format_currency(total)
          ]
        ],
        [
          #text(size: 9pt, fill: gray)[Weight]
          #v(2pt)
          #text(weight: "medium")[#str(int(comp.at("weight", default: 0.0) * 100))%]
        ],
      )

      #let zillow_url = comp.at("zillow_url", default: none)
      #if zillow_url != none [
        #v(8pt)
        #text(size: 9pt, fill: rgb("#2563eb"))[
          #link(zillow_url)[View on Zillow #sym.arrow.r]
        ]
      ]
    ]

    #v(12pt)
  ]
] else [
  #block(
    fill: rgb("#fef3c7"),
    radius: 4pt,
    inset: 12pt,
    width: 100%,
  )[
    #text(fill: rgb("#92400e"), size: 10pt)[
      No comparable properties available. The valuation may be based on limited market information.
    ]
  ]
]

// Footer disclaimer
#v(1fr)
#block(
  fill: rgb("#f9fafb"),
  radius: 6pt,
  inset: 12pt,
  width: 100%,
)[
  #text(size: 8pt, fill: gray)[
    *Disclaimer:* This Comparative Market Analysis is provided for informational purposes only and
    does not constitute an appraisal. Values are estimates based on comparable sales and may differ
    from actual market values. Consult a licensed appraiser for official property valuations.

    Generated by #site_name on #data.at("generated_at", default: datetime.today().display("[month repr:long] [day], [year]")).
  ]
]
