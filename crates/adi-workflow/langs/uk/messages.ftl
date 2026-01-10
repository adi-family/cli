# ============================================================================
# ADI WORKFLOW - UKRAINIAN TRANSLATIONS
# ============================================================================

# Help and descriptions
workflow-description = Запуск робочих процесів, визначених у TOML файлах
workflow-help-title = ADI Workflow - Запуск робочих процесів з TOML файлів
workflow-help-commands = Команди:
workflow-help-run = Запустити робочий процес за назвою
workflow-help-list = Показати доступні робочі процеси
workflow-help-show = Показати визначення робочого процесу
workflow-help-locations = Розташування робочих процесів:
workflow-help-local = (локальний, найвищий пріоритет)
workflow-help-global = (глобальний)
workflow-help-usage = Використання:

# List command
workflow-list-title = Доступні робочі процеси:
workflow-list-empty = Робочих процесів не знайдено.
workflow-list-hint-create = Створіть робочі процеси у:
workflow-list-scope-local = [локальний]
workflow-list-scope-global = [глобальний]

# Show command
workflow-show-title = Робочий процес: { $name }
workflow-show-description = Опис: { $description }
workflow-show-path = Шлях: { $path }
workflow-show-inputs = Вхідні дані:
workflow-show-input-options = Варіанти: { $options }
workflow-show-input-default = За замовчуванням: { $default }
workflow-show-steps = Кроки:
workflow-show-step-if = якщо: { $condition }
workflow-show-step-run = виконати: { $command }
workflow-show-error-missing-name = Відсутня назва робочого процесу. Використання: show <назва>
workflow-show-error-not-found = Робочий процес '{ $name }' не знайдено

# Run command
workflow-run-title = Запуск робочого процесу: { $name }
workflow-run-collecting-inputs = Збір вхідних даних...
workflow-run-executing-steps = Виконання кроків...
workflow-run-step-running = Виконання кроку { $number }: { $name }
workflow-run-step-skipping = Пропуск кроку { $number }: { $name } (умова не виконана)
workflow-run-success = Робочий процес '{ $name }' успішно завершено!
workflow-run-error-not-found = Робочий процес '{ $name }' не знайдено
workflow-run-error-no-steps = Робочий процес не має кроків для виконання

# Input prompts
workflow-input-error-tty = Інтерактивні запити потребують TTY
workflow-input-error-options = { $type } вхід потребує варіантів
workflow-input-error-options-empty = { $type } вхід потребує принаймні один варіант
workflow-input-error-validation = Невірний шаблон перевірки: { $error }
workflow-input-error-prompt = Помилка запиту: { $error }
workflow-input-validation-failed = Вхід повинен відповідати шаблону: { $pattern }

# Execution
workflow-exec-error-spawn = Не вдалося запустити команду: { $error }
workflow-exec-error-wait = Не вдалося дочекатися команди: { $error }
workflow-exec-error-exit-code = Команда завершилася з кодом помилки: { $code }
workflow-exec-error-template = Помилка шаблону: { $error }

# Common
workflow-common-error-parse = Не вдалося розібрати робочий процес: { $error }
workflow-common-error-read = Не вдалося прочитати файл робочого процесу: { $error }
