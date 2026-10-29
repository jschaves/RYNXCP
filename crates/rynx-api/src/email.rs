//! Minimal SMTP client for panel notifications.
//!
//! The production default talks to Postfix on loopback without authentication
//! or TLS. Messages are UTF-8 and base64 encoded so headers stay ASCII-safe.

use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use rynx_core::crypto::random_hex;
use rynx_core::{validate_email, ApiConfig};
use time::format_description::well_known::Rfc2822;
use time::OffsetDateTime;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;

use crate::state::ApiState;

const SMTP_TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Debug, Clone)]
pub struct Mailer {
    host: String,
    port: u16,
    from: String,
    helo: String,
}

impl Mailer {
    pub fn from_config(cfg: &ApiConfig) -> Result<Option<Self>> {
        if !cfg.smtp_enabled {
            return Ok(None);
        }
        if cfg.smtp_host.trim().is_empty() {
            bail!("smtp_host is required when smtp_enabled is true");
        }
        if cfg.smtp_port == 0 {
            bail!("smtp_port must be between 1 and 65535");
        }
        if cfg.smtp_tls != "none" {
            bail!("smtp_tls only supports \"none\" for the local Postfix transport");
        }
        let from = cfg
            .smtp_from
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| anyhow!("smtp_from is required when smtp_enabled is true"))?;
        validate_email(from).map_err(|err| anyhow!("invalid smtp_from: {err}"))?;
        let helo = from
            .rsplit_once('@')
            .map(|(_, domain)| domain.to_ascii_lowercase())
            .filter(|domain| !domain.is_empty())
            .ok_or_else(|| anyhow!("smtp_from must include a domain"))?;
        Ok(Some(Self {
            host: cfg.smtp_host.trim().to_string(),
            port: cfg.smtp_port,
            from: from.to_string(),
            helo,
        }))
    }

    pub async fn send(&self, to: &str, subject: &str, body: &str) -> Result<()> {
        validate_email(to).map_err(|err| anyhow!("invalid recipient: {err}"))?;
        let message = self.render(to, subject, body);
        tokio::time::timeout(SMTP_TIMEOUT, self.deliver(to, &message))
            .await
            .map_err(|_| anyhow!("SMTP timed out"))??;
        Ok(())
    }

    async fn deliver(&self, to: &str, message: &str) -> Result<()> {
        let stream = TcpStream::connect((self.host.as_str(), self.port))
            .await
            .with_context(|| format!("cannot connect to SMTP {}:{}", self.host, self.port))?;
        let (read, mut write) = stream.into_split();
        let mut read = BufReader::new(read);

        expect(read_reply(&mut read).await?, &[220], "greeting")?;
        send_line(&mut write, &format!("EHLO {}", self.helo)).await?;
        expect(read_reply(&mut read).await?, &[250], "EHLO")?;
        send_line(&mut write, &format!("MAIL FROM:<{}>", self.from)).await?;
        expect(read_reply(&mut read).await?, &[250], "MAIL FROM")?;
        send_line(&mut write, &format!("RCPT TO:<{to}>")).await?;
        expect(read_reply(&mut read).await?, &[250, 251], "RCPT TO")?;
        send_line(&mut write, "DATA").await?;
        expect(read_reply(&mut read).await?, &[354], "DATA")?;
        write.write_all(message.as_bytes()).await?;
        write.write_all(b".\r\n").await?;
        write.flush().await?;
        expect(read_reply(&mut read).await?, &[250], "message")?;
        let _ = send_line(&mut write, "QUIT").await;
        let _ = read_reply(&mut read).await;
        Ok(())
    }

    fn render(&self, to: &str, subject: &str, body: &str) -> String {
        let date = OffsetDateTime::now_utc()
            .format(&Rfc2822)
            .unwrap_or_else(|_| "Thu, 01 Jan 1970 00:00:00 +0000".into());
        let message_id = format!("<{}@{}>", random_hex(16), self.helo);
        let encoded_body = wrap_base64(&STANDARD.encode(body.as_bytes()));
        format!(
            "Date: {date}\r\n\
             From: RYNXCP <{from}>\r\n\
             To: <{to}>\r\n\
             Subject: {subject}\r\n\
             Message-ID: {message_id}\r\n\
             MIME-Version: 1.0\r\n\
             Content-Type: text/plain; charset=utf-8\r\n\
             Content-Transfer-Encoding: base64\r\n\
             Auto-Submitted: auto-generated\r\n\
             \r\n\
             {encoded_body}",
            from = self.from,
            subject = encode_header(subject),
        )
    }
}

