#!/bin/bash
# Token handling timing test for ESP32 implementation reference

AC_IP="192.168.96.5"
USERNAME="isms"
PASSWORD="33QQrnYH2sefbwK"  # Pre-encoded

echo "=== Mercury AC Token Timing Test ==="
echo "Date: $(date)"
echo ""

# Test 1: Login response time
echo "### Test 1: Login Response Time ###"
START=$(python3 -c "import time; print(time.time())")
LOGIN_RESP=$(curl -s -w "\n%{time_total}" -X POST "http://${AC_IP}/" \
  -H "Content-Type: application/json; charset=UTF-8" \
  -d '{"method":"do","login":{"username":"'${USERNAME}'","password":"'${PASSWORD}'","force":0}}')
LOGIN_TIME=$(echo "$LOGIN_RESP" | tail -1)
LOGIN_JSON=$(echo "$LOGIN_RESP" | head -1)
TOKEN=$(echo "$LOGIN_JSON" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('stok',''))" 2>/dev/null)
echo "Login response time: ${LOGIN_TIME}s"
echo "Token acquired: ${TOKEN:0:16}..."
echo ""

# Test 2: Valid token - data fetch response time
echo "### Test 2: Valid Token - Data Fetch Time ###"
DATA_RESP=$(curl -s -w "\n%{time_total}" -X POST "http://${AC_IP}/stok=${TOKEN}/ds" \
  -H "Content-Type: application/json; charset=UTF-8" \
  -d '{"method":"get","apmng_client":{"table":"client_list","filter":[{"group_id":["-1"]}],"para":{"start":0,"end":99}}}')
DATA_TIME=$(echo "$DATA_RESP" | tail -1)
DATA_JSON=$(echo "$DATA_RESP" | head -1)
ERROR_CODE=$(echo "$DATA_JSON" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('error_code',''))" 2>/dev/null)
echo "Data fetch time: ${DATA_TIME}s"
echo "Error code: ${ERROR_CODE}"
echo ""

# Test 3: Invalid token response time
echo "### Test 3: Invalid Token Response Time ###"
INVALID_RESP=$(curl -s -w "\n%{time_total}" -X POST "http://${AC_IP}/stok=invalidtoken12345678901234567890/ds" \
  -H "Content-Type: application/json; charset=UTF-8" \
  -d '{"method":"get","apmng_client":{"table":"client_list","filter":[{"group_id":["-1"]}],"para":{"start":0,"end":99}}}')
INVALID_TIME=$(echo "$INVALID_RESP" | tail -1)
INVALID_JSON=$(echo "$INVALID_RESP" | head -1)
INVALID_CODE=$(echo "$INVALID_JSON" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('error_code',''))" 2>/dev/null)
echo "Invalid token response time: ${INVALID_TIME}s"
echo "Error code: ${INVALID_CODE}"
echo "Full response: ${INVALID_JSON}"
echo ""

# Test 4: Immediate re-login after error
echo "### Test 4: Re-login After Token Error ###"
RELOGIN_RESP=$(curl -s -w "\n%{time_total}" -X POST "http://${AC_IP}/" \
  -H "Content-Type: application/json; charset=UTF-8" \
  -d '{"method":"do","login":{"username":"'${USERNAME}'","password":"'${PASSWORD}'","force":0}}')
RELOGIN_TIME=$(echo "$RELOGIN_RESP" | tail -1)
RELOGIN_JSON=$(echo "$RELOGIN_RESP" | head -1)
NEW_TOKEN=$(echo "$RELOGIN_JSON" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('stok',''))" 2>/dev/null)
echo "Re-login time: ${RELOGIN_TIME}s"
echo "New token: ${NEW_TOKEN:0:16}..."
echo ""

# Test 5: Verify new token works
echo "### Test 5: Verify New Token Works ###"
VERIFY_RESP=$(curl -s -w "\n%{time_total}" -X POST "http://${AC_IP}/stok=${NEW_TOKEN}/ds" \
  -H "Content-Type: application/json; charset=UTF-8" \
  -d '{"method":"get","apmng_status":{"table":"ap_list","filter":[{"group_id":["-1"]}],"para":{"start":0,"end":99}}}')
VERIFY_TIME=$(echo "$VERIFY_RESP" | tail -1)
VERIFY_JSON=$(echo "$VERIFY_RESP" | head -1)
VERIFY_CODE=$(echo "$VERIFY_JSON" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('error_code',''))" 2>/dev/null)
AP_COUNT=$(echo "$VERIFY_JSON" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('apmng_status',{}).get('count',{}).get('ap_list',0))" 2>/dev/null)
echo "Verify time: ${VERIFY_TIME}s"
echo "Error code: ${VERIFY_CODE}"
echo "AP count: ${AP_COUNT}"
echo ""

# Test 6: Consecutive requests with same token
echo "### Test 6: Consecutive Requests (Same Token) ###"
for i in 1 2 3; do
  CONSEC_RESP=$(curl -s -w "%{time_total}" -X POST "http://${AC_IP}/stok=${NEW_TOKEN}/ds" \
    -H "Content-Type: application/json; charset=UTF-8" \
    -d '{"method":"get","apmng_client":{"table":"client_list","filter":[{"group_id":["-1"]}],"para":{"start":0,"end":99}}}')
  echo "  Request ${i}: ${CONSEC_RESP}s"
done
echo ""

echo "=== TIMING SUMMARY ==="
echo "Login:          ${LOGIN_TIME}s"
echo "Data fetch:     ${DATA_TIME}s"
echo "Invalid token:  ${INVALID_TIME}s"
echo "Re-login:       ${RELOGIN_TIME}s"
echo "Verify:         ${VERIFY_TIME}s"

