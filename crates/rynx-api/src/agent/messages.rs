//! Localized text for deterministic findings and agent alert emails.
//!
//! Rule findings are rendered in the panel owner's locale. Spanish and English
//! are complete; other locales fall back to English so the agent never sends
//! Spanish text to an English-speaking administrator by accident.

use serde_json::Value;

pub struct FindingText {
    pub title: String,
    pub summary: String,
    pub recommendation: String,
}

pub fn email_subject(locale: &str, count: usize) -> String {
    match locale {
        "es" => format!("RYNXCP: {count} alerta(s) del agente"),
        "pt" => format!("RYNXCP: {count} alerta(s) do agente"),
        "fr" => format!("RYNXCP : {count} alerte(s) de l'agent"),
        "it" => format!("RYNXCP: {count} avviso/i dell'agente"),
        "de" => format!("RYNXCP: {count} Agent-Warnung(en)"),
        "ar" => format!("RYNXCP: {count} تنبيه(ات) من الوكيل"),
        "ru" => format!("RYNXCP: {count} оповещение(й) агента"),
        "zh" => format!("RYNXCP：{count} 条代理告警"),
        "ja" => format!("RYNXCP: エージェントのアラート {count} 件"),
        _ => format!("RYNXCP: {count} agent alert(s)"),
    }
}

pub fn email_labels(locale: &str) -> (&'static str, &'static str) {
    match locale {
        "es" => ("Recomendación", "Severidad"),
        "pt" => ("Recomendação", "Severidade"),
        "fr" => ("Recommandation", "Gravité"),
        "it" => ("Raccomandazione", "Gravità"),
        "de" => ("Empfehlung", "Schweregrad"),
        "ar" => ("التوصية", "الخطورة"),
        "ru" => ("Рекомендация", "Важность"),
        "zh" => ("建议", "严重程度"),
        "ja" => ("推奨事項", "重要度"),
        _ => ("Recommendation", "Severity"),
    }
}

pub fn english(code: &str, params: &Value) -> Option<FindingText> {
    let (title, summary, recommendation) = match code {
        "auth-failures" => (
            "Elevated failed login attempts",
            "{failures} failed authentication attempts in the last 24 hours.",
            "Review Security, block suspicious IPs and check fail2ban.",
        ),
        "auth-blocked" => (
            "Authentication lockouts",
            "{blocked} login attempts were blocked in the last 24 hours.",
            "Check whether the lockouts come from brute force or legitimate users.",
        ),
        "service-failed" => (
            "Service down: {name}",
            "Unit {unit} is not running correctly (status: {status}) and may affect hosting.",
            "Check the service status and journal; restart it from Programs if appropriate.",
        ),
        "mail-filter-missing" => (
            "Incomplete mail filters",
            "Missing antivirus/antispam components: {missing}.",
            "Reinstall the mail stack with the installer or from Programs.",
        ),
        "mail-filter-inactive" => (
            "Mail filter stopped: {name}",
            "{name} is installed but its unit reports '{status}'.",
            "Restart the component from Antivirus and antispam or Programs.",
        ),
        "antivirus-detection" => (
            "Malware detected in {service}",
            "{message}",
            "Isolate the file, review the affected mailbox or account and run a full scan.",
        ),
        "antispam-volume" => (
            "High recent spam volume",
            "{spam} messages were tagged or rejected by SpamAssassin in the recent journal.",
            "Review SpamAssassin rules, mail volume and whether a spam campaign is in progress.",
        ),
        "security-updates" => (
            "Security updates pending",
            "{count} updates marked as security are pending.",
            "Apply the updates from Programs during a maintenance window.",
        ),
        "disk-usage" => (
            "High disk usage",
            "Disk usage is at {percent}% and may fill up.",
            "Review backups, logs and accounts with the highest usage before the disk fills.",
        ),
        "memory-usage" => (
            "High memory usage",
            "Memory usage is at {percent}%.",
            "Review processes, containers and application limits before the OOM killer acts.",
        ),
        "load-average" => (
            "High system load",
            "Load 1m per CPU: {per_cpu}.",
            "Identify CPU-heavy processes and review managed applications.",
        ),
        "certificate-expired" => (
            "Certificate expired: {domain}",
            "The certificate for {domain} has expired (expires {not_after}).",
            "Renew the certificate from Certificates or republish the site.",
        ),
        "certificate-expiring" => (
            "Certificate expiring: {domain}",
            "The certificate for {domain} is about to expire (expires {not_after}).",
            "Renew the certificate from Certificates or republish the site.",
        ),
        "firewall-blocks" => (
            "Many blocked IPs",
            "{blocked} IP addresses are currently blocked.",
            "Check for a distributed attack and review the fail2ban rules.",
        ),
        _ => return None,
    };
    Some(FindingText {
        title: fill(title, params),
        summary: fill(summary, params),
        recommendation: fill(recommendation, params),
    })
}

