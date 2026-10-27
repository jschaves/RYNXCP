//! Package limit checks shared by the API routes.

use rynx_panel::PackageInput;
use rynx_proto::{
    MailAccountSpec, MailDomainSpec, Request, Response as DaemonResponse, WebsiteSpec,
};

use crate::{error::ApiError, scope, state::ApiState};

pub async fn package_for(
    state: &ApiState,
    account: &str,
) -> Result<Option<PackageInput>, ApiError> {
    let Some(assignment) = state.db.get_account_package(account)? else {
        return Ok(None);
    };
    let Some(package) = state.db.get_package(&assignment.package_id)? else {
        return Ok(None);
    };
    Ok(Some(package.package))
}

pub fn enforce(limit: u64, current: u64, label: &str) -> Result<(), ApiError> {
    if limit > 0 && current >= limit {
        return Err(ApiError::Conflict(format!(
            "the package limit for {label} has been reached ({limit})"
        )));
    }
    Ok(())
}

async fn account_websites(
    state: &ApiState,
    actor: &str,
    account: &str,
) -> Result<Vec<rynx_proto::WebsiteView>, ApiError> {
    match state.daemon.call(Request::WebsiteList, actor).await? {
        DaemonResponse::Websites { websites } => Ok(websites
            .into_iter()
            .filter(|website| website.spec.username == account)
            .collect()),
        _ => Err(ApiError::internal("unexpected daemon response")),
    }
}

pub async fn check_website(
    state: &ApiState,
    actor: &str,
    spec: &WebsiteSpec,
) -> Result<(), ApiError> {
    let Some(package) = package_for(state, &spec.username).await? else {
        return Ok(());
    };
    let websites = account_websites(state, actor, &spec.username).await?;
    let existing = websites
        .iter()
        .find(|website| website.spec.domain.eq_ignore_ascii_case(&spec.domain));
    if existing.is_none() {
        enforce(package.web_domains, websites.len() as u64, "web domains")?;
    }
    let current_aliases: usize = websites
        .iter()
        .filter(|website| existing.map(|item| item.id.as_str()) != Some(website.id.as_str()))
        .map(|website| website.spec.aliases.len())
        .sum();
    enforce(
        package.web_aliases,
        (current_aliases + spec.aliases.len()) as u64,
        "web aliases",
    )
}

pub async fn check_mail_domain(
    state: &ApiState,
    actor: &str,
    spec: &MailDomainSpec,
) -> Result<(), ApiError> {
    let Some(package) = package_for(state, &spec.username).await? else {
        return Ok(());
    };
    let response = state.daemon.call(Request::MailDomainList, actor).await?;
    let DaemonResponse::MailDomains { domains } = response else {
        return Err(ApiError::internal("unexpected daemon response"));
    };
    let existing = domains
        .iter()
        .any(|domain| domain.domain.eq_ignore_ascii_case(&spec.domain));
    if !existing {
        let current = domains
            .iter()
            .filter(|domain| domain.username == spec.username)
            .count();
        enforce(package.mail_domains, current as u64, "mail domains")?;
    }
    Ok(())
}

pub async fn check_mail_account(
    state: &ApiState,
    actor: &str,
    spec: &MailAccountSpec,
) -> Result<(), ApiError> {
    let response = state.daemon.call(Request::MailDomainList, actor).await?;
    let DaemonResponse::MailDomains { domains } = response else {
        return Err(ApiError::internal("unexpected daemon response"));
    };
    let owner = domains
        .iter()
        .find(|domain| domain.domain.eq_ignore_ascii_case(&spec.domain))
        .map(|domain| domain.username.clone())
        .ok_or_else(|| ApiError::NotFound("mail domain not found".into()))?;
    let Some(package) = package_for(state, &owner).await? else {
        return Ok(());
    };
    let response = state.daemon.call(Request::MailAccountList, actor).await?;
    let DaemonResponse::MailAccounts { accounts } = response else {
        return Err(ApiError::internal("unexpected daemon response"));
    };
    let current = accounts
        .iter()
        .filter(|account| account.domain.eq_ignore_ascii_case(&spec.domain))
        .filter(|account| Some(account.id.as_str()) != spec.id.as_deref())
        .count();
    enforce(package.mail_accounts, current as u64, "mail accounts")
}

pub async fn check_database(state: &ApiState, actor: &str, domain: &str) -> Result<(), ApiError> {
    let website = scope::website_for_domain(state, actor, domain).await?;
    let Some(package) = package_for(state, &website.spec.username).await? else {
        return Ok(());
    };
    let domains = scope::account_domains(state, actor, &website.spec.username).await?;
    let response = state.daemon.call(Request::DatabaseList, actor).await?;
    let DaemonResponse::Databases { databases } = response else {
        return Err(ApiError::internal("unexpected daemon response"));
    };
    let current = databases
        .iter()
        .filter(|database| domains.contains(&database.domain))
        .count();
    enforce(package.databases, current as u64, "databases")
}

pub async fn check_ftp(state: &ApiState, actor: &str, domain: &str) -> Result<(), ApiError> {
    let website = scope::website_for_domain(state, actor, domain).await?;
    let Some(package) = package_for(state, &website.spec.username).await? else {
        return Ok(());
    };
    let domains = scope::account_domains(state, actor, &website.spec.username).await?;
    let response = state.daemon.call(Request::FtpList, actor).await?;
    let DaemonResponse::Ftps { accounts } = response else {
        return Err(ApiError::internal("unexpected daemon response"));
    };
    let current = accounts
        .iter()
        .filter(|account| domains.contains(&account.domain))
        .count();
    enforce(package.ftp_accounts, current as u64, "FTP accounts")
}

pub async fn check_backup(state: &ApiState, actor: &str, account: &str) -> Result<(), ApiError> {
    let Some(package) = package_for(state, account).await? else {
        return Ok(());
    };
    let response = state.daemon.call(Request::BackupList, actor).await?;
    let DaemonResponse::Backups { backups } = response else {
        return Err(ApiError::internal("unexpected daemon response"));
    };
    let current = backups
        .iter()
        .filter(|backup| backup.username == account)
        .count();
    enforce(package.backups, current as u64, "backups")
}
