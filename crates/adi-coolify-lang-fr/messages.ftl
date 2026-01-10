# ADI Coolify Plugin - Traductions Françaises

# Commandes
cmd-status = Afficher l'état de tous les services
cmd-deploy = Déployer un service
cmd-watch = Surveiller la progression du déploiement
cmd-logs = Afficher les journaux de déploiement
cmd-list = Lister les déploiements récents
cmd-services = Lister les services disponibles
cmd-config = Afficher la configuration actuelle
cmd-config-set = Définir une valeur de configuration

# Aide
help-title = ADI Coolify - Gestion des Déploiements
help-commands = Commandes
help-services = Services
help-config = Configuration
help-usage = Utilisation: adi coolify <commande> [arguments]

# Noms des services
svc-auth = API d'Authentification
svc-platform = API de Plateforme
svc-signaling = Serveur de Signalisation
svc-web = Interface Web
svc-analytics-ingestion = Ingestion d'Analytiques
svc-analytics = API d'Analytiques
svc-registry = Registre de Plugins

# État
status-title = État de Déploiement ADI
status-service = SERVICE
status-name = NOM
status-status = ÉTAT
status-healthy = sain
status-unhealthy = non sain
status-unknown = inconnu
status-building = en construction
status-running = en cours
status-queued = en file d'attente
status-finished = terminé
status-failed = échoué
status-error = erreur

# Déploiement
deploy-starting = Déploiement des services...
deploy-started = Démarré
deploy-failed = Échoué
deploy-uuid = UUIDs de Déploiement
deploy-use-watch = Utilisez 'adi coolify watch <service>' pour surveiller la progression
deploy-service-required = Nom du service requis. Utilisation: deploy <service|all> [--force]
deploy-unknown-service = Service inconnu '{ $service }'. Disponibles: { $available }

# Surveillance
watch-title = Surveillance des déploiements de { $service }...
watch-latest = Dernier déploiement
watch-uuid = UUID
watch-status = État
watch-commit = Commit
watch-no-deployments = Aucun déploiement trouvé pour { $service }
watch-live-tip = Note: Pour une surveillance en direct, utilisez: ./scripts/deploy.sh watch { $service }
watch-service-required = Nom du service requis. Utilisation: watch <service>

# Journaux
logs-title = Journaux de déploiement pour { $service }
logs-deployment = Déploiement
logs-no-logs = Aucun journal disponible
logs-service-required = Nom du service requis. Utilisation: logs <service>

# Liste
list-title = Déploiements récents pour { $service }
list-created = CRÉÉ
list-commit = COMMIT
list-service-required = Nom du service requis. Utilisation: list <service> [nombre]

# Services
services-title = Services Disponibles
services-id = ID
services-uuid = UUID

# Configuration
config-title = Configuration ADI Coolify
config-current = Valeurs Actuelles
config-files = Fichiers de Configuration
config-user = Utilisateur
config-project = Projet
config-env-vars = Variables d'Environnement
config-set-usage = Définir la configuration
config-encryption = Chiffrement
config-encrypted-at-rest = (secret, chiffré au repos)
config-encrypted = (chiffré)
config-not-set = (non défini)
config-unavailable = (indisponible)
config-no-project = (pas de projet)
config-encryption-algo = Les secrets sont chiffrés avec ChaCha20-Poly1305.
config-master-key = Clé maître stockée dans: ~/.config/adi/secrets.key

# Définir la configuration
config-set-success = Défini { $key } = { $value } dans la configuration { $level }
config-set-file = Fichier: { $path }
config-set-usage-full = Utilisation: config set <clé> <valeur> [--user|--project]
config-unknown-key = Clé de configuration inconnue: '{ $key }'. Clés valides: url, api_key
config-no-project-dir = Répertoire de projet non défini. Exécutez depuis un répertoire de projet.
config-save-failed = Échec de sauvegarde de la configuration: { $error }

# Erreurs
error-api-key-not-set = Clé API non configurée. Configurez via:
error-api-key-env = - Variable d'environnement: ADI_PLUGIN_ADI_COOLIFY_API_KEY=<clé>
error-api-key-user = - Config utilisateur: adi coolify config set api_key <clé>
error-api-key-project = - Config projet: adi coolify config set api_key <clé> --project
error-request-failed = Requête échouée: { $error }
error-json-parse = Erreur d'analyse JSON: { $error }
error-unknown-command = Commande inconnue: { $command }
error-invalid-context = Contexte invalide: { $error }
error-invalid-response = Format de réponse invalide
error-no-deployment-uuid = Pas d'UUID de déploiement
error-unknown-service = Service inconnu: { $service }
