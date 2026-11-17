# 10 — Alert Engine and Notification Router

Melengkapi `00-design.md` §3/§12. Alert adalah projection + keputusan, bukan
sumber kebenaran terpisah.

## 1. Aturan dan ambang default

| Rule id | Kondisi | Level | Cooldown | Subjek |
|---|---|---|---|---|
| `agent.waiting` | status `MenungguRizal` > 15 menit | Warning | 30 m | agent |
| `agent.disappeared` | `AgentDisappeared` saat sebelumnya aktif | Critical | 10 m | agent |
| `agent.error` | status `Error` > 2 menit | Critical | 15 m | agent |
| `quota.warn` | `used_pct >= 90` | Warning | 6 j | provider/model |
| `quota.full` | `used_pct >= 100` | Critical | 6 j | provider/model |
| `balance.low` | estimasi saldo < $1 | Critical | 12 j | provider |
| `disk.high` | disk ≥ 90% | Warning | 1 j | system |
| `disk.critical` | disk ≥ 95% | Critical | 30 m | system |
| `connector.auth` | `ConnectorAuthFailed` | Critical | 1 j | connector |
| `connector.down` | health Down > 10 menit | Warning | 30 m | connector |
| `event.lag` | projection lag > 60 detik | Warning | 15 m | system |
| `cost.anomaly` | biaya hari ini > 2× median 7 hari | Warning | 12 j | agent/provider |
| `approval.expiring` | approval < 5 menit dari kedaluwarsa | Info | 5 m | approval |

Ambang dapat diubah di `config.toml`; nilai default di atas adalah kontrak awal.
Setiap rule punya `enabled`, `level`, `cooldown_ms`, `channels`, `quiet_hours`.

## 2. Fingerprint dan dedup

```
fingerprint = sha256(rule_id || "|" || subject_key || "|" || dedup_key)
```

`dedup_key` default = `subject_key`; rule tertentu menambahkan dimensi
(`quota.warn` → provider+model). Satu fingerprint = satu baris `alerts`.
Kejadian berulang selama state belum resolved menaikkan `occurrences` dan
memperbarui `updated_ms`, bukan membuat alert baru. Alert yang resolved lalu
muncul lagi membuat baris baru dengan id baru dan fingerprint sama (unique
index dilepas saat resolved, atau fingerprint diberi suffix `#<n>` — pilih
yang pertama: `fingerprint` unik hanya untuk state non-resolved, ditegakkan di
kode karena SQLite partial index).

## 3. State machine

```
open ──ack──▶ acknowledged ──resolve──▶ resolved
  │                │
  ├──snooze──▶ snoozed ──waktu habis──▶ open
  └──recover──▶ resolved
```

- `open`: butuh perhatian; muncul di attention rail.
- `acknowledged`: manusia sudah melihat; tetap dihitung sampai resolve.
- `snoozed`: tidak muncul sampai `snooze_until_ms`; saat habis kembali `open`
  dan memicu notifikasi sekali.
- `resolved`: hilang dari hitungan; alasan dicatat (`recovered`, `acked`,
  `manual`, `expired`).
- Transisi selalu lewat event `AlertAcknowledged/Snoozed/Resolved`, bukan SQL.

## 4. Eskalasi

| Langkah | Waktu | Aksi |
|---|---|---|
| 0 | saat raise | event + UI + kanal yang diaktifkan |
| 1 | +10 menit tanpa ack (Critical) | kirim ulang ke kanal utama, tag `reminder` |
| 2 | +30 menit tanpa ack (Critical) | kanal sekunder (Telegram → email/Slack) |
| 3 | +2 jam tanpa ack | tandai `stale`, tampilkan di banner |

Warning hanya langkah 0–1. Info tidak dieskalasi. Quiet hours menunda langkah
non-Critical sampai jam tenang berakhir, tetapi alert tetap muncul di UI.
Setiap pengiriman menghasilkan `notification.sent` dengan `ok`/`error`; gagal
kirim dicoba ulang dengan backoff 1 m, 5 m, 15 m (maks 3), tidak memblokir
alert lain.

## 5. Notification router

```rust
#[async_trait]
pub trait Notifier: Send + Sync {
    fn id(&self) -> &'static str;                 // "telegram", "slack", "email"
    async fn send(&self, msg: &Notification) -> Result<Delivery, NotifyError>;
}

pub struct Notification {
    pub alert_id: AlertId,
    pub level: AlertLevel,
    pub title: Sentence,
    pub detail: Sentence,
    pub action_url: Option<String>,
    pub quick_actions: Vec<QuickAction>,           // ack, snooze, approve, deny
    pub reminder: bool,
}
```

Aturan:

- Kanal hanya aktif jika owner mengaktifkannya di config; tidak ada approval
  per pesan (K17).
- Pengiriman tidak pernah memblokir transaksi event; router berjalan setelah
  commit, dengan antrean persisten (`outbox`).
- Isi pesan dirender per locale penerima; tidak ada secret.
- Telegram inline button membawa nonce; ack/approve divalidasi + expiry.
- Rate limit per kanal (Telegram 30 pesan/detik global; Slac 1 pesan/detik per
  channel) dan per alert (maks 4 pesan/hari).
- Jika semua kanal gagal, alert tetap `open` dan `delivery_failed` terlihat di
  detail alert.

## 6. Anti-storm

- Satu fingerprint = satu alert; occurrences dihitung.
- Grouping: maksimal 3 alert per agent di attention rail; sisanya "+N".
- Rule `firing` tidak pernah membuat alert saat startup pertama (hindari badai
  saat boot); butuh minimal 2 observasi.
- Recovery otomatis: kondisi hilang selama 2 siklus berturut-turut → resolved.
- Setelah deploy/restart, alert yang sudah `acknowledged` tidak dikirim ulang
  kecuali levelnya naik.

## 7. API dan UI

- `GET /api/v1/alerts?state=&level=` → daftar.
- `GET /api/v1/alerts/{id}` → detail + riwayat keputusan + delivery.
- `POST /api/v1/alerts/{id}/{ack,snooze,resolve}` → event + patch.
- Attention rail: Critical dulu, lalu Warning; Info di halaman Alerts.
- Setiap baris: level dot, satu kalimat, subjek, umur, satu aksi utama
  (`Ack`) dan menu (`Snooze 1 jam`, `Resolve`).
- Tidak ada emoji, tidak ada warning icon di depan judul (tactile rule).

## 8. Tes S11

| Tes | Isi |
|---|---|
| `same_fingerprint_does_not_duplicate` | dua event sama → satu alert, occurrences=2 |
| `resolved_then_recurring_creates_new_alert` | resolve lalu raise lagi → id baru |
| `snooze_expires_and_reopens` | time travel → state open, satu reminder |
| `quiet_hours_delay_non_critical` | Warning jam tenang → tidak kirim, tetap open |
| `critical_escalates_after_10m` | time travel → langkah 1 terkirim |
| `delivery_failure_is_retried_then_visible` | fake notifier gagal 3× → status |
| `startup_does_not_storm` | 100 event awal → 0 alert |
| `recovery_resolves_after_two_clean_cycles` | dua siklus tanpa kondisi → resolved |
| `storm_grouping_caps_attention_rail` | 10 alert agent → 3 + N |

## 9. Batas

- Engine tidak mengirim HTTP langsung; hanya lewat `Notifier` yang didaftarkan
  `hq-connectors`.
- Engine tidak menulis tabel; semua transisi lewat event + projector.
- Ambang bukan feature flag tersembunyi; semuanya di config yang terlihat.
