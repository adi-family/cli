# ============================================================================
# ADI WORKFLOW - RUSSIAN TRANSLATIONS (Русский)
# ============================================================================

# Help and descriptions
workflow-description = Запуск рабочих процессов, определённых в TOML файлах
workflow-help-title = ADI Workflow - Запуск рабочих процессов из TOML файлов
workflow-help-commands = Команды:
workflow-help-run = Запустить рабочий процесс по имени
workflow-help-list = Показать доступные рабочие процессы
workflow-help-show = Показать определение рабочего процесса
workflow-help-locations = Расположение рабочих процессов:
workflow-help-local = (локальный, наивысший приоритет)
workflow-help-global = (глобальный)
workflow-help-usage = Использование:

# List command
workflow-list-title = Доступные рабочие процессы:
workflow-list-empty = Рабочие процессы не найдены.
workflow-list-hint-create = Создайте рабочие процессы в:
workflow-list-scope-local = [локальный]
workflow-list-scope-global = [глобальный]

# Show command
workflow-show-title = Рабочий процесс: { $name }
workflow-show-description = Описание: { $description }
workflow-show-path = Путь: { $path }
workflow-show-inputs = Входные данные:
workflow-show-input-options = Варианты: { $options }
workflow-show-input-default = По умолчанию: { $default }
workflow-show-steps = Шаги:
workflow-show-step-if = если: { $condition }
workflow-show-step-run = выполнить: { $command }
workflow-show-error-missing-name = Отсутствует имя рабочего процесса. Использование: show <имя>
workflow-show-error-not-found = Рабочий процесс '{ $name }' не найден

# Run command
workflow-run-title = Запуск рабочего процесса: { $name }
workflow-run-collecting-inputs = Сбор входных данных...
workflow-run-executing-steps = Выполнение шагов...
workflow-run-step-running = Выполнение шага { $number }: { $name }
workflow-run-step-skipping = Пропуск шага { $number }: { $name } (условие не выполнено)
workflow-run-success = Рабочий процесс '{ $name }' успешно завершён!
workflow-run-error-not-found = Рабочий процесс '{ $name }' не найден
workflow-run-error-no-steps = Рабочий процесс не имеет шагов для выполнения

# Input prompts
workflow-input-error-tty = Интерактивные запросы требуют TTY
workflow-input-error-options = { $type } ввод требует вариантов
workflow-input-error-options-empty = { $type } ввод требует хотя бы один вариант
workflow-input-error-validation = Неверный шаблон проверки: { $error }
workflow-input-error-prompt = Ошибка запроса: { $error }
workflow-input-validation-failed = Ввод должен соответствовать шаблону: { $pattern }

# Execution
workflow-exec-error-spawn = Не удалось запустить команду: { $error }
workflow-exec-error-wait = Не удалось дождаться команды: { $error }
workflow-exec-error-exit-code = Команда завершилась с кодом ошибки: { $code }
workflow-exec-error-template = Ошибка шаблона: { $error }

# Common
workflow-common-error-parse = Не удалось разобрать рабочий процесс: { $error }
workflow-common-error-read = Не удалось прочитать файл рабочего процесса: { $error }
