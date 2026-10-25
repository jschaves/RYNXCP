use std::path::{Path, PathBuf};
use std::time::Duration;

use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use once_cell::sync::OnceCell;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use rynx_core::crypto::{now_rfc3339, now_unix, random_hex, sha256_hex};
use rynx_core::{validate_email, validate_panel_username, PasswordPolicy};

use crate::role::Role;

pub const SESSION_COOKIE: &str = "rynx_session";
pub const CSRF_HEADER: &str = "x-csrf-token";
pub const MIN_PASSWORD_LEN: usize = 12;

const SCHEMA: &str = r#"
PRAGMA journal_mode=WAL;
PRAGMA foreign_keys=ON;

CREATE TABLE IF NOT EXISTS panel_users (
    id            TEXT PRIMARY KEY,
    username      TEXT NOT NULL UNIQUE,
    email         TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    role          TEXT NOT NULL,
    active        INTEGER NOT NULL DEFAULT 1,
    created_at    TEXT NOT NULL,
    updated_at    TEXT NOT NULL,
    account       TEXT
);

CREATE TABLE IF NOT EXISTS sessions (
    token_hash  TEXT PRIMARY KEY,
    user_id     TEXT NOT NULL REFERENCES panel_users(id) ON DELETE CASCADE,
    csrf_token  TEXT NOT NULL,
    created_at  TEXT NOT NULL,
    expires_at  INTEGER NOT NULL,
    last_seen   INTEGER NOT NULL,
    ip          TEXT NOT NULL DEFAULT '',
    user_agent  TEXT NOT NULL DEFAULT '',
    impersonated_by TEXT
);

CREATE INDEX IF NOT EXISTS idx_sessions_user ON sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_expiry ON sessions(expires_at);

CREATE TABLE IF NOT EXISTS panel_settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS auth_events (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    at         TEXT NOT NULL,
    at_unix    INTEGER NOT NULL,
    username   TEXT NOT NULL,
    ip         TEXT NOT NULL,
    user_agent TEXT NOT NULL,
    outcome    TEXT NOT NULL,
    reason     TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_auth_events_at ON auth_events(at_unix);
CREATE INDEX IF NOT EXISTS idx_auth_events_outcome ON auth_events(outcome, at_unix);
"#;

#[derive(Debug, thiserror::Error)]
pub enum PanelError {
    #[error("database error: {0}")]
    Db(String),
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("not found: {0}")]
    NotFound(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("validation: {0}")]
    Validation(String),
    #[error("cryptographic error: {0}")]
    Crypto(String),
}

