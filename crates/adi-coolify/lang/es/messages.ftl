# ADI Coolify Plugin - Traducciones en Español

# Comandos
cmd-status = Mostrar estado de todos los servicios
cmd-deploy = Desplegar un servicio
cmd-watch = Observar el progreso del despliegue
cmd-logs = Mostrar registros de despliegue
cmd-list = Listar despliegues recientes
cmd-services = Listar servicios disponibles
cmd-config = Mostrar configuración actual
cmd-config-set = Establecer un valor de configuración

# Ayuda
help-title = ADI Coolify - Gestión de Despliegues
help-commands = Comandos
help-services = Servicios
help-config = Configuración
help-usage = Uso: adi coolify <comando> [argumentos]

# Nombres de servicios
svc-auth = API de Autenticación
svc-platform = API de Plataforma
svc-signaling = Servidor de Señalización
svc-web = Interfaz Web
svc-analytics-ingestion = Ingesta de Analíticas
svc-analytics = API de Analíticas
svc-registry = Registro de Plugins

# Estado
status-title = Estado de Despliegue ADI
status-service = SERVICIO
status-name = NOMBRE
status-status = ESTADO
status-healthy = saludable
status-unhealthy = no saludable
status-unknown = desconocido
status-building = construyendo
status-running = ejecutando
status-queued = en cola
status-finished = terminado
status-failed = fallido
status-error = error

# Despliegue
deploy-starting = Desplegando servicios...
deploy-started = Iniciado
deploy-failed = Fallido
deploy-uuid = UUIDs de Despliegue
deploy-use-watch = Use 'adi coolify watch <servicio>' para monitorear el progreso
deploy-service-required = Se requiere nombre del servicio. Uso: deploy <servicio|all> [--force]
deploy-unknown-service = Servicio desconocido '{ $service }'. Disponibles: { $available }

# Observar
watch-title = Observando despliegues de { $service }...
watch-latest = Último despliegue
watch-uuid = UUID
watch-status = Estado
watch-commit = Commit
watch-no-deployments = No se encontraron despliegues para { $service }
watch-live-tip = Nota: Para observación en vivo, use: adi workflow deploy { $service }
watch-service-required = Se requiere nombre del servicio. Uso: watch <servicio>

# Registros
logs-title = Registros de despliegue para { $service }
logs-deployment = Despliegue
logs-no-logs = No hay registros disponibles
logs-service-required = Se requiere nombre del servicio. Uso: logs <servicio>

# Lista
list-title = Despliegues recientes para { $service }
list-created = CREADO
list-commit = COMMIT
list-service-required = Se requiere nombre del servicio. Uso: list <servicio> [cantidad]

# Servicios
services-title = Servicios Disponibles
services-id = ID
services-uuid = UUID

# Configuración
config-title = Configuración de ADI Coolify
config-current = Valores Actuales
config-files = Archivos de Configuración
config-user = Usuario
config-project = Proyecto
config-env-vars = Variables de Entorno
config-set-usage = Establecer configuración
config-encryption = Cifrado
config-encrypted-at-rest = (secreto, cifrado en reposo)
config-encrypted = (cifrado)
config-not-set = (no establecido)
config-unavailable = (no disponible)
config-no-project = (sin proyecto)
config-encryption-algo = Los secretos se cifran usando ChaCha20-Poly1305.
config-master-key = Clave maestra almacenada en: ~/.config/adi/secrets.key

# Establecer configuración
config-set-success = Establecido { $key } = { $value } en configuración de { $level }
config-set-file = Archivo: { $path }
config-set-usage-full = Uso: config set <clave> <valor> [--user|--project]
config-unknown-key = Clave de configuración desconocida: '{ $key }'. Claves válidas: url, api_key
config-no-project-dir = Directorio de proyecto no establecido. Ejecute desde un directorio de proyecto.
config-save-failed = Error al guardar configuración: { $error }

# Errores
error-api-key-not-set = Clave API no configurada. Configure mediante:
error-api-key-env = - Variable de entorno: ADI_PLUGIN_ADI_COOLIFY_API_KEY=<clave>
error-api-key-user = - Config de usuario: adi coolify config set api_key <clave>
error-api-key-project = - Config de proyecto: adi coolify config set api_key <clave> --project
error-request-failed = Solicitud fallida: { $error }
error-json-parse = Error de análisis JSON: { $error }
error-unknown-command = Comando desconocido: { $command }
error-invalid-context = Contexto inválido: { $error }
error-invalid-response = Formato de respuesta inválido
error-no-deployment-uuid = Sin UUID de despliegue
error-unknown-service = Servicio desconocido: { $service }