pub fn localized(locale: &str, code: &str, params: &Value) -> Option<FindingText> {
    let (title, summary, recommendation) = match locale {
        "pt" => pt(code),
        "fr" => fr(code),
        "it" => it(code),
        "de" => de(code),
        "ar" => ar(code),
        "ru" => ru(code),
        "zh" => zh(code),
        "ja" => ja(code),
        _ => return english(code, params),
    }?;
    Some(FindingText {
        title: fill(title, params),
        summary: fill(summary, params),
        recommendation: fill(recommendation, params),
    })
}

type Triple = (&'static str, &'static str, &'static str);

fn pt(code: &str) -> Option<Triple> {
    Some(match code {
        "auth-failures" => (
            "Tentativas de login falhadas elevadas",
            "{failures} tentativas de autenticação falhadas nas últimas 24 horas.",
            "Reveja Segurança, bloqueie IPs suspeitas e verifique o fail2ban.",
        ),
        "auth-blocked" => (
            "Bloqueios de autenticação",
            "{blocked} tentativas de login foram bloqueadas nas últimas 24 horas.",
            "Verifique se os bloqueios vêm de força bruta ou de utilizadores legítimos.",
        ),
        "service-failed" => (
            "Serviço em baixo: {name}",
            "A unidade {unit} não está a funcionar corretamente (estado: {status}) e pode afetar o alojamento.",
            "Verifique o estado e o journal do serviço; reinicie-o em Programas se necessário.",
        ),
        "mail-filter-missing" => (
            "Filtros de correio incompletos",
            "Faltam componentes de antivírus/antispam: {missing}.",
            "Reinstale o stack de correio com o instalador ou em Programas.",
        ),
        "mail-filter-inactive" => (
            "Filtro de correio parado: {name}",
            "{name} está instalado mas a unidade reporta '{status}'.",
            "Reinicie o componente em Antivírus e antispam ou em Programas.",
        ),
        "antivirus-detection" => (
            "Malware detetado em {service}",
            "{message}",
            "Isole o ficheiro, reveja a caixa ou conta afetada e execute uma análise completa.",
        ),
        "antispam-volume" => (
            "Volume elevado de spam recente",
            "{spam} mensagens foram marcadas ou rejeitadas pelo SpamAssassin no journal recente.",
            "Reveja as regras do SpamAssassin, o volume de correio e se há uma campanha de spam em curso.",
        ),
        "security-updates" => (
            "Atualizações de segurança pendentes",
            "{count} atualizações marcadas como de segurança estão pendentes.",
            "Aplique as atualizações em Programas durante uma janela de manutenção.",
        ),
        "disk-usage" => (
            "Uso de disco elevado",
            "O disco está a {percent}% e pode encher.",
            "Reveja backups, logs e contas com maior consumo antes que o disco encha.",
        ),
        "memory-usage" => (
            "Uso de memória elevado",
            "A memória está a {percent}%.",
            "Reveja processos, contentores e limites das aplicações antes que o OOM killer atue.",
        ),
        "load-average" => (
            "Carga do sistema elevada",
            "Load 1m por CPU: {per_cpu}.",
            "Identifique processos que consomem CPU e reveja as aplicações geridas.",
        ),
        "certificate-expired" => (
            "Certificado expirado: {domain}",
            "O certificado de {domain} expirou (expira {not_after}).",
            "Renove o certificado em Certificados ou republique o site.",
        ),
        "certificate-expiring" => (
            "Certificado a expirar: {domain}",
            "O certificado de {domain} está prestes a expirar (expira {not_after}).",
            "Renove o certificado em Certificados ou republique o site.",
        ),
        "firewall-blocks" => (
            "Muitos IPs bloqueados",
            "{blocked} endereços IP estão bloqueados neste momento.",
            "Verifique se há um ataque distribuído e reveja as regras do fail2ban.",
        ),
        _ => return None,
    })
}

fn fr(code: &str) -> Option<Triple> {
    Some(match code {
        "auth-failures" => (
            "Tentatives de connexion échouées élevées",
            "{failures} tentatives d'authentification échouées au cours des dernières 24 heures.",
            "Consultez Sécurité, bloquez les IP suspectes et vérifiez fail2ban.",
        ),
        "auth-blocked" => (
            "Blocages d'authentification",
            "{blocked} tentatives de connexion ont été bloquées au cours des dernières 24 heures.",
            "Vérifiez si les blocages proviennent d'une attaque par force brute ou d'utilisateurs légitimes.",
        ),
        "service-failed" => (
            "Service arrêté : {name}",
            "L'unité {unit} ne fonctionne pas correctement (état : {status}) et peut affecter l'hébergement.",
            "Vérifiez l'état et le journal du service ; redémarrez-le depuis Programmes si nécessaire.",
        ),
        "mail-filter-missing" => (
            "Filtres de messagerie incomplets",
            "Composants antivirus/antispam manquants : {missing}.",
            "Réinstallez la pile de messagerie avec l'installateur ou depuis Programmes.",
        ),
        "mail-filter-inactive" => (
            "Filtre de messagerie arrêté : {name}",
            "{name} est installé mais son unité signale « {status} ».",
            "Redémarrez le composant depuis Antivirus et antispam ou Programmes.",
        ),
        "antivirus-detection" => (
            "Malware détecté dans {service}",
            "{message}",
            "Isolez le fichier, examinez la boîte ou le compte concerné et lancez une analyse complète.",
        ),
        "antispam-volume" => (
            "Volume de spam récent élevé",
            "{spam} messages ont été marqués ou rejetés par SpamAssassin dans le journal récent.",
            "Vérifiez les règles SpamAssassin, le volume de courrier et l'existence d'une campagne de spam.",
        ),
        "security-updates" => (
            "Mises à jour de sécurité en attente",
            "{count} mises à jour marquées comme critiques sont en attente.",
            "Appliquez les mises à jour depuis Programmes pendant une fenêtre de maintenance.",
        ),
        "disk-usage" => (
            "Utilisation du disque élevée",
            "Le disque est à {percent} % et risque de se remplir.",
            "Vérifiez les sauvegardes, les journaux et les comptes les plus volumineux avant que le disque ne soit plein.",
        ),
        "memory-usage" => (
            "Utilisation de la mémoire élevée",
            "La mémoire est utilisée à {percent} %.",
            "Vérifiez les processus, conteneurs et limites d'applications avant que l'OOM killer n'intervienne.",
        ),
        "load-average" => (
            "Charge système élevée",
            "Charge 1m par CPU : {per_cpu}.",
            "Identifiez les processus gourmands en CPU et vérifiez les applications gérées.",
        ),
        "certificate-expired" => (
            "Certificat expiré : {domain}",
            "Le certificat de {domain} a expiré (expire le {not_after}).",
            "Renouvelez le certificat depuis Certificats ou republiez le site.",
        ),
        "certificate-expiring" => (
            "Certificat bientôt expiré : {domain}",
            "Le certificat de {domain} va bientôt expirer (expire le {not_after}).",
            "Renouvelez le certificat depuis Certificats ou republiez le site.",
        ),
        "firewall-blocks" => (
            "Nombreuses IP bloquées",
            "{blocked} adresses IP sont actuellement bloquées.",
            "Vérifiez s'il s'agit d'une attaque distribuée et examinez les règles fail2ban.",
        ),
        _ => return None,
    })
}

fn it(code: &str) -> Option<Triple> {
    Some(match code {
        "auth-failures" => (
            "Tentativi di accesso falliti elevati",
            "{failures} tentativi di autenticazione falliti nelle ultime 24 ore.",
            "Controlla Sicurezza, blocca gli IP sospetti e verifica fail2ban.",
        ),
        "auth-blocked" => (
            "Blocchi di autenticazione",
            "{blocked} tentativi di accesso sono stati bloccati nelle ultime 24 ore.",
            "Verifica se i blocchi derivano da forza bruta o da utenti legittimi.",
        ),
        "service-failed" => (
            "Servizio inattivo: {name}",
            "L'unità {unit} non funziona correttamente (stato: {status}) e può influire sull'hosting.",
            "Controlla lo stato e il journal del servizio; riavvialo da Programmi se opportuno.",
        ),
        "mail-filter-missing" => (
            "Filtri di posta incompleti",
            "Componenti antivirus/antispam mancanti: {missing}.",
            "Reinstalla lo stack di posta con l'installer o da Programmi.",
        ),
        "mail-filter-inactive" => (
            "Filtro di posta fermo: {name}",
            "{name} è installato ma l'unità riporta '{status}'.",
            "Riavvia il componente da Antivirus e antispam o da Programmi.",
        ),
        "antivirus-detection" => (
            "Malware rilevato in {service}",
            "{message}",
            "Isola il file, controlla la casella o l'account interessato ed esegui una scansione completa.",
        ),
        "antispam-volume" => (
            "Volume di spam recente elevato",
            "{spam} messaggi sono stati contrassegnati o rifiutati da SpamAssassin nel journal recente.",
            "Controlla le regole di SpamAssassin, il volume di posta e se è in corso una campagna di spam.",
        ),
        "security-updates" => (
            "Aggiornamenti di sicurezza in sospeso",
            "{count} aggiornamenti contrassegnati come di sicurezza sono in sospeso.",
            "Applica gli aggiornamenti da Programmi durante una finestra di manutenzione.",
        ),
        "disk-usage" => (
            "Utilizzo del disco elevato",
            "Il disco è al {percent}% e potrebbe riempirsi.",
            "Controlla backup, log e account con il maggiore utilizzo prima che il disco si riempia.",
        ),
        "memory-usage" => (
            "Utilizzo della memoria elevato",
            "La memoria è al {percent}%.",
            "Controlla processi, container e limiti delle applicazioni prima che intervenga l'OOM killer.",
        ),
        "load-average" => (
            "Carico di sistema elevato",
            "Carico 1m per CPU: {per_cpu}.",
            "Identifica i processi che consumano CPU e controlla le applicazioni gestite.",
        ),
        "certificate-expired" => (
            "Certificato scaduto: {domain}",
            "Il certificato di {domain} è scaduto (scade il {not_after}).",
            "Rinnova il certificato da Certificati o ripubblica il sito.",
        ),
        "certificate-expiring" => (
            "Certificato in scadenza: {domain}",
            "Il certificato di {domain} sta per scadere (scade il {not_after}).",
            "Rinnova il certificato da Certificati o ripubblica il sito.",
        ),
        "firewall-blocks" => (
            "Molti IP bloccati",
            "{blocked} indirizzi IP sono attualmente bloccati.",
            "Controlla se c'è un attacco distribuito e rivedi le regole di fail2ban.",
        ),
        _ => return None,
    })
}

fn de(code: &str) -> Option<Triple> {
    Some(match code {
        "auth-failures" => (
            "Erhöhte fehlgeschlagene Anmeldeversuche",
            "{failures} fehlgeschlagene Authentifizierungsversuche in den letzten 24 Stunden.",
            "Prüfe Sicherheit, blockiere verdächtige IPs und kontrolliere fail2ban.",
        ),
        "auth-blocked" => (
            "Authentifizierungssperren",
            "{blocked} Anmeldeversuche wurden in den letzten 24 Stunden blockiert.",
            "Prüfe, ob die Sperren von Brute-Force oder legitimen Benutzern stammen.",
        ),
        "service-failed" => (
            "Dienst ausgefallen: {name}",
            "Die Unit {unit} läuft nicht korrekt (Status: {status}) und kann das Hosting beeinträchtigen.",
            "Prüfe Status und Journal des Dienstes; starte ihn bei Bedarf unter Programme neu.",
        ),
        "mail-filter-missing" => (
            "Unvollständige Mailfilter",
            "Fehlende Antivirus-/Antispam-Komponenten: {missing}.",
            "Installiere den Mail-Stack mit dem Installer oder unter Programme neu.",
        ),
        "mail-filter-inactive" => (
            "Mailfilter gestoppt: {name}",
            "{name} ist installiert, aber die Unit meldet '{status}'.",
            "Starte die Komponente unter Antivirus und Antispam oder Programme neu.",
        ),
        "antivirus-detection" => (
            "Malware in {service} erkannt",
            "{message}",
            "Isoliere die Datei, prüfe das betroffene Postfach oder Konto und führe einen vollständigen Scan aus.",
        ),
        "antispam-volume" => (
            "Hohes aktuelles Spam-Aufkommen",
            "{spam} Nachrichten wurden im aktuellen Journal von SpamAssassin markiert oder abgelehnt.",
            "Prüfe die SpamAssassin-Regeln, das Mailvolumen und ob eine Spam-Kampagne läuft.",
        ),
        "security-updates" => (
            "Sicherheitsupdates ausstehend",
            "{count} als Sicherheitsupdates markierte Aktualisierungen sind ausstehend.",
            "Wende die Updates während eines Wartungsfensters unter Programme an.",
        ),
        "disk-usage" => (
            "Hohe Festplattennutzung",
            "Die Festplatte ist zu {percent} % belegt und könnte voll werden.",
            "Prüfe Backups, Logs und Konten mit dem höchsten Verbrauch, bevor die Festplatte voll ist.",
        ),
        "memory-usage" => (
            "Hohe Speichernutzung",
            "Der Speicher ist zu {percent} % belegt.",
            "Prüfe Prozesse, Container und Anwendungslimits, bevor der OOM-Killer eingreift.",
        ),
        "load-average" => (
            "Hohe Systemlast",
            "Load 1m pro CPU: {per_cpu}.",
            "Identifiziere CPU-intensive Prozesse und prüfe die verwalteten Anwendungen.",
        ),
        "certificate-expired" => (
            "Zertifikat abgelaufen: {domain}",
            "Das Zertifikat für {domain} ist abgelaufen (läuft am {not_after} ab).",
            "Erneuere das Zertifikat unter Zertifikate oder veröffentliche die Website erneut.",
        ),
        "certificate-expiring" => (
            "Zertifikat läuft ab: {domain}",
            "Das Zertifikat für {domain} läuft bald ab (läuft am {not_after} ab).",
            "Erneuere das Zertifikat unter Zertifikate oder veröffentliche die Website erneut.",
        ),
        "firewall-blocks" => (
            "Viele blockierte IPs",
            "{blocked} IP-Adressen sind derzeit blockiert.",
            "Prüfe, ob ein verteilter Angriff vorliegt, und überprüfe die fail2ban-Regeln.",
        ),
        _ => return None,
    })
}

fn ar(code: &str) -> Option<Triple> {
    Some(match code {
        "auth-failures" => (
            "محاولات تسجيل دخول فاشلة مرتفعة",
            "{failures} محاولة مصادقة فاشلة خلال آخر 24 ساعة.",
            "راجع الأمان، واحظر عناوين IP المشبوهة، وتحقّق من fail2ban.",
        ),
        "auth-blocked" => (
            "عمليات قفل المصادقة",
            "تم حظر {blocked} محاولة تسجيل دخول خلال آخر 24 ساعة.",
            "تحقّق مما إذا كانت عمليات القفل ناتجة عن هجوم تخمين أم عن مستخدمين شرعيين.",
        ),
        "service-failed" => (
            "الخدمة متوقفة: {name}",
            "الوحدة {unit} لا تعمل بشكل صحيح (الحالة: {status}) وقد تؤثر على الاستضافة.",
            "راجع حالة الخدمة وسجلها، وأعد تشغيلها من البرامج إذا لزم الأمر.",
        ),
        "mail-filter-missing" => (
            "مرشحات البريد غير مكتملة",
            "مكوّنات مكافحة الفيروسات/البريد المزعج المفقودة: {missing}.",
            "أعد تثبيت حزمة البريد باستخدام المثبّت أو من البرامج.",
        ),
        "mail-filter-inactive" => (
            "مرشح البريد متوقف: {name}",
            "{name} مثبّت لكن الوحدة تُبلغ عن الحالة '{status}'.",
            "أعد تشغيل المكوّن من مكافحة الفيروسات والبريد المزعج أو من البرامج.",
        ),
        "antivirus-detection" => (
            "تم اكتشاف برمجية خبيثة في {service}",
            "{message}",
            "اعزل الملف، وراجع صندوق البريد أو الحساب المتأثر، ونفّذ فحصاً كاملاً.",
        ),
        "antispam-volume" => (
            "حجم بريد مزعج مرتفع حديثاً",
            "تم وسم أو رفض {spam} رسالة بواسطة SpamAssassin في السجل الأخير.",
            "راجع قواعد SpamAssassin وحجم البريد وما إذا كانت هناك حملة بريد مزعج جارية.",
        ),
        "security-updates" => (
            "تحديثات أمنية معلّقة",
            "هناك {count} تحديثاً موسوماً كأمني معلّق.",
            "طبّق التحديثات من البرامج خلال نافذة صيانة.",
        ),
        "disk-usage" => (
            "استخدام مرتفع للقرص",
            "استخدام القرص عند {percent}% وقد يمتلئ.",
            "راجع النسخ الاحتياطية والسجلات والحسابات الأكثر استهلاكاً قبل امتلاء القرص.",
        ),
        "memory-usage" => (
            "استخدام مرتفع للذاكرة",
            "استخدام الذاكرة عند {percent}%.",
            "راجع العمليات والحاويات وحدود التطبيقات قبل أن يتدخل OOM killer.",
        ),
        "load-average" => (
            "حمل النظام مرتفع",
            "متوسط الحمل 1د لكل نواة: {per_cpu}.",
            "حدّد العمليات المستهلكة للمعالج وراجع التطبيقات المُدارة.",
        ),
        "certificate-expired" => (
            "انتهت صلاحية الشهادة: {domain}",
            "انتهت صلاحية شهادة {domain} (تنتهي في {not_after}).",
            "جدّد الشهادة من الشهادات أو أعد نشر الموقع.",
        ),
        "certificate-expiring" => (
            "شهادة على وشك الانتهاء: {domain}",
            "شهادة {domain} على وشك الانتهاء (تنتهي في {not_after}).",
            "جدّد الشهادة من الشهادات أو أعد نشر الموقع.",
        ),
        "firewall-blocks" => (
            "عدد كبير من عناوين IP المحظورة",
            "هناك {blocked} عنوان IP محظور حالياً.",
            "تحقّق من وجود هجوم موزّع وراجع قواعد fail2ban.",
        ),
        _ => return None,
    })
}

fn ru(code: &str) -> Option<Triple> {
    Some(match code {
        "auth-failures" => (
            "Повышенное число неудачных входов",
            "{failures} неудачных попыток аутентификации за последние 24 часа.",
            "Проверьте раздел «Безопасность», заблокируйте подозрительные IP и проверьте fail2ban.",
        ),
        "auth-blocked" => (
            "Блокировки аутентификации",
            "{blocked} попыток входа заблокировано за последние 24 часа.",
            "Проверьте, вызваны ли блокировки брутфорсом или легитимными пользователями.",
        ),
        "service-failed" => (
            "Служба не работает: {name}",
            "Юнит {unit} работает некорректно (состояние: {status}) и может влиять на хостинг.",
            "Проверьте состояние и журнал службы; перезапустите её из «Программ» при необходимости.",
        ),
        "mail-filter-missing" => (
            "Неполные почтовые фильтры",
            "Отсутствуют компоненты антивируса/антиспама: {missing}.",
            "Переустановите почтовый стек с помощью установщика или из «Программ».",
        ),
        "mail-filter-inactive" => (
            "Почтовый фильтр остановлен: {name}",
            "{name} установлен, но юнит сообщает «{status}».",
            "Перезапустите компонент в разделе «Антивирус и антиспам» или «Программы».",
        ),
        "antivirus-detection" => (
            "Обнаружено вредоносное ПО в {service}",
            "{message}",
            "Изолируйте файл, проверьте затронутый ящик или аккаунт и запустите полную проверку.",
        ),
        "antispam-volume" => (
            "Высокий объём недавнего спама",
            "{spam} сообщений помечено или отклонено SpamAssassin в недавнем журнале.",
            "Проверьте правила SpamAssassin, объём почты и наличие спам-кампании.",
        ),
        "security-updates" => (
            "Ожидаются обновления безопасности",
            "Ожидается {count} обновлений, помеченных как безопасность.",
            "Примените обновления из «Программ» в окно обслуживания.",
        ),
        "disk-usage" => (
            "Высокое использование диска",
            "Диск заполнен на {percent}% и может переполниться.",
            "Проверьте резервные копии, журналы и наиболее требовательные аккаунты до заполнения диска.",
        ),
        "memory-usage" => (
            "Высокое использование памяти",
            "Использование памяти: {percent}%.",
            "Проверьте процессы, контейнеры и лимиты приложений до срабатывания OOM killer.",
        ),
        "load-average" => (
            "Высокая нагрузка на систему",
            "Load 1m на CPU: {per_cpu}.",
            "Определите процессы с высокой нагрузкой CPU и проверьте управляемые приложения.",
        ),
        "certificate-expired" => (
            "Срок сертификата истёк: {domain}",
            "Срок действия сертификата {domain} истёк (до {not_after}).",
            "Обновите сертификат в разделе «Сертификаты» или опубликуйте сайт заново.",
        ),
        "certificate-expiring" => (
            "Сертификат скоро истекает: {domain}",
            "Срок действия сертификата {domain} скоро истекает (до {not_after}).",
            "Обновите сертификат в разделе «Сертификаты» или опубликуйте сайт заново.",
        ),
        "firewall-blocks" => (
            "Много заблокированных IP",
            "Сейчас заблокировано {blocked} IP-адресов.",
            "Проверьте наличие распределённой атаки и пересмотрите правила fail2ban.",
        ),
        _ => return None,
    })
}

fn zh(code: &str) -> Option<Triple> {
    Some(match code {
        "auth-failures" => (
            "登录失败次数偏高",
            "过去 24 小时内有 {failures} 次身份验证失败。",
            "请检查“安全”页面，封禁可疑 IP，并检查 fail2ban。",
        ),
        "auth-blocked" => (
            "身份验证锁定",
            "过去 24 小时内有 {blocked} 次登录被阻止。",
            "请确认锁定来自暴力破解还是正常用户。",
        ),
        "service-failed" => (
            "服务已停止：{name}",
            "单元 {unit} 运行不正常（状态：{status}），可能影响主机服务。",
            "请检查服务状态和日志；如需要，在“程序”中重启。",
        ),
        "mail-filter-missing" => (
            "邮件过滤器不完整",
            "缺少防病毒/反垃圾邮件组件：{missing}。",
            "请使用安装程序或在“程序”中重新安装邮件组件。",
        ),
        "mail-filter-inactive" => (
            "邮件过滤器已停止：{name}",
            "{name} 已安装，但单元报告“{status}”。",
            "请在“防病毒与反垃圾邮件”或“程序”中重启该组件。",
        ),
        "antivirus-detection" => (
            "在 {service} 中检测到恶意软件",
            "{message}",
            "请隔离该文件，检查受影响的邮箱或账户，并运行完整扫描。",
        ),
        "antispam-volume" => (
            "近期垃圾邮件量偏高",
            "最近的日志中有 {spam} 封邮件被 SpamAssassin 标记或拒绝。",
            "请检查 SpamAssassin 规则、邮件量以及是否存在垃圾邮件活动。",
        ),
        "security-updates" => (
            "有待处理的安全更新",
            "有 {count} 个标记为安全更新的更新待处理。",
            "请在维护窗口内从“程序”应用更新。",
        ),
        "disk-usage" => (
            "磁盘使用率偏高",
            "磁盘使用率为 {percent}%，可能会被写满。",
            "请在磁盘写满前检查备份、日志和占用最高的账户。",
        ),
        "memory-usage" => (
            "内存使用率偏高",
            "内存使用率为 {percent}%。",
            "请在 OOM killer 介入前检查进程、容器和应用限制。",
        ),
        "load-average" => (
            "系统负载偏高",
            "每 CPU 的 1 分钟负载：{per_cpu}。",
            "请找出占用 CPU 较高的进程，并检查受管应用。",
        ),
        "certificate-expired" => (
            "证书已过期：{domain}",
            "{domain} 的证书已过期（到期时间 {not_after}）。",
            "请在“证书”中续期，或重新发布站点。",
        ),
        "certificate-expiring" => (
            "证书即将过期：{domain}",
            "{domain} 的证书即将过期（到期时间 {not_after}）。",
            "请在“证书”中续期，或重新发布站点。",
        ),
        "firewall-blocks" => (
            "大量 IP 被封锁",
            "当前有 {blocked} 个 IP 地址被封锁。",
            "请检查是否存在分布式攻击，并复查 fail2ban 规则。",
        ),
        _ => return None,
    })
}

fn ja(code: &str) -> Option<Triple> {
    Some(match code {
        "auth-failures" => (
            "ログイン失敗の増加",
            "過去24時間で {failures} 回の認証失敗がありました。",
            "「セキュリティ」を確認し、不審な IP をブロックして fail2ban を確認してください。",
        ),
        "auth-blocked" => (
            "認証ロックアウト",
            "過去24時間で {blocked} 回のログインがブロックされました。",
            "ブルートフォースか正規ユーザーかを確認してください。",
        ),
        "service-failed" => (
            "サービス停止: {name}",
            "ユニット {unit} が正常に動作していません（状態: {status}）。ホスティングに影響する可能性があります。",
            "サービスの状態とジャーナルを確認し、必要なら「プログラム」から再起動してください。",
        ),
        "mail-filter-missing" => (
            "メールフィルターが不完全",
            "ウイルス対策/スパム対策コンポーネントが不足しています: {missing}。",
            "インストーラーまたは「プログラム」からメールスタックを再インストールしてください。",
        ),
        "mail-filter-inactive" => (
            "メールフィルター停止: {name}",
            "{name} はインストール済みですが、ユニットは「{status}」を報告しています。",
            "「ウイルス対策とスパム対策」または「プログラム」から再起動してください。",
        ),
        "antivirus-detection" => (
            "{service} でマルウェアを検出",
            "{message}",
            "ファイルを隔離し、影響を受けたメールボックスまたはアカウントを確認して、完全スキャンを実行してください。",
        ),
        "antispam-volume" => (
            "最近のスパム量が多い",
            "最近のジャーナルで {spam} 件のメッセージが SpamAssassin によりタグ付けまたは拒否されました。",
            "SpamAssassin のルール、メール量、スパムキャンペーンの有無を確認してください。",
        ),
        "security-updates" => (
            "セキュリティ更新が保留中",
            "セキュリティ更新としてマークされた {count} 件の更新が保留中です。",
            "メンテナンス時間に「プログラム」から更新を適用してください。",
        ),
        "disk-usage" => (
            "ディスク使用率が高い",
            "ディスク使用率は {percent}% で、満杯になる可能性があります。",
            "ディスクが満杯になる前に、バックアップ、ログ、使用量の多いアカウントを確認してください。",
        ),
        "memory-usage" => (
            "メモリ使用率が高い",
            "メモリ使用率は {percent}% です。",
            "OOM killer が作動する前に、プロセス、コンテナ、アプリケーションの制限を確認してください。",
        ),
        "load-average" => (
            "システム負荷が高い",
            "CPU あたりの 1 分負荷: {per_cpu}。",
            "CPU を多く使うプロセスを特定し、管理対象アプリケーションを確認してください。",
        ),
        "certificate-expired" => (
            "証明書の有効期限切れ: {domain}",
            "{domain} の証明書は期限切れです（有効期限 {not_after}）。",
            "「証明書」から更新するか、サイトを再公開してください。",
        ),
        "certificate-expiring" => (
            "証明書の期限が近い: {domain}",
            "{domain} の証明書はまもなく期限切れです（有効期限 {not_after}）。",
            "「証明書」から更新するか、サイトを再公開してください。",
        ),
        "firewall-blocks" => (
            "ブロックされた IP が多い",
            "現在 {blocked} 個の IP アドレスがブロックされています。",
            "分散攻撃の可能性を確認し、fail2ban のルールを見直してください。",
        ),
        _ => return None,
    })
}

fn fill(template: &str, params: &Value) -> String {
    let mut output = template.to_string();
    if let Some(map) = params.as_object() {
        for (key, value) in map {
            let rendered = match value {
                Value::String(text) => text.clone(),
                other => other.to_string(),
            };
            output = output.replace(&format!("{{{key}}}"), &rendered);
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn renders_placeholders() {
        let text = english("auth-failures", &json!({ "failures": 42, "unique_ips": 3 }))
            .expect("template");
        assert!(text.summary.contains("42"));
        assert!(text.title.contains("failed"));
    }

    #[test]
    fn unknown_codes_have_no_template() {
        assert!(english("unknown", &json!({})).is_none());
    }

    #[test]
    fn labels_and_subject_follow_locale() {
        assert!(email_subject("es", 2).contains("alerta"));
        assert!(email_subject("fr", 2).contains("alerte"));
        assert!(email_subject("ja", 2).contains("アラート"));
        assert_eq!(email_labels("es").0, "Recomendación");
        assert_eq!(email_labels("fr").0, "Recommandation");
        assert_eq!(email_labels("ar").0, "التوصية");
        assert_eq!(email_labels("unknown").0, "Recommendation");
    }

    #[test]
    fn localized_templates_render() {
        let text = localized("fr", "disk-usage", &json!({ "percent": 91.2 })).expect("template");
        assert!(text.title.contains("disque"));
        assert!(text.summary.contains("91.2"));
        let text = localized(
            "ja",
            "service-failed",
            &json!({ "name": "nginx", "unit": "nginx", "status": "inactive" }),
        )
        .expect("template");
        assert!(text.title.contains("nginx"));
        assert!(text.summary.contains("nginx"));
    }
}
