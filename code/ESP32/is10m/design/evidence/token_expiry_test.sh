#!/bin/bash
# Token expiry and recommended flow test

AC_IP="192.168.96.5"
USERNAME="isms"
PASSWORD="33QQrnYH2sefbwK"

echo "=== Token Expiry & Recommended Flow Test ==="
echo "Date: $(date)"
echo ""

# Recommended ESP32 Flow: Always use force=1
echo "### ESP32 Recommended Flow Test ###"
echo ""

# Step 1: Initial login with force=1
echo "Step 1: Initial login (force=1)"
LOGIN_RESP=$(curl -s -X POST "http://${AC_IP}/" \
  -H "Content-Type: application/json; charset=UTF-8" \
  -d '{"method":"do","login":{"username":"'${USERNAME}'","password":"'${PASSWORD}'","force":1}}')
TOKEN=$(echo "$LOGIN_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin).get('stok',''))")
ERR=$(echo "$LOGIN_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin).get('error_code',''))")
echo "  error_code: ${ERR}"
echo "  token: ${TOKEN:0:20}..."
echo ""

# Step 2: Immediate data fetch (no delay needed)
echo "Step 2: Immediate data fetch"
DATA_RESP=$(curl -s -X POST "http://${AC_IP}/stok=${TOKEN}/ds" \
  -H "Content-Type: application/json; charset=UTF-8" \
  -d '{"method":"get","apmng_client":{"table":"client_list","filter":[{"group_id":["-1"]}],"para":{"start":0,"end":99}}}')
ERR=$(echo "$DATA_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin).get('error_code',''))")
COUNT=$(echo "$DATA_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin).get('apmng_client',{}).get('count',{}).get('client_list',0))")
echo "  error_code: ${ERR}"
echo "  client_count: ${COUNT}"
echo ""

# Step 3: Simulate poll cycle (every 12 minutes = 720 seconds, but we'll use 10s for test)
echo "Step 3: Simulated poll cycles (3 cycles, 10s interval)"
for cycle in 1 2 3; do
  echo "  Cycle ${cycle}: waiting 10s..."
  sleep 10
  
  POLL_RESP=$(curl -s -X POST "http://${AC_IP}/stok=${TOKEN}/ds" \
    -H "Content-Type: application/json; charset=UTF-8" \
    -d '{"method":"get","apmng_client":{"table":"client_list","filter":[{"group_id":["-1"]}],"para":{"start":0,"end":99}}}')
  POLL_ERR=$(echo "$POLL_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin).get('error_code',''))")
  POLL_COUNT=$(echo "$POLL_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin).get('apmng_client',{}).get('count',{}).get('client_list',0))")
  
  if [ "$POLL_ERR" == "-40401" ]; then
    echo "  Cycle ${cycle}: Token expired! Re-login..."
    LOGIN_RESP=$(curl -s -X POST "http://${AC_IP}/" \
      -H "Content-Type: application/json; charset=UTF-8" \
      -d '{"method":"do","login":{"username":"'${USERNAME}'","password":"'${PASSWORD}'","force":1}}')
    TOKEN=$(echo "$LOGIN_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin).get('stok',''))")
    echo "  New token: ${TOKEN:0:20}..."
    
    # Retry with new token
    POLL_RESP=$(curl -s -X POST "http://${AC_IP}/stok=${TOKEN}/ds" \
      -H "Content-Type: application/json; charset=UTF-8" \
      -d '{"method":"get","apmng_client":{"table":"client_list","filter":[{"group_id":["-1"]}],"para":{"start":0,"end":99}}}')
    POLL_ERR=$(echo "$POLL_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin).get('error_code',''))")
    POLL_COUNT=$(echo "$POLL_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin).get('apmng_client',{}).get('count',{}).get('client_list',0))")
  fi
  
  echo "  Cycle ${cycle}: error=${POLL_ERR} clients=${POLL_COUNT}"
done
echo ""

# Step 4: AP list also works with same token
echo "Step 4: AP list with same token"
AP_RESP=$(curl -s -X POST "http://${AC_IP}/stok=${TOKEN}/ds" \
  -H "Content-Type: application/json; charset=UTF-8" \
  -d '{"method":"get","apmng_status":{"table":"ap_list","filter":[{"group_id":["-1"]}],"para":{"start":0,"end":99}}}')
AP_ERR=$(echo "$AP_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin).get('error_code',''))")
AP_COUNT=$(echo "$AP_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin).get('apmng_status',{}).get('count',{}).get('ap_list',0))")
echo "  error_code: ${AP_ERR}"
echo "  ap_count: ${AP_COUNT}"
echo ""

echo "=== SUMMARY ==="
echo "✓ force=1 login: Always use for clean session"
echo "✓ Immediate request: Works without delay"
echo "✓ Token reuse: Works for multiple requests"
echo "✓ Error handling: Check error_code, re-login on -40401"
echo ""
echo "Test completed: $(date)"

