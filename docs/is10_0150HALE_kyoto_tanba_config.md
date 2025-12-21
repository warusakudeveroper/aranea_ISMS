# IS10 - HALE京都丹波口 ルーター設定

## 概要

- **施設ID (FID)**: 0150
- **ルーター台数**: 16台
- **ルーター機種**: ASUS RT-AC-59U (ASUSWRT)
- **SSHポート**: 65305
- **ユーザー名**: HALE_admin
- **パスワード**: Hale_tam_2020063

## ルーター設定一覧

| RID | モデル | WAN IP | LAN IP | SSHポート |
|-----|--------|--------|--------|-----------|
| 101 | RT-AC-59U | 192.168.125.171 | 192.168.2.1 | 65305 |
| 104 | RT-AC-59U | 192.168.125.172 | 192.168.9.1 | 65305 |
| 105 | RT-AC-59U | 192.168.125.173 | 192.168.10.1 | 65305 |
| 106 | RT-AC-59U | 192.168.125.174 | 192.168.11.1 | 65305 |
| 201 | RT-AC-59U | 192.168.125.175 | 192.168.3.1 | 65305 |
| 202 | RT-AC-59U | 192.168.125.176 | 192.168.4.1 | 65305 |
| 203 | RT-AC-59U | 192.168.125.177 | 192.168.5.1 | 65305 |
| 204 | RT-AC-59U | 192.168.125.178 | 192.168.12.1 | 65305 |
| 205 | RT-AC-59U | 192.168.125.179 | 192.168.13.1 | 65305 |
| 206 | RT-AC-59U | 192.168.125.180 | 192.168.14.1 | 65305 |
| 301 | RT-AC-59U | 192.168.125.181 | 192.168.6.1 | 65305 |
| 302 | RT-AC-59U | 192.168.125.182 | 192.168.7.1 | 65305 |
| 303 | RT-AC-59U | 192.168.125.183 | 192.168.8.1 | 65305 |
| 304 | RT-AC-59U | 192.168.125.184 | 192.168.15.1 | 65305 |
| 305 | RT-AC-59U | 192.168.125.185 | 192.168.16.1 | 65305 |
| 306 | RT-AC-59U | 192.168.125.186 | 192.168.17.1 | 65305 |

## SSH設定

- Telnet: いいえ
- SSHを有効にする: はい
- SSHポート: 65305
- パスワードのログインを有効にする: はい
- 認証キー:
```
ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAACAQC70kyeJL2Bv0aeUunhy6Y9sRO66YX/SyJ2TLv30nRl5q/wx72S3qb+pvbQIWTIvBz/O3hQR8NStnes9GrDL3QwDQE52fqBQFajpFisT6WcyJWajamKo/XY6+dv6NeSn+9lIjcC2LsEG/OBvpuU0HrdK5PqXXIIfuGrsEsedT/j1CFwhw8hGRKfhxFOQi4vsotqvujaH5bUCIM3F7DLjr1xmFyg3Kt9y3b/VJAhgdivXL0Xsms+agBwlIFTpBTCDaFr2Yc6AgLoxCe8WICnIuBKhs1kAEGqaESY9S7Sk7wacvI4UY1WxTbbfJ+JXH0+5mrD4lpGCQxGdkKHfhFijFmp0AVjec9ucCsnC5LIwfaZYG8lQV/EjLI4eyd3qQ6letEjFq19yg24xhBvOxIqdVoMpb/TqckS/jx3BNIBwt4Ktysw2Vrg/s8CAr+Dn5WGOqP8KrMn7RdMus5iQZ1/nv8JB/Ie3bGz67x7SiCpZIKJ5+1XX30LKwuf6L4N2N13Ix8CUfQL9ULGJSdRJd7fbLTGlNXvdY6/15drYX5bUBZ69kdEMSsVk02A2p9bNfSor0iPr0q+DPL2UGCzp3wFmoLTNy7Itpsfz9v2pHfX1pGvB3mGzZNElKLcg5uxlgHpsnl84MyUZ66nNMrmgZtrRBBOfmoHcO3gLK3Pw1MuX17vqQ== hideakikurata@NucBoxUbuntu
```

## 注意事項

- ASUSWRTは反応が非常に遅い - タイムアウトを60秒以上に設定
- ルーターはWAN側からPING/ポートスキャンに応答しない（正常動作）
- SSH接続の成否でのみ疎通確認を行う