async fn send_line(write: &mut OwnedWriteHalf, line: &str) -> Result<()> {
    write.write_all(line.as_bytes()).await?;
    write.write_all(b"\r\n").await?;
    write.flush().await?;
    Ok(())
}

async fn read_reply(read: &mut BufReader<OwnedReadHalf>) -> Result<(u16, String)> {
    let mut code: Option<u16> = None;
    let mut lines = Vec::new();
    loop {
        let mut line = String::new();
        let bytes = read.read_line(&mut line).await?;
        if bytes == 0 {
            bail!("SMTP connection closed unexpectedly");
        }
        if line.len() > 4096 {
            bail!("SMTP reply is too long");
        }
        let line = line.trim_end_matches(['\r', '\n']);
        if line.len() < 3 {
            bail!("invalid SMTP reply");
        }
        let parsed: u16 = line
            .get(..3)
            .ok_or_else(|| anyhow!("invalid SMTP reply code"))?
            .parse()
            .map_err(|_| anyhow!("invalid SMTP reply code"))?;
        match code {
            Some(expected) if expected != parsed => bail!("inconsistent SMTP reply code"),
            Some(_) => {}
            None => code = Some(parsed),
        }
        lines.push(line.get(3..).unwrap_or_default().trim().to_string());
        if !line.as_bytes().get(3).is_some_and(|byte| *byte == b'-') {
            break;
        }
    }
    Ok((code.unwrap_or(0), lines.join(" ")))
}

fn expect(reply: (u16, String), expected: &[u16], step: &str) -> Result<()> {
    if expected.contains(&reply.0) {
        Ok(())
    } else {
        bail!("SMTP {step} failed with code {}: {}", reply.0, reply.1)
    }
}

fn encode_header(value: &str) -> String {
    let clean: String = value
        .chars()
        .take(200)
        .map(|ch| if ch.is_control() { ' ' } else { ch })
        .collect();
    let clean = clean.trim();
    if clean.is_ascii() {
        clean.to_string()
    } else {
        format!("=?UTF-8?B?{}?=", STANDARD.encode(clean.as_bytes()))
    }
}

fn wrap_base64(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + value.len() / 76 * 2 + 2);
    for chunk in value.as_bytes().chunks(76) {
        out.push_str(std::str::from_utf8(chunk).unwrap_or_default());
        out.push_str("\r\n");
    }
    out
}

pub fn base_url(cfg: &ApiConfig) -> String {
    cfg.public_url
        .clone()
        .unwrap_or_else(|| "http://127.0.0.1:8080".into())
        .trim_end_matches('/')
        .to_string()
}

fn email_locale(locale: Option<&str>) -> &'static str {
    match locale.unwrap_or("es") {
        "es" => "es",
        "en" => "en",
        "pt" => "pt",
        "fr" => "fr",
        "ar" => "ar",
        "ru" => "ru",
        "zh" => "zh",
        "ja" => "ja",
        "it" => "it",
        "de" => "de",
        _ => "en",
    }
}

