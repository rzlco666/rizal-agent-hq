//! Mesin Humanisasi — lapisan ATURAN (K2-C: aturan untuk log/task per baris;
//! LLM $0 hanya untuk ringkasan sesi, dikerjakan hq-server dan selalu punya
//! fallback ke template di sini).
//!
//! Pola kalimat seragam (PRD §2.3): [Agent] [kata kerja] [objek] — [detail].
//! Tidak pernah mengarang: fakta yang tidak diketahui diganti frasa jujur.

use crate::redact::redact;

/// Fakta mentah hasil ekstraksi kolektor.
#[derive(Debug, Clone, Default)]
pub struct RawActivity {
    pub agent_name: String,
    /// Baris perintah / tool mentah (akan disensor).
    pub raw: String,
    /// Nama project yang dikenali dari path/sesi.
    pub project: Option<String>,
    /// Selesai dengan sukses? None = masih berjalan / tidak diketahui.
    pub success: Option<bool>,
    /// Detail hasil singkat, mis. "188 tes hijau", nama file, hash commit.
    pub detail: Option<String>,
}

/// Nama ramah untuk provider/model.
pub fn friendly_model(provider_model: &str) -> String {
    let s = provider_model.to_lowercase();
    if s.contains("codex") || s.contains("gpt") {
        "GPT (Codex)".into()
    } else if s.contains("deepseek") {
        "DeepSeek".into()
    } else if s.contains("claude") {
        "Claude".into()
    } else if s.contains("gemini") {
        "Gemini".into()
    } else if s.contains("grok") || s.contains("xai") {
        "Grok".into()
    } else if s.contains("kimi") {
        "Kimi".into()
    } else {
        provider_model.to_string()
    }
}

fn with_detail(base: String, detail: &Option<String>) -> String {
    match detail {
        Some(d) if !d.trim().is_empty() => format!("{base} — {d}"),
        _ => base,
    }
}

/// Ubah satu aktivitas mentah menjadi SATU kalimat manusiawi.
pub fn humanize(a: &RawActivity) -> String {
    let raw = redact(&a.raw);
    let low = raw.to_lowercase();
    let agent = a.agent_name.as_str();
    let proj = a.project.as_deref();
    let proj_s = proj.unwrap_or("project berjalan");

    let base = if low.contains("cargo test")
        || low.contains("npm test")
        || low.contains("bun test")
        || low.contains("pnpm test")
    {
        match a.success {
            Some(true) => format!("{agent} menjalankan tes {proj_s} dan semuanya lulus"),
            Some(false) => format!("{agent} menjalankan tes {proj_s} — ada yang gagal"),
            None => format!("{agent} sedang menjalankan tes {proj_s}"),
        }
    } else if low.contains("cargo build")
        || low.contains("cargo clippy")
        || low.contains("npm run build")
        || low.contains("bun run build")
    {
        format!("{agent} sedang membangun {proj_s}")
    } else if low.contains("git commit") {
        format!("{agent} menyelesaikan satu tahap di {proj_s}: commit terbuat")
    } else if low.contains("git push") {
        format!("{agent} mengirim hasil kerja {proj_s} ke GitHub")
    } else if low.contains("gh run watch") || low.contains("gh run view") {
        format!("{agent} menunggu hasil CI GitHub untuk {proj_s}")
    } else if low.contains("--model") || low.contains("omp") && low.contains("-p ") {
        // Pemanggilan OMP dengan model tertentu.
        let model = raw
            .split("--model")
            .nth(1)
            .and_then(|r| r.split_whitespace().next())
            .map(friendly_model)
            .unwrap_or_else(|| "model yang dipilih".into());
        format!("{agent} mulai tugas baru memakai {model}")
    } else if low.contains("cursor-agent") {
        format!("{agent} sedang mengerjakan {proj_s} di WSL")
    } else if low.contains("docker") && (low.contains(" up") || low.contains("healthy")) {
        format!("Layanan pendukung {proj_s} siap dipakai")
    } else if low.contains("sleep") || low.contains("idle") {
        format!("{agent} sedang santai, menunggu tugas berikutnya")
    } else if let Some(p) = proj {
        // Jujur tanpa mengarang isi tugas.
        format!("{agent} sedang menjalankan perintah di {p}")
    } else {
        format!("{agent} sedang bekerja")
    };
    with_detail(base, &a.detail)
}