impl From<rusqlite::Error> for PanelError {
    fn from(value: rusqlite::Error) -> Self {
        PanelError::Db(value.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserView {
    pub id: String,
    pub username: String,
    pub email: String,
    pub role: Role,
    pub active: bool,
    pub created_at: String,
    pub updated_at: String,
    /// Hosting account linked to this panel user, when it was auto-created.
    pub account: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SessionContext {
    pub user: UserView,
    pub csrf_token: String,
    pub expires_at: i64,
    pub impersonated_by: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NewSession {
    pub token: String,
    pub csrf_token: String,
    pub expires_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthEventView {
    pub id: i64,
    pub at: String,
    pub at_unix: i64,
    pub username: String,
    pub ip: String,
    pub user_agent: String,
    pub outcome: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthStats {
    pub window_secs: i64,
    pub successes: i64,
    pub failures: i64,
    pub blocked: i64,
    pub unique_ips: i64,
}

struct UserRecord {
    view: UserView,
    password_hash: String,
}

pub struct PanelDb {
    path: PathBuf,
}

static DUMMY_HASH: OnceCell<String> = OnceCell::new();

fn dummy_hash() -> &'static str {
    DUMMY_HASH.get_or_init(|| {
        hash_password("rynx-dummy-password-not-used")
            .unwrap_or_else(|_| "$argon2id$v=19$m=19456,t=2,p=1$c29tZXNhbHQ$Zm9vYmFy".into())
    })
}

fn hash_password(password: &str) -> Result<String, PanelError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| PanelError::Crypto(e.to_string()))
}

fn verify_password(password: &str, encoded: &str) -> bool {
    match PasswordHash::new(encoded) {
        Ok(parsed) => Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => false,
    }
}

pub fn validate_password_with(
    policy: &PasswordPolicy,
    password: &str,
    username: &str,
    email: &str,
) -> Result<(), PanelError> {
    policy
        .validate(password, username, email)
        .map_err(|e| PanelError::Validation(e.to_string()))
}

pub fn validate_password(password: &str) -> Result<(), PanelError> {
    let policy = PasswordPolicy::default();
    validate_password_with(&policy, password, "", "")
}

fn map_user(row: &rusqlite::Row<'_>) -> rusqlite::Result<UserView> {
    let role_raw: String = row.get(3)?;
    let role = Role::parse(&role_raw).unwrap_or(Role::Viewer);
    Ok(UserView {
        id: row.get(0)?,
        username: row.get(1)?,
        email: row.get(2)?,
        role,
        active: row.get::<_, i64>(4)? != 0,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
        account: row.get(7)?,
    })
}

impl PanelDb {
    pub fn open(path: &Path) -> Result<Self, PanelError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| PanelError::Db(format!("cannot create database dir: {e}")))?;
        }
        let db = Self {
            path: path.to_path_buf(),
        };
        let conn = db.conn()?;
        conn.execute_batch(SCHEMA)?;
        let has_impersonation: bool = conn
            .prepare("PRAGMA table_info(sessions)")?
            .query_map([], |row| row.get::<_, String>(1))?
            .filter_map(Result::ok)
            .any(|name| name == "impersonated_by");
        if !has_impersonation {
            conn.execute("ALTER TABLE sessions ADD COLUMN impersonated_by TEXT", [])?;
        }
        let has_account: bool = conn
            .prepare("PRAGMA table_info(panel_users)")?
            .query_map([], |row| row.get::<_, String>(1))?
            .filter_map(Result::ok)
            .any(|name| name == "account");
        if !has_account {
            conn.execute("ALTER TABLE panel_users ADD COLUMN account TEXT", [])?;
        }
        conn.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_panel_users_account ON panel_users(account);
             CREATE INDEX IF NOT EXISTS idx_panel_users_email_lower ON panel_users(lower(email));",
        )?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
        }
        Ok(db)
    }

    fn conn(&self) -> Result<Connection, PanelError> {
        let conn = Connection::open(&self.path)?;
        conn.busy_timeout(Duration::from_secs(5))?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        Ok(conn)
    }

    fn get_user(
        &self,
        conn: &Connection,
        username: &str,
    ) -> Result<Option<UserRecord>, PanelError> {
        let record = conn
            .query_row(
                "SELECT id, username, email, role, active, created_at, updated_at, account, password_hash \
                 FROM panel_users WHERE username = ?1",
                params![username],
                |row| {
                    let view = map_user(row)?;
                    let password_hash: String = row.get(8)?;
                    Ok(UserRecord {
                        view,
                        password_hash,
                    })
                },
            )
            .optional()?;
        Ok(record)
    }

    pub fn count_users(&self) -> Result<i64, PanelError> {
        let conn = self.conn()?;
        let count: i64 =
            conn.query_row("SELECT COUNT(*) FROM panel_users", [], |row| row.get(0))?;
        Ok(count)
    }

    pub fn has_owner(&self) -> Result<bool, PanelError> {
        let conn = self.conn()?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM panel_users WHERE role = 'owner'",
            [],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    pub fn create_user(
        &self,
        username: &str,
        email: &str,
        password: &str,
        role: Role,
    ) -> Result<UserView, PanelError> {
        self.create_user_with_policy(&PasswordPolicy::default(), username, email, password, role)
    }

    pub fn create_user_with_policy(
        &self,
        policy: &PasswordPolicy,
        username: &str,
        email: &str,
        password: &str,
        role: Role,
    ) -> Result<UserView, PanelError> {
        self.create_user_inner(policy, username, email, password, role, None)
    }

    /// Create a panel user linked to a hosting account. The link lets the
    /// admin impersonate the account owner from the Accounts view.
    pub fn create_account_user(
        &self,
        policy: &PasswordPolicy,
        username: &str,
        email: &str,
        password: &str,
        role: Role,
        account: &str,
    ) -> Result<UserView, PanelError> {
        self.create_user_inner(policy, username, email, password, role, Some(account))
    }

    fn create_user_inner(
        &self,
        policy: &PasswordPolicy,
        username: &str,
        email: &str,
        password: &str,
        role: Role,
        account: Option<&str>,
    ) -> Result<UserView, PanelError> {
        validate_panel_username(username).map_err(|e| PanelError::Validation(e.to_string()))?;
        validate_email(email).map_err(|e| PanelError::Validation(e.to_string()))?;
        validate_password_with(policy, password, username, email)?;

        let conn = self.conn()?;
        if self.get_user(&conn, username)?.is_some() {
            return Err(PanelError::Conflict(format!(
                "user '{username}' already exists"
            )));
        }
        let email_taken: i64 = conn.query_row(
            "SELECT COUNT(*) FROM panel_users WHERE lower(email) = lower(?1)",
            params![email],
            |row| row.get(0),
        )?;
        if email_taken > 0 {
            return Err(PanelError::Conflict(
                "email is already used by another panel user".into(),
            ));
        }
        let hash = hash_password(password)?;
        let now = now_rfc3339();
        let id = uuid::Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO panel_users (id, username, email, password_hash, role, active, created_at, updated_at, account) \
             VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6, ?6, ?7)",
            params![id, username, email, hash, role.as_str(), now, account],
        )?;
        Ok(UserView {
            id,
            username: username.to_string(),
            email: email.to_string(),
            role,
            active: true,
            created_at: now.clone(),
            updated_at: now,
            account: account.map(str::to_string),
        })
    }

    pub fn authenticate(
        &self,
        username: &str,
        password: &str,
    ) -> Result<Option<UserView>, PanelError> {
        let conn = self.conn()?;
        let record = self.get_user(&conn, username)?;
        // Always perform a verification so account existence cannot be
        // inferred from timing.
        let Some(record) = record else {
            let _ = verify_password(password, dummy_hash());
            return Ok(None);
        };
        if !verify_password(password, &record.password_hash) {
            return Ok(None);
        }
        if !record.view.active {
            return Ok(None);
        }
        Ok(Some(record.view))
    }

    pub fn find_user(&self, username: &str) -> Result<Option<UserView>, PanelError> {
        let conn = self.conn()?;
        Ok(self.get_user(&conn, username)?.map(|record| record.view))
    }

    pub fn find_user_by_account(&self, account: &str) -> Result<Option<UserView>, PanelError> {
        let conn = self.conn()?;
        let record = conn
            .query_row(
                "SELECT id, username, email, role, active, created_at, updated_at, account \
                 FROM panel_users WHERE account = ?1",
                params![account],
                map_user,
            )
            .optional()?;
        Ok(record)
    }

    pub fn find_user_by_email(&self, email: &str) -> Result<Option<UserView>, PanelError> {
        let conn = self.conn()?;
        let record = conn
            .query_row(
                "SELECT id, username, email, role, active, created_at, updated_at, account \
                 FROM panel_users WHERE lower(email) = lower(?1)",
                params![email],
                map_user,
            )
            .optional()?;
        Ok(record)
    }

    pub fn email_exists(&self, email: &str) -> Result<bool, PanelError> {
        Ok(self.find_user_by_email(email)?.is_some())
    }

    pub fn count_owners(&self) -> Result<i64, PanelError> {
        let conn = self.conn()?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM panel_users WHERE role = 'owner' AND active = 1",
            [],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    pub fn list_users(&self) -> Result<Vec<UserView>, PanelError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, username, email, role, active, created_at, updated_at, account \
             FROM panel_users ORDER BY username",
        )?;
        let rows = stmt.query_map([], map_user)?;
        let mut users = Vec::new();
        for row in rows {
            users.push(row?);
        }
        Ok(users)
    }

    pub fn set_password(&self, username: &str, password: &str) -> Result<(), PanelError> {
        self.set_password_with_policy(&PasswordPolicy::default(), username, password)
    }

    pub fn set_password_with_policy(
        &self,
        policy: &PasswordPolicy,
        username: &str,
        password: &str,
    ) -> Result<(), PanelError> {
        let conn = self.conn()?;
        let record = self
            .get_user(&conn, username)?
            .ok_or_else(|| PanelError::NotFound(username.into()))?;
        validate_password_with(policy, password, username, &record.view.email)?;
        let hash = hash_password(password)?;
        let changed = conn.execute(
            "UPDATE panel_users SET password_hash = ?2, updated_at = ?3 WHERE username = ?1",
            params![username, hash, now_rfc3339()],
        )?;
        if changed == 0 {
            return Err(PanelError::NotFound(username.into()));
        }
        conn.execute(
            "DELETE FROM sessions WHERE user_id = (SELECT id FROM panel_users WHERE username = ?1)",
            params![username],
        )?;
        Ok(())
    }

    pub fn set_email(&self, username: &str, email: &str) -> Result<(), PanelError> {
        validate_email(email).map_err(|e| PanelError::Validation(e.to_string()))?;
        let conn = self.conn()?;
        let email_taken: i64 = conn.query_row(
            "SELECT COUNT(*) FROM panel_users WHERE lower(email) = lower(?1) AND username <> ?2",
            params![email, username],
            |row| row.get(0),
        )?;
        if email_taken > 0 {
            return Err(PanelError::Conflict(
                "email is already used by another panel user".into(),
            ));
        }
        let changed = conn.execute(
            "UPDATE panel_users SET email = ?2, updated_at = ?3 WHERE username = ?1",
            params![username, email, now_rfc3339()],
        )?;
        if changed == 0 {
            return Err(PanelError::NotFound(username.into()));
        }
        Ok(())
    }

    pub fn set_active(&self, username: &str, active: bool) -> Result<(), PanelError> {
        let conn = self.conn()?;
        let changed = conn.execute(
            "UPDATE panel_users SET active = ?2, updated_at = ?3 WHERE username = ?1",
            params![username, if active { 1 } else { 0 }, now_rfc3339()],
        )?;
        if changed == 0 {
            return Err(PanelError::NotFound(username.into()));
        }
        if !active {
            conn.execute(
                "DELETE FROM sessions WHERE user_id = (SELECT id FROM panel_users WHERE username = ?1)",
                params![username],
            )?;
        }
        Ok(())
    }

    pub fn set_role(&self, username: &str, role: Role) -> Result<(), PanelError> {
        let conn = self.conn()?;
        let changed = conn.execute(
            "UPDATE panel_users SET role = ?2, updated_at = ?3 WHERE username = ?1",
            params![username, role.as_str(), now_rfc3339()],
        )?;
        if changed == 0 {
            return Err(PanelError::NotFound(username.into()));
        }
        Ok(())
    }

    pub fn delete_user(&self, username: &str) -> Result<(), PanelError> {
        let conn = self.conn()?;
        let changed = conn.execute(
            "DELETE FROM panel_users WHERE username = ?1",
            params![username],
        )?;
        if changed == 0 {
            return Err(PanelError::NotFound(username.into()));
        }
        Ok(())
    }

    pub fn create_session(
        &self,
        user_id: &str,
        ttl_secs: i64,
        ip: &str,
        user_agent: &str,
        impersonated_by: Option<&str>,
    ) -> Result<NewSession, PanelError> {
        let conn = self.conn()?;
        let token = random_hex(32);
        let token_hash = sha256_hex(token.as_bytes());
        let csrf = random_hex(24);
        let now = now_unix();
        let expires_at = now + ttl_secs.max(60);
        conn.execute(
            "INSERT INTO sessions (token_hash, user_id, csrf_token, created_at, expires_at, last_seen, ip, user_agent, impersonated_by) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?4, ?6, ?7, ?8)",
            params![
                token_hash,
                user_id,
                csrf,
                now.to_string(),
                expires_at,
                ip,
                user_agent,
                impersonated_by
            ],
        )?;
        let _ = conn.execute("DELETE FROM sessions WHERE expires_at < ?1", params![now]);
        Ok(NewSession {
            token,
            csrf_token: csrf,
            expires_at,
        })
    }

    pub fn lookup_session(
        &self,
        token: &str,
        idle_secs: i64,
    ) -> Result<Option<SessionContext>, PanelError> {
        if token.len() != 64 {
            return Ok(None);
        }
        let token_hash = sha256_hex(token.as_bytes());
        let conn = self.conn()?;
        let row = conn
            .query_row(
                "SELECT s.csrf_token, s.expires_at, s.last_seen, s.impersonated_by, u.id, u.username, u.email, u.role, u.active, u.created_at, u.updated_at, u.account \
                 FROM sessions s JOIN panel_users u ON u.id = s.user_id WHERE s.token_hash = ?1",
                params![token_hash],
                |row| {
                    let role_raw: String = row.get(7)?;
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        UserView {
                            id: row.get(4)?,
                            username: row.get(5)?,
                            email: row.get(6)?,
                            role: Role::parse(&role_raw).unwrap_or(Role::Viewer),
                            active: row.get::<_, i64>(8)? != 0,
                            created_at: row.get(9)?,
                            updated_at: row.get(10)?,
                            account: row.get(11)?,
                        },
                    ))
                },
            )
            .optional()?;

        let Some((csrf_token, expires_at, last_seen, impersonated_by, user)) = row else {
            return Ok(None);
        };
        let now = now_unix();
        let idle_expired = idle_secs > 0 && now.saturating_sub(last_seen) > idle_secs;
        if expires_at <= now || idle_expired || !user.active {
            let _ = conn.execute(
                "DELETE FROM sessions WHERE token_hash = ?1",
                params![token_hash],
            );
            return Ok(None);
        }
        let _ = conn.execute(
            "UPDATE sessions SET last_seen = ?2 WHERE token_hash = ?1",
            params![token_hash, now],
        );
        Ok(Some(SessionContext {
            user,
            csrf_token,
            expires_at,
            impersonated_by,
        }))
    }

    pub fn record_auth_event(
        &self,
        username: &str,
        ip: &str,
        user_agent: &str,
        outcome: &str,
        reason: &str,
    ) -> Result<(), PanelError> {
        let conn = self.conn()?;
        let now = now_unix();
        conn.execute(
            "INSERT INTO auth_events (at, at_unix, username, ip, user_agent, outcome, reason) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                now_rfc3339(),
                now,
                username,
                ip,
                user_agent,
                outcome,
                reason
            ],
        )?;
        let _ = conn.execute(
            "DELETE FROM auth_events WHERE at_unix < ?1",
            params![now - 30 * 24 * 60 * 60],
        );
        Ok(())
    }

    pub fn list_auth_events(&self, limit: u32) -> Result<Vec<AuthEventView>, PanelError> {
        let conn = self.conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, at, at_unix, username, ip, user_agent, outcome, reason \
             FROM auth_events ORDER BY id DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit.clamp(1, 1000) as i64], |row| {
            Ok(AuthEventView {
                id: row.get(0)?,
                at: row.get(1)?,
                at_unix: row.get(2)?,
                username: row.get(3)?,
                ip: row.get(4)?,
                user_agent: row.get(5)?,
                outcome: row.get(6)?,
                reason: row.get(7)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn auth_stats(&self, window_secs: i64) -> Result<AuthStats, PanelError> {
        let conn = self.conn()?;
        let since = now_unix() - window_secs.max(60);
        let successes: i64 = conn.query_row(
            "SELECT COUNT(*) FROM auth_events WHERE outcome = 'success' AND at_unix >= ?1",
            params![since],
            |row| row.get(0),
        )?;
        let failures: i64 = conn.query_row(
            "SELECT COUNT(*) FROM auth_events WHERE outcome = 'failure' AND at_unix >= ?1",
            params![since],
            |row| row.get(0),
        )?;
        let blocked: i64 = conn.query_row(
            "SELECT COUNT(*) FROM auth_events WHERE outcome = 'blocked' AND at_unix >= ?1",
            params![since],
            |row| row.get(0),
        )?;
        let unique_ips: i64 = conn.query_row(
            "SELECT COUNT(DISTINCT ip) FROM auth_events WHERE outcome <> 'success' AND at_unix >= ?1",
            params![since],
            |row| row.get(0),
        )?;
        Ok(AuthStats {
            window_secs,
            successes,
            failures,
            blocked,
            unique_ips,
        })
    }

    pub fn delete_session(&self, token: &str) -> Result<(), PanelError> {
        let token_hash = sha256_hex(token.as_bytes());
        let conn = self.conn()?;
        conn.execute(
            "DELETE FROM sessions WHERE token_hash = ?1",
            params![token_hash],
        )?;
        Ok(())
    }

    pub fn purge_expired_sessions(&self) -> Result<usize, PanelError> {
        let conn = self.conn()?;
        Ok(conn.execute(
            "DELETE FROM sessions WHERE expires_at < ?1",
            params![now_unix()],
        )?)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), PanelError> {
        let conn = self.conn()?;
        conn.execute(
            "INSERT INTO panel_settings (key, value) VALUES (?1, ?2) \
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>, PanelError> {
        let conn = self.conn()?;
        Ok(conn
            .query_row(
                "SELECT value FROM panel_settings WHERE key = ?1",
                params![key],
                |row| row.get(0),
            )
            .optional()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db() -> (tempfile::TempDir, PanelDb) {
        let dir = tempfile::tempdir().unwrap();
        let db = PanelDb::open(&dir.path().join("panel.db")).unwrap();
        (dir, db)
    }

    #[test]
    fn create_and_authenticate() {
        let (_dir, db) = temp_db();
        let user = db
            .create_user(
                "admin",
                "admin@example.com",
                "Very-Strong-Pass1",
                Role::Owner,
            )
            .unwrap();
        assert_eq!(user.username, "admin");
        assert!(db
            .authenticate("admin", "Very-Strong-Pass1")
            .unwrap()
            .is_some());
        assert!(db
            .authenticate("admin", "wrong-password")
            .unwrap()
            .is_none());
        assert!(db
            .authenticate("ghost", "Very-Strong-Pass1")
            .unwrap()
            .is_none());
    }

    #[test]
    fn links_account_users_and_checks_email() {
        let (_dir, db) = temp_db();
        let policy = PasswordPolicy::default();
        let user = db
            .create_account_user(
                &policy,
                "acme",
                "acme@example.com",
                "Very-Strong-Pass1",
                Role::Viewer,
                "acme",
            )
            .unwrap();
        assert_eq!(user.account.as_deref(), Some("acme"));
        assert!(db.email_exists("ACME@example.com").unwrap());
        assert_eq!(
            db.find_user_by_account("acme").unwrap().unwrap().username,
            "acme"
        );
        assert!(db.find_user_by_email("acme@example.com").unwrap().is_some());
        assert!(matches!(
            db.create_user("bob", "acme@example.com", "Very-Strong-Pass1", Role::Viewer),
            Err(PanelError::Conflict(_))
        ));
    }

    #[test]
    fn rejects_duplicate_and_weak_password() {
        let (_dir, db) = temp_db();
        db.create_user(
            "admin",
            "admin@example.com",
            "Very-Strong-Pass1",
            Role::Owner,
        )
        .unwrap();
        assert!(matches!(
            db.create_user(
                "admin",
                "other@example.com",
                "Very-Strong-Pass1",
                Role::Admin
            ),
            Err(PanelError::Conflict(_))
        ));
        assert!(matches!(
            db.create_user("bob", "bob@example.com", "short", Role::Admin),
            Err(PanelError::Validation(_))
        ));
    }

    #[test]
    fn session_lifecycle() {
        let (_dir, db) = temp_db();
        let user = db
            .create_user(
                "admin",
                "admin@example.com",
                "Very-Strong-Pass1",
                Role::Owner,
            )
            .unwrap();
        let session = db
            .create_session(&user.id, 3600, "127.0.0.1", "test", None)
            .unwrap();
        let ctx = db.lookup_session(&session.token, 3600).unwrap().unwrap();
        assert_eq!(ctx.user.username, "admin");
        assert_eq!(ctx.csrf_token, session.csrf_token);
        db.delete_session(&session.token).unwrap();
        assert!(db.lookup_session(&session.token, 3600).unwrap().is_none());
    }

    #[test]
    fn password_change_revokes_sessions() {
        let (_dir, db) = temp_db();
        let user = db
            .create_user(
                "admin",
                "admin@example.com",
                "Very-Strong-Pass1",
                Role::Owner,
            )
            .unwrap();
        let session = db
            .create_session(&user.id, 3600, "127.0.0.1", "test", None)
            .unwrap();
        db.set_password("admin", "Another-Strong-Pass2").unwrap();
        assert!(db.lookup_session(&session.token, 3600).unwrap().is_none());
        assert!(db
            .authenticate("admin", "Another-Strong-Pass2")
            .unwrap()
            .is_some());
    }
}
