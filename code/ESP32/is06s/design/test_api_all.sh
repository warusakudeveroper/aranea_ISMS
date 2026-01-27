#!/bin/bash
# IS06S 全デバイス APIアクセステスト
# 対象: 192.168.77.101 〜 192.168.77.109

SUBNET="192.168.77"
START=101
END=109
TIMEOUT=3

GREEN="\033[32m"
RED="\033[31m"
YELLOW="\033[33m"
CYAN="\033[36m"
RESET="\033[0m"

echo "======================================"
echo " IS06S API Access Test"
echo " Range: ${SUBNET}.${START} - ${SUBNET}.${END}"
echo " $(date '+%Y-%m-%d %H:%M:%S')"
echo "======================================"
echo ""

PASS=0
FAIL=0
OFFLINE=0

for i in $(seq $START $END); do
  IP="${SUBNET}.${i}"
  printf "${CYAN}%-18s${RESET} " "${IP}"

  # 1. Ping check
  if ! ping -c 1 -W 1 "$IP" > /dev/null 2>&1; then
    printf "${RED}OFFLINE${RESET} (ping failed)\n"
    OFFLINE=$((OFFLINE + 1))
    continue
  fi

  # 2. GET /api/status
  STATUS=$(curl -s -m $TIMEOUT "http://${IP}/api/status" 2>/dev/null)
  if [ $? -ne 0 ] || [ -z "$STATUS" ]; then
    printf "${RED}FAIL${RESET} /api/status unreachable\n"
    FAIL=$((FAIL + 1))
    continue
  fi

  # Extract device info from nested JSON
  DEVICE_INFO=$(echo "$STATUS" | python3 -c "
import sys, json
d = json.load(sys.stdin)
dev = d.get('device', {})
sys_info = d.get('system', {})
fw = d.get('firmware', {})
net = d.get('network', {})
print(dev.get('hostname', '?'))
print(dev.get('mac', '?'))
print(sys_info.get('heap', '?'))
print(sys_info.get('uptimeHuman', '?'))
print(fw.get('version', '?'))
print(fw.get('uiVersion', '?'))
print(net.get('rssi', '?'))
print(sys_info.get('chipTemp', '?'))
" 2>/dev/null)

  HOSTNAME=$(echo "$DEVICE_INFO" | sed -n '1p')
  MAC=$(echo "$DEVICE_INFO" | sed -n '2p')
  HEAP=$(echo "$DEVICE_INFO" | sed -n '3p')
  UPTIME=$(echo "$DEVICE_INFO" | sed -n '4p')
  FW_VER=$(echo "$DEVICE_INFO" | sed -n '5p')
  UI_VER=$(echo "$DEVICE_INFO" | sed -n '6p')
  RSSI=$(echo "$DEVICE_INFO" | sed -n '7p')
  TEMP=$(echo "$DEVICE_INFO" | sed -n '8p')

  # 3. GET /api/pin/all
  PIN_ALL=$(curl -s -m $TIMEOUT "http://${IP}/api/pin/all" 2>/dev/null)
  PIN_OK="NG"
  if echo "$PIN_ALL" | python3 -c "import sys,json; d=json.load(sys.stdin); assert 'pins' in d" 2>/dev/null; then
    PIN_COUNT=$(echo "$PIN_ALL" | python3 -c "import sys,json; d=json.load(sys.stdin); print(len(d['pins']))" 2>/dev/null)
    PIN_OK="OK(${PIN_COUNT}ch)"
  fi

  # 4. GET /api/ota/status
  OTA=$(curl -s -m $TIMEOUT "http://${IP}/api/ota/status" 2>/dev/null)
  OTA_OK="NG"
  if echo "$OTA" | python3 -c "import sys,json; d=json.load(sys.stdin); assert 'status' in d" 2>/dev/null; then
    OTA_STATUS=$(echo "$OTA" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['status'])" 2>/dev/null)
    OTA_OK="OK(${OTA_STATUS})"
  fi

  # 5. POST /api/pin/1/setting (write test - read current, write same back)
  WRITE_OK="NG"
  CURRENT=$(curl -s -m $TIMEOUT "http://${IP}/api/pin/1/setting" 2>/dev/null)
  if [ -n "$CURRENT" ]; then
    WRITE_RESP=$(curl -s -m $TIMEOUT -X POST -H "Content-Type: application/json" -d "$CURRENT" "http://${IP}/api/pin/1/setting" 2>/dev/null)
    if echo "$WRITE_RESP" | grep -q '"ok":true' 2>/dev/null; then
      WRITE_OK="OK"
    fi
  fi

  # Print result
  printf "${GREEN}PASS${RESET}  %-12s mac=%-13s fw=%s/UI%s heap=%-6s rssi=%-4s temp=%-4s up=%-12s pin=%-9s ota=%-10s write=%s\n" \
    "$HOSTNAME" "$MAC" "$FW_VER" "$UI_VER" "$HEAP" "$RSSI" "${TEMP}C" "$UPTIME" "$PIN_OK" "$OTA_OK" "$WRITE_OK"
  PASS=$((PASS + 1))
done

echo ""
echo "======================================"
printf " Results: ${GREEN}PASS=${PASS}${RESET}  ${RED}FAIL=${FAIL}${RESET}  ${YELLOW}OFFLINE=${OFFLINE}${RESET}  / Total=%d\n" $((END - START + 1))
echo "======================================"
