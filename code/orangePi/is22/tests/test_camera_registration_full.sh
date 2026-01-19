#!/bin/bash
# =============================================================================
# Camera Registration Flow Full Integration Test
# =============================================================================
# Complete test suite that:
# 1. Injects test devices directly into DB (ipcamscan_devices)
# 2. Tests Category B registration (verified + credentials)
# 3. Tests Category C registration (force_register)
# 4. Verifies cameras are created correctly
# 5. Cleans up ALL test data (both tables)
#
# Usage:
#   ./test_camera_registration_full.sh
#
# Requirements:
#   - sshpass installed
#   - Network access to 192.168.125.246
# =============================================================================

set -e

# Configuration
SERVER_HOST="192.168.125.246"
SERVER_PORT="3000"
SERVER_URL="http://${SERVER_HOST}:${SERVER_PORT}"
SSH_USER="${CAMSERVER_SSH_USER:-mijeosadmin}"
SSH_PASS="${CAMSERVER_SSH_PASS:?Error: CAMSERVER_SSH_PASS environment variable not set}"
DB_USER="${CAMSERVER_DB_USER:-root}"
DB_PASS="${CAMSERVER_DB_PASS:?Error: CAMSERVER_DB_PASS environment variable not set}"
DB_NAME="${CAMSERVER_DB_NAME:-camserver}"

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
NC='\033[0m' # No Color

# Counters
TESTS_RUN=0
TESTS_PASSED=0
TESTS_FAILED=0

