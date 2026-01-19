#!/bin/bash
# =============================================================================
# IS22 Test Runner - Run All Integration Tests
# =============================================================================
# Usage:
#   ./run_all_tests.sh           # Run all tests
#   ./run_all_tests.sh --list    # List available tests
#   ./run_all_tests.sh <test>    # Run specific test
# =============================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Track results
TOTAL_SUITES=0
PASSED_SUITES=0
FAILED_SUITES=0

run_test() {
    local test_script="$1"
    local test_name=$(basename "$test_script" .sh)

    echo -e "\n${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BLUE}  Running: $test_name${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

    ((TOTAL_SUITES++))

    if bash "$test_script"; then
        echo -e "${GREEN}[SUITE PASSED] $test_name${NC}"
        ((PASSED_SUITES++))
    else
        echo -e "${RED}[SUITE FAILED] $test_name${NC}"
        ((FAILED_SUITES++))
    fi
}

list_tests() {
    echo "Available test suites:"
    echo ""
    for test_file in "$SCRIPT_DIR"/test_*.sh; do
        if [ -f "$test_file" ]; then
            local name=$(basename "$test_file" .sh)
            local desc=$(head -10 "$test_file" | grep -E "^#.*Test" | head -1 | sed 's/^# *//')
            echo "  - $name"
            [ -n "$desc" ] && echo "      $desc"
        fi
    done
    echo ""
    echo "Usage: $0 [test_name]"
}

print_summary() {
    echo ""
    echo "============================================="
    echo "  TEST RUNNER SUMMARY"
    echo "============================================="
    echo -e "  Total suites:  $TOTAL_SUITES"
    echo -e "  Passed:        ${GREEN}$PASSED_SUITES${NC}"
    echo -e "  Failed:        ${RED}$FAILED_SUITES${NC}"
    echo "============================================="

    if [ "$FAILED_SUITES" -eq 0 ]; then
        echo -e "${GREEN}  ALL TEST SUITES PASSED${NC}"
        return 0
    else
        echo -e "${RED}  SOME TEST SUITES FAILED${NC}"
        return 1
    fi
}

main() {
    echo "============================================="
    echo "  IS22 Integration Test Runner"
    echo "============================================="
    echo "  Date: $(date)"
    echo "  Host: $(hostname)"
    echo "============================================="

    # Check arguments
    if [ "$1" = "--list" ] || [ "$1" = "-l" ]; then
        list_tests
        exit 0
    fi

    # Check for specific test
    if [ -n "$1" ]; then
        local specific_test="$SCRIPT_DIR/$1"
        [ ! -f "$specific_test" ] && specific_test="$SCRIPT_DIR/test_$1.sh"
        [ ! -f "$specific_test" ] && specific_test="$SCRIPT_DIR/${1}.sh"

        if [ -f "$specific_test" ]; then
            run_test "$specific_test"
        else
            echo -e "${RED}Error: Test not found: $1${NC}"
            list_tests
            exit 1
        fi
    else
        # Run all tests
        for test_file in "$SCRIPT_DIR"/test_*.sh; do
            if [ -f "$test_file" ]; then
                run_test "$test_file"
            fi
        done
    fi

    print_summary
}

main "$@"
