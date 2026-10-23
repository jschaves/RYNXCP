use std::sync::Arc;

use rynx_core::ApiConfig;
use rynx_panel::PanelDb;

use crate::daemon::DaemonClient;
use crate::ratelimit::LoginLimiter;

#[derive(Clone)]
pub struct ApiState {
    pub cfg: Arc<ApiConfig>,
    pub db: Arc<PanelDb>,
    pub daemon: Arc<DaemonClient>,
    pub limiter: Arc<LoginLimiter>,
}
