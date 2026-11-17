//! hq-alert — Mesin Aturan Alert (PRD §4.5).
//! Setiap alert wajib menjawab: siapa, apa yang dibutuhkan Rizal persisnya, sejak kapan.
//! Alert yang kondisinya hilang ditutup sebagai PulihSendiri oleh pemanggil.

use hq_core::timefmt::now_ms;
use hq_core::*;

fn mk(agent: &str, level: AlertLevel, title: String, needs: String, since: i64) -> Alert {
    Alert {
        id: format!(
            "{}-{}",
            agent,
            title
                .chars()
                .filter(|c| c.is_alphanumeric())
                .take(24)
                .collect::<String>()
                .to_lowercase()
        ),
        agent_id: agent.into(),
        level,
        title,
        needs,
        since,
        state: AlertState::Aktif,
        snoozed_until: None,
    }
}

/// Evaluasi snapshot -> daftar alert yang SEHARUSNYA aktif saat ini.
pub fn evaluate(snap: &Snapshot, prev_agents: &[Agent]) -> Vec<Alert> {
    let now = now_ms();
    let mut out = Vec::new();

    // Kuota provider: >=90 Peringatan, 100 Kritis.
    let mut per_provider: std::collections::HashMap<&str, f64> = std::collections::HashMap::new();
    for q in &snap.quotas {
        if let Some(p) = q.percent {
            let e = per_provider.entry(q.provider.as_str()).or_insert(0.0);
            if p > *e {
                *e = p;
            }
        }
    }
    for (provider, pct) in per_provider {
        let agent = match provider {
            "cursor" => "cursor",
            "devin" => "devin",
            _ => "omp",
        };
        if pct >= 100.0 {
            out.push(mk(
                agent,
                AlertLevel::Kritis,
                format!("Kuota {provider} PENUH (100%)"),
                format!("Putuskan: tunggu reset kuota {provider}, atau alihkan pekerjaan ke provider lain. Agent terkait tidak bisa bekerja normal selama kuota penuh."),
                now,
            ));
        } else if pct >= 90.0 {
            out.push(mk(
                agent,
                AlertLevel::Peringatan,
                format!("Kuota {provider} hampir penuh ({pct:.0}%)"),
                "Tidak perlu action sekarang; pertimbangkan pengalihan beban bila pekerjaan besar akan jalan.".into(),
                now,
            ));
        }
    }

    // Saldo DeepSeek menipis (< $1 Kritis sesuai aturan Rizal).
    if let Some(bal) = snap.deepseek_balance_est {
        if bal < 1.0 {
            out.push(mk(
                "omp",
                AlertLevel::Kritis,
                format!("Saldo DeepSeek menipis (estimasi ${bal:.2})"),
                "Top-up saldo API DeepSeek bila pekerjaan berbayar tunai masih akan berjalan."
                    .into(),
                now,
            ));
        }
    }

    // Agent yang tadinya Bekerja lalu hilang dari daftar/proses -> Perlu Action.
    for prev in prev_agents {
        if prev.status == Status::Bekerja {
            let still = snap.agents.iter().find(|a| a.id == prev.id);
            let gone = match still {
                None => true,
                Some(a) => a.status == Status::Offline && prev.processes > 0,
            };
            if gone {
                out.push(mk(
                    &prev.id,
                    AlertLevel::PerluAction,
                    format!("{} berhenti mendadak", prev.name),
                    format!(
                        "Cek {} di mesin: prosesnya hilang saat sebelumnya sedang bekerja.",
                        prev.name
                    ),
                    now,
                ));
            }
        }
    }

    // Agent berstatus MenungguRizal -> alert Perlu Action dari aktivitasnya.
    for a in &snap.agents {
        if a.status == Status::MenungguRizal {
            out.push(mk(
                &a.id,
                AlertLevel::PerluAction,
                format!("{} menunggu kamu", a.name),
                a.activity.clone(),
                now,
            ));
        }
        if a.status == Status::Error {
            out.push(mk(
                &a.id,
                AlertLevel::Kritis,
                format!("{} dalam keadaan error", a.name),
                a.activity.clone(),
                now,
            ));
        }
    }

    // Lingkungan kritis.
    if snap.env.disk_percent >= 90.0 {
        out.push(mk(
            "wsl",
            AlertLevel::Peringatan,
            format!("Disk WSL hampir penuh ({:.0}%)", snap.env.disk_percent),
            "Bersihkan ruang disk di WSL sebelum pekerjaan besar berikutnya.".into(),
            now,
        ));
    }
    if snap.env.tailscale_muse_online == Some(false) {
        out.push(mk(
            "Muse",
            AlertLevel::PerluAction,
            "Muse terputus dari Tailscale".into(),
            "Periksa koneksi Tailscale di VM Muse / laptop agar Muse bisa kembali bekerja di WSL."
                .into(),
            now,
        ));
    }

    out
}

/// Gabung alert hasil evaluasi dengan status penanganan yang sudah ada:
/// alert lama yang tidak lagi dievaluasi -> PulihSendiri; yang ditangani/ditunda dipertahankan.
pub fn reconcile(mut existing: Vec<Alert>, evaluated: Vec<Alert>) -> Vec<Alert> {
    let now = now_ms();
    for old in existing.iter_mut() {
        if old.state == AlertState::Aktif && !evaluated.iter().any(|e| e.id == old.id) {
            old.state = AlertState::PulihSendiri;
        }
    }
    let mut out = existing;
    for ev in evaluated {
        if let Some(o) = out.iter_mut().find(|o| o.id == ev.id) {
            if o.state == AlertState::PulihSendiri {
                o.state = AlertState::Aktif;
                o.since = now;
            }
            // Ditangani/Ditunda: hormati sampai snooze lewat / kondisi berubah total.
            if o.state == AlertState::Ditunda {
                if let Some(until) = o.snoozed_until {
                    if now >= until {
                        o.state = AlertState::Aktif;
                    }
                }
            }
        } else {
            out.push(ev);
        }
    }
    out.retain(|a| a.state != AlertState::PulihSendiri || now - a.since < 24 * 3600 * 1000);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kuota_penuh_jadi_kritis() {
        let snap = Snapshot {
            quotas: vec![Quota {
                provider: "cursor".into(),
                label: "Cursor Models".into(),
                percent: Some(100.0),
                extra: String::new(),
            }],
            ..Default::default()
        };
        let alerts = evaluate(&snap, &[]);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].level, AlertLevel::Kritis);
        assert_eq!(alerts[0].agent_id, "cursor");
    }

    #[test]
    fn saldo_menipis_jadi_kritis() {
        let snap = Snapshot {
            deepseek_balance_est: Some(0.5),
            ..Default::default()
        };
        let alerts = evaluate(&snap, &[]);
        assert!(alerts.iter().any(|a| a.level == AlertLevel::Kritis));
    }
}
