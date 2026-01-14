#!/bin/bash
#
# KV Cleanup Script for docsign-worker
#
# Usage:
#   ./kv-cleanup.sh list              List all unverified accounts
#   ./kv-cleanup.sh delete            Delete unverified accounts (with confirmation)
#   ./kv-cleanup.sh count             Count verified vs unverified accounts
#   ./kv-cleanup.sh help              Show this help message
#
# Prerequisites:
#   - wrangler CLI installed and authenticated
#   - jq installed for JSON parsing
#
# KV Namespace IDs (from worker/wrangler.toml):
#   USERS: bfa8b86b600641b18c8b326b7245384b
#

set -e

# KV namespace IDs from wrangler.toml
USERS_NS="bfa8b86b600641b18c8b326b7245384b"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Change to worker directory for wrangler context
cd "$(dirname "$0")/../worker"

check_dependencies() {
    if ! command -v wrangler &> /dev/null; then
        echo -e "${RED}Error: wrangler CLI not found. Install with: npm install -g wrangler${NC}"
        exit 1
    fi

    if ! command -v jq &> /dev/null; then
        echo -e "${RED}Error: jq not found. Install with: brew install jq${NC}"
        exit 1
    fi
}

list_unverified() {
    echo -e "${YELLOW}Listing unverified accounts...${NC}"
    echo "================================"

    local count=0

    # List all keys with user: prefix
    while IFS= read -r key; do
        # Skip email index keys
        if [[ "$key" == user_email:* ]]; then
            continue
        fi

        # Get user data
        local user
        user=$(wrangler kv key get "$key" --namespace-id="$USERS_NS" 2>/dev/null || echo "")

        if [ -n "$user" ] && [ "$user" != "null" ]; then
            local email_verified
            email_verified=$(echo "$user" | jq -r '.email_verified // false')

            if [ "$email_verified" = "false" ]; then
                local email created_at
                email=$(echo "$user" | jq -r '.email // "unknown"')
                created_at=$(echo "$user" | jq -r '.created_at // "unknown"')

                echo -e "${RED}[UNVERIFIED]${NC} $email"
                echo "  Key: $key"
                echo "  Created: $created_at"
                echo ""

                ((count++)) || true
            fi
        fi
    done < <(wrangler kv key list --namespace-id="$USERS_NS" 2>/dev/null | jq -r '.[].name')

    echo "================================"
    echo -e "Found ${RED}$count${NC} unverified accounts"
}

count_accounts() {
    echo -e "${YELLOW}Counting accounts...${NC}"
    echo "===================="

    local verified=0
    local unverified=0
    local total=0

    while IFS= read -r key; do
        # Skip email index keys
        if [[ "$key" == user_email:* ]]; then
            continue
        fi

        local user
        user=$(wrangler kv key get "$key" --namespace-id="$USERS_NS" 2>/dev/null || echo "")

        if [ -n "$user" ] && [ "$user" != "null" ]; then
            ((total++)) || true

            local email_verified
            email_verified=$(echo "$user" | jq -r '.email_verified // false')

            if [ "$email_verified" = "true" ]; then
                ((verified++)) || true
            else
                ((unverified++)) || true
            fi
        fi
    done < <(wrangler kv key list --namespace-id="$USERS_NS" 2>/dev/null | jq -r '.[].name')

    echo -e "Total accounts:    ${YELLOW}$total${NC}"
    echo -e "Verified:          ${GREEN}$verified${NC}"
    echo -e "Unverified:        ${RED}$unverified${NC}"
}

delete_unverified() {
    echo -e "${YELLOW}Finding unverified accounts to delete...${NC}"
    echo "========================================="

    local keys_to_delete=()
    local emails_to_show=()

    while IFS= read -r key; do
        # Skip email index keys for now
        if [[ "$key" == user_email:* ]]; then
            continue
        fi

        local user
        user=$(wrangler kv key get "$key" --namespace-id="$USERS_NS" 2>/dev/null || echo "")

        if [ -n "$user" ] && [ "$user" != "null" ]; then
            local email_verified
            email_verified=$(echo "$user" | jq -r '.email_verified // false')

            if [ "$email_verified" = "false" ]; then
                local email
                email=$(echo "$user" | jq -r '.email // "unknown"')

                # Add user key to delete list
                keys_to_delete+=("$key")
                # Add email index key to delete list
                keys_to_delete+=("user_email:${email,,}")  # lowercase email
                emails_to_show+=("$email")
            fi
        fi
    done < <(wrangler kv key list --namespace-id="$USERS_NS" 2>/dev/null | jq -r '.[].name')

    if [ ${#emails_to_show[@]} -eq 0 ]; then
        echo -e "${GREEN}No unverified accounts found. Nothing to delete.${NC}"
        return 0
    fi

    echo -e "Found ${RED}${#emails_to_show[@]}${NC} unverified accounts to delete:"
    echo ""
    for email in "${emails_to_show[@]}"; do
        echo -e "  ${RED}✗${NC} $email"
    done
    echo ""

    read -p "Are you sure you want to delete these accounts? (y/N) " confirm

    if [ "$confirm" != "y" ] && [ "$confirm" != "Y" ]; then
        echo "Cancelled."
        return 0
    fi

    echo ""
    echo "Deleting..."

    for key in "${keys_to_delete[@]}"; do
        echo "  Deleting: $key"
        wrangler kv key delete "$key" --namespace-id="$USERS_NS" --force 2>/dev/null || true
    done

    echo ""
    echo -e "${GREEN}Deleted ${#emails_to_show[@]} unverified accounts.${NC}"
}

show_help() {
    head -17 "$0" | tail -15
}

# Main
check_dependencies

case "${1:-help}" in
    list)
        list_unverified
        ;;
    delete)
        delete_unverified
        ;;
    count)
        count_accounts
        ;;
    help|--help|-h)
        show_help
        ;;
    *)
        echo -e "${RED}Unknown command: $1${NC}"
        echo ""
        show_help
        exit 1
        ;;
esac
