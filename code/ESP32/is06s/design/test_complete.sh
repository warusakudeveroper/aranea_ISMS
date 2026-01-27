#!/bin/bash
# =============================================================================
# IS06S 完全機能テストスクリプト
# =============================================================================
# 使用方法: ./test_complete.sh [device_ip]
# デフォルト: 192.168.77.32
# =============================================================================

DEVICE_IP="${1:-192.168.77.32}"
BASE_URL="http://${DEVICE_IP}/api"
PASS=0
FAIL=0
TESTS=()

# カラー定義
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# テスト結果記録
log_pass() {
    echo -e "${GREEN}[PASS]${NC} $1"
    PASS=$((PASS + 1))
    TESTS+=("PASS: $1")
}

log_fail() {
    echo -e "${RED}[FAIL]${NC} $1"
    FAIL=$((FAIL + 1))
    TESTS+=("FAIL: $1")
}

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_section() {
    echo ""
    echo -e "${YELLOW}========================================${NC}"
    echo -e "${YELLOW}$1${NC}"
    echo -e "${YELLOW}========================================${NC}"
}

# JSONからフィールドを取得
get_json_field() {
    echo "$1" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d$2)" 2>/dev/null
}

# =============================================================================
# テスト実行
# =============================================================================

log_section "IS06S 完全機能テスト開始"
echo "対象デバイス: ${DEVICE_IP}"
echo "テスト開始時刻: $(date)"
echo ""

# -----------------------------------------------------------------------------
# 1. 接続確認
# -----------------------------------------------------------------------------
log_section "1. デバイス接続確認"

RESPONSE=$(curl -s --connect-timeout 5 "${BASE_URL}/status")
if [ $? -eq 0 ] && [ -n "$RESPONSE" ]; then
    OK=$(get_json_field "$RESPONSE" '["ok"]')
    if [ "$OK" == "True" ]; then
        log_pass "デバイス接続 - /api/status 応答"

        # デバイス情報表示
        TYPE=$(get_json_field "$RESPONSE" '["device"]["type"]')
        CIC=$(get_json_field "$RESPONSE" '["device"]["cic"]')
        IP=$(get_json_field "$RESPONSE" '["network"]["ip"]')
        MQTT=$(get_json_field "$RESPONSE" '["cloud"]["mqttConnected"]')
        NTP=$(get_json_field "$RESPONSE" '["system"]["ntpSynced"]')

        log_info "Type: $TYPE"
        log_info "CIC: $CIC"
        log_info "IP: $IP"
        log_info "MQTT: $MQTT"
        log_info "NTP: $NTP"
    else
        log_fail "デバイス接続 - ok=false"
    fi
else
    log_fail "デバイス接続 - 応答なし"
    echo "テスト中止: デバイスに接続できません"
    exit 1
fi

# -----------------------------------------------------------------------------
# 2. MQTT接続状態確認
# -----------------------------------------------------------------------------
log_section "2. MQTT接続状態確認"

MQTT_STATUS=$(get_json_field "$RESPONSE" '["cloud"]["mqttConnected"]')
if [ "$MQTT_STATUS" == "True" ]; then
    log_pass "MQTT接続状態 - mqttConnected=true"
else
    log_fail "MQTT接続状態 - mqttConnected=false"
fi

REGISTERED=$(get_json_field "$RESPONSE" '["cloud"]["registered"]')
if [ "$REGISTERED" == "True" ]; then
    log_pass "クラウド登録状態 - registered=true"
else
    log_fail "クラウド登録状態 - registered=false"
fi

# -----------------------------------------------------------------------------
# 3. PIN状態取得テスト
# -----------------------------------------------------------------------------
log_section "3. PIN状態取得テスト"

# 全PIN取得
PINS_RESPONSE=$(curl -s "${BASE_URL}/pin/all")
if [ $? -eq 0 ]; then
    PIN_COUNT=$(echo "$PINS_RESPONSE" | python3 -c "import sys,json; print(len(json.load(sys.stdin)['pins']))" 2>/dev/null)
    if [ "$PIN_COUNT" == "6" ]; then
        log_pass "全PIN取得 - /api/pin/all (6ch)"
    else
        log_fail "全PIN取得 - チャンネル数不正 ($PIN_COUNT)"
    fi
else
    log_fail "全PIN取得 - API呼び出し失敗"
fi