pub async fn send_credentials(
    state: &ApiState,
    to: &str,
    username: &str,
    password: &str,
    locale: Option<&str>,
) -> Result<()> {
    let Some(mailer) = state.mailer.as_deref() else {
        return Ok(());
    };
    let url = base_url(&state.cfg);
    let (subject, body) = match email_locale(locale) {
        "es" => (
            "Tu acceso al panel RYNXCP".to_string(),
            format!(
                "Se ha creado tu acceso al panel de control RYNXCP.\n\n\
                 Panel: {url}\n\
                 Usuario: {username}\n\
                 Contraseña: {password}\n\n\
                 Cambia la contraseña después del primer inicio de sesión.\n"
            ),
        ),
        "pt" => (
            "Seu acesso ao painel RYNXCP".to_string(),
            format!(
                "O acesso ao painel de controle RYNXCP foi criado.\n\n\
                 Painel: {url}\n\
                 Usuário: {username}\n\
                 Senha: {password}\n\n\
                 Altere a senha após o primeiro login.\n"
            ),
        ),
        "fr" => (
            "Votre accès au panneau RYNXCP".to_string(),
            format!(
                "Votre accès au panneau de contrôle RYNXCP a été créé.\n\n\
                 Panneau : {url}\n\
                 Utilisateur : {username}\n\
                 Mot de passe : {password}\n\n\
                 Changez votre mot de passe après votre première connexion.\n"
            ),
        ),
        "ar" => (
            "وصولك إلى لوحة RYNXCP".to_string(),
            format!(
                "تم إنشاء وصولك إلى لوحة التحكم RYNXCP.\n\n\
                 اللوحة: {url}\n\
                 المستخدم: {username}\n\
                 كلمة المرور: {password}\n\n\
                 غيّر كلمة المرور بعد أول تسجيل دخول.\n"
            ),
        ),
        "ru" => (
            "Ваш доступ к панели RYNXCP".to_string(),
            format!(
                "Доступ к панели управления RYNXCP создан.\n\n\
                 Панель: {url}\n\
                 Пользователь: {username}\n\
                 Пароль: {password}\n\n\
                 Смените пароль после первого входа.\n"
            ),
        ),
        "zh" => (
            "您的 RYNXCP 面板访问权限".to_string(),
            format!(
                "您的 RYNXCP 控制面板访问权限已创建。\n\n\
                 面板：{url}\n\
                 用户：{username}\n\
                 密码：{password}\n\n\
                 首次登录后请修改密码。\n"
            ),
        ),
        "ja" => (
            "RYNXCP パネルへのアクセス".to_string(),
            format!(
                "RYNXCP コントロールパネルへのアクセスが作成されました。\n\n\
                 パネル: {url}\n\
                 ユーザー: {username}\n\
                 パスワード: {password}\n\n\
                 初回ログイン後にパスワードを変更してください。\n"
            ),
        ),
        "it" => (
            "Il tuo accesso al pannello RYNXCP".to_string(),
            format!(
                "Il tuo accesso al pannello di controllo RYNXCP è stato creato.\n\n\
                 Pannello: {url}\n\
                 Utente: {username}\n\
                 Password: {password}\n\n\
                 Cambia la password dopo il primo accesso.\n"
            ),
        ),
        "de" => (
            "Ihr Zugang zum RYNXCP-Panel".to_string(),
            format!(
                "Ihr Zugang zum RYNXCP-Kontrollpanel wurde erstellt.\n\n\
                 Panel: {url}\n\
                 Benutzer: {username}\n\
                 Passwort: {password}\n\n\
                 Ändern Sie Ihr Passwort nach der ersten Anmeldung.\n"
            ),
        ),
        _ => (
            "Your RYNXCP panel access".to_string(),
            format!(
                "Your RYNXCP control panel access has been created.\n\n\
                 Panel: {url}\n\
                 User: {username}\n\
                 Password: {password}\n\n\
                 Change your password after your first login.\n"
            ),
        ),
    };
    mailer.send(to, &subject, &body).await
}