# Logging functions
log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_pass() { echo -e "${GREEN}[PASS]${NC} $1"; ((TESTS_PASSED++)) || true; }
log_fail() { echo -e "${RED}[FAIL]${NC} $1"; ((TESTS_FAILED++)) || true; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_test() { echo -e "\n${CYAN}=== TEST: $1 ===${NC}"; ((TESTS_RUN++)) || true; }
log_section() { echo -e "\n${YELLOW}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"; echo -e "${YELLOW}  $1${NC}"; echo -e "${YELLOW}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"; }

# SSH command helper
ssh_cmd() {
    sshpass -p "$SSH_PASS" ssh -o StrictHostKeyChecking=no -o ConnectTimeout=10 "${SSH_USER}@${SERVER_HOST}" "$1"
}

# MySQL command helper
mysql_cmd() {
    ssh_cmd "mysql -u${DB_USER} -p'${DB_PASS}' ${DB_NAME} -e \"$1\""
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

    log_test "Verify SSH access"
    if ssh_cmd "echo 'SSH OK'" > /dev/null 2>&1; then
        log_pass "SSH access to server OK"
    else
        log_fail "Cannot SSH to server"
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
    local deleted_cameras=$(mysql_cmd "DELETE FROM cameras WHERE ip_address LIKE '192.168.99.%'; SELECT ROW_COUNT();" 2>/dev/null | tail -1)
    log_info "Deleted $deleted_cameras test cameras from cameras table"

    # Delete test devices (192.168.99.x range)
    local deleted_devices=$(mysql_cmd "DELETE FROM ipcamscan_devices WHERE ip LIKE '192.168.99.%'; SELECT ROW_COUNT();" 2>/dev/null | tail -1)
    log_info "Deleted $deleted_devices test devices from ipcamscan_devices table"

    log_pass "Pre-test cleanup completed"
}

# =============================================================================
# Phase 3: Inject Test Devices
# =============================================================================
phase_inject_devices() {
    log_section "Phase 3: Inject Test Devices"

    local now=$(date -u '+%Y-%m-%d %H:%M:%S.000')

    log_test "Insert Category B test device (verified + credentials success)"

    # Column order based on actual table structure:
    # device_id, ip, subnet, mac, oui_vendor, hostnames, open_ports,
    # discovery_onvif, discovery_ssdp, discovery_mdns, score, verified,
    # credential_status, credential_username, credential_password,
    # manufacturer, model, firmware, family, confidence, rtsp_uri,
    # first_seen, last_seen, status, detection_json

    # Create SQL file on server and execute it to avoid escaping issues
    local sql_b="INSERT INTO ipcamscan_devices (device_id, ip, subnet, mac, oui_vendor, hostnames, open_ports, discovery_onvif, score, verified, credential_status, credential_username, credential_password, manufacturer, model, family, confidence, rtsp_uri, first_seen, last_seen, status) VALUES ('${TEST_DEVICE_ID_B}', '${TEST_IP_B}', '${TEST_SUBNET}', 'AA:BB:CC:DD:EE:01', 'Test Vendor', '[\\\"testcam-b.local\\\"]', '[554, 80, 8080]', '{\\\"profiles\\\": [{\\\"token\\\": \\\"main\\\"}]}', 100, 1, 'success', 'admin', 'test123', 'TestManufacturer', 'TestModel-B', 'tapo', 90, 'rtsp://${TEST_IP_B}:554/stream1', '${now}', '${now}', 'verified');"

    ssh_cmd "echo \"$sql_b\" | mysql -u${DB_USER} -p'${DB_PASS}' ${DB_NAME}" 2>&1

    if mysql_cmd "SELECT ip FROM ipcamscan_devices WHERE device_id='${TEST_DEVICE_ID_B}';" 2>/dev/null | grep -q "${TEST_IP_B}"; then
        log_pass "Category B test device inserted: ${TEST_IP_B} (verified, credentials success)"
    else
        log_fail "Failed to insert Category B test device"
        return 1
    fi

    log_test "Insert Category C test device (discovered, no credentials)"

    local sql_c="INSERT INTO ipcamscan_devices (device_id, ip, subnet, mac, oui_vendor, hostnames, open_ports, score, verified, credential_status, manufacturer, model, family, confidence, first_seen, last_seen, status) VALUES ('${TEST_DEVICE_ID_C}', '${TEST_IP_C}', '${TEST_SUBNET}', 'AA:BB:CC:DD:EE:02', 'Test Vendor C', '[\\\"testcam-c.local\\\"]', '[554, 80]', 50, 0, 'not_tried', 'TestManufacturer', 'TestModel-C', 'unknown', 30, '${now}', '${now}', 'discovered');"

    ssh_cmd "echo \"$sql_c\" | mysql -u${DB_USER} -p'${DB_PASS}' ${DB_NAME}" 2>&1

    if mysql_cmd "SELECT ip FROM ipcamscan_devices WHERE device_id='${TEST_DEVICE_ID_C}';" 2>/dev/null | grep -q "${TEST_IP_C}"; then
        log_pass "Category C test device inserted: ${TEST_IP_C} (discovered, no credentials)"
    else
        log_fail "Failed to insert Category C test device"
        return 1
    fi

    log_test "Verify test devices in database"
    local device_count=$(mysql_cmd "SELECT COUNT(*) FROM ipcamscan_devices WHERE ip LIKE '192.168.99.%';" 2>/dev/null | tail -1)
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
    local error=$(echo "$response" | jq -r '.results[0].error // empty' 2>/dev/null)

    if [ "$ok" = "true" ] && [ "$success" = "1" ] && [ -n "$camera_id" ]; then
        log_pass "Category B device registered successfully"
        log_info "  Camera ID: $camera_id"

        # Verify camera in DB - ALL important fields
        log_test "Verify Category B camera fields in database"
        local db_camera=$(mysql_cmd "SELECT camera_id, name, ip_address, status, fid, family, access_family, mac_address, manufacturer, model, lacis_id, onvif_endpoint FROM cameras WHERE ip_address='${TEST_IP_B}'\\G" 2>/dev/null)
        if echo "$db_camera" | grep -q "$camera_id"; then
            log_pass "Camera found in database with correct camera_id"

            # Check status
            local status=$(mysql_cmd "SELECT status FROM cameras WHERE ip_address='${TEST_IP_B}';" 2>/dev/null | tail -1)
            log_info "  status: $status"

            # Check fid
            local db_fid=$(mysql_cmd "SELECT fid FROM cameras WHERE ip_address='${TEST_IP_B}';" 2>/dev/null | tail -1)
            if [ "$db_fid" = "$TEST_FID" ]; then
                log_pass "  fid: $db_fid ✓"
            else
                log_fail "  fid: expected $TEST_FID, got $db_fid"
            fi

            # Check family (from device)
            local db_family=$(mysql_cmd "SELECT family FROM cameras WHERE ip_address='${TEST_IP_B}';" 2>/dev/null | tail -1)
            if [ "$db_family" = "tapo" ]; then
                log_pass "  family: $db_family ✓"
            else
                log_fail "  family: expected 'tapo', got '$db_family'"
            fi

            # Check access_family (auto-set from manufacturer/model)
            local db_access_family=$(mysql_cmd "SELECT access_family FROM cameras WHERE ip_address='${TEST_IP_B}';" 2>/dev/null | tail -1)
            if [ -n "$db_access_family" ]; then
                log_pass "  access_family: $db_access_family ✓"
            else
                log_warn "  access_family: not set (may be NULL)"
            fi

            # Check mac_address
            local db_mac=$(mysql_cmd "SELECT mac_address FROM cameras WHERE ip_address='${TEST_IP_B}';" 2>/dev/null | tail -1)
            if [ "$db_mac" = "AA:BB:CC:DD:EE:01" ]; then
                log_pass "  mac_address: $db_mac ✓"
            else
                log_fail "  mac_address: expected 'AA:BB:CC:DD:EE:01', got '$db_mac'"
            fi

            # Check manufacturer
            local db_manufacturer=$(mysql_cmd "SELECT manufacturer FROM cameras WHERE ip_address='${TEST_IP_B}';" 2>/dev/null | tail -1)
            if [ "$db_manufacturer" = "TestManufacturer" ]; then
                log_pass "  manufacturer: $db_manufacturer ✓"
            else
                log_fail "  manufacturer: expected 'TestManufacturer', got '$db_manufacturer'"
            fi

            # Check model
            local db_model=$(mysql_cmd "SELECT model FROM cameras WHERE ip_address='${TEST_IP_B}';" 2>/dev/null | tail -1)
            if [ "$db_model" = "TestModel-B" ]; then
                log_pass "  model: $db_model ✓"
            else
                log_fail "  model: expected 'TestModel-B', got '$db_model'"
            fi

            # Check lacis_id (should be generated from MAC)
            local db_lacis_id=$(mysql_cmd "SELECT lacis_id FROM cameras WHERE ip_address='${TEST_IP_B}';" 2>/dev/null | tail -1)
            if [[ "$db_lacis_id" == 3022AABBCCDDEE01* ]]; then
                log_pass "  lacis_id: $db_lacis_id ✓ (MAC-based)"
            else
                log_fail "  lacis_id: expected '3022AABBCCDDEE01*', got '$db_lacis_id'"
            fi

            # Check onvif_endpoint (auto-generated from family)
            local db_onvif=$(mysql_cmd "SELECT onvif_endpoint FROM cameras WHERE ip_address='${TEST_IP_B}';" 2>/dev/null | tail -1)
            if [ -n "$db_onvif" ]; then
                log_pass "  onvif_endpoint: $db_onvif ✓"
            else
                log_warn "  onvif_endpoint: not set"
            fi

            # Check ptz_supported (auto-detected via ONVIF probe, will be 0 for test devices)
            local db_ptz=$(mysql_cmd "SELECT ptz_supported FROM cameras WHERE ip_address='${TEST_IP_B}';" 2>/dev/null | tail -1)
            if [ "$db_ptz" = "0" ] || [ "$db_ptz" = "1" ]; then
                log_pass "  ptz_supported: $db_ptz ✓ (field exists)"
            else
                log_fail "  ptz_supported: unexpected value '$db_ptz'"
            fi
        else
            log_fail "Camera not found in database"
        fi
    else
        if [ -n "$error" ]; then
            log_fail "Category B registration failed: $error"
        else
            log_fail "Category B registration failed: unexpected response"
        fi
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
    local error=$(echo "$response" | jq -r '.results[0].error // empty' 2>/dev/null)

    if [ "$ok" = "true" ] && [ "$success" = "1" ]; then
        if [ "$force_registered" = "true" ]; then
            log_pass "Category C device force-registered successfully (force_registered=true)"
        else
            log_pass "Category C device registered successfully"
        fi
        log_info "  Camera ID: $camera_id"

        # Verify camera in DB - ALL important fields
        log_test "Verify Category C camera fields in database"
        local db_camera=$(mysql_cmd "SELECT camera_id, name, ip_address, status, fid, family, access_family, mac_address, manufacturer, model, lacis_id, onvif_endpoint, polling_enabled FROM cameras WHERE ip_address='${TEST_IP_C}'\\G" 2>/dev/null)
        if echo "$db_camera" | grep -q "$camera_id"; then
            log_pass "Camera found in database"

            # Check status should be 'pending_auth' for force-registered cameras
            local status=$(mysql_cmd "SELECT status FROM cameras WHERE ip_address='${TEST_IP_C}';" 2>/dev/null | tail -1)
            if [ "$status" = "pending_auth" ]; then
                log_pass "  status: pending_auth ✓"
            else
                log_fail "  status: expected 'pending_auth', got '$status'"
            fi

            # Check fid
            local db_fid=$(mysql_cmd "SELECT fid FROM cameras WHERE ip_address='${TEST_IP_C}';" 2>/dev/null | tail -1)
            if [ "$db_fid" = "$TEST_FID" ]; then
                log_pass "  fid: $db_fid ✓"
            else
                log_fail "  fid: expected $TEST_FID, got $db_fid"
            fi

            # Check polling_enabled should be false for force-registered
            local polling=$(mysql_cmd "SELECT polling_enabled FROM cameras WHERE ip_address='${TEST_IP_C}';" 2>/dev/null | tail -1)
            if [ "$polling" = "0" ]; then
                log_pass "  polling_enabled: false ✓"
            else
                log_fail "  polling_enabled: expected '0', got '$polling'"
            fi

            # Check family (from device, or 'unknown' if not set)
            local db_family=$(mysql_cmd "SELECT family FROM cameras WHERE ip_address='${TEST_IP_C}';" 2>/dev/null | tail -1)
            if [ "$db_family" = "unknown" ]; then
                log_pass "  family: $db_family ✓ (expected for discovered device)"
            else
                log_info "  family: $db_family (inherited from device)"
            fi

            # Check access_family
            local db_access_family=$(mysql_cmd "SELECT access_family FROM cameras WHERE ip_address='${TEST_IP_C}';" 2>/dev/null | tail -1)
            if [ -n "$db_access_family" ]; then
                log_pass "  access_family: $db_access_family ✓"
            else
                log_warn "  access_family: not set (may be NULL)"
            fi

            # Check mac_address
            local db_mac=$(mysql_cmd "SELECT mac_address FROM cameras WHERE ip_address='${TEST_IP_C}';" 2>/dev/null | tail -1)
            if [ "$db_mac" = "AA:BB:CC:DD:EE:02" ]; then
                log_pass "  mac_address: $db_mac ✓"
            else
                log_fail "  mac_address: expected 'AA:BB:CC:DD:EE:02', got '$db_mac'"
            fi

            # Check manufacturer
            local db_manufacturer=$(mysql_cmd "SELECT manufacturer FROM cameras WHERE ip_address='${TEST_IP_C}';" 2>/dev/null | tail -1)
            if [ "$db_manufacturer" = "TestManufacturer" ]; then
                log_pass "  manufacturer: $db_manufacturer ✓"
            else
                log_fail "  manufacturer: expected 'TestManufacturer', got '$db_manufacturer'"
            fi

            # Check model
            local db_model=$(mysql_cmd "SELECT model FROM cameras WHERE ip_address='${TEST_IP_C}';" 2>/dev/null | tail -1)
            if [ "$db_model" = "TestModel-C" ]; then
                log_pass "  model: $db_model ✓"
            else
                log_fail "  model: expected 'TestModel-C', got '$db_model'"
            fi

            # Check lacis_id (should be generated from MAC)
            local db_lacis_id=$(mysql_cmd "SELECT lacis_id FROM cameras WHERE ip_address='${TEST_IP_C}';" 2>/dev/null | tail -1)
            if [[ "$db_lacis_id" == 3022AABBCCDDEE02* ]]; then
                log_pass "  lacis_id: $db_lacis_id ✓ (MAC-based)"
            else
                log_fail "  lacis_id: expected '3022AABBCCDDEE02*', got '$db_lacis_id'"
            fi

            # Check onvif_endpoint
            local db_onvif=$(mysql_cmd "SELECT onvif_endpoint FROM cameras WHERE ip_address='${TEST_IP_C}';" 2>/dev/null | tail -1)
            if [ -n "$db_onvif" ] && [ "$db_onvif" != "NULL" ]; then
                log_pass "  onvif_endpoint: $db_onvif ✓"
            else
                log_info "  onvif_endpoint: not set (expected for force_register without credentials)"
            fi

            # Check ptz_supported (will be 0 for force_register without probe)
            local db_ptz=$(mysql_cmd "SELECT ptz_supported FROM cameras WHERE ip_address='${TEST_IP_C}';" 2>/dev/null | tail -1)
            if [ "$db_ptz" = "0" ]; then
                log_pass "  ptz_supported: $db_ptz ✓ (expected for force_register)"
            elif [ "$db_ptz" = "1" ]; then
                log_pass "  ptz_supported: $db_ptz ✓"
            else
                log_fail "  ptz_supported: unexpected value '$db_ptz'"
            fi
        else
            log_fail "Camera not found in database"
        fi
    else
        if [ -n "$error" ]; then
            log_fail "Category C registration failed: $error"
        else
            log_fail "Category C registration failed: unexpected response"
        fi
    fi
}

# =============================================================================
# Phase 6: Verify All Test Cameras via API
# =============================================================================
phase_verify_api() {
    log_section "Phase 6: Verify Test Cameras via API"

    log_test "List all cameras and find test cameras"

    local cameras=$(api_call GET "/api/cameras")
    local test_cameras=$(echo "$cameras" | jq -r '.data[]? | select(.ip_address | startswith("192.168.99."))' 2>/dev/null)

    local count=$(echo "$cameras" | jq -r '[.data[]? | select(.ip_address | startswith("192.168.99."))] | length' 2>/dev/null)

    if [ "$count" = "2" ]; then
        log_pass "Found exactly 2 test cameras via API"
    else
        log_fail "Expected 2 test cameras via API, found: $count"
    fi

    # Show test cameras
    log_info "Test cameras found:"
    echo "$cameras" | jq -r '.data[]? | select(.ip_address | startswith("192.168.99.")) | "  - \(.camera_id): \(.name) (\(.ip_address)) status=\(.status)"' 2>/dev/null
}

# =============================================================================
# Phase 7: Test Camera Deletion
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
            log_pass "Camera $cam_id deleted successfully via API"
        else
            log_fail "Failed to delete camera $cam_id: $response"
        fi
    done
}

