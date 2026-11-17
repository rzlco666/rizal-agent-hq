mod docker;
mod git;
mod proc;
mod run;
mod system;
mod tmux;

pub use docker::{ContainerInfo, parse_docker, probe_docker};
pub use git::{RepoCommit, parse_git, probe_git};
pub use proc::{ProcessInfo, parse_ps, probe_processes};
pub use run::{CommandOutput, run_timeout};
pub use system::{SystemMetrics, parse_meminfo, probe_system};
pub use tmux::{TmuxSession, parse_tmux, probe_tmux};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Probes {
    pub processes: Vec<ProcessInfo>,
    pub tmux: Vec<TmuxSession>,
    pub docker: Vec<ContainerInfo>,
    pub git: Vec<RepoCommit>,
    pub system: Option<SystemMetrics>,
}

impl Probes {
    pub fn collect(repos: &[String], data_dir: &std::path::Path) -> Self {
        Self {
            processes: probe_processes(),
            tmux: probe_tmux(),
            docker: probe_docker(),
            git: probe_git(repos),
            system: probe_system(data_dir),
        }
    }

    pub fn all_failed(&self) -> bool {
        self.processes.is_empty()
            && self.tmux.is_empty()
            && self.docker.is_empty()
            && self.git.is_empty()
            && self.system.is_none()
    }
}