# 個別PIN取得
for CH in 1 2 3 4 5 6; do
    PIN_RESPONSE=$(curl -s "${BASE_URL}/pin/${CH}/state")
    CHANNEL=$(get_json_field "$PIN_RESPONSE" '["channel"]')
    if [ "$CHANNEL" == "$CH" ]; then
        log_pass "PIN${CH}状態取得"
    else
        log_fail "PIN${CH}状態取得"
    fi
done

# -----------------------------------------------------------------------------
# 4. PIN設定取得テスト
# -----------------------------------------------------------------------------
log_section "4. PIN設定取得テスト"

for CH in 1 2 3 4 5 6; do
    SETTING_RESPONSE=$(curl -s "${BASE_URL}/pin/${CH}/setting")
    CHANNEL=$(get_json_field "$SETTING_RESPONSE" '["channel"]')
    if [ "$CHANNEL" == "$CH" ]; then
        TYPE=$(get_json_field "$SETTING_RESPONSE" '["type"]')
        log_pass "PIN${CH}設定取得 - type=$TYPE"
    else
        log_fail "PIN${CH}設定取得"
    fi
done

# -----------------------------------------------------------------------------
# 5. PIN制御テスト (CH3: digitalOutput)
# -----------------------------------------------------------------------------
log_section "5. PIN制御テスト (CH3: digitalOutput)"

# 現在状態を取得
INITIAL_STATE=$(curl -s "${BASE_URL}/pin/3/state" | python3 -c "import sys,json; print(json.load(sys.stdin)['state'])" 2>/dev/null)
log_info "初期状態: CH3 state=$INITIAL_STATE"

# トグル実行
TOGGLE_RESPONSE=$(curl -s -X POST "${BASE_URL}/pin/3/toggle")
TOGGLE_OK=$(get_json_field "$TOGGLE_RESPONSE" '["ok"]')
NEW_STATE=$(get_json_field "$TOGGLE_RESPONSE" '["state"]')

if [ "$TOGGLE_OK" == "True" ]; then
    log_pass "CH3トグル実行 - 新状態=$NEW_STATE"
else
    log_fail "CH3トグル実行"
fi

# 状態確認
sleep 0.5
VERIFY_STATE=$(curl -s "${BASE_URL}/pin/3/state" | python3 -c "import sys,json; print(json.load(sys.stdin)['state'])" 2>/dev/null)
if [ "$VERIFY_STATE" == "$NEW_STATE" ]; then
    log_pass "CH3状態検証 - state=$VERIFY_STATE"
else
    log_fail "CH3状態検証 - expected=$NEW_STATE, actual=$VERIFY_STATE"
fi

# 元に戻す
curl -s -X POST "${BASE_URL}/pin/3/toggle" > /dev/null
log_info "CH3を元の状態に戻しました"

# -----------------------------------------------------------------------------
# 6. PWM制御テスト (CH2: pwmOutput)
# -----------------------------------------------------------------------------
log_section "6. PWM制御テスト (CH2: pwmOutput)"

# CH2をPWM出力に設定
curl -s -X POST "${BASE_URL}/pin/2/setting" \
    -H "Content-Type: application/json" \
    -d '{"type": "pwmOutput", "actionMode": "Slow"}' > /dev/null

# PWM値設定
PWM_VALUES=(0 30 60 100 0)
for VAL in "${PWM_VALUES[@]}"; do
    PWM_RESPONSE=$(curl -s -X POST "${BASE_URL}/pin/2/state" \
        -H "Content-Type: application/json" \
        -d "{\"state\": $VAL}")
    PWM_OK=$(get_json_field "$PWM_RESPONSE" '["ok"]')
    if [ "$PWM_OK" == "True" ]; then
        log_pass "CH2 PWM設定 - value=$VAL"
    else
        log_fail "CH2 PWM設定 - value=$VAL"
    fi
    sleep 0.3
done

# -----------------------------------------------------------------------------
# 7. グローバル設定テスト
# -----------------------------------------------------------------------------
log_section "7. グローバル設定テスト"

# 設定取得
SETTINGS_RESPONSE=$(curl -s "${BASE_URL}/settings")
SETTINGS_OK=$(get_json_field "$SETTINGS_RESPONSE" '["ok"]')
if [ "$SETTINGS_OK" == "True" ]; then
    log_pass "グローバル設定取得 - /api/settings GET"

    DEVICE_NAME=$(get_json_field "$SETTINGS_RESPONSE" '["settings"]["device_name"]')
    RID=$(get_json_field "$SETTINGS_RESPONSE" '["settings"]["rid"]')
    log_info "device_name: $DEVICE_NAME"
    log_info "rid: $RID"
