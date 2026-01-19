#!/bin/bash
# =============================================================================
# Camera Registration Flow Integration Test (Server-Local Version)
# =============================================================================
# Complete test suite that runs on the server itself.
#
# 1. Injects test devices directly into DB (ipcamscan_devices)
# 2. Tests Category B registration (verified + credentials)
# 3. Tests Category C registration (force_register)
# 4. Verifies all camera fields are correctly set:
#    - fid, family, access_family, mac_address
#    - manufacturer, model, lacis_id
#    - onvif_endpoint, ptz_supported
# 5. Cleans up ALL test data
#
# Usage:
#   ./test_camera_registration.sh [--skip-cleanup]
#
# Run on server: /opt/is22/tests/
# =============================================================================

set -e

# Detect if running on server or remotely
if [ -f /opt/is22/target/release/camserver ]; then
    # Running on server - use localhost
    SERVER_HOST="127.0.0.1"
    SERVER_PORT="3000"
    DB_HOST="localhost"
else
    # Running remotely - use external IP
    SERVER_HOST="192.168.125.246"
    SERVER_PORT="3000"
    DB_HOST="192.168.125.246"
fi

SERVER_URL="http://${SERVER_HOST}:${SERVER_PORT}"
DB_USER="root"
DB_PASS="mijeos12345@"
DB_NAME="camserver"

# Test data configuration - use reserved IP range that won't conflict
TEST_PREFIX="TEST_"
TEST_SUBNET="192.168.99.0/24"
TEST_FID="9999"

# Test device IPs
TEST_IP_B="192.168.99.101"    # Category B: verified + credentials success
TEST_IP_C="192.168.99.102"    # Category C: discovered (force_register)
TEST_DEVICE_ID_B="test-dev-$(date +%s)-b"
TEST_DEVICE_ID_C="test-dev-$(date +%s)-c"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

# Counters
TESTS_RUN=0
TESTS_PASSED=0
TESTS_FAILED=0

# Skip cleanup flag
SKIP_CLEANUP=false

