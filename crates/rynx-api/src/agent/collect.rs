//! Read-only telemetry collection for the deterministic analyzer.

use rynx_panel::AuthStats;
use rynx_proto::{
    CertView, FilterEvent, FirewallView, PackageUpdate, ProgramInfo, Request, Response,
    ServiceStat, SystemInfo,
};

use crate::state::ApiState;

#[derive(Default)]
pub struct Snapshot {
    pub system: Option<SystemInfo>,
    pub services: Vec<ServiceStat>,
    pub programs: Vec<ProgramInfo>,
    pub updates: Vec<PackageUpdate>,
    pub firewall: Option<FirewallView>,
    pub certs: Vec<CertView>,
    pub filter_events: Vec<FilterEvent>,
    pub auth_stats: AuthStats,
}

/// Collects everything the rules engine needs. Individual daemon failures are
/// tolerated so a partial outage still produces useful findings.
pub async fn collect(state: &ApiState, max_events: u32) -> Snapshot {
    let mut snapshot = Snapshot {
        auth_stats: state.db.auth_stats(24 * 3600).unwrap_or_default(),
        ..Snapshot::default()
    };

    if let Ok(Response::SystemInfo { info }) = state.daemon.call(Request::SystemInfo, "agent").await
    {
        snapshot.system = Some(info);
    }
    if let Ok(Response::ServiceStats { services }) =
        state.daemon.call(Request::ServiceStats, "agent").await
    {
        snapshot.services = services;
    }
    if let Ok(Response::Programs { programs }) = state.daemon.call(Request::Programs, "agent").await
    {
        snapshot.programs = programs;
    }
    if let Ok(Response::Updates { updates }) = state.daemon.call(Request::Updates, "agent").await {
        snapshot.updates = updates;
    }
    if let Ok(Response::Firewall { state }) =
        state.daemon.call(Request::FirewallStatus, "agent").await
    {
        snapshot.firewall = Some(state);
    }
    if let Ok(Response::Certs { certs }) = state.daemon.call(Request::CertList, "agent").await {
        snapshot.certs = certs;
    }
    if let Ok(Response::FilterActivity { events }) = state
        .daemon
        .call(
            Request::FilterActivity {
                limit: max_events.clamp(50, 1000),
            },
            "agent",
        )
        .await
    {
        snapshot.filter_events = events;
    }
    snapshot
}