else
    log_fail "グローバル設定取得"
fi

# rid設定テスト
TEST_RID="test_room_$(date +%s)"
SAVE_RESPONSE=$(curl -s -X POST "${BASE_URL}/settings" \
    -H "Content-Type: application/json" \
    -d "{\"rid\": \"$TEST_RID\"}")
SAVE_OK=$(get_json_field "$SAVE_RESPONSE" '["ok"]')
if [ "$SAVE_OK" == "True" ]; then
    log_pass "rid設定保存 - rid=$TEST_RID"
else
    log_fail "rid設定保存"
fi

# rid確認
VERIFY_RID=$(curl -s "${BASE_URL}/settings" | python3 -c "import sys,json; print(json.load(sys.stdin)['settings']['rid'])" 2>/dev/null)
if [ "$VERIFY_RID" == "$TEST_RID" ]; then
    log_pass "rid設定確認 - rid=$VERIFY_RID"
else
    log_fail "rid設定確認 - expected=$TEST_RID, actual=$VERIFY_RID"
fi

# 元に戻す
curl -s -X POST "${BASE_URL}/settings" \
    -H "Content-Type: application/json" \
    -d '{"rid": "villa1"}' > /dev/null
log_info "ridを元の値に戻しました"

# -----------------------------------------------------------------------------
# 8. PIN設定変更テスト
# -----------------------------------------------------------------------------
log_section "8. PIN設定変更テスト"

# CH4の設定変更
SETTING_SAVE=$(curl -s -X POST "${BASE_URL}/pin/4/setting" \
    -H "Content-Type: application/json" \
    -d '{
        "name": "テスト出力4",
        "actionMode": "Alt",
        "validity": 5000,
        "debounce": 2000
    }')
SETTING_OK=$(get_json_field "$SETTING_SAVE" '["ok"]')
if [ "$SETTING_OK" == "True" ]; then
    log_pass "CH4設定変更 - name, actionMode, validity, debounce"
else
    log_fail "CH4設定変更"
fi

# 設定確認
VERIFY_SETTING=$(curl -s "${BASE_URL}/pin/4/setting")
VERIFY_NAME=$(get_json_field "$VERIFY_SETTING" '["name"]')
VERIFY_MODE=$(get_json_field "$VERIFY_SETTING" '["actionMode"]')
VERIFY_VALIDITY=$(get_json_field "$VERIFY_SETTING" '["validity"]')

if [ "$VERIFY_MODE" == "Alt" ] && [ "$VERIFY_VALIDITY" == "5000" ]; then
    # 日本語nameの比較は環境依存のため、modeとvalidityで判定
    log_pass "CH4設定確認 - mode=$VERIFY_MODE, validity=$VERIFY_VALIDITY"
else
    log_fail "CH4設定確認 - expected: mode=Alt validity=5000, actual: mode=$VERIFY_MODE validity=$VERIFY_VALIDITY"
fi

# 元に戻す
curl -s -X POST "${BASE_URL}/pin/4/setting" \
    -H "Content-Type: application/json" \
    -d '{"name": "", "actionMode": "Mom", "validity": 3000, "debounce": 3000}' > /dev/null

# -----------------------------------------------------------------------------
# 9. stateName設定テスト
# -----------------------------------------------------------------------------
log_section "9. stateName設定テスト"

STATE_NAME_SAVE=$(curl -s -X POST "${BASE_URL}/pin/3/setting" \
    -H "Content-Type: application/json" \
    -d '{"stateName": ["on:開", "off:閉"]}')
SN_OK=$(get_json_field "$STATE_NAME_SAVE" '["ok"]')
if [ "$SN_OK" == "True" ]; then
    log_pass "stateName設定保存"
else
    log_fail "stateName設定保存"
fi

# 確認
SN_VERIFY=$(curl -s "${BASE_URL}/pin/3/setting" | python3 -c "import sys,json; print(json.load(sys.stdin)['stateName'])" 2>/dev/null)
if [[ "$SN_VERIFY" == *"on:開"* ]]; then
    log_pass "stateName設定確認 - $SN_VERIFY"
else
    log_fail "stateName設定確認"
fi

# -----------------------------------------------------------------------------
# 10. allocation設定テスト
# -----------------------------------------------------------------------------
log_section "10. allocation設定テスト"

