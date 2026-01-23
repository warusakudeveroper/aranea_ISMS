IS06SはaraneaDeviceシリーズのリレー・スイッチコントローラです。
主用途としては自動販売機、電磁錠、照明、調光照明をコントロールし
ESP32DevkitC(Wroom32)をコア基盤として使用します。

# ハードウェアコンポーネント
ハードウェアコンポーネントはuserObjectDetailの`  
hardware components`セクションに記載します
```
"  
hardware components":{
	"SBC":"ESP32Wroom32(ESP32E,DevkitC)",
	"I2COLED":"0.96inch_128*64_3.3V",
	"baseboard":"aranea_04"
	},
```

# PIN基本仕様
araneaDevice共通仕様により共通の管理画面、設定を持ちます。
`is06pinManager`クラスを新設しこれらの機能を実装します。
巨大クラスになりすぎてしまう懸念があれば適宜に分割します。


| 機能名称      | Type   | PIN     | 仕様                     |
| --------- | ------ | ------- | ---------------------- |
| CH1       | D/P    | GPIO 18 | 3.3V OUTPUT/PWM OUTPUT |
| CH2       | D/P    | GPIO 05 | 3.3V OUTPUT/PWM OUTPUT |
| CH3       | D/P    | GPIO 15 | 3.3V OUTPUT/PWM OUTPUT |
| CH4       | D/P    | GPIO 27 | 3.3V OUTPUT/PWM OUTPUT |
| CH5       | I/O    | GPIO 14 | 3.3V INPUT/3.3V OUTPUT |
| CH6       | I/O    | GPIO 12 | 3.3V INPUT/3.3V OUTPUT |
| Reconnect | System | GPIO 25 | 3.3VINPUT/             |
| Reset     | System | GPIO 26 | 3.3VINPUT/             |
| I2Cdata   | SDA    | GPIO 21 |                        |
| I2Cclock  | SCL    | GPIO 22 |                        |
|           |        |         |                        |

## D/P (Digitaloutput or PWM output)
適用PIN :GPIO,05,15,27(CH1〜4)
このタイプのPINではDigitaloutput と PWM outputをwebUIおよびmobes側のuserObjectDetailとの同期によって切り替えが可能です。
### Digitaloutput
デジタルアウトプットを行います。
この形式の場合には主に電気錠の施錠解錠、自動販売機のA接点トリガー、Onoffの照明スイッチ、そのほかのトリガーを動作させます。
```

"PINSettings":{　//PINSettingセクション
	"CH2":{
		"type":"digitalOutput",　//ピン動作タイプを示す
		"name":"照明スイッチ",　//ピンの表示名任意入力名、キーとしては仕様しない
		"stateName":["on:解錠","off:施錠"],　//on,offが示す状態の表示名
		"actionMode":"Mom",　//Mom(モーメンタリ)かAlt(オルタネート)か
		"defaultValidity":"3000",　//デフォルトのMomの動作時間(ms)Altでは無効
		"defaultDebounce":"3000",　//デフォルトのデバウンス(ms)Altでも必要に応じて
		"defaultexpiry":"1", //デフォルトの有効期限(day)
		}
	},
"PINState":{　//PINstateセクション
	"CH2":{
		"state":"on",　//状態を示す
		"Validity":"0",　//0の場合は無期限としてオーバーライド
		"Debounce":"0",　//同じ
		"expiryDate":"202601231100"　//有効期限
		}
	}
```
動作上のポイント
Settingで定められた"defaultValidity","defaultexpiry"に対して"State側で"オーバーライドを行います。"expiryDate"は主に宿泊客の滞在期間中に特定のドアを有効にするなどの目的で仕様されます。
動作としては`Validity`>`expiry`で使用されますが`expiry`の期限を超えることはできません。`expiryDate`でオーバーライドされた場合はexpiryDateが使用されます

### PWMoutput
PWMアウトプットを行います。
この形式の場合には主に照明器具の調光に使用されます。
```

"PINSettings":{　//PINSettingセクション
	"CH2":{
		"type":"pwmOutput",　//ピン動作タイプを示す
		"name":"調光LED",　//ピンの表示名任意入力名、キーとしては仕様しない
		"stateName":["0:0%","30:30%","60:60%","100:100%"],　//プリセットとしての状態の表示名
		"actionMode":"Slow",　//Slow(なだらか)かRapid(急激)か
		"RateOfChange":"4000",　//Slowの時に0-100の変化に使用する時間Rapidでは0とする(ms)
		"defaultexpiry":"1", //デフォルトの有効期限(day)
		}
	},
"PINState":{　//PINstateセクション
	"CH2":{
		"state":"40",　//状態を示す、%`stateName`に縛られる必要はない、0-100%
		"expiryDate":"202601231100"　//有効期限
		}
	}
```
動作上のポイント
Settingで定められた"stateName"でのプリセットはI/Oでのスイッチと連動して使用されます。後述しますがINPUTで押すたびに0%→30%→60%→100%→0％の動作(`rotate`)に使用されます。

## I/O (Digitaloutput or DigitalInput)
適用PIN :GPIO14,12(CH5〜6)
このタイプのPINではDigitaloutput と ### DigitalinputをwebUIおよびmobes側のuserObjectDetailとの同期によって切り替えが可能です。
### Digitaloutput
デジタルアウトプットを行います。これはD/Pタイプと同様の動作です
I/OタイプのPINではPWMに対応しません

