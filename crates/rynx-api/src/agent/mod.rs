//! Deterministic system analyzer.
//!
//! The agent only reads state through the daemon and the panel database. It
//! stores findings, deduplicates them by fingerprint, optionally enriches them
//! with a local or external LLM and emails new or escalating high/critical
//! findings. It never executes remediation.

pub mod collect;
pub mod provider;
pub mod redact;
pub mod rules;

use std::time::Duration;

use rynx_core::crypto::{now_rfc3339, now_unix, random_hex};
use rynx_panel::{AgentFinding, AgentRun};

use crate::state::ApiState;

pub struct RunSummary {
    pub run: AgentRun,
    pub new_findings: usize,
    pub total_findings: usize,
    pub notified: usize,
}

/// Runs one analysis under the shared agent lock.
pub async fn run_once(state: &ApiState) -> Result<RunSummary, String> {
    let _guard = state.agent_lock.lock().await;
    let started_at = now_rfc3339();
    let snapshot = collect::collect(state, state.cfg.agent.max_events).await;
    if snapshot.system.is_none() && snapshot.services.is_empty() && snapshot.programs.is_empty() {
        let run = AgentRun {
            id: random_hex(8),
            started_at: started_at.clone(),
            finished_at: now_rfc3339(),
            status: "error".into(),
            findings: 0,
            error: "daemon unavailable".into(),
            model: "rules".into(),
            summary: String::new(),
        };
        if let Err(err) = state.db.record_agent_run(&run) {
            tracing::warn!(error = %err, "agent run could not be recorded");
        }
        return Err("daemon unavailable".into());
    }

    let mut findings = rules::evaluate(&snapshot);
    let enhancement = provider::enhance(state, &snapshot, &findings)
        .await
        .unwrap_or_else(|err| {
            tracing::warn!(
                error = %err,
                "local agent provider unavailable; continuing with rules"
            );
            provider::Enhancement::default()
        });
    findings.extend(enhancement.findings);
    let model = if enhancement.model.is_empty() {
        "rules".to_string()
    } else {
        enhancement.model
    };
    let summary = enhancement.summary;
    let run_started_unix = now_unix();
    let mut new_findings = 0usize;
    let mut alerts: Vec<AgentFinding> = Vec::new();
    for finding in &findings {
        match state.db.upsert_agent_finding(finding) {
            Ok((stored, is_new, escalated)) => {
                if is_new {
                    new_findings += 1;
                }
                if (is_new || escalated) && matches!(stored.severity.as_str(), "critical" | "high")
                {
                    alerts.push(stored);
                }
            }
            Err(err) => tracing::warn!(error = %err, "agent finding could not be stored"),
        }
    }
    match state.db.resolve_stale_agent_findings(run_started_unix) {
        Ok(resolved) if resolved > 0 => {
            tracing::info!(
                resolved,
                "agent resolved findings that are no longer present"
            )
        }
        Ok(_) => {}
        Err(err) => tracing::warn!(error = %err, "stale agent findings could not be resolved"),
    }

    let run = AgentRun {
        id: random_hex(8),
        started_at,
        finished_at: now_rfc3339(),
        status: "ok".into(),
        findings: findings.len() as i64,
        error: String::new(),
        model,
        summary,
    };
    if let Err(err) = state.db.record_agent_run(&run) {
        tracing::warn!(error = %err, "agent run could not be recorded");
    }
    let notified = notify(state, &alerts).await;

    Ok(RunSummary {
        run,
        new_findings,
        total_findings: findings.len(),
        notified,
    })
}

/// Starts the background scheduler. Disabled when `agent.enabled = false`.
pub fn spawn(state: ApiState) {
    if !state.cfg.agent.enabled {
        tracing::info!("agent disabled by configuration");
        return;
    }
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(20)).await;
        loop {
            match run_once(&state).await {
                Ok(summary) => tracing::info!(
                    findings = summary.total_findings,
                    new = summary.new_findings,
                    notified = summary.notified,
                    "agent analysis completed"
                ),
                Err(err) => tracing::warn!(error = %err, "agent analysis failed"),
            }
            let secs = state.cfg.agent.interval_secs.clamp(60, 86_400);
            tokio::time::sleep(Duration::from_secs(secs)).await;
        }
    });
}

async fn notify(state: &ApiState, findings: &[AgentFinding]) -> usize {
    if !state.cfg.agent.email_alerts || findings.is_empty() {
        return 0;
    }
    let Some(to) = state.cfg.smtp_alert_to.as_deref() else {
        return 0;
    };
    let Some(mailer) = state.mailer.as_ref() else {
        return 0;
    };
    let subject = format!("RYNXCP: {} alerta(s) del agente", findings.len());
    let body = findings
        .iter()
        .take(20)
        .map(|finding| {
            format!(
                "[{}] {}: {}\n{}\nRecomendación: {}\n",
                finding.severity.to_uppercase(),
                finding.category,
                finding.title,
                finding.summary,
                finding.recommendation
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    match mailer.send(to, &subject, &body).await {
        Ok(()) => findings.len(),
        Err(err) => {
            tracing::warn!(error = %err, "agent alert email could not be sent");
            0
        }
    }
}
