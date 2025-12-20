//! Example: Generate Florida Real Estate Contract PDFs
//!
//! This example demonstrates how to generate PDF documents from the
//! Florida real estate templates (purchase contract, escalation addendum, listing agreement).
//!
//! Run with:
//!   cargo run --example generate_florida_realestate --features server
//!
//! Output files will be written to the `output/` directory.

use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use typst_engine::compiler::OutputFormat;
use typst_engine::templates::registry::get_template_source;
use typst_engine::{compile_document_sync, RenderRequest};

fn main() {
    // Create output directory
    let output_dir = std::path::Path::new("output");
    if !output_dir.exists() {
        fs::create_dir_all(output_dir).expect("Failed to create output directory");
    }

    println!("Generating Florida Real Estate Contract PDFs...\n");

    // Generate Florida Purchase Contract
    generate_purchase_contract(&output_dir);

    // Generate Escalation Addendum
    generate_escalation_addendum(&output_dir);

    // Generate Listing Agreement
    generate_listing_agreement(&output_dir);

    println!("\nAll documents generated successfully!");
    println!("Check the output/ directory for PDF files.");
}

fn generate_purchase_contract(output_dir: &std::path::Path) {
    println!("1. Generating Florida Purchase Contract...");

    let source = get_template_source("florida_purchase_contract")
        .expect("Failed to load florida_purchase_contract template");

    let mut inputs: HashMap<String, serde_json::Value> = HashMap::new();

    // Required inputs
    inputs.insert("seller_name".to_string(), json!("John and Jane Smith"));
    inputs.insert("buyer_name".to_string(), json!("Robert Johnson"));
    inputs.insert(
        "property_address".to_string(),
        json!("123 Palm Beach Boulevard"),
    );
    inputs.insert("property_city".to_string(), json!("Miami"));
    inputs.insert("property_county".to_string(), json!("Miami-Dade"));
    inputs.insert("property_zip".to_string(), json!("33101"));
    inputs.insert("purchase_price".to_string(), json!(525000));
    inputs.insert("earnest_money".to_string(), json!(15000));
    inputs.insert("closing_date".to_string(), json!("February 15, 2026"));

    // Optional inputs - Party details
    inputs.insert(
        "seller_address".to_string(),
        json!("456 Ocean Drive, Miami, FL 33139"),
    );
    inputs.insert("seller_phone".to_string(), json!("(305) 555-1234"));
    inputs.insert("seller_email".to_string(), json!("smithfamily@email.com"));
    inputs.insert(
        "buyer_address".to_string(),
        json!("789 Main Street, Tampa, FL 33601"),
    );
    inputs.insert("buyer_phone".to_string(), json!("(813) 555-5678"));
    inputs.insert("buyer_email".to_string(), json!("rjohnson@email.com"));

    // Property details
    inputs.insert("parcel_id".to_string(), json!("01-2345-678-9012"));
    inputs.insert(
        "legal_description".to_string(),
        json!("Lot 15, Block 3, Palm Beach Estates, as recorded in Plat Book 45, Page 123"),
    );
    inputs.insert(
        "property_type".to_string(),
        json!("Single Family Residence"),
    );
    inputs.insert("year_built".to_string(), json!("2005"));

    // Financing
    inputs.insert("financing_type".to_string(), json!("conventional"));
    inputs.insert("loan_amount".to_string(), json!(420000));
    inputs.insert("max_interest_rate".to_string(), json!("7.5"));
    inputs.insert("loan_term".to_string(), json!("30"));
    inputs.insert(
        "loan_application_deadline".to_string(),
        json!("January 10, 2026"),
    );
    inputs.insert(
        "loan_approval_deadline".to_string(),
        json!("January 31, 2026"),
    );

    // Deposits
    inputs.insert(
        "escrow_agent_name".to_string(),
        json!("Florida Title & Trust Co."),
    );
    inputs.insert(
        "escrow_agent_address".to_string(),
        json!("100 Brickell Ave, Suite 500, Miami, FL 33131"),
    );

    // Inspections
    inputs.insert("inspection_period_days".to_string(), json!("15"));
    inputs.insert("inspection_contingency_type".to_string(), json!("standard"));

    // Flood disclosure (§ 689.302)
    inputs.insert("has_prior_flooding".to_string(), json!(false));
    inputs.insert("has_flood_claims".to_string(), json!(false));
    inputs.insert("has_flood_assistance".to_string(), json!(false));

    // HOA (§ 720.401)
    inputs.insert("has_hoa".to_string(), json!(true));
    inputs.insert("hoa_name".to_string(), json!("Palm Beach Estates HOA"));
    inputs.insert("hoa_assessment".to_string(), json!(350));
    inputs.insert("hoa_assessment_frequency".to_string(), json!("month"));
    inputs.insert(
        "hoa_contact".to_string(),
        json!("Palm Beach Property Management"),
    );

    // Lead paint (property built 2005, so not required)
    // Seller disclosure
    inputs.insert("known_defects".to_string(), json!("None known."));
    inputs.insert(
        "past_repairs".to_string(),
        json!("Roof replaced in 2020. HVAC system serviced annually."),
    );
    inputs.insert("has_environmental_issues".to_string(), json!(false));

    inputs.insert("mediation_required".to_string(), json!(true));

    let request = RenderRequest {
        source,
        inputs,
        assets: HashMap::new(),
        format: OutputFormat::Pdf,
        ppi: None,
    };

    let response = compile_document_sync(request).expect("Compilation failed");

    match response.artifact {
        Some(artifact) => {
            let pdf_data = STANDARD
                .decode(&artifact.data_base64)
                .expect("Failed to decode PDF");
            let output_path = output_dir.join("florida_purchase_contract.pdf");
            fs::write(&output_path, pdf_data).expect("Failed to write PDF");
            println!(
                "   ✓ Generated: {} ({} pages)",
                output_path.display(),
                artifact.page_count
            );
        }
        None => {
            println!("   ✗ Failed to generate PDF");
            for error in &response.errors {
                println!("     Error: {}", error.message);
            }
        }
    }
}