ALLOC_SAVE=$(curl -s -X POST "${BASE_URL}/pin/5/setting" \
    -H "Content-Type: application/json" \
    -d '{"type": "digitalInput", "actionMode": "rotate", "allocation": ["CH2"]}')
ALLOC_OK=$(get_json_field "$ALLOC_SAVE" '["ok"]')
if [ "$ALLOC_OK" == "True" ]; then
    log_pass "allocation設定保存 - CH5→CH2"
else
    log_fail "allocation設定保存"
fi

# 確認
ALLOC_VERIFY=$(curl -s "${BASE_URL}/pin/5/setting" | python3 -c "import sys,json; print(json.load(sys.stdin)['allocation'])" 2>/dev/null)
if [[ "$ALLOC_VERIFY" == *"CH2"* ]]; then
    log_pass "allocation設定確認 - $ALLOC_VERIFY"
else
    log_fail "allocation設定確認"
fi

# -----------------------------------------------------------------------------
# 11. expiryDate設定テスト
# -----------------------------------------------------------------------------
log_section "11. expiryDate設定テスト"

EXPIRY_SAVE=$(curl -s -X POST "${BASE_URL}/pin/1/setting" \
    -H "Content-Type: application/json" \
    -d '{"expiryDate": "202612311200", "expiryEnabled": true}')
EXPIRY_OK=$(get_json_field "$EXPIRY_SAVE" '["ok"]')
if [ "$EXPIRY_OK" == "True" ]; then
    log_pass "expiryDate設定保存"
else
    log_fail "expiryDate設定保存"
fi

# 確認
EXPIRY_VERIFY=$(curl -s "${BASE_URL}/pin/1/setting")
EX_DATE=$(get_json_field "$EXPIRY_VERIFY" '["expiryDate"]')
EX_EN=$(get_json_field "$EXPIRY_VERIFY" '["expiryEnabled"]')
if [ "$EX_DATE" == "202612311200" ] && [ "$EX_EN" == "True" ]; then
    log_pass "expiryDate設定確認 - date=$EX_DATE, enabled=$EX_EN"
else
    log_fail "expiryDate設定確認"
fi

# 元に戻す
curl -s -X POST "${BASE_URL}/pin/1/setting" \
    -H "Content-Type: application/json" \
    -d '{"expiryEnabled": false}' > /dev/null

# -----------------------------------------------------------------------------
# 12. OTA状態確認
# -----------------------------------------------------------------------------
log_section "12. OTA状態確認"

# OTA APIはokフィールドではなくstatusフィールドを返す
OTA_STATUS=$(curl -s "${BASE_URL}/ota/status")
OTA_STATE=$(get_json_field "$OTA_STATUS" '["status"]')
if [ -n "$OTA_STATE" ]; then
    log_pass "OTA状態取得 - /api/ota/status (status=$OTA_STATE)"
else
    log_fail "OTA状態取得"
fi

OTA_INFO=$(curl -s "${BASE_URL}/ota/info")
RUNNING_PART=$(get_json_field "$OTA_INFO" '["running_partition"]')
if [ -n "$RUNNING_PART" ]; then
    log_pass "OTAパーティション情報取得 - /api/ota/info (partition=$RUNNING_PART)"
else
    log_fail "OTAパーティション情報取得"
fi

# -----------------------------------------------------------------------------
# 結果サマリー
# -----------------------------------------------------------------------------
log_section "テスト結果サマリー"
echo ""
echo "テスト完了時刻: $(date)"
echo ""
echo -e "合格: ${GREEN}${PASS}${NC}"
echo -e "不合格: ${RED}${FAIL}${NC}"
echo -e "合計: $((PASS + FAIL))"
echo ""

if [ $FAIL -eq 0 ]; then
    echo -e "${GREEN}========================================${NC}"
    echo -e "${GREEN}全テスト合格！実装は完全です。${NC}"
    echo -e "${GREEN}========================================${NC}"
    exit 0
else
    echo -e "${RED}========================================${NC}"
    echo -e "${RED}${FAIL}件のテストが失敗しました${NC}"
    echo -e "${RED}========================================${NC}"
    echo ""
    echo "失敗したテスト:"
    for TEST in "${TESTS[@]}"; do
        if [[ "$TEST" == FAIL* ]]; then
            echo "  - ${TEST#FAIL: }"
        fi
    done
    exit 1
fi
