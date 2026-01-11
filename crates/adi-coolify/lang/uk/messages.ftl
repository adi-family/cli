# ADI Coolify Plugin - Ukrainian Translations

# Commands
cmd-status = Показати статус усіх сервісів
cmd-deploy = Розгорнути сервіс
cmd-watch = Спостерігати за процесом розгортання
cmd-logs = Показати логи розгортання
cmd-list = Показати останні розгортання
cmd-services = Показати доступні сервіси
cmd-config = Показати поточну конфігурацію
cmd-config-set = Встановити значення конфігурації

# Help
help-title = ADI Coolify - Керування розгортанням
help-commands = Команди
help-services = Сервіси
help-config = Конфігурація
help-usage = Використання: adi coolify <команда> [аргументи]

# Service names
svc-auth = API автентифікації
svc-platform = API платформи
svc-signaling = Сервер сигналізації
svc-web = Веб-інтерфейс
svc-analytics-ingestion = Збір аналітики
svc-analytics = API аналітики
svc-registry = Реєстр плагінів

# Status
status-title = Статус розгортання ADI
status-service = СЕРВІС
status-name = НАЗВА
status-status = СТАТУС
status-healthy = справний
status-unhealthy = несправний
status-unknown = невідомо
status-building = збірка
status-running = працює
status-queued = в черзі
status-finished = завершено
status-failed = невдача
status-error = помилка

# Deploy
deploy-starting = Розгортання сервісів...
deploy-started = Розпочато
deploy-failed = Невдача
deploy-uuid = UUID розгортань
deploy-use-watch = Використовуйте 'adi coolify watch <сервіс>' для моніторингу
deploy-service-required = Потрібна назва сервісу. Використання: deploy <сервіс|all> [--force]
deploy-unknown-service = Невідомий сервіс '{ $service }'. Доступні: { $available }

# Watch
watch-title = Спостереження за розгортаннями { $service }...
watch-latest = Останнє розгортання
watch-uuid = UUID
watch-status = Статус
watch-commit = Коміт
watch-no-deployments = Розгортань не знайдено для { $service }
watch-live-tip = Примітка: Для живого спостереження використовуйте: adi workflow deploy { $service }
watch-service-required = Потрібна назва сервісу. Використання: watch <сервіс>

# Logs
logs-title = Логи розгортання для { $service }
logs-deployment = Розгортання
logs-no-logs = Логи недоступні
logs-service-required = Потрібна назва сервісу. Використання: logs <сервіс>

# List
list-title = Останні розгортання для { $service }
list-created = СТВОРЕНО
list-commit = КОМІТ
list-service-required = Потрібна назва сервісу. Використання: list <сервіс> [кількість]

# Services
services-title = Доступні сервіси
services-id = ID
services-uuid = UUID

# Config
config-title = Конфігурація ADI Coolify
config-current = Поточні значення
config-files = Файли конфігурації
config-user = Користувач
config-project = Проєкт
config-env-vars = Змінні середовища
config-set-usage = Встановити конфігурацію
config-encryption = Шифрування
config-encrypted-at-rest = (секрет, зашифровано)
config-encrypted = (зашифровано)
config-not-set = (не встановлено)
config-unavailable = (недоступно)
config-no-project = (немає проєкту)
config-encryption-algo = Секрети зашифровані за допомогою ChaCha20-Poly1305.
config-master-key = Майстер-ключ зберігається в: ~/.config/adi/secrets.key

# Config set
config-set-success = Встановлено { $key } = { $value } в конфігурації { $level }
config-set-file = Файл: { $path }
config-set-usage-full = Використання: config set <ключ> <значення> [--user|--project]
config-unknown-key = Невідомий ключ конфігурації: '{ $key }'. Допустимі ключі: url, api_key
config-no-project-dir = Директорія проєкту не встановлена. Запустіть з директорії проєкту.
config-save-failed = Не вдалося зберегти конфігурацію: { $error }

# Errors
error-api-key-not-set = API ключ не налаштовано. Встановіть через:
error-api-key-env = - Змінна середовища: ADI_PLUGIN_ADI_COOLIFY_API_KEY=<ключ>
error-api-key-user = - Конфігурація користувача: adi coolify config set api_key <ключ>
error-api-key-project = - Конфігурація проєкту: adi coolify config set api_key <ключ> --project
error-request-failed = Помилка запиту: { $error }
error-json-parse = Помилка розбору JSON: { $error }
error-unknown-command = Невідома команда: { $command }
error-invalid-context = Невірний контекст: { $error }
error-invalid-response = Невірний формат відповіді
error-no-deployment-uuid = Немає UUID розгортання
error-unknown-service = Невідомий сервіс: { $service }