pub async fn send_password_reset(
    state: &ApiState,
    to: &str,
    username: &str,
    token: &str,
    locale: Option<&str>,
) -> Result<()> {
    let Some(mailer) = state.mailer.as_deref() else {
        return Ok(());
    };
    let link = format!("{}/#reset_token={token}", base_url(&state.cfg));
    let (subject, body) = match email_locale(locale) {
        "es" => (
            "Restablecer contraseña de RYNXCP".to_string(),
            format!(
                "Se solicitó restablecer la contraseña de la cuenta {username}.\n\n\
                 Abre este enlace para elegir una nueva contraseña:\n\n{link}\n\n\
                 El enlace caduca en 60 minutos y solo puede usarse una vez.\n\
                 Si no solicitaste el cambio, ignora este mensaje.\n"
            ),
        ),
        "pt" => (
            "Redefinir senha do RYNXCP".to_string(),
            format!(
                "Foi solicitada a redefinição da senha da conta {username}.\n\n\
                 Abra este link para escolher uma nova senha:\n\n{link}\n\n\
                 O link expira em 60 minutos e só pode ser usado uma vez.\n\
                 Se você não solicitou esta alteração, ignore esta mensagem.\n"
            ),
        ),
        "fr" => (
            "Réinitialiser votre mot de passe RYNXCP".to_string(),
            format!(
                "Une réinitialisation du mot de passe a été demandée pour le compte {username}.\n\n\
                 Ouvrez ce lien pour choisir un nouveau mot de passe :\n\n{link}\n\n\
                 Le lien expire dans 60 minutes et ne peut être utilisé qu’une seule fois.\n\
                 Si vous n’êtes pas à l’origine de cette demande, ignorez ce message.\n"
            ),
        ),
        "ar" => (
            "إعادة تعيين كلمة مرور RYNXCP".to_string(),
            format!(
                "تم طلب إعادة تعيين كلمة المرور للحساب {username}.\n\n\
                 افتح هذا الرابط لاختيار كلمة مرور جديدة:\n\n{link}\n\n\
                 ينتهي صلاحية الرابط خلال 60 دقيقة ويمكن استخدامه مرة واحدة فقط.\n\
                 إذا لم تطلب هذا التغيير، فتجاهل هذه الرسالة.\n"
            ),
        ),
        "ru" => (
            "Сброс пароля RYNXCP".to_string(),
            format!(
                "Запрошен сброс пароля для учётной записи {username}.\n\n\
                 Откройте эту ссылку, чтобы выбрать новый пароль:\n\n{link}\n\n\
                 Ссылка действует 60 минут и может быть использована только один раз.\n\
                 Если вы не запрашивали изменение, проигнорируйте это сообщение.\n"
            ),
        ),
        "zh" => (
            "重置 RYNXCP 密码".to_string(),
            format!(
                "已请求重置账户 {username} 的密码。\n\n\
                 打开此链接设置新密码：\n\n{link}\n\n\
                 链接将在 60 分钟后过期，且只能使用一次。\n\
                 如果您没有请求此更改，请忽略此邮件。\n"
            ),
        ),
        "ja" => (
            "RYNXCP パスワードのリセット".to_string(),
            format!(
                "アカウント {username} のパスワードリセットが要求されました。\n\n\
                 新しいパスワードを設定するには、次のリンクを開いてください:\n\n{link}\n\n\
                 リンクの有効期限は 60 分で、一度だけ使用できます。\n\
                 この変更を要求していない場合は、このメッセージを無視してください。\n"
            ),
        ),
        "it" => (
            "Reimposta la password di RYNXCP".to_string(),
            format!(
                "È stata richiesta la reimpostazione della password per l'account {username}.\n\n\
                 Apri questo link per scegliere una nuova password:\n\n{link}\n\n\
                 Il link scade tra 60 minuti e può essere usato una sola volta.\n\
                 Se non hai richiesto questa modifica, ignora questo messaggio.\n"
            ),
        ),
        "de" => (
            "RYNXCP-Passwort zurücksetzen".to_string(),
            format!(
                "Für das Konto {username} wurde das Zurücksetzen des Passworts angefordert.\n\n\
                 Öffnen Sie diesen Link, um ein neues Passwort zu wählen:\n\n{link}\n\n\
                 Der Link läuft nach 60 Minuten ab und kann nur einmal verwendet werden.\n\
                 Wenn Sie diese Änderung nicht angefordert haben, ignorieren Sie diese Nachricht.\n"
            ),
        ),
        _ => (
            "Reset your RYNXCP password".to_string(),
            format!(
                "A password reset was requested for the account {username}.\n\n\
                 Open this link to choose a new password:\n\n{link}\n\n\
                 The link expires in 60 minutes and can only be used once.\n\
                 If you did not request this change, ignore this message.\n"
            ),
        ),
    };
    mailer.send(to, &subject, &body).await
}