fn generate_escalation_addendum(output_dir: &std::path::Path) {
    println!("2. Generating Florida Escalation Addendum...");

    let source = get_template_source("florida_escalation_addendum")
        .expect("Failed to load florida_escalation_addendum template");

    let mut inputs: HashMap<String, serde_json::Value> = HashMap::new();

    // Required inputs
    inputs.insert("seller_name".to_string(), json!("John and Jane Smith"));
    inputs.insert("buyer_name".to_string(), json!("Robert Johnson"));
    inputs.insert(
        "property_address".to_string(),
        json!("123 Palm Beach Boulevard, Miami, FL 33101"),
    );
    inputs.insert("contract_date".to_string(), json!("January 5, 2026"));
    inputs.insert("base_purchase_price".to_string(), json!(525000));
    inputs.insert("escalation_increment".to_string(), json!(5000));
    inputs.insert("maximum_purchase_price".to_string(), json!(575000));

    // Optional inputs
    inputs.insert(
        "escalation_deadline".to_string(),
        json!("January 7, 2026 at 5:00 PM EST"),
    );
    inputs.insert("require_full_offer_copy".to_string(), json!(true));
    inputs.insert("proof_deadline_hours".to_string(), json!("24"));

    // Appraisal gap coverage
    inputs.insert("appraisal_gap_coverage".to_string(), json!(true));
    inputs.insert("appraisal_gap_amount".to_string(), json!(25000));
    inputs.insert("appraisal_waiver".to_string(), json!(false));

    // Financing
    inputs.insert("financing_type".to_string(), json!("conventional"));

    let request = RenderRequest {
        source,
        inputs,
        assets: HashMap::new(),
        format: OutputFormat::Pdf,
        ppi: None,
    };

    let response = compile_document_sync(request).expect("Compilation failed");

    match response.artifact {
        Some(artifact) => {
            let pdf_data = STANDARD
                .decode(&artifact.data_base64)
                .expect("Failed to decode PDF");
            let output_path = output_dir.join("florida_escalation_addendum.pdf");
            fs::write(&output_path, pdf_data).expect("Failed to write PDF");
            println!(
                "   ✓ Generated: {} ({} pages)",
                output_path.display(),
                artifact.page_count
            );
        }
        None => {
            println!("   ✗ Failed to generate PDF");
            for error in &response.errors {
                println!("     Error: {}", error.message);
            }
        }
    }
}

