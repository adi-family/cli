# ============================================================================
# ADI WORKFLOW - GERMAN TRANSLATIONS (Deutsch)
# ============================================================================

# Help and descriptions
workflow-description = Workflows ausführen, die in TOML-Dateien definiert sind
workflow-help-title = ADI Workflow - Workflows aus TOML-Dateien ausführen
workflow-help-commands = Befehle:
workflow-help-run = Workflow nach Name ausführen
workflow-help-list = Verfügbare Workflows auflisten
workflow-help-show = Workflow-Definition anzeigen
workflow-help-locations = Workflow-Speicherorte:
workflow-help-local = (lokal, höchste Priorität)
workflow-help-global = (global)
workflow-help-usage = Verwendung:

# List command
workflow-list-title = Verfügbare Workflows:
workflow-list-empty = Keine Workflows gefunden.
workflow-list-hint-create = Workflows erstellen in:
workflow-list-scope-local = [lokal]
workflow-list-scope-global = [global]

# Show command
workflow-show-title = Workflow: { $name }
workflow-show-description = Beschreibung: { $description }
workflow-show-path = Pfad: { $path }
workflow-show-inputs = Eingaben:
workflow-show-input-options = Optionen: { $options }
workflow-show-input-default = Standard: { $default }
workflow-show-steps = Schritte:
workflow-show-step-if = wenn: { $condition }
workflow-show-step-run = ausführen: { $command }
workflow-show-error-missing-name = Workflow-Name fehlt. Verwendung: show <name>
workflow-show-error-not-found = Workflow '{ $name }' nicht gefunden

# Run command
workflow-run-title = Workflow wird ausgeführt: { $name }
workflow-run-collecting-inputs = Eingaben werden gesammelt...
workflow-run-executing-steps = Schritte werden ausgeführt...
workflow-run-step-running = Schritt { $number } wird ausgeführt: { $name }
workflow-run-step-skipping = Schritt { $number } wird übersprungen: { $name } (Bedingung nicht erfüllt)
workflow-run-success = Workflow '{ $name }' erfolgreich abgeschlossen!
workflow-run-error-not-found = Workflow '{ $name }' nicht gefunden
workflow-run-error-no-steps = Workflow hat keine auszuführenden Schritte

# Input prompts
workflow-input-error-tty = Interaktive Eingabeaufforderungen benötigen ein TTY
workflow-input-error-options = { $type }-Eingabe benötigt Optionen
workflow-input-error-options-empty = { $type }-Eingabe benötigt mindestens eine Option
workflow-input-error-validation = Ungültiges Validierungsmuster: { $error }
workflow-input-error-prompt = Eingabeaufforderungsfehler: { $error }
workflow-input-validation-failed = Eingabe muss dem Muster entsprechen: { $pattern }

# Execution
workflow-exec-error-spawn = Befehl konnte nicht gestartet werden: { $error }
workflow-exec-error-wait = Auf Befehl konnte nicht gewartet werden: { $error }
workflow-exec-error-exit-code = Befehl fehlgeschlagen mit Exit-Code: { $code }
workflow-exec-error-template = Template-Fehler: { $error }

# Common
workflow-common-error-parse = Workflow konnte nicht geparst werden: { $error }
workflow-common-error-read = Workflow-Datei konnte nicht gelesen werden: { $error }