pub async fn send_login_code(
    state: &ApiState,
    to: &str,
    username: &str,
    code: &str,
    locale: Option<&str>,
) -> Result<()> {
    let mailer = state
        .mailer
        .as_deref()
        .ok_or_else(|| anyhow!("SMTP is not configured for two-factor login"))?;
    let (subject, body) = match email_locale(locale) {
        "es" => (
            "Código de acceso a RYNXCP".to_string(),
            format!(
                "Hola {username},\n\n\
                 Tu código de verificación para acceder a RYNXCP es:\n\n\
                     {code}\n\n\
                 Caduca en 10 minutos. Si no has intentado iniciar sesión, \
                 cambia tu contraseña y revisa los accesos al panel.\n"
            ),
        ),
        "pt" => (
            "Código de acesso ao RYNXCP".to_string(),
            format!(
                "Olá {username},\n\n\
                 Seu código de verificação para acessar o RYNXCP é:\n\n\
                     {code}\n\n\
                 Ele expira em 10 minutos. Se você não tentou entrar, \
                 altere sua senha e revise os acessos ao painel.\n"
            ),
        ),
        "fr" => (
            "Votre code de connexion RYNXCP".to_string(),
            format!(
                "Bonjour {username},\n\n\
                 Votre code de vérification pour accéder à RYNXCP est :\n\n\
                     {code}\n\n\
                 Il expire dans 10 minutes. Si vous n’avez pas tenté de vous connecter, \
                 changez votre mot de passe et vérifiez les accès au panneau.\n"
            ),
        ),
        "ar" => (
            "رمز الدخول إلى RYNXCP".to_string(),
            format!(
                "مرحبًا {username}،\n\n\
                 رمز التحقق الخاص بك للدخول إلى RYNXCP هو:\n\n\
                     {code}\n\n\
                 تنتهي صلاحيته خلال 10 دقائق. إذا لم تحاول تسجيل الدخول، \
                 فغيّر كلمة المرور وراجع عمليات الوصول إلى اللوحة.\n"
            ),
        ),
        "ru" => (
            "Код входа в RYNXCP".to_string(),
            format!(
                "Здравствуйте, {username}!\n\n\
                 Ваш код подтверждения для входа в RYNXCP:\n\n\
                     {code}\n\n\
                 Он действует 10 минут. Если вы не пытались войти, \
                 смените пароль и проверьте доступ к панели.\n"
            ),
        ),
        "zh" => (
            "您的 RYNXCP 登录验证码".to_string(),
            format!(
                "您好 {username}：\n\n\
                 您登录 RYNXCP 的验证码是：\n\n\
                     {code}\n\n\
                 验证码将在 10 分钟后过期。如果您没有尝试登录，\
                 请修改密码并检查面板访问记录。\n"
            ),
        ),
        "ja" => (
            "RYNXCP ログインコード".to_string(),
            format!(
                "{username} さん\n\n\
                 RYNXCP にアクセスするための確認コードは次のとおりです:\n\n\
                     {code}\n\n\
                 有効期限は 10 分です。ログインを試みていない場合は、\
                 パスワードを変更し、パネルへのアクセスを確認してください。\n"
            ),
        ),
        "it" => (
            "Il tuo codice di accesso RYNXCP".to_string(),
            format!(
                "Ciao {username},\n\n\
                 Il tuo codice di verifica per accedere a RYNXCP è:\n\n\
                     {code}\n\n\
                 Scade tra 10 minuti. Se non hai tentato di accedere, \
                 cambia la password e controlla gli accessi al pannello.\n"
            ),
        ),
        "de" => (
            "Ihr RYNXCP-Anmeldecode".to_string(),
            format!(
                "Hallo {username},\n\n\
                 Ihr Bestätigungscode für den Zugriff auf RYNXCP lautet:\n\n\
                     {code}\n\n\
                 Er läuft in 10 Minuten ab. Wenn Sie sich nicht anmelden wollten, \
                 ändern Sie Ihr Passwort und prüfen Sie die Panel-Zugriffe.\n"
            ),
        ),
        _ => (
            "Your RYNXCP login code".to_string(),
            format!(
                "Hello {username},\n\n\
                 Your verification code to access RYNXCP is:\n\n\
                     {code}\n\n\
                 It expires in 10 minutes. If you did not try to sign in, \
                 change your password and review panel access.\n"
            ),
        ),
    };
    mailer.send(to, &subject, &body).await
}

