use crate::{Store, StoreError};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceRecord {
    pub id: String,
    pub name: String,
    pub token_hash: String,
    pub scopes: Vec<String>,
    pub created_ms: i64,
    pub expires_ms: Option<i64>,
    pub last_seen_ms: Option<i64>,
    pub revoked_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingRecord {
    pub name: String,
    pub scopes: Vec<String>,
    pub created_ms: i64,
    pub expires_ms: i64,
}

pub enum AuthWrite {
    InsertDevice(DeviceRecord),
    RevokeDevice {
        id: String,
        at_ms: i64,
    },
    CreatePairing {
        code_hash: String,
        record: PairingRecord,
    },
    ConsumePairing {
        code_hash: String,
        now_ms: i64,
    },
}

fn row_to_device(row: &rusqlite::Row<'_>) -> Result<DeviceRecord, rusqlite::Error> {
    let scopes: String = row.get(3)?;
    Ok(DeviceRecord {
        id: row.get(0)?,
        name: row.get(1)?,
        token_hash: row.get(2)?,
        scopes: serde_json::from_str(&scopes).unwrap_or_default(),
        created_ms: row.get(4)?,
        expires_ms: row.get(5)?,
        last_seen_ms: row.get(6)?,
        revoked_ms: row.get(7)?,
    })
}

fn pairing_key(code_hash: &str) -> String {
    format!("pairing:{code_hash}")
}

pub fn apply_write(
    conn: &mut Connection,
    op: AuthWrite,
) -> Result<Option<PairingRecord>, StoreError> {
    match op {
        AuthWrite::InsertDevice(device) => {
            conn.execute(
                "INSERT INTO devices (id,name,token_hash,scopes,created_ms,expires_ms,last_seen_ms,revoked_ms) \
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8) \
                 ON CONFLICT(id) DO UPDATE SET name=excluded.name, token_hash=excluded.token_hash, \
                 scopes=excluded.scopes, expires_ms=excluded.expires_ms, revoked_ms=NULL",
                params![
                    device.id,
                    device.name,
                    device.token_hash,
                    serde_json::to_string(&device.scopes).unwrap_or_default(),
                    device.created_ms,
                    device.expires_ms,
                    device.last_seen_ms,
                    device.revoked_ms
                ],
            )?;
            Ok(None)
        }
        AuthWrite::RevokeDevice { id, at_ms } => {
            conn.execute(
                "UPDATE devices SET revoked_ms=?1 WHERE id=?2 AND revoked_ms IS NULL",
                params![at_ms, id],
            )?;
            Ok(None)
        }
        AuthWrite::CreatePairing { code_hash, record } => {
            conn.execute(
                "INSERT INTO settings (key,value) VALUES (?1,?2) \
                 ON CONFLICT(key) DO UPDATE SET value=excluded.value",
                params![
                    pairing_key(&code_hash),
                    serde_json::to_string(&record).unwrap_or_default()
                ],
            )?;
            Ok(None)
        }
        AuthWrite::ConsumePairing { code_hash, now_ms } => {
            let tx = conn.transaction()?;
            let key = pairing_key(&code_hash);
            let value: Option<String> = tx
                .query_row(
                    "SELECT value FROM settings WHERE key=?1",
                    params![key],
                    |r| r.get(0),
                )
                .optional()?;
            tx.execute("DELETE FROM settings WHERE key=?1", params![key])?;
            tx.commit()?;
            let Some(value) = value else {
                return Ok(None);
            };
            let record: PairingRecord = serde_json::from_str(&value)
                .map_err(|e| StoreError::Validation(format!("corrupt pairing record: {e}")))?;
            if record.expires_ms <= now_ms {
                return Ok(None);
            }
            Ok(Some(record))
        }
    }
}

impl Store {
    pub async fn insert_device(&self, device: DeviceRecord) -> Result<(), StoreError> {
        let (reply_tx, reply_rx) = oneshot_channel();
        self.inner
            .writer_tx
            .send(crate::WriteCmd::AuthWrite {
                op: AuthWrite::InsertDevice(device),
                reply: reply_tx,
            })
            .map_err(|_| StoreError::ChannelClosed)?;
        reply_rx
            .await
            .map_err(|_| StoreError::ChannelClosed)?
            .map(|_| ())
    }

    pub async fn revoke_device(&self, id: &str, at_ms: i64) -> Result<(), StoreError> {
        let (reply_tx, reply_rx) = oneshot_channel();
        self.inner
            .writer_tx
            .send(crate::WriteCmd::AuthWrite {
                op: AuthWrite::RevokeDevice {
                    id: id.to_string(),
                    at_ms,
                },
                reply: reply_tx,
            })
            .map_err(|_| StoreError::ChannelClosed)?;
        reply_rx
            .await
            .map_err(|_| StoreError::ChannelClosed)?
            .map(|_| ())
    }

    pub async fn create_pairing(
        &self,
        code_hash: &str,
        record: PairingRecord,
    ) -> Result<(), StoreError> {
        let (reply_tx, reply_rx) = oneshot_channel();
        self.inner
            .writer_tx
            .send(crate::WriteCmd::AuthWrite {
                op: AuthWrite::CreatePairing {
                    code_hash: code_hash.to_string(),
                    record,
                },
                reply: reply_tx,
            })
            .map_err(|_| StoreError::ChannelClosed)?;
        reply_rx
            .await
            .map_err(|_| StoreError::ChannelClosed)?
            .map(|_| ())
    }

    pub async fn consume_pairing(
        &self,
        code_hash: &str,
        now_ms: i64,
    ) -> Result<Option<PairingRecord>, StoreError> {
        let (reply_tx, reply_rx) = oneshot_channel();
        self.inner
            .writer_tx
            .send(crate::WriteCmd::AuthWrite {
                op: AuthWrite::ConsumePairing {
                    code_hash: code_hash.to_string(),
                    now_ms,
                },
                reply: reply_tx,
            })
            .map_err(|_| StoreError::ChannelClosed)?;
        reply_rx.await.map_err(|_| StoreError::ChannelClosed)?
    }

    pub async fn device_by_token_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<DeviceRecord>, StoreError> {
        let pool = self.inner.read_pool.clone();
        let hash = token_hash.to_string();
        tokio::task::spawn_blocking(move || -> Result<Option<DeviceRecord>, StoreError> {
            let conn = pool.get().map_err(|e| StoreError::Pool(e.to_string()))?;
            Ok(conn
                .query_row(
                    "SELECT id,name,token_hash,scopes,created_ms,expires_ms,last_seen_ms,revoked_ms \
                     FROM devices WHERE token_hash=?1 AND revoked_ms IS NULL",
                    params![hash],
                    row_to_device,
                )
                .optional()?)
        })
        .await
        .map_err(|e| StoreError::Pool(e.to_string()))?
    }

    pub async fn devices(&self) -> Result<Vec<DeviceRecord>, StoreError> {
        let pool = self.inner.read_pool.clone();
        tokio::task::spawn_blocking(move || -> Result<Vec<DeviceRecord>, StoreError> {
            let conn = pool.get().map_err(|e| StoreError::Pool(e.to_string()))?;
            let mut stmt = conn.prepare(
                "SELECT id,name,token_hash,scopes,created_ms,expires_ms,last_seen_ms,revoked_ms \
                 FROM devices ORDER BY created_ms,id",
            )?;
            let rows = stmt.query_map([], row_to_device)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
        })
        .await
        .map_err(|e| StoreError::Pool(e.to_string()))?
    }
}

fn oneshot_channel<T>() -> (
    tokio::sync::oneshot::Sender<T>,
    tokio::sync::oneshot::Receiver<T>,
) {
    tokio::sync::oneshot::channel()
}
