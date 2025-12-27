# is02a: DEVICE_TYPE名不一致による登録失敗

**日付**: 2025-12-27
**カテゴリ**: incident
**デバイス**: is02a (BLE Gateway)

## 現象

新規ESP32にis02aファームウェアを書き込むと、クラウド登録が400エラーで失敗。

```
[ARANEA] Error 400: No default permission configured for araneaDevice_aranea_ar-is02a
```

## 原因

コード内のDEVICE_TYPEとクラウド側の登録タイプ名が不一致。

| 場所 | タイプ名 |
|------|----------|
| コード (is02a.ino) | `aranea_ar-is02a` |
| クラウド登録済み | `ISMS_ar-is02` |

## 修正

`is02a.ino:30`

```cpp
// 修正前
static const char* DEVICE_TYPE = "aranea_ar-is02a";

// 修正後
static const char* DEVICE_TYPE = "ISMS_ar-is02";
```

## なぜ発覚が遅れたか

1. **既存デバイスはNVSにCICを保持** - 再登録不要で動作継続
2. **新規デバイスで初めて登録試行** - その時点でエラー発生
3. **エラーメッセージが「permission not configured」** - クラウド側の問題と誤認しやすい

## 教訓

1. **既存デバイスがNVSにCICを持っていると再登録されない** - 問題が発覚しにくい
2. **新規デバイスで初めて登録エラーが発生** - フラッシュ消去後に問題が顕在化
3. **クラウド側のタイプ名を必ず確認** - mobes管理画面でaraneaDevice一覧を見る
4. **「クラウド側の問題」と決めつけない** - 同一テナントで他デバイスが登録できているなら、コード側を疑う

## 確認方法

mobes管理画面 → araneaDevice → 既存デバイスのタイプ名を確認

## 関連ファイル

- `code/ESP32/is02a/is02a.ino` (DEVICE_TYPE定義)
