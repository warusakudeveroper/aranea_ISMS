# サブネット削除時クリーンアップ 設計ドキュメント

## 1. 概要

### 1.1 目的
管理対象からサブネットを削除した際に、そのサブネットに属するスキャン結果を自動的にクリーンアップし、データの整合性を維持する。

### 1.2 対象ファイル
- バックエンド: `src/web_api/routes.rs` (サブネット削除API)
- データベース: `scan_devices`テーブル

### 1.3 現状の問題点（Camscan_designers_review.md #11より）
- サブネット削除後もスキャン結果に「登録可能カメラ」として残る
- 削除済みサブネットのカメラを誤って登録できてしまう

---

## 2. 設計

### 2.1 削除対象

| 項目 | 削除する | 削除しない |
|------|---------|-----------|
| scan_devicesテーブルのレコード | ✓ | - |
| 登録済みカメラ（camerasテーブル） | - | ✓（ユーザー操作必要）|
| クレデンシャル設定 | - | ✓ |

### 2.2 削除フロー

```
ユーザーがサブネット削除
    │
    ▼
サブネット設定削除
    │
    ▼
scan_devicesから該当サブネットのレコード削除
    │
    ▼
（登録済みカメラは削除しない）
    │
    ▼
ユーザーに通知:
"サブネット 192.168.125.0/24 を削除しました。
 登録済みカメラ8台はそのまま残ります。
 カメラを削除する場合は個別に操作してください。"
```

### 2.3 API実装

```rust
async fn delete_subnet(
    State(state): State<AppState>,
    Path(subnet_cidr): Path<String>,
) -> impl IntoResponse {
    // 1. サブネット設定を削除
    state.config_store.service().delete_subnet(&subnet_cidr).await?;

    // 2. scan_devicesからスキャン結果を削除
    sqlx::query("DELETE FROM scan_devices WHERE subnet = ?")
        .bind(&subnet_cidr)
        .execute(&state.pool)
        .await?;

    // 3. 登録済みカメラ数を取得（削除はしない）
    let registered_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM cameras WHERE INET_ATON(ip_address) & 0xFFFFFF00 = INET_ATON(?)"
    )
    .bind(subnet_cidr.split('/').next().unwrap())
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(ApiResponse::success(DeleteSubnetResult {
        subnet: subnet_cidr,
        scan_results_deleted: true,
        registered_cameras_remaining: registered_count,
        message: format!(
            "サブネットを削除しました。登録済みカメラ{}台はそのまま残ります。",
            registered_count
        ),
    })))
}
```

---

## 3. テスト計画

### 3.1 単体テスト
1. scan_devices削除クエリテスト
2. 登録済みカメラ残存確認テスト

### 3.2 UIテスト
1. サブネット削除後のスキャン結果表示確認
2. 通知メッセージ表示確認

---

## 4. MECE確認

- [x] 削除対象/非対象が明確に定義
- [x] ユーザーへの通知が設計されている
- [x] データ整合性が維持される

---

**作成日**: 2026-01-07
**作成者**: Claude Code
**ステータス**: 設計完了・レビュー待ち