/// Kalimat manusiawi untuk SATU panggilan tool dari file sesi.
/// `intent` adalah maksud "i" yang ditulis agent sendiri (bahasa apa adanya);
/// dipakai hanya bila target tidak ada, agar kalimat tetap jujur.
pub fn humanize_tool(agent: &str, tool: &str, intent: &str, target: &str, project: &str) -> String {
    let base = target.rsplit('/').next().unwrap_or(target);
    match tool {
        "read" => {
            if !target.is_empty() {
                format!("{agent} membaca berkas {base} di {project}")
            } else {
                format!("{agent} membaca berkas di {project}")
            }
        }
        "grep" => format!("{agent} menelusuri kode {project} untuk menemukan yang relevan"),
        "glob" => format!("{agent} memetakan struktur berkas {project}"),
        "edit" => {
            let b = format!("{agent} mengubah berkas {base} di {project}");
            if intent.is_empty() {
                b
            } else {
                format!("{b} — {}", intent.trim_end_matches('.'))
            }
        }
        "write" => format!("{agent} menulis berkas {base} di {project}"),
        "bash" => {
            let b = humanize(&RawActivity {
                agent_name: agent.into(),
                raw: target.to_string(),
                project: Some(project.to_string()),
                success: None,
                detail: None,
            });
            // Bila kalimatnya masih umum, pakai maksud agent sendiri sebagai detail.
            if b.contains("sedang menjalankan perintah") && !intent.is_empty() {
                format!("{b} — {}", intent.trim_end_matches('.'))
            } else {
                b
            }
        }
        "task" | "spawn" => format!("{agent} memanggil worker untuk membantu di {project}"),
        "web_search" | "search" => format!("{agent} mencari informasi di web untuk {project}"),
        "web_fetch" | "fetch" => format!("{agent} membuka halaman web untuk {project}"),
        _ => {
            if !intent.is_empty() {
                format!(
                    "{agent} sedang bekerja di {project}: {}",
                    intent.chars().take(90).collect::<String>()
                )
            } else if !target.is_empty() {
                format!("{agent} memakai perkakas {tool} pada {base} di {project}")
            } else {
                format!("{agent} sedang bekerja di {project}")
            }
        }
    }
}

/// Judul tugas manusiawi dari daftar aktivitas (aktivitas dominan/terakhir).
pub fn task_title(activities: &[RawActivity], project: Option<&str>) -> String {
    if let Some(last) = activities.last() {
        let raw = last.raw.to_lowercase();
        let p = project.unwrap_or("project ini");
        if raw.contains("mysql")
            && (raw.contains("fix") || raw.contains("decode") || raw.contains("bug"))
        {
            return format!("Memperbaiki bug MySQL di {p}");
        }
        if raw.contains("test") {
            return format!("Menjalankan dan memastikan tes {p}");
        }
        if raw.contains("build") {
            return format!("Membangun {p}");
        }
    }
    match project {
        Some(p) => format!("Mengerjakan {p}"),
        None => "Mengerjakan tugas berjalan".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ra(raw: &str, project: Option<&str>, success: Option<bool>) -> RawActivity {
        RawActivity {
            agent_name: "OMP".into(),
            raw: raw.into(),
            project: project.map(|s| s.to_string()),
            success,
            detail: None,
        }
    }

    #[test]
    fn contoh_prd_omp_model() {
        let s = humanize(&ra(
            "bun /home/rizal/.bun/bin/omp --model openai-codex/gpt-6.1-sol",
            None,
            None,
        ));
        assert_eq!(s, "OMP mulai tugas baru memakai GPT (Codex)");
    }

    #[test]
    fn contoh_tes_hijau() {
        let mut a = ra("cargo test -p mcp-sql", Some("strut-stack-sql"), Some(true));
        a.detail = Some("188 tes hijau".into());
        let s = humanize(&a);
        assert!(s.contains("menjalankan tes strut-stack-sql"), "{s}");
        assert!(s.contains("188 tes hijau"), "{s}");
    }

    #[test]
    fn contoh_ci_dan_cursor() {
        assert!(humanize(&ra(
            "gh run watch 41405856207",
            Some("strut-stack-sql"),
            None
        ))
        .contains("menunggu hasil CI"));
        let mut c = ra("cursor-agent worker start", Some("sample-next"), None);
        c.agent_name = "Cursor".into();
        assert!(humanize(&c).contains("sedang mengerjakan sample-next"));
    }

    #[test]
    fn tidak_mengarang_dan_tetap_disensor() {
        let s = humanize(&ra(
            "some-tool --api-key crsr_1234567890abcdef --weird",
            Some("svelte-rust"),
            None,
        ));
        assert_eq!(s, "OMP sedang menjalankan perintah di svelte-rust");
    }
}
