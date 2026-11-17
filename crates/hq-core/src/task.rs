use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TaskPhase {
    Antre,
    Berjalan,
    MenungguRizal,
    Selesai,
    Gagal,
    Batal,
}
