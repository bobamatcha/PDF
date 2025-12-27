#!/bin/bash
# Detects which apps need testing based on changed files.
#
# Usage:
#   ./scripts/detect-changed-apps.sh              # Use git staged files
#   ./scripts/detect-changed-apps.sh --stdin      # Read file paths from stdin (for testing)
#   echo -e "apps/pdfjoin-web/foo.ts" | ./scripts/detect-changed-apps.sh --stdin
#
# Output: Space-separated list of apps to test (e.g., "pdfjoin docsign" or "all")
# Exit code: 0 if any apps need testing, 1 if no browser tests needed

set -e

# Get changed files either from git or stdin
get_changed_files() {
    if [[ "$1" == "--stdin" ]]; then
        cat
    else
        # Get staged files for commit
        git diff --cached --name-only 2>/dev/null || true
    fi
}

# Determine which apps are affected by a list of file paths
detect_apps() {
    local files="$1"

    local run_pdfjoin=false
    local run_agentpdf=false
    local run_docsign=false
    local run_all=false

    while IFS= read -r file; do
        [[ -z "$file" ]] && continue

        case "$file" in
            # App-specific paths
            apps/pdfjoin-web/*)
                run_pdfjoin=true
                ;;
            apps/agentpdf-web/*)
                run_agentpdf=true
                ;;
            apps/docsign-web/*)
                run_docsign=true
                ;;

            # Crate paths map to their apps
            crates/pdfjoin-*/*)
                run_pdfjoin=true
                ;;
            crates/agentpdf-*/*)
                run_agentpdf=true
                ;;
            crates/docsign-*/*)
                run_docsign=true
                ;;

            # Shared code affects all apps
            crates/shared-*/*|crates/compliance-engine/*|crates/typst-engine/*)
                run_all=true
                ;;

            # Infrastructure changes affect all
            scripts/*|Cargo.toml|Cargo.lock|.github/*)
                run_all=true
                ;;

            # Browser test harness changes
            crates/benchmark-harness/*)
                run_all=true
                ;;

            # Docs and other files - no browser tests needed
            *.md|docs/*|.gitignore|.rustfmt.toml)
                # Skip - doesn't require browser tests
                ;;
        esac
    done <<< "$files"

    # If run_all is set, output "all"
    if [[ "$run_all" == "true" ]]; then
        echo "all"
        return 0
    fi

    # Build list of specific apps
    local apps=""
    [[ "$run_pdfjoin" == "true" ]] && apps="$apps pdfjoin"
    [[ "$run_agentpdf" == "true" ]] && apps="$apps agentpdf"
    [[ "$run_docsign" == "true" ]] && apps="$apps docsign"

    # Trim leading space and output
    apps="${apps# }"

    if [[ -z "$apps" ]]; then
        echo "none"
        return 1
    fi

    echo "$apps"
    return 0
}

# Main
files=$(get_changed_files "$1")
detect_apps "$files"