pub fn test_email_text(locale: Option<&str>, host: &str, port: u16, tls: &str) -> (String, String) {
    match email_locale(locale) {
        "es" => (
            "Prueba de correo de RYNXCP".into(),
            format!(
                "Este es un correo de prueba de RYNXCP.\n\n\
                 Si lo has recibido, el panel puede enviar credenciales, avisos y enlaces \
                 de restablecimiento de contraseña.\n\n\
                 Servidor SMTP local: {host}:{port} (TLS: {tls}).\n"
            ),
        ),
        "pt" => (
            "Teste de e-mail do RYNXCP".into(),
            format!(
                "Este é um e-mail de teste do RYNXCP.\n\n\
                 Se você o recebeu, o painel pode enviar credenciais, avisos e links \
                 de redefinição de senha.\n\n\
                 Servidor SMTP local: {host}:{port} (TLS: {tls}).\n"
            ),
        ),
        "fr" => (
            "Test e-mail RYNXCP".into(),
            format!(
                "Ceci est un e-mail de test RYNXCP.\n\n\
                 Si vous l’avez reçu, le panneau peut envoyer des identifiants, des alertes \
                 et des liens de réinitialisation de mot de passe.\n\n\
                 Serveur SMTP local : {host}:{port} (TLS : {tls}).\n"
            ),
        ),
        "ar" => (
            "اختبار البريد RYNXCP".into(),
            format!(
                "هذه رسالة اختبار من RYNXCP.\n\n\
                 إذا وصلتك، فإن اللوحة تستطيع إرسال بيانات الدخول والتنبيهات \
                 وروابط إعادة تعيين كلمة المرور.\n\n\
                 خادم SMTP المحلي: {host}:{port} (TLS: {tls}).\n"
            ),
        ),
        "ru" => (
            "Тест почты RYNXCP".into(),
            format!(
                "Это тестовое письмо RYNXCP.\n\n\
                 Если вы его получили, панель может отправлять учётные данные, уведомления \
                 и ссылки для сброса пароля.\n\n\
                 Локальный SMTP-сервер: {host}:{port} (TLS: {tls}).\n"
            ),
        ),
        "zh" => (
            "RYNXCP 邮件测试".into(),
            format!(
                "这是一封 RYNXCP 测试邮件。\n\n\
                 如果您收到此邮件，说明面板可以发送凭据、通知和密码重置链接。\n\n\
                 本地 SMTP 服务器：{host}:{port}（TLS：{tls}）。\n"
            ),
        ),
        "ja" => (
            "RYNXCP メールテスト".into(),
            format!(
                "これは RYNXCP のテストメールです。\n\n\
                 受信できていれば、パネルは認証情報、通知、パスワードリセットリンクを送信できます。\n\n\
                 ローカル SMTP サーバー: {host}:{port}（TLS: {tls}）。\n"
            ),
        ),
        "it" => (
            "Test email RYNXCP".into(),
            format!(
                "Questa è un'email di test RYNXCP.\n\n\
                 Se l'hai ricevuta, il pannello può inviare credenziali, avvisi e link \
                 per reimpostare la password.\n\n\
                 Server SMTP locale: {host}:{port} (TLS: {tls}).\n"
            ),
        ),
        "de" => (
            "RYNXCP-E-Mail-Test".into(),
            format!(
                "Dies ist eine Test-E-Mail von RYNXCP.\n\n\
                 Wenn Sie sie erhalten haben, kann das Panel Anmeldedaten, Hinweise und \
                 Links zum Zurücksetzen des Passworts senden.\n\n\
                 Lokaler SMTP-Server: {host}:{port} (TLS: {tls}).\n"
            ),
        ),
        _ => (
            "RYNXCP email test".into(),
            format!(
                "This is a RYNXCP test email.\n\n\
                 If you received it, the panel can send credentials, alerts and password \
                 reset links.\n\n\
                 Local SMTP server: {host}:{port} (TLS: {tls}).\n"
            ),
        ),
    }
}

