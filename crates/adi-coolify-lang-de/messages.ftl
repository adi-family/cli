# ADI Coolify Plugin - Deutsche Übersetzungen

# Befehle
cmd-status = Status aller Dienste anzeigen
cmd-deploy = Einen Dienst bereitstellen
cmd-watch = Bereitstellungsfortschritt überwachen
cmd-logs = Bereitstellungsprotokolle anzeigen
cmd-list = Letzte Bereitstellungen auflisten
cmd-services = Verfügbare Dienste auflisten
cmd-config = Aktuelle Konfiguration anzeigen
cmd-config-set = Einen Konfigurationswert setzen

# Hilfe
help-title = ADI Coolify - Bereitstellungsverwaltung
help-commands = Befehle
help-services = Dienste
help-config = Konfiguration
help-usage = Verwendung: adi coolify <befehl> [argumente]

# Dienstnamen
svc-auth = Authentifizierungs-API
svc-platform = Plattform-API
svc-signaling = Signalisierungsserver
svc-web = Web-Oberfläche
svc-analytics-ingestion = Analytik-Aufnahme
svc-analytics = Analytik-API
svc-registry = Plugin-Registry

# Status
status-title = ADI Bereitstellungsstatus
status-service = DIENST
status-name = NAME
status-status = STATUS
status-healthy = gesund
status-unhealthy = ungesund
status-unknown = unbekannt
status-building = wird erstellt
status-running = läuft
status-queued = in Warteschlange
status-finished = abgeschlossen
status-failed = fehlgeschlagen
status-error = Fehler

# Bereitstellung
deploy-starting = Dienste werden bereitgestellt...
deploy-started = Gestartet
deploy-failed = Fehlgeschlagen
deploy-uuid = Bereitstellungs-UUIDs
deploy-use-watch = Verwenden Sie 'adi coolify watch <dienst>' um den Fortschritt zu überwachen
deploy-service-required = Dienstname erforderlich. Verwendung: deploy <dienst|all> [--force]
deploy-unknown-service = Unbekannter Dienst '{ $service }'. Verfügbar: { $available }

# Überwachung
watch-title = Überwache Bereitstellungen von { $service }...
watch-latest = Letzte Bereitstellung
watch-uuid = UUID
watch-status = Status
watch-commit = Commit
watch-no-deployments = Keine Bereitstellungen für { $service } gefunden
watch-live-tip = Hinweis: Für Live-Überwachung verwenden Sie: ./scripts/deploy.sh watch { $service }
watch-service-required = Dienstname erforderlich. Verwendung: watch <dienst>

# Protokolle
logs-title = Bereitstellungsprotokolle für { $service }
logs-deployment = Bereitstellung
logs-no-logs = Keine Protokolle verfügbar
logs-service-required = Dienstname erforderlich. Verwendung: logs <dienst>

# Liste
list-title = Letzte Bereitstellungen für { $service }
list-created = ERSTELLT
list-commit = COMMIT
list-service-required = Dienstname erforderlich. Verwendung: list <dienst> [anzahl]

# Dienste
services-title = Verfügbare Dienste
services-id = ID
services-uuid = UUID

# Konfiguration
config-title = ADI Coolify Konfiguration
config-current = Aktuelle Werte
config-files = Konfigurationsdateien
config-user = Benutzer
config-project = Projekt
config-env-vars = Umgebungsvariablen
config-set-usage = Konfiguration setzen
config-encryption = Verschlüsselung
config-encrypted-at-rest = (Geheimnis, verschlüsselt gespeichert)
config-encrypted = (verschlüsselt)
config-not-set = (nicht gesetzt)
config-unavailable = (nicht verfügbar)
config-no-project = (kein Projekt)
config-encryption-algo = Geheimnisse werden mit ChaCha20-Poly1305 verschlüsselt.
config-master-key = Hauptschlüssel gespeichert in: ~/.config/adi/secrets.key

# Konfiguration setzen
config-set-success = { $key } = { $value } in { $level }-Konfiguration gesetzt
config-set-file = Datei: { $path }
config-set-usage-full = Verwendung: config set <schlüssel> <wert> [--user|--project]
config-unknown-key = Unbekannter Konfigurationsschlüssel: '{ $key }'. Gültige Schlüssel: url, api_key
config-no-project-dir = Kein Projektverzeichnis gesetzt. Führen Sie den Befehl aus einem Projektverzeichnis aus.
config-save-failed = Konfiguration konnte nicht gespeichert werden: { $error }

# Fehler
error-api-key-not-set = API-Schlüssel nicht konfiguriert. Konfigurieren Sie über:
error-api-key-env = - Umgebungsvariable: ADI_PLUGIN_ADI_COOLIFY_API_KEY=<schlüssel>
error-api-key-user = - Benutzer-Konfiguration: adi coolify config set api_key <schlüssel>
error-api-key-project = - Projekt-Konfiguration: adi coolify config set api_key <schlüssel> --project
error-request-failed = Anfrage fehlgeschlagen: { $error }
error-json-parse = JSON-Analysefehler: { $error }
error-unknown-command = Unbekannter Befehl: { $command }
error-invalid-context = Ungültiger Kontext: { $error }
error-invalid-response = Ungültiges Antwortformat
error-no-deployment-uuid = Keine Bereitstellungs-UUID
error-unknown-service = Unbekannter Dienst: { $service }
