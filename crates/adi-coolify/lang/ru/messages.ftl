# ADI Coolify Плагин - Русские переводы

# Команды
cmd-status = Показать статус всех сервисов
cmd-deploy = Развернуть сервис
cmd-watch = Отслеживать прогресс развертывания
cmd-logs = Показать логи развертывания
cmd-list = Показать последние развертывания
cmd-services = Показать доступные сервисы
cmd-config = Показать текущую конфигурацию
cmd-config-set = Установить значение конфигурации

# Справка
help-title = ADI Coolify - Управление развертыванием
help-commands = Команды
help-services = Сервисы
help-config = Конфигурация
help-usage = Использование: adi coolify <команда> [аргументы]

# Названия сервисов
svc-auth = API аутентификации
svc-platform = API платформы
svc-signaling = Сервер сигнализации
svc-web = Веб-интерфейс
svc-analytics-ingestion = Сбор аналитики
svc-analytics = API аналитики
svc-registry = Реестр плагинов

# Статус
status-title = Статус развертывания ADI
status-service = СЕРВИС
status-name = НАЗВАНИЕ
status-status = СТАТУС
status-healthy = исправен
status-unhealthy = неисправен
status-unknown = неизвестно
status-building = сборка
status-running = работает
status-queued = в очереди
status-finished = завершено
status-failed = неудача
status-error = ошибка

# Развертывание
deploy-starting = Развертывание сервисов...
deploy-started = Запущено
deploy-failed = Неудача
deploy-uuid = UUID развертываний
deploy-use-watch = Используйте 'adi coolify watch <сервис>' для мониторинга
deploy-service-required = Требуется название сервиса. Использование: deploy <сервис|all> [--force]
deploy-unknown-service = Неизвестный сервис '{ $service }'. Доступные: { $available }

# Мониторинг
watch-title = Отслеживание развертываний { $service }...
watch-latest = Последнее развертывание
watch-uuid = UUID
watch-status = Статус
watch-commit = Коммит
watch-no-deployments = Развертывания не найдены для { $service }
watch-live-tip = Примечание: Для отслеживания в реальном времени используйте: ./scripts/deploy.sh watch { $service }
watch-service-required = Требуется название сервиса. Использование: watch <сервис>

# Логи
logs-title = Логи развертывания для { $service }
logs-deployment = Развертывание
logs-no-logs = Логи недоступны
logs-service-required = Требуется название сервиса. Использование: logs <сервис>

# Список
list-title = Последние развертывания для { $service }
list-created = СОЗДАНО
list-commit = КОММИТ
list-service-required = Требуется название сервиса. Использование: list <сервис> [количество]

# Список сервисов
services-title = Доступные сервисы
services-id = ID
services-uuid = UUID

# Конфигурация
config-title = Конфигурация ADI Coolify
config-current = Текущие значения
config-files = Файлы конфигурации
config-user = Пользователь
config-project = Проект
config-env-vars = Переменные среды
config-set-usage = Установить конфигурацию
config-encryption = Шифрование
config-encrypted-at-rest = (секрет, зашифровано)
config-encrypted = (зашифровано)
config-not-set = (не установлено)
config-unavailable = (недоступно)
config-no-project = (нет проекта)
config-encryption-algo = Секреты шифруются с помощью ChaCha20-Poly1305.
config-master-key = Мастер-ключ хранится в: ~/.config/adi/secrets.key

# Установка конфигурации
config-set-success = Установлено { $key } = { $value } в конфигурации { $level }
config-set-file = Файл: { $path }
config-set-usage-full = Использование: config set <ключ> <значение> [--user|--project]
config-unknown-key = Неизвестный ключ конфигурации: '{ $key }'. Допустимые ключи: url, api_key
config-no-project-dir = Каталог проекта не установлен. Запустите из каталога проекта.
config-save-failed = Не удалось сохранить конфигурацию: { $error }

# Ошибки
error-api-key-not-set = API ключ не настроен. Настройте через:
error-api-key-env = - Переменная среды: ADI_PLUGIN_ADI_COOLIFY_API_KEY=<ключ>
error-api-key-user = - Конфигурация пользователя: adi coolify config set api_key <ключ>
error-api-key-project = - Конфигурация проекта: adi coolify config set api_key <ключ> --project
error-request-failed = Ошибка запроса: { $error }
error-json-parse = Ошибка разбора JSON: { $error }
error-unknown-command = Неизвестная команда: { $command }
error-invalid-context = Неверный контекст: { $error }
error-invalid-response = Неверный формат ответа
error-no-deployment-uuid = Нет UUID развертывания
error-unknown-service = Неизвестный сервис: { $service }
