#!/bin/bash
# Session conflict and force login test

AC_IP="192.168.96.5"
USERNAME="isms"
PASSWORD="33QQrnYH2sefbwK"

echo "=== Session Conflict Investigation ==="
echo "Date: $(date)"
echo ""

# Test A: Single login, multiple requests
echo "### Test A: Fresh Login + Multiple Requests ###"
TOKEN_A=$(curl -s -X POST "http://${AC_IP}/" \
  -H "Content-Type: application/json; charset=UTF-8" \
  -d '{"method":"do","login":{"username":"'${USERNAME}'","password":"'${PASSWORD}'","force":0}}' | \
  python3 -c "import sys,json; print(json.load(sys.stdin).get('stok',''))")
echo "Token A: ${TOKEN_A:0:16}..."

for i in 1 2 3; do
  sleep 0.5  # 500ms delay
  RESULT=$(curl -s -X POST "http://${AC_IP}/stok=${TOKEN_A}/ds" \
    -H "Content-Type: application/json; charset=UTF-8" \
    -d '{"method":"get","apmng_client":{"table":"client_list","filter":[{"group_id":["-1"]}],"para":{"start":0,"end":99}}}' | \
    python3 -c "import sys,json; d=json.load(sys.stdin); print(f'error={d.get(\"error_code\")} clients={d.get(\"apmng_client\",{}).get(\"count\",{}).get(\"client_list\",\"N/A\")}')")
  echo "  Request ${i} (after 500ms): ${RESULT}"
done
echo ""

# Test B: force=1 login (should kick existing session)
echo "### Test B: Force Login (force=1) ###"
sleep 1
FORCE_RESP=$(curl -s -X POST "http://${AC_IP}/" \
  -H "Content-Type: application/json; charset=UTF-8" \
  -d '{"method":"do","login":{"username":"'${USERNAME}'","password":"'${PASSWORD}'","force":1}}')
TOKEN_B=$(echo "$FORCE_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin).get('stok',''))")
ERROR_B=$(echo "$FORCE_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin).get('error_code',''))")
echo "Force login error_code: ${ERROR_B}"
echo "Token B: ${TOKEN_B:0:16}..."

sleep 0.5
RESULT_B=$(curl -s -X POST "http://${AC_IP}/stok=${TOKEN_B}/ds" \
  -H "Content-Type: application/json; charset=UTF-8" \
  -d '{"method":"get","apmng_client":{"table":"client_list","filter":[{"group_id":["-1"]}],"para":{"start":0,"end":99}}}' | \
  python3 -c "import sys,json; d=json.load(sys.stdin); print(f'error={d.get(\"error_code\")} clients={d.get(\"apmng_client\",{}).get(\"count\",{}).get(\"client_list\",\"N/A\")}')")
echo "After force login: ${RESULT_B}"
echo ""

# Test C: Old token after force login
echo "### Test C: Old Token A After Force Login B ###"
RESULT_OLD=$(curl -s -X POST "http://${AC_IP}/stok=${TOKEN_A}/ds" \
  -H "Content-Type: application/json; charset=UTF-8" \
  -d '{"method":"get","apmng_client":{"table":"client_list","filter":[{"group_id":["-1"]}],"para":{"start":0,"end":99}}}' | \
  python3 -c "import sys,json; d=json.load(sys.stdin); print(f'error={d.get(\"error_code\")}')")
echo "Old token A result: ${RESULT_OLD}"
echo ""

# Test D: Delay between login and first request
echo "### Test D: Login + Delay + Request ###"
sleep 1
TOKEN_D=$(curl -s -X POST "http://${AC_IP}/" \
  -H "Content-Type: application/json; charset=UTF-8" \
  -d '{"method":"do","login":{"username":"'${USERNAME}'","password":"'${PASSWORD}'","force":1}}' | \
  python3 -c "import sys,json; print(json.load(sys.stdin).get('stok',''))")
echo "Token D: ${TOKEN_D:0:16}..."

echo "Waiting 2 seconds..."
sleep 2

RESULT_D=$(curl -s -X POST "http://${AC_IP}/stok=${TOKEN_D}/ds" \
  -H "Content-Type: application/json; charset=UTF-8" \
  -d '{"method":"get","apmng_client":{"table":"client_list","filter":[{"group_id":["-1"]}],"para":{"start":0,"end":99}}}' | \
  python3 -c "import sys,json; d=json.load(sys.stdin); print(f'error={d.get(\"error_code\")} clients={d.get(\"apmng_client\",{}).get(\"count\",{}).get(\"client_list\",\"N/A\")}')")
echo "After 2s delay: ${RESULT_D}"

