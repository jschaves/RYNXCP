use std::sync::Arc;

use rynx_core::ApiConfig;
use rynx_panel::PanelDb;

use crate::cert_jobs::CertJobs;
use crate::daemon::DaemonClient;
use crate::email::Mailer;
use crate::ratelimit::LoginLimiter;

#[derive(Clone)]
pub struct ApiState {
    pub cfg: Arc<ApiConfig>,
    pub db: Arc<PanelDb>,
    pub daemon: Arc<DaemonClient>,
    pub limiter: Arc<LoginLimiter>,
    pub reset_limiter: Arc<LoginLimiter>,
    pub cert_jobs: CertJobs,
    pub mailer: Option<Arc<Mailer>>,
}
