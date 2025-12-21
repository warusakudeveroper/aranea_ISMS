#!/bin/bash
# Get raw API response samples

AC_IP="192.168.96.5"
USERNAME="isms"
PASSWORD="33QQrnYH2sefbwK"

echo "=== Getting Raw API Samples ==="
echo "Date: $(date)"
echo ""

# Login
echo "Step 1: Login..."
LOGIN_RESP=$(curl -s -X POST "http://${AC_IP}/" \
  -H "Content-Type: application/json; charset=UTF-8" \
  -d '{"method":"do","login":{"username":"'${USERNAME}'","password":"'${PASSWORD}'","force":1}}')
echo "$LOGIN_RESP" | python3 -m json.tool > samples/login_response.json 2>/dev/null || echo "$LOGIN_RESP" > samples/login_response.json

TOKEN=$(echo "$LOGIN_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin).get('stok',''))")
echo "Token: ${TOKEN:0:16}..."
echo ""

# Client List
echo "Step 2: Client List..."
CLIENT_RESP=$(curl -s -X POST "http://${AC_IP}/stok=${TOKEN}/ds" \
  -H "Content-Type: application/json; charset=UTF-8" \
  -d '{"method":"get","apmng_client":{"table":"client_list","filter":[{"group_id":["-1"]}],"para":{"start":0,"end":99}}}')
echo "$CLIENT_RESP" | python3 -m json.tool > samples/client_list_response.json 2>/dev/null || echo "$CLIENT_RESP" > samples/client_list_response.json
CLIENT_COUNT=$(echo "$CLIENT_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin).get('apmng_client',{}).get('count',{}).get('client_list',0))")
echo "Clients: ${CLIENT_COUNT}"
echo ""

# AP List
echo "Step 3: AP List..."
AP_RESP=$(curl -s -X POST "http://${AC_IP}/stok=${TOKEN}/ds" \
  -H "Content-Type: application/json; charset=UTF-8" \
  -d '{"method":"get","apmng_status":{"table":"ap_list","filter":[{"group_id":["-1"]}],"para":{"start":0,"end":99}}}')
echo "$AP_RESP" | python3 -m json.tool > samples/ap_list_response.json 2>/dev/null || echo "$AP_RESP" > samples/ap_list_response.json
AP_COUNT=$(echo "$AP_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin).get('apmng_status',{}).get('count',{}).get('ap_list',0))")
echo "APs: ${AP_COUNT}"
echo ""

echo "=== Samples saved to samples/ ==="
ls -la samples/