# =============================================================================
# Phase 8: Final Cleanup
# =============================================================================
phase_cleanup_final() {
    log_section "Phase 8: Final Cleanup"

    log_test "Remove all test data from both tables"

    # Delete any remaining test cameras
    local deleted_cameras=$(mysql_cmd "DELETE FROM cameras WHERE ip_address LIKE '192.168.99.%'; SELECT ROW_COUNT();" 2>/dev/null | tail -1)
    log_info "Deleted $deleted_cameras remaining test cameras"

    # Delete test devices from ipcamscan_devices
    local deleted_devices=$(mysql_cmd "DELETE FROM ipcamscan_devices WHERE ip LIKE '192.168.99.%'; SELECT ROW_COUNT();" 2>/dev/null | tail -1)
    log_info "Deleted $deleted_devices test devices from ipcamscan_devices"

    # Verify cleanup
    log_test "Verify complete cleanup"

    local remaining_cameras=$(mysql_cmd "SELECT COUNT(*) FROM cameras WHERE ip_address LIKE '192.168.99.%';" 2>/dev/null | tail -1)
    local remaining_devices=$(mysql_cmd "SELECT COUNT(*) FROM ipcamscan_devices WHERE ip LIKE '192.168.99.%';" 2>/dev/null | tail -1)

    if [ "$remaining_cameras" = "0" ] && [ "$remaining_devices" = "0" ]; then
        log_pass "All test data cleaned up successfully"
        log_info "  Remaining test cameras: 0"
        log_info "  Remaining test devices: 0"
    else
        log_fail "Test data cleanup incomplete"
        log_info "  Remaining test cameras: $remaining_cameras"
        log_info "  Remaining test devices: $remaining_devices"
    fi
}

# =============================================================================
# Print test summary
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
        echo -e "${GREEN}✓ ALL TESTS PASSED${NC}"
        return 0
    else
        echo -e "${RED}✗ SOME TESTS FAILED${NC}"
        return 1
    fi
}

# =============================================================================
# Main execution
# =============================================================================
main() {
    echo "============================================="
    echo "Camera Registration Flow Full Integration Test"
    echo "============================================="
    echo "Server: $SERVER_URL"
    echo "SSH:    ${SSH_USER}@${SERVER_HOST}"
    echo "Date:   $(date)"
    echo "============================================="

    # Run all phases
    phase_setup
    phase_cleanup_pre
    phase_inject_devices
    phase_test_category_b
    phase_test_category_c
    phase_verify_api
    phase_test_deletion
    phase_cleanup_final

    print_summary
}

# Run main
main "$@"
