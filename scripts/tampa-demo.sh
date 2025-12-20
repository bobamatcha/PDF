#!/bin/bash
set -e

# Tampa REIA Demo Script
# Usage: ./scripts/tampa-demo.sh
#
# This script starts the local dev server and walks through the demo flow
# for Tampa REIA meetings (2nd Thursday of each month).

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
PORT=8080

echo "============================================"
echo "      agentPDF.org - Tampa REIA Demo"
echo "============================================"
echo ""

# Check if trunk is installed
if ! command -v trunk &> /dev/null; then
    echo "Error: trunk not found. Install with: cargo install trunk"
    exit 1
fi

# Check if port is already in use
if lsof -Pi :$PORT -sTCP:LISTEN -t >/dev/null 2>&1; then
    echo "Port $PORT already in use. Assuming server is running..."
    echo ""
else
    echo "Starting Trunk dev server on port $PORT..."
    cd "$PROJECT_ROOT/apps/agentpdf-web"
    trunk serve --port $PORT &
    TRUNK_PID=$!

    # Wait for server to start
    echo "Waiting for server to start..."
    sleep 5

    # Trap to kill trunk on script exit
    trap "echo 'Stopping server...'; kill $TRUNK_PID 2>/dev/null" EXIT
fi

echo ""
echo "============================================"
echo "           DEMO FLOW FOR REIA"
echo "============================================"
echo ""
echo "Opening Tampa landing page..."
open "http://localhost:$PORT/tampa.html" 2>/dev/null || xdg-open "http://localhost:$PORT/tampa.html" 2>/dev/null || echo "Open http://localhost:$PORT/tampa.html in your browser"
echo ""
echo "--------------------------------------------"
echo "STEP 1: Tampa Landing Page"
echo "--------------------------------------------"
echo "  - Show the 4 compliance tools:"
echo "    1. Flood Disclosure (SB 948 / Section 83.512)"
echo "    2. Email Consent (HB 615)"
echo "    3. Complete Lease"
echo "    4. Compliance Check"
echo ""
echo "  KEY TALKING POINT:"
echo "  'Florida passed SB 948 requiring flood history disclosure."
echo "   As of October 2024, you MUST disclose prior flooding.'"
echo ""
echo "Press Enter to continue to Step 2..."
read

echo ""
echo "--------------------------------------------"
echo "STEP 2: Generate Flood Disclosure"
echo "--------------------------------------------"
echo "  - Click 'Flood Disclosure' card"
echo "  - Or go to main app: http://localhost:$PORT"
echo "  - Click 'Use a Template' button"
echo "  - Select 'Florida Lease'"
echo "  - Scroll to 'Optional Fields (11)'"
echo "  - Show the flood disclosure fields:"
echo "    - has_prior_flooding"
echo "    - has_flood_claims"
echo "    - has_fema_assistance"
echo "    - flooding_description"
echo ""
echo "  KEY TALKING POINT:"
echo "  'This generates a legally compliant disclosure form in seconds."
echo "   No lawyer needed. Works offline - perfect for showings.'"
echo ""
open "http://localhost:$PORT" 2>/dev/null || true
echo "Press Enter to continue to Step 3..."
read

echo ""
echo "--------------------------------------------"
echo "STEP 3: HB 615 Email Consent"
echo "--------------------------------------------"
echo "  - In the Optional Fields, show 'email_consent' field"
echo "  - Explain HB 615 (effective Oct 2024):"
echo "    'Tenants can now consent to receive legal notices by email."
echo "     This addendum documents that consent properly.'"
echo ""
echo "  - Fill in sample data and click 'Generate PDF'"
echo "  - Download the PDF"
echo ""
echo "  KEY TALKING POINT:"
echo "  'One form, fully compliant. Takes 30 seconds vs 30 minutes"
echo "   with traditional forms.'"
echo ""
echo "Press Enter to continue to Step 4..."
read

echo ""
echo "--------------------------------------------"
echo "STEP 4: Compliance Check (Existing Leases)"
echo "--------------------------------------------"
echo "  - Click 'Upload PDF' on main app"
echo "  - Upload an existing lease document"
echo "  - Select 'Florida (F.S. Chapter 83)' from dropdown"
echo "  - Click 'Run Compliance Check'"
echo "  - Show any violations found"
echo ""
echo "  KEY TALKING POINT:"
echo "  'Already have a lease? Upload it and we check for violations."
echo "   Catches missing disclosures, illegal clauses, deposit limits.'"
echo ""
echo "Press Enter to continue to Step 5..."
read

echo ""
echo "--------------------------------------------"
echo "STEP 5: Call to Action"
echo "--------------------------------------------"
echo "  Website: https://agentpdf.org"
echo "  Tampa Page: https://agentpdf.org/tampa.html"
echo ""
echo "  KEY POINTS:"
echo "  - 100% free, no signup required"
echo "  - Works offline (great for property showings)"
echo "  - Updates automatically when laws change"
echo "  - 16 states supported (FL, TX, CA, NY, etc.)"
echo ""
echo "  BUSINESS CARDS / QR CODE:"
echo "  - Point to: agentpdf.org/tampa"
echo "  - 'Scan to get Florida lease tools'"
echo ""
echo "============================================"
echo "           END OF DEMO FLOW"
echo "============================================"
echo ""
echo "Upcoming Tampa REIA Meetings (2nd Thursday):"
echo "  - January 9, 2026"
echo "  - February 13, 2026"
echo "  - March 13, 2026"
echo ""
echo "Beach REIA (3rd Thursday):"
echo "  - January 16, 2026"
echo "  - February 20, 2026"
echo ""
echo "Press Enter to exit (this will stop the dev server)..."
read

echo "Demo complete!"
