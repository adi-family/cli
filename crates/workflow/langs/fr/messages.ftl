# ============================================================================
# ADI WORKFLOW - FRENCH TRANSLATIONS (Français)
# ============================================================================

# Help and descriptions
workflow-description = Exécuter des workflows définis dans des fichiers TOML
workflow-help-title = ADI Workflow - Exécuter des workflows définis dans des fichiers TOML
workflow-help-commands = Commandes :
workflow-help-run = Exécuter un workflow par son nom
workflow-help-list = Lister les workflows disponibles
workflow-help-show = Afficher la définition du workflow
workflow-help-locations = Emplacements des workflows :
workflow-help-local = (local, priorité la plus haute)
workflow-help-global = (global)
workflow-help-usage = Utilisation :

# List command
workflow-list-title = Workflows disponibles :
workflow-list-empty = Aucun workflow trouvé.
workflow-list-hint-create = Créer des workflows dans :
workflow-list-scope-local = [local]
workflow-list-scope-global = [global]

# Show command
workflow-show-title = Workflow : { $name }
workflow-show-description = Description : { $description }
workflow-show-path = Chemin : { $path }
workflow-show-inputs = Entrées :
workflow-show-input-options = Options : { $options }
workflow-show-input-default = Par défaut : { $default }
workflow-show-steps = Étapes :
workflow-show-step-if = si : { $condition }
workflow-show-step-run = exécuter : { $command }
workflow-show-error-missing-name = Nom du workflow manquant. Utilisation : show <nom>
workflow-show-error-not-found = Workflow '{ $name }' non trouvé

# Run command
workflow-run-title = Exécution du workflow : { $name }
workflow-run-collecting-inputs = Collecte des entrées...
workflow-run-executing-steps = Exécution des étapes...
workflow-run-step-running = Exécution de l'étape { $number } : { $name }
workflow-run-step-skipping = Saut de l'étape { $number } : { $name } (condition non remplie)
workflow-run-success = Workflow '{ $name }' terminé avec succès !
workflow-run-error-not-found = Workflow '{ $name }' non trouvé
workflow-run-error-no-steps = Le workflow n'a pas d'étapes à exécuter

# Input prompts
workflow-input-error-tty = Les prompts interactifs nécessitent un TTY
workflow-input-error-options = L'entrée { $type } nécessite des options
workflow-input-error-options-empty = L'entrée { $type } nécessite au moins une option
workflow-input-error-validation = Motif de validation invalide : { $error }
workflow-input-error-prompt = Erreur de prompt : { $error }
workflow-input-validation-failed = L'entrée doit correspondre au motif : { $pattern }

# Execution
workflow-exec-error-spawn = Impossible de lancer la commande : { $error }
workflow-exec-error-wait = Impossible d'attendre la commande : { $error }
workflow-exec-error-exit-code = La commande a échoué avec le code de sortie : { $code }
workflow-exec-error-template = Erreur de template : { $error }

# Common
workflow-common-error-parse = Impossible d'analyser le workflow : { $error }
workflow-common-error-read = Impossible de lire le fichier workflow : { $error }