pub fn certificate_failure_text(
    locale: Option<&str>,
    domain: &str,
    message: &str,
) -> (String, String) {
    match email_locale(locale) {
        "es" => (
            format!("[RYNXCP] Falló el certificado de {domain}"),
            format!(
                "La emisión del certificado TLS para {domain} ha fallado.\n\n\
                 Detalle: {message}\n\n\
                 Revisa Certificados en el panel y los logs de rynxd.\n"
            ),
        ),
        "pt" => (
            format!("[RYNXCP] Falha no certificado de {domain}"),
            format!(
                "Não foi possível emitir o certificado TLS para {domain}.\n\n\
                 Detalhe: {message}\n\n\
                 Verifique Certificados no painel e os logs do rynxd.\n"
            ),
        ),
        "fr" => (
            format!("[RYNXCP] Échec du certificat pour {domain}"),
            format!(
                "L’émission du certificat TLS pour {domain} a échoué.\n\n\
                 Détail : {message}\n\n\
                 Vérifiez Certificats dans le panneau et les journaux de rynxd.\n"
            ),
        ),
        "ar" => (
            format!("[RYNXCP] فشل شهادة {domain}"),
            format!(
                "تعذّر إصدار شهادة TLS للنطاق {domain}.\n\n\
                 التفاصيل: {message}\n\n\
                 راجع الشهادات في اللوحة وسجلات rynxd.\n"
            ),
        ),
        "ru" => (
            format!("[RYNXCP] Ошибка сертификата для {domain}"),
            format!(
                "Не удалось выпустить TLS-сертификат для {domain}.\n\n\
                 Подробности: {message}\n\n\
                 Проверьте раздел «Сертификаты» в панели и журналы rynxd.\n"
            ),
        ),
        "zh" => (
            format!("[RYNXCP] {domain} 的证书失败"),
            format!(
                "无法为 {domain} 签发 TLS 证书。\n\n\
                 详情：{message}\n\n\
                 请检查面板中的证书和 rynxd 日志。\n"
            ),
        ),
        "ja" => (
            format!("[RYNXCP] {domain} の証明書エラー"),
            format!(
                "{domain} の TLS 証明書を発行できませんでした。\n\n\
                 詳細: {message}\n\n\
                 パネルの「証明書」と rynxd のログを確認してください。\n"
            ),
        ),
        "it" => (
            format!("[RYNXCP] Errore certificato per {domain}"),
            format!(
                "Impossibile emettere il certificato TLS per {domain}.\n\n\
                 Dettaglio: {message}\n\n\
                 Controlla Certificati nel pannello e i log di rynxd.\n"
            ),
        ),
        "de" => (
            format!("[RYNXCP] Zertifikat für {domain} fehlgeschlagen"),
            format!(
                "Das TLS-Zertifikat für {domain} konnte nicht ausgestellt werden.\n\n\
                 Details: {message}\n\n\
                 Prüfen Sie „Zertifikate“ im Panel und die rynxd-Protokolle.\n"
            ),
        ),
        _ => (
            format!("[RYNXCP] Certificate failed for {domain}"),
            format!(
                "The TLS certificate for {domain} could not be issued.\n\n\
                 Detail: {message}\n\n\
                 Check Certificates in the panel and the rynxd logs.\n"
            ),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> ApiConfig {
        ApiConfig {
            smtp_enabled: true,
            smtp_from: Some("noreply@example.com".into()),
            ..ApiConfig::default()
        }
    }

    #[test]
    fn disabled_mailer_is_none() {
        let cfg = ApiConfig::default();
        assert!(Mailer::from_config(&cfg).unwrap().is_none());
    }

    #[test]
    fn validates_sender_domain() {
        let mut cfg = config();
        cfg.smtp_from = Some("noreply".into());
        assert!(Mailer::from_config(&cfg).is_err());
    }

    #[test]
    fn encodes_non_ascii_subjects() {
        let encoded = encode_header("Aviso de contraseña\r\nBcc: evil@example.com");
        assert!(!encoded.contains('\r'));
        assert!(!encoded.contains('\n'));
        assert!(encoded.starts_with("=?UTF-8?B?"));
    }

    #[test]
    fn localizes_test_email() {
        let (subject, body) = test_email_text(Some("fr"), "127.0.0.1", 25, "none");
        assert_eq!(subject, "Test e-mail RYNXCP");
        assert!(body.contains("Ceci est un e-mail de test RYNXCP."));
        let (subject, body) = test_email_text(None, "127.0.0.1", 25, "none");
        assert_eq!(subject, "Prueba de correo de RYNXCP");
        assert!(body.contains("Este es un correo de prueba"));
    }

    #[test]
    fn renders_safe_headers_and_base64_body() {
        let mailer = Mailer::from_config(&config()).unwrap().unwrap();
        let message = mailer.render("admin@example.com", "Hola", "línea 1\nlínea 2");
        assert!(message.contains("To: <admin@example.com>\r\n"));
        assert!(message.contains("Content-Transfer-Encoding: base64\r\n"));
        assert!(message.ends_with("\r\n"));
        assert!(!message.contains("línea 1"));
    }

    #[tokio::test]
    async fn delivers_to_a_local_smtp_server() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let (read, mut write) = stream.into_split();
            let mut read = BufReader::new(read);
            write.write_all(b"220 test\r\n").await.unwrap();
            let mut transcript = String::new();
            let mut in_data = false;
            loop {
                let mut line = String::new();
                if read.read_line(&mut line).await.unwrap() == 0 {
                    break;
                }
                transcript.push_str(&line);
                let upper = line.trim_end().to_ascii_uppercase();
                if in_data {
                    if upper == "." {
                        in_data = false;
                        write.write_all(b"250 queued\r\n").await.unwrap();
                    }
                    continue;
                }
                let response = if upper.starts_with("EHLO") {
                    "250-test\r\n250 OK\r\n"
                } else if upper == "DATA" {
                    in_data = true;
                    "354 go\r\n"
                } else if upper.starts_with("QUIT") {
                    "221 bye\r\n"
                } else {
                    "250 OK\r\n"
                };
                write.write_all(response.as_bytes()).await.unwrap();
                if upper.starts_with("QUIT") {
                    break;
                }
            }
            let _ = tx.send(transcript);
        });

        let mailer = Mailer {
            host: "127.0.0.1".into(),
            port,
            from: "noreply@example.com".into(),
            helo: "example.com".into(),
        };
        mailer
            .send("admin@example.com", "Prueba", "hola")
            .await
            .unwrap();
        let transcript = rx.recv().await.unwrap();
        assert!(transcript.contains("EHLO example.com"));
        assert!(transcript.contains("MAIL FROM:<noreply@example.com>"));
        assert!(transcript.contains("RCPT TO:<admin@example.com>"));
        assert!(transcript.contains("Content-Type: text/plain"));
    }
}