fn generate_listing_agreement(output_dir: &std::path::Path) {
    println!("3. Generating Florida Listing Agreement...");

    let source = get_template_source("florida_listing_agreement")
        .expect("Failed to load florida_listing_agreement template");

    let mut inputs: HashMap<String, serde_json::Value> = HashMap::new();

    // Required inputs
    inputs.insert("seller_name".to_string(), json!("John and Jane Smith"));
    inputs.insert("broker_name".to_string(), json!("Michael Torres"));
    inputs.insert("broker_license".to_string(), json!("BK3456789"));
    inputs.insert(
        "property_address".to_string(),
        json!("123 Palm Beach Boulevard"),
    );
    inputs.insert("listing_price".to_string(), json!(549000));
    inputs.insert("listing_start_date".to_string(), json!("January 1, 2026"));
    inputs.insert(
        "listing_expiration_date".to_string(),
        json!("June 30, 2026"),
    );
    inputs.insert("commission_rate".to_string(), json!(6.0));

    // Brokerage relationship (§ 475.278)
    inputs.insert("brokerage_relationship".to_string(), json!("single_agent"));

    // Seller information
    inputs.insert(
        "seller_address".to_string(),
        json!("456 Ocean Drive, Miami, FL 33139"),
    );
    inputs.insert("seller_phone".to_string(), json!("(305) 555-1234"));
    inputs.insert("seller_email".to_string(), json!("smithfamily@email.com"));

    // Broker/Agent information
    inputs.insert("brokerage_firm".to_string(), json!("Sunshine Realty Group"));
    inputs.insert(
        "broker_address".to_string(),
        json!("1000 Brickell Ave, Miami, FL 33131"),
    );
    inputs.insert("broker_phone".to_string(), json!("(305) 555-9999"));
    inputs.insert(
        "broker_email".to_string(),
        json!("mtorres@sunshinerealty.com"),
    );
    inputs.insert("agent_name".to_string(), json!("Michael Torres"));
    inputs.insert("agent_license".to_string(), json!("SL1234567"));

    // Property details
    inputs.insert("property_city".to_string(), json!("Miami"));
    inputs.insert("property_county".to_string(), json!("Miami-Dade"));
    inputs.insert("property_zip".to_string(), json!("33101"));
    inputs.insert("parcel_id".to_string(), json!("01-2345-678-9012"));
    inputs.insert(
        "property_type".to_string(),
        json!("Single Family Residence"),
    );

    // Pricing options
    inputs.insert("accept_cash".to_string(), json!(true));
    inputs.insert("accept_conventional".to_string(), json!(true));
    inputs.insert("accept_fha".to_string(), json!(true));
    inputs.insert("accept_va".to_string(), json!(true));

    // Commission
    inputs.insert("commission_type".to_string(), json!("percentage"));
    inputs.insert("coop_commission_rate".to_string(), json!(3.0));
    inputs.insert("protection_period_days".to_string(), json!("90"));

    // Marketing
    inputs.insert("list_on_mls".to_string(), json!(true));
    inputs.insert("professional_photos".to_string(), json!(true));
    inputs.insert("virtual_tour".to_string(), json!(true));
    inputs.insert("open_houses".to_string(), json!(true));

    // Access
    inputs.insert("lockbox_authorized".to_string(), json!(true));
    inputs.insert(
        "showing_instructions".to_string(),
        json!("Please contact listing agent 24 hours in advance for all showings."),
    );

    // Property status
    inputs.insert("property_occupied".to_string(), json!(true));
    inputs.insert("occupant_type".to_string(), json!("Owner"));
    inputs.insert("has_hoa".to_string(), json!(true));

    inputs.insert("mediation_required".to_string(), json!(true));
    inputs.insert("agreement_date".to_string(), json!("January 1, 2026"));

    let request = RenderRequest {
        source,
        inputs,
        assets: HashMap::new(),
        format: OutputFormat::Pdf,
        ppi: None,
    };

    let response = compile_document_sync(request).expect("Compilation failed");

    match response.artifact {
        Some(artifact) => {
            let pdf_data = STANDARD
                .decode(&artifact.data_base64)
                .expect("Failed to decode PDF");
            let output_path = output_dir.join("florida_listing_agreement.pdf");
            fs::write(&output_path, pdf_data).expect("Failed to write PDF");
            println!(
                "   ✓ Generated: {} ({} pages)",
                output_path.display(),
                artifact.page_count
            );
        }
        None => {
            println!("   ✗ Failed to generate PDF");
            for error in &response.errors {
                println!("     Error: {}", error.message);
            }
        }
    }
}
