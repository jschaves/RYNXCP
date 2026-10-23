//! Append-only, tamper-evident audit log.
//!
//! Each entry commits to the previous one by hashing a canonical JSON
//! preimage that includes `prev_hash`. Removing, reordering or editing any
//! entry breaks the chain, which `verify` detects.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::crypto::{now_rfc3339, sha256_hex};
use crate::{CoreError, CoreResult};
use rynx_proto::AuditEntry;

pub const GENESIS_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[derive(Debug, Serialize, Deserialize)]
struct Preimage<'a> {
    seq: i64,
    at: &'a str,
    actor: &'a str,
    action: &'a str,
    target: &'a str,
    detail: &'a str,
    prev_hash: &'a str,
}

#[derive(Debug, Default)]
struct ChainState {
    seq: i64,
    prev_hash: String,
}

pub struct AuditLog {
    path: PathBuf,
    state: Mutex<ChainState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditReport {
    pub entries: usize,
    pub valid: bool,
    pub first_invalid_seq: Option<i64>,
}

impl AuditLog {
    pub fn open(path: &Path) -> CoreResult<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut seq = 0i64;
        let mut prev_hash = GENESIS_HASH.to_string();
        if path.exists() {
            let file = File::open(path)?;
            for line in BufReader::new(file).lines() {
                let line = line?;
                if line.trim().is_empty() {
                    continue;
                }
                let entry: AuditEntry = serde_json::from_str(&line)
                    .map_err(|e| CoreError::Audit(format!("corrupt audit log: {e}")))?;
                seq = entry.seq;
                prev_hash = entry.hash;
            }
        }
        let log = Self {
            path: path.to_path_buf(),
            state: Mutex::new(ChainState { seq, prev_hash }),
        };
        log.restrict_permissions()?;
        Ok(log)
    }

    fn restrict_permissions(&self) -> CoreResult<()> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&self.path, std::fs::Permissions::from_mode(0o600));
        }
        Ok(())
    }

    pub fn append(
        &self,
        actor: &str,
        action: &str,
        target: &str,
        detail: &str,
    ) -> CoreResult<AuditEntry> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| CoreError::Audit("audit lock poisoned".into()))?;
        let seq = state.seq + 1;
        let at = now_rfc3339();
        let prev_hash = state.prev_hash.clone();
        let preimage = Preimage {
            seq,
            at: &at,
            actor,
            action,
            target,
            detail,
            prev_hash: &prev_hash,
        };
        let encoded = serde_json::to_vec(&preimage)
            .map_err(|e| CoreError::Audit(format!("cannot encode audit preimage: {e}")))?;
        let hash = sha256_hex(&encoded);
        let entry = AuditEntry {
            seq,
            at: at.clone(),
            actor: actor.to_string(),
            action: action.to_string(),
            target: target.to_string(),
            detail: detail.to_string(),
            prev_hash,
            hash: hash.clone(),
        };
        let line = serde_json::to_string(&entry)
            .map_err(|e| CoreError::Audit(format!("cannot encode audit entry: {e}")))?;

        let mut opts = OpenOptions::new();
        opts.create(true).append(true);
        let mut file = opts.open(&self.path)?;
        file.write_all(line.as_bytes())?;
        file.write_all(b"\n")?;
        file.sync_all()?;

        state.seq = seq;
        state.prev_hash = hash;
        Ok(entry)
    }
}

/// Recompute the whole chain and report the first broken sequence number.
pub fn verify(path: &Path) -> CoreResult<AuditReport> {
    if !path.exists() {
        return Ok(AuditReport {
            entries: 0,
            valid: true,
            first_invalid_seq: None,
        });
    }
    let file = File::open(path)?;
    let mut seq_expected = 1i64;
    let mut prev_hash = GENESIS_HASH.to_string();
    let mut entries = 0usize;
    let mut first_invalid = None;

    for line in BufReader::new(file).lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        entries += 1;
        let entry: AuditEntry = match serde_json::from_str(&line) {
            Ok(e) => e,
            Err(_) => {
                if first_invalid.is_none() {
                    first_invalid = Some(seq_expected);
                }
                break;
            }
        };
        let preimage = Preimage {
            seq: entry.seq,
            at: &entry.at,
            actor: &entry.actor,
            action: &entry.action,
            target: &entry.target,
            detail: &entry.detail,
            prev_hash: &entry.prev_hash,
        };
        let encoded = serde_json::to_vec(&preimage)
            .map_err(|e| CoreError::Audit(format!("cannot encode audit preimage: {e}")))?;
        let expected_hash = sha256_hex(&encoded);
        if entry.seq != seq_expected || entry.prev_hash != prev_hash || entry.hash != expected_hash
        {
            if first_invalid.is_none() {
                first_invalid = Some(entry.seq);
            }
            break;
        }
        prev_hash = entry.hash;
        seq_expected += 1;
    }

    Ok(AuditReport {
        entries,
        valid: first_invalid.is_none(),
        first_invalid_seq: first_invalid,
    })
}

pub fn tail(path: &Path, limit: usize) -> CoreResult<Vec<AuditEntry>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let file = File::open(path)?;
    let mut all = Vec::new();
    for line in BufReader::new(file).lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(entry) = serde_json::from_str::<AuditEntry>(&line) {
            all.push(entry);
        }
    }
    let start = all.len().saturating_sub(limit);
    Ok(all.split_off(start))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_detects_tampering() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit.log");
        let log = AuditLog::open(&path).unwrap();
        log.append("admin", "account.create", "acme", "ok").unwrap();
        log.append("admin", "website.upsert", "example.com", "ok")
            .unwrap();
        let report = verify(&path).unwrap();
        assert!(report.valid);
        assert_eq!(report.entries, 2);

        let content = std::fs::read_to_string(&path).unwrap();
        std::fs::write(&path, content.replacen("\"acme\"", "\"evil\"", 1)).unwrap();
        let report = verify(&path).unwrap();
        assert!(!report.valid);
        assert_eq!(report.first_invalid_seq, Some(1));
    }

    #[test]
    fn reopen_continues_chain() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit.log");
        {
            let log = AuditLog::open(&path).unwrap();
            log.append("admin", "a", "t", "d").unwrap();
        }
        {
            let log = AuditLog::open(&path).unwrap();
            let entry = log.append("admin", "b", "t", "d").unwrap();
            assert_eq!(entry.seq, 2);
        }
        assert!(verify(&path).unwrap().valid);
    }
}
