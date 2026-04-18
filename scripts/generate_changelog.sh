#!/bin/bash
# CHANGELOG Generator for OpenClaw-RS
# Analyzes git log and generates conventional changelog entries
#
# Usage: ./scripts/generate_changelog.sh [--from REF] [--to REF] [--format markdown|json]
#
# Exit codes:
#   0 = Success
#   1 = No commits found
#   2 = Invalid arguments

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
FORMAT="markdown"
FROM_REF=""
TO_REF="HEAD"

usage() {
    cat <<EOF
Usage: $(basename "$0") [OPTIONS]

Generate changelog from git commit history.

OPTIONS:
    -h, --help          Show this help message
    -f, --from REF      Starting commit (exclusive)
    -t, --to REF       Ending commit (inclusive, default: HEAD)
    -o, --format FMT   Output format: markdown (default), json
    -n, --count N      Number of commits to process

EXAMPLES:
    $(basename "$0")                          # Recent commits
    $(basename "$0") -f v0.1.0 -t v0.2.0     # Between versions
    $(basename "$0") -o json                  # JSON output

EOF
}

# Category detection based on commit message patterns
categorize_commit() {
    local msg="$1"
    local type=""
    
    # Conventional commits
    case "$msg" in
        feat:*|feature:*)
            type="Added"
            ;;
        fix:*|bugfix:*|bug:*)
            type="Fixed"
            ;;
        docs:*|doc:*)
            type="Changed"  # docs changes don't need prominent listing
            ;;
        style:*|refactor:*)
            type="Changed"
            ;;
        perf:*|performance:*)
            type="Changed"
            ;;
        test:*)
            type="Changed"
            ;;
        build:*|ci:*|chore:*)
            type="Changed"
            ;;
        breaking:*|BREAKING:*)
            type="BREAKING"
            ;;
        deprecate:*|deprecation:*)
            type="Deprecated"
            ;;
        remove:*|removal:*|deleted:*)
            type="Removed"
            ;;
        security:*|security-fix:*)
            type="Security"
            ;;
        *)
            # Fallback: check for keywords in body
            if echo "$msg" | grep -qiE '(add|new|introduce|implement)'; then
                type="Added"
            elif echo "$msg" | grep -qiE '(fix|bug|resolve)'; then
                type="Fixed"
            elif echo "$msg" | grep -qiE '(breaking|backwards?.*incompatible)'; then
                type="BREAKING"
            else
                type="Changed"
            fi
            ;;
    esac
    
    echo "$type"
}

# Extract subject line
extract_subject() {
    echo "$1" | head -1 | sed 's/^.*:[[:space:]]*//' | sed 's/^.*([[:space:]]*//' | sed 's/[[:space:]]*]])//'
}

# Process git log
process_log() {
    local from="$1"
    local to="$2"
    
    # Get commits in range
    local range=""
    if [[ -n "$from" ]]; then
        range="${from}..${to}"
    else
        range="${to}"
    fi
    
    # Check if we have commits
    if ! git log --oneline "$range" &>/dev/null; then
        echo "No commits found in range: ${range:-HEAD}" >&2
        return 1
    fi
    
    # Parse commits
    local commits_json="[]"
    local breaking_count=0
    
    while IFS= read -r line; do
        [[ -z "$line" ]] && continue
        
        local hash="${line%% *}"
        local subject="${line#* }"
        local type
        type=$(categorize_commit "$subject")
        
        local scope=""
        if [[ "$subject" =~ ^([a-z]+)\((.+)\): ]]; then
            scope="${BASH_REMATCH[2]}"
        fi
        
        if [[ "$type" == "BREAKING" ]]; then
            ((breaking_count++))
        fi
        
        # Build JSON entry
        local entry
        entry=$(printf '{"hash":"%s","type":"%s","scope":"%s","subject":"%s"}' \
            "$hash" "$type" "$scope" "$(extract_subject "$subject")")
        
        commits_json=$(echo "$commits_json" | jq --argjson e "$entry" '. += [$e]')
    done < <(git log --format="%H %s" "$range" 2>/dev/null | head -100)
    
    echo "$commits_json"
}

# Generate markdown changelog
generate_markdown() {
    local commits="$1"
    local version="${2:-Unreleased}"
    local date="${3:-$(date +%Y-%m-%d)}"
    
    cat <<EOF
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [$version] - $date

EOF
    
    # Group by type
    local types=("BREAKING" "Added" "Changed" "Deprecated" "Removed" "Fixed" "Security")
    local has_content=false
    
    for type in "${types[@]}"; do
        local entries
        entries=$(echo "$commits" | jq -r ".[] | select(.type == \"$type\") | \"  - \\(.subject)\"")
        
        if [[ -n "$entries" ]]; then
            has_content=true
            echo "### $type"
            echo ""
            echo "$entries"
            echo ""
        fi
    done
    
    if ! $has_content; then
        echo "_No significant changes in this release._"
        echo ""
    fi
}

# Generate JSON changelog
generate_json() {
    local commits="$1"
    local version="${2:-Unreleased}"
    local date="${3:-$(date +%Y-%m-%d)}"
    
    cat <<EOF
{
  "version": "$version",
  "date": "$date",
  "commits": $commits
}
EOF
}

# Parse arguments
COUNT=100
while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--help)
            usage
            exit 0
            ;;
        -f|--from)
            FROM_REF="$2"
            shift 2
            ;;
        -t|--to)
            TO_REF="$2"
            shift 2
            ;;
        -o|--format)
            FORMAT="$2"
            shift 2
            ;;
        -n|--count)
            COUNT="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1" >&2
            usage
            exit 2
            ;;
    esac
done

cd "$PROJECT_ROOT"

# Check if git available
if ! command -v git &> /dev/null; then
    echo "Error: git not found" >&2
    exit 3
fi

# Check if inside git repo
if ! git rev-parse --is-inside-work-tree &>/dev/null; then
    echo "Error: Not inside a git repository" >&2
    exit 3
fi

# Process commits
commits_json=$(process_log "$FROM_REF" "$TO_REF")

if [[ -z "$commits_json" ]] || [[ "$commits_json" == "[]" ]]; then
    echo "No commits to process" >&2
    exit 1
fi

# Generate output
case "$FORMAT" in
    json)
        generate_json "$commits_json"
        ;;
    markdown|*)
        generate_markdown "$commits_json"
        ;;
esac

exit 0