### Digitalinput
デジタルインプットを行います。これはD/PタイプのPINを物理スイッチで操作するための機能です。
```

"PINSettings":{　//PINSettingセクション
	"CH6":{
		"type":"digitalInput",　//ピン動作タイプを示す
		"allocation":["CH1","CH2"], //配列記載、複数指定可能
		"name":"physicalUnlock",　//ピンの表示名任意入力名、キーとしては仕様しない
		"triggerType":"Digital",　//Digital or PWM PIN側から継承する
		"actionMode":"Mom",　//Mom(モーメンタリ)かAlt(オルタネート)かrotate(ローテート)、PIN側から継承、PWMの場合は強制的にrotateのみ
		"defaultValidity":"3000",　//デフォルトのMomの動作時間(ms)Altでは無効、PINから継承
		"defaultDebounce":"3000",　//デフォルトのデバウンス(ms)Altでも必要に応じてPIN側から継承
		"defaultexpiry":"1", //デフォルトの有効期限(day)PIN側から継承
		}
	},
"PINState":{　//PINstateセクション、基本的にはMQTTでこのinputを操作する可能性はないのでステート取得のみ
	"CH6":{
		"state":"on",　//状態を示す
		"Validity":"0",　//0の場合は無期限としてオーバーライド
		"Debounce":"0",　//同じ
		"expiryDate":"202601231100"　//有効期限
		}
	}
```
`allocation`では同一仕様のPINのみを複数指定可能なバリデーションが必要です、DigitaloutputとPWMoutputの混在は許可されません。

### そのほかのPIN操作
#### GPIO25
- 5000msの長押しでwifiのリコネクト、NTP同期を行います。
  入力開始時からI2C OLEDに"Reconnect"とカウントダウンを表示します
#### GPIO26
- 15000msの長押しでNVS,SPIFFSをリセットし全設定をクリア、初期状態にリセットします。入力開始時からI2C OLEDに"Reset"とカウントダウンを表示します
### GlobalSettings
userObjectDetailにGlobalSettingsセクションを設けてこのIS06sと同期を行います。
原則として設定項目はすべてここに記述されmobes2.0のaraneadeviceGateを通じて設定同期を行います。
```
"globalSettings":{
	"wifiSetting"{ //最大6個のSSIDを登録、順次接続試行、SSIDが設定されていないor全SSIDに接続不能の時はAPモードで動作する
		"SSID1":"PASS1",
		"SSID1":"PASS2",
		"SSID1":"PASS3",
		"SSID1":"PASS4",
		"SSID1":"PASS5",
		"SSID1":"PASS6"
		},
	"APmode"{
		"APSSID":"APPASS", //APモードで使用するSSIDとPASS APPASSが""の時はNopassで接続許可
		"APsubnet":"192.168.250.0/24",
		"APaddr":"192.168.250.1",
		"exclusiveConnection":"true"//単一の接続、原則としてtrue
		},
	"network":{
		"DHCP":"true", //DHCPがfalseの時はStaticとする
		"Static"{ //DHCPがtrueの時は無視される
			"gateway":"192.168.XX.XX",
			"subnet":"255.255.255.0",
			"staticIP":"192.168.XX.XX"
			}
		},
	"WEBUI":{
		"localUID":["",""], //webUIにアクセスする際のローカルアカウント
		"lacisOath":"false" //lacisOathアカウントによるアクセス許可、同一TIDでpermission41以上であれば許可するがlacisOath側のAPI整備状況の確認が必要
		},
	"PINglobal":{
		"Digital":}
		
		//その他リブートスケジュール、各種設定をuserObjectDetailと相互同期する
```
### Global State
```
ここではIPアドレス、uptime、rssi、ヒープメモリ、リブートログなどの情報を同期します。
MQTTベースでのステート変更に対応します
```

# WEBUI
共通コンポーネントの他に
```
"PINControl":PIN操作を行います
"PINSetting":PIN設定を行います
```
のタブを新設しPIN関連の機能を操作します。

# HTTP API
MQTTによる操作の他にHTTPアクセスによるPIN設定、PIN操作を行います。

# 共通コンポーネントに依存する機能
- araneaDeviceGateへの登録まわり
- MQTT系のioT操作機能
- wifi接続
- araneaSettings,Settingmanager系
- ntp同期
- I2COLED系の表示
- リブートスケジューラー系
- lacisIDGenerator系
- WEB UIの共通コンポーネント
- HTTP OTAアップデート
- 基本的な機能はIS04aやis05aなどのような共通仕様

# テスト環境設定情報
```
"SSID情報":{
	"SSID1":"sorapia_facility_wifi",
	"PASS1":"JYuDzjhi",
	"subnet":"192.168.77.0/24",
	"gateway":"192.168.77.1",
	"環境情報":"Omadaクラウドコントローラ管理下"
	},
"設置施設情報":{
	"faciltyName":"Sorapia Villa Mt.FUJI front",
	"fid":"0100",
	"rid":["villa1","villa2","villa3","villa4"]
	},
"テナント情報":{
	"tenantName":"mijeos.inc",
	"tid":"T2025120621041161827",
	"tenantPrimaryUser":"soejim@mijeos.com",
	"tenantPrimaryLacisID":"18217487937895888001",
	"tenantPrimaryCIC":"204965",
	"tenantPrimaryPASS":"UDgY7KH&fNrg" //現在は使用しない仕様になっている
	}
```

# スキーマ登録系について
- 現在userObjectDetailスキーマをSSoTとしたaraneaDeviceGateとのMQTT連携が中途半端なので今回のIS06Sの実装に伴い詳細に整備します
- araneaSDK,ArduinoMCPを使用してください。
- The_golden_rules.mdに基づいて進めてください。