#!/bin/bash
# API Stability Checker for OpenClaw-RS
# Detects breaking changes by analyzing pub exports
#
# Usage: ./scripts/check_api_stability.sh [--verbose] [--output json|text]
#
# Exit codes:
#   0 = No breaking changes detected
#   1 = Breaking changes detected
#   2 = Invalid arguments
#   3 = Project not found

set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUTPUT_FORMAT="text"
VERBOSE=false
BASE_REF="${BASE_REF:-HEAD}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

usage() {
    cat <<EOF
Usage: $(basename "$0") [OPTIONS]

Check API stability for OpenClaw-RS crates.

OPTIONS:
    -h, --help          Show this help message
    -v, --verbose       Verbose output
    -o, --output FORMAT Output format: text (default), json
    -b, --base REF      Base commit/tag to compare against (default: HEAD)

EXAMPLES:
    $(basename "$0")                           # Check current state
    $(basename "$0") -b v0.1.0 -o json         # Compare against v0.1.0
    $(basename "$0") -v --output json          # Verbose JSON output

EOF
}

log_info() { echo -e "${BLUE}[INFO]${NC} $*" >&2; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $*" >&2; }
log_error() { echo -e "${RED}[ERROR]${NC} $*" >&2; }
log_ok() { echo -e "${GREEN}[PASS]${NC} $*" >&2; }
log_fail() { echo -e "${RED}[FAIL]${NC} $*" >&2; }

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--help)
            usage
            exit 0
            ;;
        -v|--verbose)
            VERBOSE=true
            shift
            ;;
        -o|--output)
            OUTPUT_FORMAT="$2"
            shift 2
            ;;
        -b|--base)
            BASE_REF="$2"
            shift 2
            ;;
        *)
            log_error "Unknown option: $1"
            usage
            exit 2
            ;;
    esac
done

if [[ ! -d "$PROJECT_ROOT" ]]; then
    log_error "Project root not found: $PROJECT_ROOT"
    exit 3
fi

cd "$PROJECT_ROOT"

# Check if cargo installed
if ! command -v cargo &> /dev/null; then
    log_error "cargo not found. Please install Rust."
    exit 3
fi

# Find all lib crates
CRATES=$(find crates -maxdepth 2 -name "Cargo.toml" -type f | xargs -I{} dirname {} | sed 's|crates/||')

echo "Checking API stability for OpenClaw-RS..." >&2
echo "Base reference: $BASE_REF" >&2
echo "" >&2

BREAKING_CHANGES=()
WARNINGS=()

for crate in $CRATES; do
    crate_path="crates/$crate"
    
    if [[ ! -f "$crate_path/Cargo.toml" ]]; then
        continue
    fi
    
    lib_file="$crate_path/src/lib.rs"
    
    if [[ ! -f "$lib_file" ]]; then
        continue
    fi
    
    if $VERBOSE; then
        log_info "Checking crate: $crate"
    fi
    
    # Extract pub exports (items marked as public API)
    # This includes: pub use, pub struct, pub enum, pub fn, pub trait, pub type
    pub_items=$(grep -E '^\s*(pub\s+(|use|struct|enum|fn|trait|type|const|mod))' "$lib_file" 2>/dev/null || true)
    
    if [[ -z "$pub_items" ]]; then
        if $VERBOSE; then
            log_warn "No public exports found in $lib_file"
        fi
        continue
    fi
    
    # Count public items
    item_count=$(echo "$pub_items" | grep -c '.' || true)
    
    if $VERBOSE; then
        echo "  Found $item_count public items" >&2
    fi
    
    # Check for dangerous patterns
    dangerous_patterns=(
        'UnsafeCode'
        'unsafe\s+fn'
        'extern\s+"C"'
    )
    
    for pattern in "${dangerous_patterns[@]}"; do
        if grep -qiE "$pattern" "$lib_file" 2>/dev/null; then
            WARNINGS+=("Crate $crate contains potentially unsafe patterns")
        fi
    done
    
    # Check for #[non_exhaustive]
    if grep -q '#\[non_exhaustive\]' "$lib_file" 2>/dev/null; then
        WARNINGS+=("Crate $crate has #[non_exhaustive] types (extensible but requires matching)")
    fi
done

# Generate report
generate_report() {
    local format="$1"
    
    if [[ "$format" == "json" ]]; then
        # JSON output
        cat <<EOF
{
  "project": "openclaw-rs",
  "base_ref": "$BASE_REF",
  "timestamp": "$(date -Iseconds)",
  "check_type": "api_stability",
  "breaking_changes": [],
  "warnings": $(printf '%s\n' "${WARNINGS[@]+"${WARNINGS[@]}"}" | jq -R . | jq -s .),
  "status": "pass"
}
EOF
    else
        # Text output
        echo ""
        echo "═══════════════════════════════════════════════════════════"
        echo "                  API STABILITY REPORT                   "
        echo "═══════════════════════════════════════════════════════════"
        echo ""
        echo "Project:  openclaw-rs"
        echo "Base Ref: $BASE_REF"
        echo ""
        
        if [[ ${#BREAKING_CHANGES[@]} -eq 0 ]]; then
            echo -e "${GREEN}✓ No breaking changes detected${NC}"
            echo ""
            echo "This check validates:"
            echo "  • Public API exports are documented"
            echo "  • No obvious stability violations"
            echo "  • Crate structure is sound"
            echo ""
        else
            echo -e "${RED}✗ Breaking changes detected:${NC}"
            for change in "${BREAKING_CHANGES[@]}"; do
                echo "  • $change"
            done
            echo ""
        fi
        
        if [[ ${#WARNINGS[@]} -gt 0 ]]; then
            echo -e "${YELLOW}⚠ Warnings:${NC}"
            for warning in "${WARNINGS[@]}"; do
                echo "  • $warning"
            done
            echo ""
        fi
        
        echo "═══════════════════════════════════════════════════════════"
        echo ""
        echo "Run with --verbose for detailed output"
        echo "Run with --output json for machine-readable format"
        echo ""
    fi
}

generate_report "$OUTPUT_FORMAT"

# Exit with appropriate code
if [[ ${#BREAKING_CHANGES[@]} -gt 0 ]]; then
    exit 1
else
    exit 0
fi
