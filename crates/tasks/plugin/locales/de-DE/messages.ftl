# ============================================================================
# ADI TASKS - DEUTSCHE ÜBERSETZUNGEN
# ============================================================================

# Plugin-Metadaten
plugin-name = Aufgaben
plugin-description = Aufgabenverwaltung mit Abhängigkeitsverfolgung

# Befehlsbeschreibungen
cmd-list-help = Alle Aufgaben auflisten
cmd-add-help = Neue Aufgabe hinzufügen
cmd-show-help = Aufgabendetails anzeigen
cmd-status-help = Aufgabenstatus aktualisieren
cmd-delete-help = Aufgabe löschen
cmd-depend-help = Abhängigkeit hinzufügen
cmd-undepend-help = Abhängigkeit entfernen
cmd-graph-help = Abhängigkeitsgraph anzeigen
cmd-search-help = Aufgaben suchen
cmd-blocked-help = Blockierte Aufgaben anzeigen
cmd-cycles-help = Zyklische Abhängigkeiten erkennen
cmd-stats-help = Aufgabenstatistik anzeigen

# Hilfetext
tasks-help-title = ADI Aufgaben - Aufgabenverwaltung mit Abhängigkeitsverfolgung
tasks-help-commands = Befehle:
tasks-help-usage = Verwendung: adi tasks <Befehl> [Argumente]

# Listenbefehl
tasks-list-empty = Keine Aufgaben gefunden
tasks-list-scope-global = [global]
tasks-list-scope-project = [Projekt]

# Hinzufügen-Befehl
tasks-add-missing-title = Titel fehlt. Verwendung: add <Titel> [--description <Beschreibung>]
tasks-add-created = Aufgabe #{ $id } erstellt: { $title }

# Anzeigen-Befehl
tasks-show-missing-id = Aufgaben-ID fehlt. Verwendung: show <id>
tasks-show-invalid-id = Ungültige Aufgaben-ID
tasks-show-title = Aufgabe #{ $id }
tasks-show-field-title = Titel: { $title }
tasks-show-field-status = Status: { $status }
tasks-show-field-description = Beschreibung: { $description }
tasks-show-field-symbol = Verknüpftes Symbol: #{ $symbol_id }
tasks-show-field-scope = Bereich: { $scope }
tasks-show-dependencies = Abhängigkeiten:
tasks-show-dependents = Abhängige:

# Status-Befehl
tasks-status-missing-args = Argumente fehlen. Verwendung: status <id> <status>
tasks-status-invalid-id = Ungültige Aufgaben-ID
tasks-status-invalid-status = Ungültiger Status: { $status }. Gültig: todo, in-progress, done, blocked, cancelled
tasks-status-updated = Status von Aufgabe #{ $id } auf { $status } aktualisiert

# Löschen-Befehl
tasks-delete-missing-id = Aufgaben-ID fehlt. Verwendung: delete <id> [--force]
tasks-delete-invalid-id = Ungültige Aufgaben-ID
tasks-delete-confirm = Aufgabe #{ $id } löschen: { $title }?
tasks-delete-confirm-hint = Verwenden Sie --force zur Bestätigung
tasks-delete-success = Aufgabe #{ $id } gelöscht: { $title }

# Abhängigkeit-Befehl
tasks-depend-missing-args = Argumente fehlen. Verwendung: depend <Aufgaben-ID> <Abhängigkeits-ID>
tasks-depend-invalid-task-id = Ungültige Aufgaben-ID
tasks-depend-invalid-depends-id = Ungültige Abhängigkeits-ID
tasks-depend-success = Aufgabe #{ $task_id } hängt jetzt von Aufgabe #{ $depends_on } ab

# Abhängigkeit-entfernen-Befehl
tasks-undepend-missing-args = Argumente fehlen. Verwendung: undepend <Aufgaben-ID> <Abhängigkeits-ID>
tasks-undepend-invalid-task-id = Ungültige Aufgaben-ID
tasks-undepend-invalid-depends-id = Ungültige Abhängigkeits-ID
tasks-undepend-success = Abhängigkeit entfernt: #{ $task_id } -> #{ $depends_on }

# Graph-Befehl
tasks-graph-title = Aufgaben-Abhängigkeitsgraph
tasks-graph-empty = Keine Aufgaben gefunden
tasks-graph-depends-on = hängt ab von #{ $id }: { $title }

# Such-Befehl
tasks-search-missing-query = Suchanfrage fehlt. Verwendung: search <Anfrage> [--limit <n>]
tasks-search-empty = Keine Aufgaben gefunden
tasks-search-results = { $count } Ergebnisse für "{ $query }" gefunden:

# Blockiert-Befehl
tasks-blocked-empty = Keine blockierten Aufgaben
tasks-blocked-title = Blockierte Aufgaben
tasks-blocked-by = blockiert von #{ $id }: { $title } ({ $status })

# Zyklen-Befehl
tasks-cycles-empty = Keine zyklischen Abhängigkeiten gefunden
tasks-cycles-found = { $count } zyklische Abhängigkeiten gefunden:
tasks-cycles-item = Zyklus { $number }:

# Statistik-Befehl
tasks-stats-title = Aufgabenstatistik
tasks-stats-total = Aufgaben gesamt: { $count }
tasks-stats-todo = Zu erledigen: { $count }
tasks-stats-in-progress = In Bearbeitung: { $count }
tasks-stats-done = Erledigt: { $count }
tasks-stats-blocked = Blockiert: { $count }
tasks-stats-cancelled = Abgebrochen: { $count }
tasks-stats-dependencies = Abhängigkeiten: { $count }
tasks-stats-cycles-yes = Zyklen: Ja (führen Sie 'cycles' aus, um sie zu sehen)
tasks-stats-cycles-no = Zyklen: Keine

# Fehler
error-not-initialized = Aufgaben nicht initialisiert
error-task-not-found = Aufgabe { $id } nicht gefunden