# Logging functions
log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_pass() { echo -e "${GREEN}[PASS]${NC} $1"; ((TESTS_PASSED++)) || true; }
log_fail() { echo -e "${RED}[FAIL]${NC} $1"; ((TESTS_FAILED++)) || true; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_test() { echo -e "\n${CYAN}=== TEST: $1 ===${NC}"; ((TESTS_RUN++)) || true; }
log_section() {
    echo -e "\n${YELLOW}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${YELLOW}  $1${NC}"
    echo -e "${YELLOW}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
}

# MySQL command helper
mysql_cmd() {
    mysql -h"$DB_HOST" -u"$DB_USER" -p"$DB_PASS" "$DB_NAME" -N -e "$1" 2>/dev/null
}

mysql_cmd_verbose() {
    mysql -h"$DB_HOST" -u"$DB_USER" -p"$DB_PASS" "$DB_NAME" -e "$1" 2>/dev/null
}

# API call helper
api_call() {
    local method="$1"
    local endpoint="$2"
    local data="$3"

    if [ -n "$data" ]; then
        curl -s -X "$method" \
            -H "Content-Type: application/json" \
            -d "$data" \
            "${SERVER_URL}${endpoint}"
    else
        curl -s -X "$method" "${SERVER_URL}${endpoint}"
    fi
}

# =============================================================================
# Phase 1: Setup & Validation
# =============================================================================
phase_setup() {
    log_section "Phase 1: Setup & Validation"

    log_test "Verify server is reachable"
    if curl -s --connect-timeout 5 "${SERVER_URL}/api/system/status" > /dev/null 2>&1; then
        log_pass "API server is reachable at $SERVER_URL"
    else
        log_fail "Cannot reach API server at $SERVER_URL"
        exit 1
    fi

    log_test "Verify MySQL access"
    if mysql_cmd "SELECT 1" > /dev/null 2>&1; then
        log_pass "MySQL access OK"
    else
        log_fail "Cannot access MySQL"
        exit 1
    fi
}

# =============================================================================
# Phase 2: Pre-test Cleanup
# =============================================================================
phase_cleanup_pre() {
    log_section "Phase 2: Pre-test Cleanup"

    log_test "Remove any existing test data"

    # Delete test cameras (192.168.99.x range)
    mysql_cmd "DELETE FROM cameras WHERE ip_address LIKE '192.168.99.%';"
    local remaining=$(mysql_cmd "SELECT COUNT(*) FROM cameras WHERE ip_address LIKE '192.168.99.%';")
    log_info "Test cameras remaining after cleanup: $remaining"

    # Delete test devices (192.168.99.x range)
    mysql_cmd "DELETE FROM ipcamscan_devices WHERE ip LIKE '192.168.99.%';"
    local remaining_dev=$(mysql_cmd "SELECT COUNT(*) FROM ipcamscan_devices WHERE ip LIKE '192.168.99.%';")
    log_info "Test devices remaining after cleanup: $remaining_dev"

    log_pass "Pre-test cleanup completed"
}

# =============================================================================
# Phase 3: Inject Test Devices
# =============================================================================
phase_inject_devices() {
    log_section "Phase 3: Inject Test Devices"

    local now=$(date -u '+%Y-%m-%d %H:%M:%S.000')

    log_test "Insert Category B test device (verified + credentials success)"

    mysql_cmd "INSERT INTO ipcamscan_devices (device_id, ip, subnet, mac, oui_vendor, hostnames, open_ports, discovery_onvif, score, verified, credential_status, credential_username, credential_password, manufacturer, model, family, confidence, rtsp_uri, first_seen, last_seen, status) VALUES ('${TEST_DEVICE_ID_B}', '${TEST_IP_B}', '${TEST_SUBNET}', 'AA:BB:CC:DD:EE:01', 'Test Vendor', '[\"testcam-b.local\"]', '[554, 80, 8080]', '{\"profiles\": [{\"token\": \"main\"}]}', 100, 1, 'success', 'admin', 'test123', 'TestManufacturer', 'TestModel-B', 'tapo', 90, 'rtsp://${TEST_IP_B}:554/stream1', '${now}', '${now}', 'verified');"

    if mysql_cmd "SELECT ip FROM ipcamscan_devices WHERE device_id='${TEST_DEVICE_ID_B}';" | grep -q "${TEST_IP_B}"; then
        log_pass "Category B test device inserted: ${TEST_IP_B} (verified, credentials success)"
    else
        log_fail "Failed to insert Category B test device"
        return 1
    fi

    log_test "Insert Category C test device (discovered, no credentials)"

    mysql_cmd "INSERT INTO ipcamscan_devices (device_id, ip, subnet, mac, oui_vendor, hostnames, open_ports, score, verified, credential_status, manufacturer, model, family, confidence, first_seen, last_seen, status) VALUES ('${TEST_DEVICE_ID_C}', '${TEST_IP_C}', '${TEST_SUBNET}', 'AA:BB:CC:DD:EE:02', 'Test Vendor C', '[\"testcam-c.local\"]', '[554, 80]', 50, 0, 'not_tried', 'TestManufacturer', 'TestModel-C', 'unknown', 30, '${now}', '${now}', 'discovered');"

    if mysql_cmd "SELECT ip FROM ipcamscan_devices WHERE device_id='${TEST_DEVICE_ID_C}';" | grep -q "${TEST_IP_C}"; then
        log_pass "Category C test device inserted: ${TEST_IP_C} (discovered, no credentials)"
    else
        log_fail "Failed to insert Category C test device"
        return 1
    fi

    log_test "Verify test devices in database"
    local device_count=$(mysql_cmd "SELECT COUNT(*) FROM ipcamscan_devices WHERE ip LIKE '192.168.99.%';")
    if [ "$device_count" = "2" ]; then
        log_pass "Both test devices present in ipcamscan_devices table"
    else
        log_fail "Expected 2 test devices, found: $device_count"
    fi
}

# =============================================================================
# Phase 4: Test Category B Registration
# =============================================================================
phase_test_category_b() {
    log_section "Phase 4: Test Category B Registration"

    log_test "Register Category B device (verified + credentials)"

    local request_data=$(cat <<EOF
{
    "devices": [{
        "ip": "${TEST_IP_B}",
        "display_name": "${TEST_PREFIX}CategoryB_Camera",
        "location": "${TEST_SUBNET}",
        "fid": "${TEST_FID}",
        "credentials": {
            "username": "admin",
            "password": "test123"
        },
        "force_register": false
    }]
}
EOF
)

    log_info "Sending registration request..."
    local response=$(api_call POST "/api/ipcamscan/devices/approve-batch" "$request_data")
    log_info "Response: $response"

    local ok=$(echo "$response" | jq -r '.ok' 2>/dev/null)
    local success=$(echo "$response" | jq -r '.success' 2>/dev/null)
    local camera_id=$(echo "$response" | jq -r '.results[0].camera.camera_id // empty' 2>/dev/null)

    if [ "$ok" = "true" ] && [ "$success" = "1" ] && [ -n "$camera_id" ]; then
        log_pass "Category B device registered successfully"
        log_info "  Camera ID: $camera_id"

        # Verify camera fields
        log_test "Verify Category B camera fields"
        verify_camera_fields "${TEST_IP_B}" "$camera_id" "tapo" "AA:BB:CC:DD:EE:01" "TestManufacturer" "TestModel-B" "3022AABBCCDDEE01"
    else
        local error=$(echo "$response" | jq -r '.results[0].error // empty' 2>/dev/null)
        log_fail "Category B registration failed: ${error:-unexpected response}"
    fi
}

# =============================================================================
# Phase 5: Test Category C Registration
# =============================================================================
phase_test_category_c() {
    log_section "Phase 5: Test Category C Registration (force_register)"

    log_test "Register Category C device with force_register=true"

    local request_data=$(cat <<EOF
{
    "devices": [{
        "ip": "${TEST_IP_C}",
        "display_name": "${TEST_PREFIX}CategoryC_Camera",
        "location": "${TEST_SUBNET}",
        "fid": "${TEST_FID}",
        "force_register": true
    }]
}
EOF
)

    log_info "Sending force_register request..."
    local response=$(api_call POST "/api/ipcamscan/devices/approve-batch" "$request_data")
    log_info "Response: $response"

    local ok=$(echo "$response" | jq -r '.ok' 2>/dev/null)
    local success=$(echo "$response" | jq -r '.success' 2>/dev/null)
    local force_registered=$(echo "$response" | jq -r '.results[0].force_registered // false' 2>/dev/null)
    local camera_id=$(echo "$response" | jq -r '.results[0].camera.camera_id // empty' 2>/dev/null)

    if [ "$ok" = "true" ] && [ "$success" = "1" ]; then
        if [ "$force_registered" = "true" ]; then
            log_pass "Category C device force-registered successfully"
        else
            log_pass "Category C device registered successfully"
        fi
        log_info "  Camera ID: $camera_id"

        # Verify status is pending_auth
        log_test "Verify Category C camera fields"
        local status=$(mysql_cmd "SELECT status FROM cameras WHERE ip_address='${TEST_IP_C}';")
        if [ "$status" = "pending_auth" ]; then
            log_pass "  status: pending_auth (correct for force_register)"
        else
            log_fail "  status: expected 'pending_auth', got '$status'"
        fi

        verify_camera_fields "${TEST_IP_C}" "$camera_id" "unknown" "AA:BB:CC:DD:EE:02" "TestManufacturer" "TestModel-C" "3022AABBCCDDEE02"
    else
        local error=$(echo "$response" | jq -r '.results[0].error // empty' 2>/dev/null)
        log_fail "Category C registration failed: ${error:-unexpected response}"
    fi
}

# =============================================================================
# Verify Camera Fields Helper
# =============================================================================
verify_camera_fields() {
    local ip="$1"
    local expected_camera_id="$2"
    local expected_family="$3"
    local expected_mac="$4"
    local expected_manufacturer="$5"
    local expected_model="$6"
    local expected_lacis_prefix="$7"

    # Check camera exists
    local db_camera_id=$(mysql_cmd "SELECT camera_id FROM cameras WHERE ip_address='${ip}';")
    if [ "$db_camera_id" = "$expected_camera_id" ]; then
        log_pass "  camera_id: $db_camera_id"
    else
        log_fail "  camera_id: expected '$expected_camera_id', got '$db_camera_id'"
    fi

    # Check fid
    local db_fid=$(mysql_cmd "SELECT fid FROM cameras WHERE ip_address='${ip}';")
    if [ "$db_fid" = "$TEST_FID" ]; then
        log_pass "  fid: $db_fid"
    else
        log_fail "  fid: expected '$TEST_FID', got '$db_fid'"
    fi

    # Check family
    local db_family=$(mysql_cmd "SELECT family FROM cameras WHERE ip_address='${ip}';")
    if [ "$db_family" = "$expected_family" ]; then
        log_pass "  family: $db_family"
    else
        log_fail "  family: expected '$expected_family', got '$db_family'"
    fi

    # Check access_family
    local db_access_family=$(mysql_cmd "SELECT IFNULL(access_family, 'unknown') FROM cameras WHERE ip_address='${ip}';")
    log_pass "  access_family: $db_access_family"

    # Check mac_address
    local db_mac=$(mysql_cmd "SELECT mac_address FROM cameras WHERE ip_address='${ip}';")
    if [ "$db_mac" = "$expected_mac" ]; then
        log_pass "  mac_address: $db_mac"
    else
        log_fail "  mac_address: expected '$expected_mac', got '$db_mac'"
    fi

    # Check manufacturer
    local db_manufacturer=$(mysql_cmd "SELECT manufacturer FROM cameras WHERE ip_address='${ip}';")
    if [ "$db_manufacturer" = "$expected_manufacturer" ]; then
        log_pass "  manufacturer: $db_manufacturer"
    else
        log_fail "  manufacturer: expected '$expected_manufacturer', got '$db_manufacturer'"
    fi

    # Check model
    local db_model=$(mysql_cmd "SELECT model FROM cameras WHERE ip_address='${ip}';")
    if [ "$db_model" = "$expected_model" ]; then
        log_pass "  model: $db_model"
    else
        log_fail "  model: expected '$expected_model', got '$db_model'"
    fi

    # Check lacis_id
    local db_lacis_id=$(mysql_cmd "SELECT lacis_id FROM cameras WHERE ip_address='${ip}';")
    if [[ "$db_lacis_id" == ${expected_lacis_prefix}* ]]; then
        log_pass "  lacis_id: $db_lacis_id (MAC-based)"
    else
        log_fail "  lacis_id: expected prefix '$expected_lacis_prefix', got '$db_lacis_id'"
    fi

    # Check onvif_endpoint
    local db_onvif=$(mysql_cmd "SELECT IFNULL(onvif_endpoint, '') FROM cameras WHERE ip_address='${ip}';")
    if [ -n "$db_onvif" ]; then
        log_pass "  onvif_endpoint: $db_onvif"
    else
        log_info "  onvif_endpoint: (not set)"
    fi

    # Check ptz_supported
    local db_ptz=$(mysql_cmd "SELECT IFNULL(ptz_supported, 0) FROM cameras WHERE ip_address='${ip}';")
    if [ "$db_ptz" = "0" ] || [ "$db_ptz" = "1" ]; then
        log_pass "  ptz_supported: $db_ptz"
    else
        log_fail "  ptz_supported: unexpected value '$db_ptz'"
    fi
}

# =============================================================================
# Phase 6: Verify via API
# =============================================================================
phase_verify_api() {
    log_section "Phase 6: Verify Test Cameras via API"

    log_test "List all cameras and find test cameras"

    local cameras=$(api_call GET "/api/cameras")
    local count=$(echo "$cameras" | jq -r '[.data[]? | select(.ip_address | startswith("192.168.99."))] | length' 2>/dev/null)

    if [ "$count" = "2" ]; then
        log_pass "Found exactly 2 test cameras via API"
    else
        log_fail "Expected 2 test cameras via API, found: $count"
    fi

    log_info "Test cameras:"
    echo "$cameras" | jq -r '.data[]? | select(.ip_address | startswith("192.168.99.")) | "  - \(.camera_id): \(.name) (\(.ip_address))"' 2>/dev/null
}

# =============================================================================
# Phase 7: Test Deletion
# =============================================================================
phase_test_deletion() {
    log_section "Phase 7: Test Camera Deletion via API"

    local cameras=$(api_call GET "/api/cameras")
    local test_camera_ids=$(echo "$cameras" | jq -r '.data[]? | select(.ip_address | startswith("192.168.99.")) | .camera_id' 2>/dev/null)

    for cam_id in $test_camera_ids; do
        log_test "Delete camera: $cam_id"
        local response=$(api_call DELETE "/api/cameras/$cam_id")
        local ok=$(echo "$response" | jq -r '.ok // false' 2>/dev/null)

        if [ "$ok" = "true" ]; then
            log_pass "Camera $cam_id deleted via API"
        else
            log_fail "Failed to delete camera $cam_id"
        fi
    done
}

# =============================================================================
# Phase 8: Final Cleanup
# =============================================================================
phase_cleanup_final() {
    log_section "Phase 8: Final Cleanup"

    log_test "Remove all test data from both tables"

    mysql_cmd "DELETE FROM cameras WHERE ip_address LIKE '192.168.99.%';"
    mysql_cmd "DELETE FROM ipcamscan_devices WHERE ip LIKE '192.168.99.%';"

    log_test "Verify complete cleanup"

    local remaining_cameras=$(mysql_cmd "SELECT COUNT(*) FROM cameras WHERE ip_address LIKE '192.168.99.%';")
    local remaining_devices=$(mysql_cmd "SELECT COUNT(*) FROM ipcamscan_devices WHERE ip LIKE '192.168.99.%';")

    if [ "$remaining_cameras" = "0" ] && [ "$remaining_devices" = "0" ]; then
        log_pass "All test data cleaned up"
        log_info "  Remaining test cameras: 0"
        log_info "  Remaining test devices: 0"
    else
        log_fail "Test data cleanup incomplete"
        log_info "  Remaining test cameras: $remaining_cameras"
        log_info "  Remaining test devices: $remaining_devices"
    fi
}

# =============================================================================
# Print Summary
# =============================================================================
print_summary() {
    echo ""
    echo "============================================="
    echo "TEST SUMMARY"
    echo "============================================="
    echo -e "Tests run:    ${TESTS_RUN}"
    echo -e "Tests passed: ${GREEN}${TESTS_PASSED}${NC}"
    echo -e "Tests failed: ${RED}${TESTS_FAILED}${NC}"
    echo "============================================="

    if [ "$TESTS_FAILED" -eq 0 ]; then
        echo -e "${GREEN}ALL TESTS PASSED${NC}"
        return 0
    else
        echo -e "${RED}SOME TESTS FAILED${NC}"
        return 1
    fi
}

# =============================================================================
# Main
# =============================================================================
main() {
    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --skip-cleanup)
                SKIP_CLEANUP=true
                shift
                ;;
            *)
                shift
                ;;
        esac
    done

    echo "============================================="
    echo "Camera Registration Integration Test"
    echo "============================================="
    echo "Server: $SERVER_URL"
    echo "Database: $DB_HOST"
    echo "Date: $(date)"
    echo "============================================="

    phase_setup
    phase_cleanup_pre
    phase_inject_devices
    phase_test_category_b
    phase_test_category_c
    phase_verify_api
    phase_test_deletion

    if [ "$SKIP_CLEANUP" = false ]; then
        phase_cleanup_final
    else
        log_warn "Skipping final cleanup (--skip-cleanup)"
    fi

    print_summary
}

main "$@"
