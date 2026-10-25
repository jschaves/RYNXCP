use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::Serialize;
use tokio::sync::Mutex;

use rynx_core::crypto::random_hex;
use rynx_proto::{CertView, Request, Response};

use crate::state::ApiState;

const JOB_TTL: Duration = Duration::from_secs(15 * 60);
const RUNNING_TTL: Duration = Duration::from_secs(60 * 60);

#[derive(Debug, Clone, Serialize)]
pub struct CertJobView {
    pub id: String,
    pub domain: String,
    pub status: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub certificate: Option<CertView>,
}

pub struct CertJob {
    pub view: CertJobView,
    pub actor: String,
    pub started: Instant,
    pub finished: Option<Instant>,
}

pub type CertJobs = Arc<Mutex<HashMap<String, CertJob>>>;

pub fn new_store() -> CertJobs {
    Arc::new(Mutex::new(HashMap::new()))
}

pub async fn start(
    state: &ApiState,
    domain: &str,
    email: &str,
    staging: bool,
    actor: &str,
) -> CertJobView {
    let id = random_hex(12);
    let view = CertJobView {
        id: id.clone(),
        domain: domain.to_string(),
        status: "running".into(),
        message: "requesting certificate".into(),
        certificate: None,
    };
    {
        let mut jobs = state.cert_jobs.lock().await;
        prune(&mut jobs);
        let existing = jobs
            .values()
            .find(|job| job.actor == actor && job.view.domain == domain && job.finished.is_none())
            .map(|job| job.view.clone());
        if let Some(view) = existing {
            return view;
        }
        jobs.insert(
            id.clone(),
            CertJob {
                view: view.clone(),
                actor: actor.to_string(),
                started: Instant::now(),
                finished: None,
            },
        );
    }

    let jobs = state.cert_jobs.clone();
    let daemon = state.daemon.clone();
    let domain = domain.to_string();
    let email = email.to_string();
    let actor = actor.to_string();
    tokio::spawn(async move {
        let result = daemon
            .call(
                Request::CertIssue {
                    domain,
                    email,
                    staging,
                },
                &actor,
            )
            .await;
        let mut jobs = jobs.lock().await;
        if let Some(job) = jobs.get_mut(&id) {
            match result {
                Ok(Response::Cert { cert }) => {
                    job.view.status = "succeeded".into();
                    job.view.message = "certificate issued".into();
                    job.view.certificate = Some(cert);
                }
                Ok(_) => {
                    job.view.status = "failed".into();
                    job.view.message = "daemon returned an unexpected response".into();
                }
                Err(err) => {
                    job.view.status = "failed".into();
                    job.view.message = err.to_string();
                }
            }
            job.finished = Some(Instant::now());
            if job.view.status == "failed" {
                tracing::warn!(
                    domain = %job.view.domain,
                    error = %job.view.message,
                    "certificate job failed"
                );
            } else {
                tracing::info!(domain = %job.view.domain, "certificate job succeeded");
            }
        }
    });
    view
}

pub async fn get(state: &ApiState, id: &str, actor: &str) -> Option<CertJobView> {
    let mut jobs = state.cert_jobs.lock().await;
    prune(&mut jobs);
    jobs.get(id)
        .filter(|job| job.actor == actor)
        .map(|job| job.view.clone())
}

fn prune(jobs: &mut HashMap<String, CertJob>) {
    jobs.retain(|_, job| match job.finished {
        Some(finished) => finished.elapsed() < JOB_TTL,
        None => job.started.elapsed() < RUNNING_TTL,
    });
}
