(function () {
  const LANGUAGES = [
    { code: 'en', label: 'English (US)' },
    { code: 'es', label: 'Español' },
    { code: 'pt', label: 'Português' },
    { code: 'fr', label: 'Français' },
    { code: 'ar', label: 'العربية' },
    { code: 'ru', label: 'Русский' },
    { code: 'zh', label: '中文' },
    { code: 'ja', label: '日本語' },
    { code: 'it', label: 'Italiano' },
    { code: 'de', label: 'Deutsch' },
  ];

  const EN = {
    "Cargando…": "Loading…",
    "Error": "Error",
    "Hecho": "Done",
    "La sesión ha expirado": "Your session has expired",
    "Actualizar": "Refresh",
    "Refrescar": "Refresh",
    "Cerrar": "Close",
    "Cancelar": "Cancel",
    "Guardar": "Save",
    "Ejecutar": "Run",
    "Ver": "Show",
    "Eliminar": "Delete",
    "Suspender": "Suspend",
    "Activar": "Activate",
    "Entrar": "Sign in",
    "Crear acceso": "Create access",
    "Volver a mi cuenta": "Back to my account",
    "Salir": "Sign out",
    "Mi cuenta": "My account",
    "cliente": "client",
    "conectando": "connecting",
    "producción": "production",
    "simulación": "simulation",
    "Control de hosting": "Hosting control",
    "RYNXCP — Panel de control": "RYNXCP — Control panel",
    "Usuario": "Username",
    "Contraseña": "Password",
    "Sesión protegida con cookies HttpOnly, CSRF y Argon2id.":
      "Session protected with HttpOnly cookies, CSRF and Argon2id.",
    "Operación": "Operations",
    "Administración": "Administration",
    "Resumen": "Overview",
    "Cuentas": "Accounts",
    "Sitios web": "Websites",
    "Aplicaciones": "Applications",
    "Bases de datos": "Databases",
    "Certificados": "Certificates",
    "Backups": "Backups",
    "Monitor": "Monitoring",
    "Usuarios": "Users",
    "Seguridad": "Security",
    "Cortafuegos": "Firewall",
    "Programas": "Programs",
    "Consola": "Console",
    "Auditoría": "Audit",
    "Sistema": "System",
    "Nueva cuenta": "New account",
    "Nuevo sitio": "New website",
    "Nueva aplicación": "New application",
    "Nuevo pool": "New pool",
    "Nueva base": "New database",
    "Nuevo usuario FTP": "New FTP user",
    "Nuevo usuario": "New user",
    "Emitir certificado": "Issue certificate",
    "Renovar todos": "Renew all",
    "Crear backup": "Create backup",
    "Restaurar": "Restore",
    "Cuenta": "Account",
    "Dominio": "Domain",
    "Estado": "Status",
    "Email": "Email",
    "Rol": "Role",
    "Versión": "Version",
    "activo": "active",
    "deshabilitado": "disabled",
    "suspendida": "suspended",
    "solo lectura": "read-only",
    "Cuenta creada": "Account created",
    "Sitio publicado": "Website published",
    "Aplicación creada": "Application created",
    "Usuario creado": "User created",
    "Usuario eliminado": "User deleted",
    "Backup creado": "Backup created",
    "Sesión de administrador restaurada": "Administrator session restored",
    "Cuenta y acceso creados": "Account and access created",
    "Acceso al panel creado": "Panel access created",
    "No puedes desactivar tu propia cuenta": "You cannot deactivate your own account",
    "Estás viendo el panel como": "You are viewing the panel as",
    "Sin registros": "No records",
    "Sin registros todavía.": "No records yet.",
    "Sin actividad registrada.": "No activity recorded.",
    "No se pudo cargar la vista": "The view could not be loaded",
    "Crea la primera cuenta para empezar.": "Create the first account to get started.",
    "Aún no hay virtual hosts.": "No virtual hosts yet.",
    "El sistema está actualizado.": "The system is up to date.",
    "No se detectó software.": "No software detected.",
    "Cuenta no encontrada": "Account not found",
    "Cuenta suspendida": "Account suspended",
    "Cuenta activada": "Account activated",
    "Cuenta eliminada": "Account deleted",
    "Sitio eliminado": "Website deleted",
    "Aplicación eliminada": "Application deleted",
    "Pool eliminado": "Pool deleted",
    "Base de datos eliminada": "Database deleted",
    "Usuario FTP eliminado": "FTP user deleted",
    "Contraseña actualizada": "Password updated",
    "Registro eliminado": "Record deleted",
    "Backup eliminado": "Backup deleted",
    "Backup restaurado": "Backup restored",
    "El acceso es de solo lectura (viewer).": "Access is read-only (viewer).",
    "El acceso ya existía": "Access already existed",
    "Ahora viendo el panel como": "Now viewing the panel as",
    "Añadir regla": "Add rule",
    "Banear IP": "Ban IP",
    "Añadir IP": "Add IP",
    "Provisionar correo": "Provision mail",
    "Añadir registro": "Add record",
    "Sin TLS": "No TLS",
    "Let's Encrypt": "Let's Encrypt",
    "Certificado manual": "Manual certificate",
    "Sitio activo": "Site active",
    "— sin pool —": "— no pool —",
    "— selecciona —": "— select —",
    "— no aplica —": "— not applicable —",
    "— sin aplicaciones compatibles —": "— no compatible applications —",
    "Arrancar aplicación": "Start application",
    "Variables de entorno": "Environment variables",
    "una KEY=VALUE por línea": "one KEY=VALUE per line",
    "Puerto interno": "Internal port",
    "Comando": "Command",
    "Cuota de disco (MB)": "Disk quota (MB)",
    "PHP por defecto": "Default PHP",
    "Cuenta activa": "Account active",
    "Acceso al panel del cliente": "Client panel access",
    "Solo lectura (viewer)": "Read-only (viewer)",
    "Operador (gestiona su cuenta)": "Operator (manages their account)",
    "Sin acceso al panel": "No panel access",
    "El operador puede gestionar los recursos de su propia cuenta; el viewer es solo lectura.":
      "The operator can manage their own account's resources; the viewer is read-only.",
    "Nueva cuenta de hosting": "New hosting account",
    "Crear cuenta": "Create account",
    "Usuario del sistema": "System user",
    "Document root": "Document root",
    "Nuevo sitio web": "New website",
    "Publicar sitio": "Publish site",
    "Crear aplicación": "Create application",
    "Nuevo pool PHP-FPM": "New PHP-FPM pool",
    "Crear pool": "Create pool",
    "Versión PHP": "PHP version",
    "Gestor de procesos": "Process manager",
    "Memoria por worker (MB)": "Memory per worker (MB)",
    "Subida máxima (MB)": "Max upload (MB)",
    "Pool activo": "Pool active",
    "Email de contacto": "Contact email",
    "Usar entorno staging (pruebas)": "Use staging environment (tests)",
    "Nuevo usuario del panel": "New panel user",
    "Crear usuario": "Create user",
    "mínimo 12 caracteres, 3 clases": "minimum 12 characters, 3 classes",
    "Nueva contraseña": "New password",
    "Usuario desactivado": "User deactivated",
    "Usuario activado": "User activated",
    "Estado re-aplicado": "State re-applied",
    "Certificados renovados": "Certificates renewed",
    "Sin salida.": "No output.",
    "Recopilando datos…": "Collecting data…",
    "CPU media": "Average CPU",
    "actualizado cada 60 s": "updated every 60 s",
    "Sin eventos.": "No events.",
    "Sin eventos de autenticación.": "No authentication events.",
    "No hay IPs baneadas.": "No banned IPs.",
    "Sin direcciones privilegiadas.": "No privileged addresses.",
    "Sin reglas personalizadas.": "No custom rules.",
    "usuario del panel:": "panel user:",
    "usuario:": "user:",
    "contraseña:": "password:",
    "Entra en": "Sign in at",
    "con esas credenciales.": "with those credentials.",
    "Guarda la contraseña ahora; no se volverá a mostrar.":
      "Save the password now; it will not be shown again.",
    "Sitio estático": "Static site",
    "PHP (FPM)": "PHP (FPM)",
    "Node.js (systemd)": "Node.js (systemd)",
    "Python (WSGI/ASGI)": "Python (WSGI/ASGI)",
    "Ruby (Puma)": "Ruby (Puma)",
    "Java (JAR)": "Java (JAR)",
    "Go (binario compilado)": "Go (compiled binary)",
    ".NET (systemd)": ".NET (systemd)",
    "activa": "active",
    "detenida": "stopped",
    "sin tls": "no TLS",
    "manual": "manual",
    "let's encrypt": "Let's Encrypt",
    "preparado": "ready",
    "pendiente": "pending",
    "auto": "auto",
    "simulated": "simulated",
    "Nombre": "Name",
    "Tipo": "Type",
    "Valor": "Value",
    "Fecha": "Date",
    "Ruta": "Path",
    "Permisos": "Permissions",
    "Login": "Login",
    "Home": "Home",
    "Memoria": "Memory",
    "Disco": "Disk",
    "Servicio": "Service",
    "Emisor": "Issuer",
    "Vence": "Expires",
    "Creada": "Created",
    "Creado": "Created",
    "No hay pools; se crean automáticamente al publicar un sitio PHP.":
      "No pools yet; they are created automatically when you publish a PHP site.",
    "No hay certificados; publica un sitio con TLS Let's Encrypt.":
      "No certificates yet; publish a site with Let's Encrypt TLS.",
    "Publica un sitio y crea su base de datos aislada.":
      "Publish a site and create its isolated database.",
    "Crea usuarios FTPS con chroot al document root del sitio.":
      "Create FTPS users chrooted to the site document root.",
    "Sin zonas DNS": "No DNS zones",
    "Aún no hay copias. Se conservan las últimas 7 por cuenta.":
      "No backups yet. The last 7 per account are kept.",
    "No hay servicios registrados.": "No services registered.",
    "Solo existe el propietario inicial.": "Only the initial owner exists.",
    "Pool PHP actualizado": "PHP pool updated",
    "Certificado emitido": "Certificate issued",
    "IP baneada": "IP banned",
    "IP desbloqueada": "IP unblocked",
    "Regla aplicada": "Rule applied",
    "Regla eliminada": "Rule deleted",
    "Estado del nodo y de los recursos gestionados": "Node status and managed resources",
    "Usuarios Linux aislados con cuota y home dedicado": "Isolated Linux users with quota and dedicated home",
    "Virtual hosts Nginx con TLS y reverse proxy": "Nginx virtual hosts with TLS and reverse proxy",
    "Procesos gestionados por systemd (Node, Python, Ruby, Java, .NET, Go)": "systemd-managed processes (Node, Python, Ruby, Java, .NET, Go)",
    "Pools por cuenta con open_basedir y funciones restringidas": "Per-account pools with open_basedir and restricted functions",
    "MariaDB aislada por dominio, con usuario y contraseña propios": "Per-domain MariaDB with its own user and password",
    "Usuarios virtuales Pure-FTPd con chroot al document root": "Pure-FTPd virtual users chrooted to the document root",
    "Zonas autoritativas con SPF, DKIM, DMARC, MX y CAA": "Authoritative zones with SPF, DKIM, DMARC, MX and CAA",
    "Emisión y estado de certificados Let's Encrypt": "Let's Encrypt certificate issuance and status",
    "Copias de home, vhosts, pools y unidades con retención": "Home, vhost, pool and unit backups with retention",
    "Series históricas y estado de cada servicio": "Historical series and per-service status",
    "Acceso al panel con roles y sesiones revocables": "Panel access with roles and revocable sessions",
    "Intentos de acceso, bloqueos y cadena de auditoría": "Login attempts, lockouts and audit chain",
    "Reglas nftables, IPs baneadas y listas blancas": "nftables rules, banned IPs and allowlists",
    "Software instalado y actualizaciones del sistema": "Installed software and system updates",
    "Acciones privilegiadas auditadas y sin shell libre": "Audited privileged actions with no free shell",
    "Información del nodo y reconciliación de estado": "Node information and state reconciliation",
    "Actividad reciente": "Recent activity",
    "Registro encadenado": "Chained log",
    "Eventos de autenticación": "Authentication events",
    "Software instalado": "Installed software",
    "Actualizaciones disponibles": "Available updates",
    "IPs baneadas": "Banned IPs",
    "Lista blanca": "Allowlist",
    "Certificados TLS": "TLS certificates",
    "Pools PHP-FPM": "PHP-FPM pools",
    "Usuarios FTP": "FTP users",
    "Bases de datos por dominio": "Databases per domain",
    "Copias de seguridad": "Backups",
    "Cuentas de hosting": "Hosting accounts",
    "Usuarios del panel": "Panel users",
    "Registro DNS guardado": "DNS record saved",
    "Protocolo CBOR v1": "CBOR protocol v1",
    "Tamaño": "Size",
    "Escucha": "Listening",
    "Motor": "Engine",
    "Unidad": "Unit",
    "Origen": "Source",
    "Programa": "Program",
    "Sitio": "Site",
    "Cuota": "Quota",
    "Ese email ya está en uso por otra cuenta de hosting.":
      "That email is already used by another hosting account.",
    "Ese email ya está en uso por un usuario del panel. Usa otro email para la cuenta.":
      "That email is already used by a panel user. Use another email for the account.",
    "Ese nombre de usuario ya está en uso por un usuario del panel.":
      "That username is already used by a panel user.",
    "Ese email ya está en uso por un usuario del panel.":
      "That email is already used by a panel user.",
    "Certificados que requieren atención": "Certificates need attention",
    "Revisar certificados": "Review certificates",
    "Certificado SSL": "SSL certificate",
    "Solicitando certificado...": "Requesting certificate...",
    "HTTPS activo": "HTTPS active",
    "Certificado emitido correctamente": "Certificate issued successfully",
    "No se pudo emitir el certificado": "Could not issue the certificate",
    "El sitio se creó, pero HTTPS no está activo.": "The site was created, but HTTPS is not active.",
    "Reintentar": "Retry",
    "Tiempo transcurrido": "Elapsed time",
    "Comprobando estado...": "Checking status...",
    "El proceso continúa en segundo plano.": "The process continues in the background.",
    "Correo": "Mail",
    "Dominios de correo": "Mail domains",
    "Servidor": "Server",
    "Webmail": "Webmail",
    "Activar correo": "Enable mail",
    "Correo activado": "Mail enabled",
    "Correo desactivado": "Mail disabled",
    "Correo eliminado": "Mail deleted",
    "Sin correo": "No mail",
    "Buzones": "Mailboxes",
    "Buzón": "Mailbox",
    "Nuevo buzón": "New mailbox",
    "Buzón creado": "Mailbox created",
    "Buzón eliminado": "Mailbox deleted",
    "Buzón no encontrado": "Mailbox not found",
    "Buzón activado": "Mailbox enabled",
    "Buzón desactivado": "Mailbox disabled",
    "Alias locales": "Local aliases",
    "Alias / reenvío": "Aliases / forwarding",
    "Reenviar a": "Forward to",
    "Buzón activo": "Mailbox active",
    "Catch-all": "Catch-all",
    "Destino": "Destination",
    "Catch-all actualizado": "Catch-all updated",
    "Todos los sitios ya tienen correo activado": "Every site already has mail enabled",
    "Sitio no encontrado": "Site not found",
    "¿Olvidaste tu contraseña?": "Forgot your password?",
    "Escribe el email de tu cuenta. Si existe, recibirás un enlace para restablecer la contraseña.":
      "Enter your account email. If it exists, you will receive a password reset link.",
    "Enviar enlace": "Send link",
    "Volver": "Back",
    "Repite la contraseña": "Repeat password",
    "Cambiar contraseña": "Change password",
    "Elige una contraseña nueva. El enlace caduca en 60 minutos y solo puede usarse una vez.":
      "Choose a new password. The link expires in 60 minutes and can only be used once.",
    "Las contraseñas no coinciden": "Passwords do not match",
    "Contraseña actualizada; ya puedes iniciar sesión": "Password updated; you can now sign in",
    "Si el email existe, recibirás un enlace para restablecer la contraseña.":
      "If the email exists, you will receive a password reset link.",
    "Correo del panel": "Panel email",
    "Servidor SMTP": "SMTP server",
    "Remitente": "Sender",
    "Avisos": "Alerts",
    "URL pública": "Public URL",
    "Enviar prueba": "Send test",
    "Probar correo del panel": "Test panel email",
    "Destinatario": "Recipient",
    "Vacío usa smtp_alert_to o tu email de usuario.":
      "Empty uses smtp_alert_to or your user email.",
    "Se usa Postfix local para credenciales, restablecimiento de contraseña y avisos de certificados. SPF, DKIM y DMARC se publican en la zona del dominio del panel.":
      "Local Postfix is used for credentials, password resets and certificate alerts. SPF, DKIM and DMARC are published in the panel domain zone.",
    "Configura smtp_enabled y smtp_from en /etc/rynxcp/api.toml.":
      "Configure smtp_enabled and smtp_from in /etc/rynxcp/api.toml.",
  };

  const PT = {
    "Cargando…": "Carregando…", "Error": "Erro", "Hecho": "Concluído",
    "La sesión ha expirado": "Sua sessão expirou", "Actualizar": "Atualizar",
    "Cerrar": "Fechar", "Cancelar": "Cancelar", "Guardar": "Guardar",
    "Ejecutar": "Executar", "Ver": "Ver", "Eliminar": "Excluir",
    "Suspender": "Suspender", "Activar": "Ativar", "Entrar": "Entrar",
    "Volver a mi cuenta": "Voltar à minha conta", "Salir": "Sair",
    "Mi cuenta": "Minha conta", "Control de hosting": "Controle de hospedagem",
    "Usuario": "Usuário", "Contraseña": "Senha", "Operação": "Operação",
    "Administración": "Administração", "Resumen": "Visão geral",
    "Cuentas": "Contas", "Sitios web": "Sites", "Aplicaciones": "Aplicações",
    "Bases de datos": "Bancos de dados", "Certificados": "Certificados",
    "Backups": "Backups", "Monitor": "Monitoramento", "Usuarios": "Usuários",
    "Seguridad": "Segurança", "Cortafuegos": "Firewall", "Programas": "Programas",
    "Consola": "Console", "Auditoría": "Auditoria", "Sistema": "Sistema",
    "Nueva cuenta": "Nova conta", "Nuevo sitio": "Novo site",
    "Nueva aplicación": "Nova aplicação", "Nuevo pool": "Novo pool",
    "Nueva base": "Novo banco de dados", "Nuevo usuario FTP": "Novo usuário FTP",
    "Nuevo usuario": "Novo usuário", "Emitir certificado": "Emitir certificado",
    "Renovar todos": "Renovar todos", "Crear backup": "Criar backup",
    "Restaurar": "Restaurar", "Cuenta": "Conta", "Dominio": "Domínio",
    "Estado": "Estado", "Email": "Email", "Rol": "Função", "Versión": "Versão",
    "Certificados que requieren atención": "Certificados que precisam de atenção",
    "Revisar certificados": "Revisar certificados",
    "Certificado SSL": "Certificado SSL",
    "Solicitando certificado...": "Solicitando certificado...",
    "HTTPS activo": "HTTPS ativo",
    "Certificado emitido correctamente": "Certificado emitido com sucesso",
    "No se pudo emitir el certificado": "Não foi possível emitir o certificado",
    "El sitio se creó, pero HTTPS no está activo.": "O site foi criado, mas o HTTPS não está ativo.",
    "Reintentar": "Tentar novamente",
    "Tiempo transcurrido": "Tempo decorrido",
    "Comprobando estado...": "Verificando status...",
    "El proceso continúa en segundo plano.": "O processo continua em segundo plano.",
  };

  const FR = {
    "Cargando…": "Chargement…", "Error": "Erreur", "Hecho": "Terminé",
    "La sesión ha expirado": "Votre session a expiré", "Actualizar": "Actualiser",
    "Cerrar": "Fermer", "Cancelar": "Annuler", "Guardar": "Enregistrer",
    "Ejecutar": "Exécuter", "Ver": "Afficher", "Eliminar": "Supprimer",
    "Suspender": "Suspendre", "Activar": "Activer", "Entrar": "Se connecter",
    "Volver a mi cuenta": "Retour à mon compte", "Salir": "Se déconnecter",
    "Mi cuenta": "Mon compte", "Control de hosting": "Gestion d'hébergement",
    "Usuario": "Nom d'utilisateur", "Contraseña": "Mot de passe",
    "Operación": "Opérations", "Administración": "Administration",
    "Resumen": "Aperçu", "Cuentas": "Comptes", "Sitios web": "Sites web",
    "Aplicaciones": "Applications", "Bases de datos": "Bases de données",
    "Certificados": "Certificats", "Backups": "Sauvegardes",
    "Monitor": "Surveillance", "Usuarios": "Utilisateurs",
    "Seguridad": "Sécurité", "Cortafuegos": "Pare-feu", "Programas": "Programmes",
    "Consola": "Console", "Auditoría": "Audit", "Sistema": "Système",
    "Nueva cuenta": "Nouveau compte", "Nuevo sitio": "Nouveau site",
    "Nueva aplicación": "Nouvelle application", "Nuevo pool": "Nouveau pool",
    "Nueva base": "Nouvelle base de données", "Nuevo usuario FTP": "Nouvel utilisateur FTP",
    "Nuevo usuario": "Nouvel utilisateur", "Emitir certificado": "Émettre un certificat",
    "Renovar todos": "Tout renouveler", "Crear backup": "Créer une sauvegarde",
    "Restaurar": "Restaurer", "Cuenta": "Compte", "Dominio": "Domaine",
    "Estado": "État", "Email": "E-mail", "Rol": "Rôle", "Versión": "Version",
    "Certificados que requieren atención": "Certificats à vérifier",
    "Revisar certificados": "Vérifier les certificats",
    "Certificado SSL": "Certificat SSL",
    "Solicitando certificado...": "Demande du certificat...",
    "HTTPS activo": "HTTPS actif",
    "Certificado emitido correctamente": "Certificat émis avec succès",
    "No se pudo emitir el certificado": "Impossible d'émettre le certificat",
    "El sitio se creó, pero HTTPS no está activo.": "Le site a été créé, mais HTTPS n'est pas actif.",
    "Reintentar": "Réessayer",
    "Tiempo transcurrido": "Temps écoulé",
    "Comprobando estado...": "Vérification de l'état...",
    "El proceso continúa en segundo plano.": "Le processus continue en arrière-plan.",
  };

  const AR = {
    "Cargando…": "جارٍ التحميل…", "Error": "خطأ", "Hecho": "تم",
    "La sesión ha expirado": "انتهت جلستك", "Actualizar": "تحديث",
    "Cerrar": "إغلاق", "Cancelar": "إلغاء", "Guardar": "حفظ",
    "Ejecutar": "تنفيذ", "Ver": "عرض", "Eliminar": "حذف",
    "Suspender": "تعليق", "Activar": "تنشيط", "Entrar": "تسجيل الدخول",
    "Volver a mi cuenta": "العودة إلى حسابي", "Salir": "تسجيل الخروج",
    "Mi cuenta": "حسابي", "Control de hosting": "إدارة الاستضافة",
    "Usuario": "اسم المستخدم", "Contraseña": "كلمة المرور",
    "Operación": "العمليات", "Administración": "الإدارة",
    "Resumen": "نظرة عامة", "Cuentas": "الحسابات",
    "Sitios web": "المواقع", "Aplicaciones": "التطبيقات",
    "Bases de datos": "قواعد البيانات", "Certificados": "الشهادات",
    "Backups": "النسخ الاحتياطية", "Monitor": "المراقبة",
    "Usuarios": "المستخدمون", "Seguridad": "الأمان",
    "Cortafuegos": "جدار الحماية", "Programas": "البرامج",
    "Consola": "الطرفية", "Auditoría": "التدقيق", "Sistema": "النظام",
    "Nueva cuenta": "حساب جديد", "Nuevo sitio": "موقع جديد",
    "Nueva aplicación": "تطبيق جديد", "Nuevo pool": "مجموعة جديدة",
    "Nueva base": "قاعدة بيانات جديدة", "Nuevo usuario FTP": "مستخدم FTP جديد",
    "Nuevo usuario": "مستخدم جديد", "Emitir certificado": "إصدار شهادة",
    "Renovar todos": "تجديد الكل", "Crear backup": "إنشاء نسخة احتياطية",
    "Restaurar": "استعادة", "Cuenta": "الحساب", "Dominio": "النطاق",
    "Estado": "الحالة", "Email": "البريد الإلكتروني", "Rol": "الدور",
    "Versión": "الإصدار",
    "Certificados que requieren atención": "شهادات تحتاج إلى مراجعة",
    "Revisar certificados": "مراجعة الشهادات",
    "Certificado SSL": "شهادة SSL",
    "Solicitando certificado...": "جارٍ طلب الشهادة...",
    "HTTPS activo": "HTTPS مفعّل",
    "Certificado emitido correctamente": "تم إصدار الشهادة بنجاح",
    "No se pudo emitir el certificado": "تعذّر إصدار الشهادة",
    "El sitio se creó, pero HTTPS no está activo.": "تم إنشاء الموقع، لكن HTTPS غير مفعّل.",
    "Reintentar": "إعادة المحاولة",
    "Tiempo transcurrido": "الوقت المنقضي",
    "Comprobando estado...": "جارٍ التحقق من الحالة...",
    "El proceso continúa en segundo plano.": "تستمر العملية في الخلفية.",
  };

  const RU = {
    "Cargando…": "Загрузка…", "Error": "Ошибка", "Hecho": "Готово",
    "La sesión ha expirado": "Сеанс истёк", "Actualizar": "Обновить",
    "Cerrar": "Закрыть", "Cancelar": "Отмена", "Guardar": "Сохранить",
    "Ejecutar": "Выполнить", "Ver": "Показать", "Eliminar": "Удалить",
    "Suspender": "Приостановить", "Activar": "Активировать", "Entrar": "Войти",
    "Volver a mi cuenta": "Вернуться в мою учётную запись", "Salir": "Выйти",
    "Mi cuenta": "Моя учётная запись", "Control de hosting": "Управление хостингом",
    "Usuario": "Имя пользователя", "Contraseña": "Пароль",
    "Operación": "Операции", "Administración": "Администрирование",
    "Resumen": "Обзор", "Cuentas": "Учётные записи", "Sitios web": "Сайты",
    "Aplicaciones": "Приложения", "Bases de datos": "Базы данных",
    "Certificados": "Сертификаты", "Backups": "Резервные копии",
    "Monitor": "Мониторинг", "Usuarios": "Пользователи",
    "Seguridad": "Безопасность", "Cortafuegos": "Брандмауэр",
    "Programas": "Программы", "Consola": "Консоль", "Auditoría": "Аудит",
    "Sistema": "Система", "Nueva cuenta": "Новая учётная запись",
    "Nuevo sitio": "Новый сайт", "Nueva aplicación": "Новое приложение",
    "Nuevo pool": "Новый пул", "Nueva base": "Новая база данных",
    "Nuevo usuario FTP": "Новый FTP-пользователь", "Nuevo usuario": "Новый пользователь",
    "Emitir certificado": "Выпустить сертификат", "Renovar todos": "Обновить все",
    "Crear backup": "Создать резервную копию", "Restaurar": "Восстановить",
    "Cuenta": "Учётная запись", "Dominio": "Домен", "Estado": "Состояние",
    "Email": "Эл. почта", "Rol": "Роль", "Versión": "Версия",
    "Certificados que requieren atención": "Сертификаты требуют внимания",
    "Revisar certificados": "Проверить сертификаты",
    "Certificado SSL": "Сертификат SSL",
    "Solicitando certificado...": "Запрос сертификата...",
    "HTTPS activo": "HTTPS активен",
    "Certificado emitido correctamente": "Сертификат успешно выпущен",
    "No se pudo emitir el certificado": "Не удалось выпустить сертификат",
    "El sitio se creó, pero HTTPS no está activo.": "Сайт создан, но HTTPS не активен.",
    "Reintentar": "Повторить",
    "Tiempo transcurrido": "Затраченное время",
    "Comprobando estado...": "Проверка состояния...",
    "El proceso continúa en segundo plano.": "Процесс продолжается в фоновом режиме.",
  };

  const ZH = {
    "Cargando…": "正在加载…", "Error": "错误", "Hecho": "完成",
    "La sesión ha expirado": "会话已过期", "Actualizar": "刷新",
    "Cerrar": "关闭", "Cancelar": "取消", "Guardar": "保存",
    "Ejecutar": "运行", "Ver": "查看", "Eliminar": "删除",
    "Suspender": "暂停", "Activar": "启用", "Entrar": "登录",
    "Volver a mi cuenta": "返回我的账户", "Salir": "退出",
    "Mi cuenta": "我的账户", "Control de hosting": "主机控制",
    "Usuario": "用户名", "Contraseña": "密码", "Operación": "操作",
    "Administración": "管理", "Resumen": "概览", "Cuentas": "账户",
    "Sitios web": "网站", "Aplicaciones": "应用", "Bases de datos": "数据库",
    "Certificados": "证书", "Backups": "备份", "Monitor": "监控",
    "Usuarios": "用户", "Seguridad": "安全", "Cortafuegos": "防火墙",
    "Programas": "程序", "Consola": "控制台", "Auditoría": "审计",
    "Sistema": "系统", "Nueva cuenta": "新账户", "Nuevo sitio": "新网站",
    "Nueva aplicación": "新应用", "Nuevo pool": "新池",
    "Nueva base": "新数据库", "Nuevo usuario FTP": "新 FTP 用户",
    "Nuevo usuario": "新用户", "Emitir certificado": "签发证书",
    "Renovar todos": "全部续期", "Crear backup": "创建备份",
    "Restaurar": "恢复", "Cuenta": "账户", "Dominio": "域名",
    "Estado": "状态", "Email": "电子邮件", "Rol": "角色", "Versión": "版本",
    "Certificados que requieren atención": "证书需要处理",
    "Revisar certificados": "查看证书",
    "Certificado SSL": "SSL 证书",
    "Solicitando certificado...": "正在申请证书...",
    "HTTPS activo": "HTTPS 已启用",
    "Certificado emitido correctamente": "证书已成功签发",
    "No se pudo emitir el certificado": "无法签发证书",
    "El sitio se creó, pero HTTPS no está activo.": "网站已创建，但 HTTPS 未启用。",
    "Reintentar": "重试",
    "Tiempo transcurrido": "已用时间",
    "Comprobando estado...": "正在检查状态...",
    "El proceso continúa en segundo plano.": "进程将在后台继续。",
  };

  const JA = {
    "Cargando…": "読み込み中…", "Error": "エラー", "Hecho": "完了",
    "La sesión ha expirado": "セッションの有効期限が切れました", "Actualizar": "更新",
    "Cerrar": "閉じる", "Cancelar": "キャンセル", "Guardar": "保存",
    "Ejecutar": "実行", "Ver": "表示", "Eliminar": "削除",
    "Suspender": "停止", "Activar": "有効化", "Entrar": "サインイン",
    "Volver a mi cuenta": "自分のアカウントに戻る", "Salir": "サインアウト",
    "Mi cuenta": "マイアカウント", "Control de hosting": "ホスティング管理",
    "Usuario": "ユーザー名", "Contraseña": "パスワード", "Operación": "操作",
    "Administración": "管理", "Resumen": "概要", "Cuentas": "アカウント",
    "Sitios web": "ウェブサイト", "Aplicaciones": "アプリケーション",
    "Bases de datos": "データベース", "Certificados": "証明書",
    "Backups": "バックアップ", "Monitor": "モニタリング",
    "Usuarios": "ユーザー", "Seguridad": "セキュリティ",
    "Cortafuegos": "ファイアウォール", "Programas": "プログラム",
    "Consola": "コンソール", "Auditoría": "監査", "Sistema": "システム",
    "Nueva cuenta": "新しいアカウント", "Nuevo sitio": "新しいウェブサイト",
    "Nueva aplicación": "新しいアプリケーション", "Nuevo pool": "新しいプール",
    "Nueva base": "新しいデータベース", "Nuevo usuario FTP": "新しい FTP ユーザー",
    "Nuevo usuario": "新しいユーザー", "Emitir certificado": "証明書を発行",
    "Renovar todos": "すべて更新", "Crear backup": "バックアップを作成",
    "Restaurar": "復元", "Cuenta": "アカウント", "Dominio": "ドメイン",
    "Estado": "ステータス", "Email": "メール", "Rol": "ロール",
    "Versión": "バージョン",
    "Certificados que requieren atención": "証明書の確認が必要です",
    "Revisar certificados": "証明書を確認",
    "Certificado SSL": "SSL 証明書",
    "Solicitando certificado...": "証明書を申請中...",
    "HTTPS activo": "HTTPS 有効",
    "Certificado emitido correctamente": "証明書が正常に発行されました",
    "No se pudo emitir el certificado": "証明書を発行できませんでした",
    "El sitio se creó, pero HTTPS no está activo.": "サイトは作成されましたが、HTTPS は有効ではありません。",
    "Reintentar": "再試行",
    "Tiempo transcurrido": "経過時間",
    "Comprobando estado...": "ステータスを確認中...",
    "El proceso continúa en segundo plano.": "処理はバックグラウンドで続行されます。",
  };

  const IT = {
    "Cargando…": "Caricamento…", "Error": "Errore", "Hecho": "Fatto",
    "La sesión ha expirado": "La sessione è scaduta", "Actualizar": "Aggiorna",
    "Cerrar": "Chiudi", "Cancelar": "Annulla", "Guardar": "Salva",
    "Ejecutar": "Esegui", "Ver": "Mostra", "Eliminar": "Elimina",
    "Suspender": "Sospendi", "Activar": "Attiva", "Entrar": "Accedi",
    "Volver a mi cuenta": "Torna al mio account", "Salir": "Esci",
    "Mi cuenta": "Il mio account", "Control de hosting": "Controllo hosting",
    "Usuario": "Nome utente", "Contraseña": "Password",
    "Operación": "Operazioni", "Administración": "Amministrazione",
    "Resumen": "Panoramica", "Cuentas": "Account", "Sitios web": "Siti web",
    "Aplicaciones": "Applicazioni", "Bases de datos": "Database",
    "Certificados": "Certificati", "Backups": "Backup",
    "Monitor": "Monitoraggio", "Usuarios": "Utenti", "Seguridad": "Sicurezza",
    "Cortafuegos": "Firewall", "Programas": "Programmi", "Consola": "Console",
    "Auditoría": "Audit", "Sistema": "Sistema", "Nueva cuenta": "Nuovo account",
    "Nuevo sitio": "Nuovo sito", "Nueva aplicación": "Nuova applicazione",
    "Nuevo pool": "Nuovo pool", "Nueva base": "Nuovo database",
    "Nuevo usuario FTP": "Nuovo utente FTP", "Nuevo usuario": "Nuovo utente",
    "Emitir certificado": "Emetti certificato", "Renovar todos": "Rinnova tutti",
    "Crear backup": "Crea backup", "Restaurar": "Ripristina",
    "Cuenta": "Account", "Dominio": "Dominio", "Estado": "Stato",
    "Email": "Email", "Rol": "Ruolo", "Versión": "Versione",
    "activo": "attivo", "deshabilitado": "disabilitato",
    "suspendida": "sospeso", "solo lectura": "sola lettura",
    "Cuenta creada": "Account creato", "Sitio publicado": "Sito pubblicato",
    "Aplicación creada": "Applicazione creata", "Usuario creado": "Utente creato",
    "Usuario eliminado": "Utente eliminato", "Backup creado": "Backup creato",
    "Sesión de administrador restaurada": "Sessione amministratore ripristinata",
    "Cuenta y acceso creados": "Account e accesso creati",
    "Acceso al panel creado": "Accesso al pannello creato",
    "No puedes desactivar tu propia cuenta": "Non puoi disattivare il tuo account",
    "Estás viendo el panel como": "Stai visualizzando il pannello come",
    "Sin registros": "Nessun record",
    "Crea la primera cuenta para empezar.": "Crea il primo account per iniziare.",
    "Aún no hay virtual hosts.": "Nessun virtual host.",
    "El sistema está actualizado.": "Il sistema è aggiornato.",
    "No se detectó software.": "Nessun software rilevato.",
    "Certificados que requieren atención": "Certificati da controllare",
    "Revisar certificados": "Controlla i certificati",
    "Certificado SSL": "Certificato SSL",
    "Solicitando certificado...": "Richiesta del certificato...",
    "HTTPS activo": "HTTPS attivo",
    "Certificado emitido correctamente": "Certificato emesso correttamente",
    "No se pudo emitir el certificado": "Impossibile emettere il certificato",
    "El sitio se creó, pero HTTPS no está activo.": "Il sito è stato creato, ma HTTPS non è attivo.",
    "Reintentar": "Riprova",
    "Tiempo transcurrido": "Tempo trascorso",
    "Comprobando estado...": "Verifica stato...",
    "El proceso continúa en segundo plano.": "Il processo continua in background.",
  };

  const DE = {
    "Cargando…": "Wird geladen…", "Error": "Fehler", "Hecho": "Fertig",
    "La sesión ha expirado": "Ihre Sitzung ist abgelaufen", "Actualizar": "Aktualisieren",
    "Cerrar": "Schließen", "Cancelar": "Abbrechen", "Guardar": "Speichern",
    "Ejecutar": "Ausführen", "Ver": "Anzeigen", "Eliminar": "Löschen",
    "Suspender": "Sperren", "Activar": "Aktivieren", "Entrar": "Anmelden",
    "Volver a mi cuenta": "Zurück zu meinem Konto", "Salir": "Abmelden",
    "Mi cuenta": "Mein Konto", "Control de hosting": "Hosting-Verwaltung",
    "Usuario": "Benutzername", "Contraseña": "Passwort",
    "Operación": "Betrieb", "Administración": "Verwaltung",
    "Resumen": "Übersicht", "Cuentas": "Konten", "Sitios web": "Websites",
    "Aplicaciones": "Anwendungen", "Bases de datos": "Datenbanken",
    "Certificados": "Zertifikate", "Backups": "Backups",
    "Monitor": "Überwachung", "Usuarios": "Benutzer", "Seguridad": "Sicherheit",
    "Cortafuegos": "Firewall", "Programas": "Programme", "Consola": "Konsole",
    "Auditoría": "Audit", "Sistema": "System", "Nueva cuenta": "Neues Konto",
    "Nuevo sitio": "Neue Website", "Nueva aplicación": "Neue Anwendung",
    "Nuevo pool": "Neuer Pool", "Nueva base": "Neue Datenbank",
    "Nuevo usuario FTP": "Neuer FTP-Benutzer", "Nuevo usuario": "Neuer Benutzer",
    "Emitir certificado": "Zertifikat ausstellen", "Renovar todos": "Alle erneuern",
    "Crear backup": "Backup erstellen", "Restaurar": "Wiederherstellen",
    "Cuenta": "Konto", "Dominio": "Domain", "Estado": "Status",
    "Email": "E-Mail", "Rol": "Rolle", "Versión": "Version",
    "activo": "aktiv", "deshabilitado": "deaktiviert",
    "suspendida": "gesperrt", "solo lectura": "schreibgeschützt",
    "Cuenta creada": "Konto erstellt", "Sitio publicado": "Website veröffentlicht",
    "Aplicación creada": "Anwendung erstellt", "Usuario creado": "Benutzer erstellt",
    "Usuario eliminado": "Benutzer gelöscht", "Backup creado": "Backup erstellt",
    "Sesión de administrador restaurada": "Administratorsitzung wiederhergestellt",
    "Cuenta y acceso creados": "Konto und Zugriff erstellt",
    "Acceso al panel creado": "Panelzugriff erstellt",
    "No puedes desactivar tu propia cuenta": "Sie können Ihr eigenes Konto nicht deaktivieren",
    "Estás viendo el panel como": "Sie sehen das Panel als",
    "Sin registros": "Keine Einträge",
    "Crea la primera cuenta para empezar.": "Erstellen Sie das erste Konto, um zu beginnen.",
    "Aún no hay virtual hosts.": "Noch keine virtuellen Hosts.",
    "El sistema está actualizado.": "Das System ist auf dem neuesten Stand.",
    "No se detectó software.": "Keine Software erkannt.",
    "Certificados que requieren atención": "Zertifikate benötigen Aufmerksamkeit",
    "Revisar certificados": "Zertifikate prüfen",
    "Certificado SSL": "SSL-Zertifikat",
    "Solicitando certificado...": "Zertifikat wird angefordert...",
    "HTTPS activo": "HTTPS aktiv",
    "Certificado emitido correctamente": "Zertifikat erfolgreich ausgestellt",
    "No se pudo emitir el certificado": "Zertifikat konnte nicht ausgestellt werden",
    "El sitio se creó, pero HTTPS no está activo.": "Die Website wurde erstellt, aber HTTPS ist nicht aktiv.",
    "Reintentar": "Erneut versuchen",
    "Tiempo transcurrido": "Verstrichene Zeit",
    "Comprobando estado...": "Status wird geprüft...",
    "El proceso continúa en segundo plano.": "Der Vorgang läuft im Hintergrund weiter.",
  };

  const DICT = {
    en: EN,
    pt: Object.assign({}, EN, PT),
    fr: Object.assign({}, EN, FR),
    ar: Object.assign({}, EN, AR),
    ru: Object.assign({}, EN, RU),
    zh: Object.assign({}, EN, ZH),
    ja: Object.assign({}, EN, JA),
    it: Object.assign({}, EN, IT),
    de: Object.assign({}, EN, DE),
  };

  const STORAGE_KEY = 'rynxcp.lang';
  const TITLE_KEY = 'RYNXCP — Panel de control';
  const SUPPORTED = new Set(LANGUAGES.map((item) => item.code));
  let current = detect();

  function detect() {
    try {
      const saved = localStorage.getItem(STORAGE_KEY);
      if (saved && SUPPORTED.has(saved)) return saved;
    } catch {
      /* localStorage unavailable */
    }
    const raw = Array.isArray(navigator.languages) && navigator.languages.length
      ? navigator.languages
      : [navigator.language];
    for (const value of raw) {
      const code = String(value || '').toLowerCase().split('-')[0];
      if (SUPPORTED.has(code)) return code;
    }
    return 'en';
  }

  function tr(text) {
    if (!text) return text;
    if (current === 'es') return text;
    const table = DICT[current] || EN;
    return table[text] || EN[text] || text;
  }

  const SKIP_SELECTOR = [
    '.cell-title', '.cell-sub', '.mono', '.secret', '#user-name',
    '#impersonated-name', '.avatar', '.brand-mark', '.lang-select',
    '[data-no-translate]',
  ].join(',');

  function translateDom(root) {
    if (!root) return;
    if (root.nodeType === Node.TEXT_NODE) {
      const raw = root.nodeValue || '';
      const value = raw.trim();
      if (!value) return;
      const translated = tr(value);
      if (translated !== value) {
        const leading = raw.match(/^\s*/)[0];
        const trailing = raw.match(/\s*$/)[0];
        root.nodeValue = leading + translated + trailing;
      }
      return;
    }
    if (root.nodeType !== Node.ELEMENT_NODE && root.nodeType !== Node.DOCUMENT_NODE) {
      return;
    }
    if (root.nodeType === Node.ELEMENT_NODE) {
      const tag = root.tagName;
      if (tag === 'SCRIPT' || tag === 'STYLE' || tag === 'PRE' || tag === 'CODE') {
        return;
      }
      if (root.matches(SKIP_SELECTOR)) return;
      for (const attr of ['placeholder', 'title', 'aria-label']) {
        if (root.hasAttribute(attr)) {
          const value = root.getAttribute(attr).trim();
          const translated = tr(value);
          if (translated !== value) root.setAttribute(attr, translated);
        }
      }
      if (tag === 'INPUT' || tag === 'TEXTAREA') return;
    }
    for (const child of [...root.childNodes]) translateDom(child);
  }

  function applyDocument() {
    document.documentElement.lang = current;
    document.documentElement.dir = current === 'ar' ? 'rtl' : 'ltr';
    translateDom(document.body);
    document.title = tr(TITLE_KEY);
    syncSelects();
  }

  function setLanguage(code) {
    const next = SUPPORTED.has(code) ? code : 'en';
    current = next;
    try {
      localStorage.setItem(STORAGE_KEY, next);
    } catch {
      /* ignore */
    }
    applyDocument();
    if (typeof window.RYNX_ON_LANG_CHANGE === 'function') {
      window.RYNX_ON_LANG_CHANGE(next);
    }
  }

  function syncSelects() {
    document.querySelectorAll('.lang-select').forEach((select) => {
      select.value = current;
    });
  }

  function initSelects() {
    document.querySelectorAll('.lang-select').forEach((select) => {
      select.innerHTML = LANGUAGES.map(
        (item) => `<option value="${item.code}">${item.label}</option>`,
      ).join('');
      select.value = current;
      select.addEventListener('change', () => setLanguage(select.value));
    });
  }

  window.RYNX_I18N = {
    tr,
    translateDom,
    setLanguage,
    getLanguage: () => current,
    applyDocument,
    LANGUAGES,
  };

  document.addEventListener('DOMContentLoaded', () => {
    initSelects();
    applyDocument();
  });
})();
