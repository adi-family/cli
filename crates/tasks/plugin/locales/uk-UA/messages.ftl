# ============================================================================
# ADI TASKS - УКРАЇНСЬКІ ПЕРЕКЛАДИ
# ============================================================================

# Метадані плагіна
plugin-name = Завдання
plugin-description = Управління завданнями з відстеженням залежностей

# Описи команд
cmd-list-help = Показати всі завдання
cmd-add-help = Додати нове завдання
cmd-show-help = Показати деталі завдання
cmd-status-help = Оновити статус завдання
cmd-delete-help = Видалити завдання
cmd-depend-help = Додати залежність між завданнями
cmd-undepend-help = Видалити залежність між завданнями
cmd-graph-help = Показати граф залежностей
cmd-search-help = Шукати завдання
cmd-blocked-help = Показати заблоковані завдання
cmd-cycles-help = Виявити циклічні залежності
cmd-stats-help = Показати статистику завдань

# Текст довідки
tasks-help-title = ADI Завдання - Управління завданнями з відстеженням залежностей
tasks-help-commands = Команди:
tasks-help-usage = Використання: adi tasks <команда> [аргументи]

# Команда списку
tasks-list-empty = Завдань не знайдено
tasks-list-scope-global = [глобальне]
tasks-list-scope-project = [проєкт]

# Команда додавання
tasks-add-missing-title = Відсутній заголовок. Використання: add <заголовок> [--description <опис>]
tasks-add-created = Створено завдання #{ $id }: { $title }

# Команда показу
tasks-show-missing-id = Відсутній ID завдання. Використання: show <id>
tasks-show-invalid-id = Невірний ID завдання
tasks-show-title = Завдання #{ $id }
tasks-show-field-title = Заголовок: { $title }
tasks-show-field-status = Статус: { $status }
tasks-show-field-description = Опис: { $description }
tasks-show-field-symbol = Пов'язаний символ: #{ $symbol_id }
tasks-show-field-scope = Область: { $scope }
tasks-show-dependencies = Залежності:
tasks-show-dependents = Залежать від цього:

# Команда статусу
tasks-status-missing-args = Відсутні аргументи. Використання: status <id> <статус>
tasks-status-invalid-id = Невірний ID завдання
tasks-status-invalid-status = Невірний статус: { $status }. Допустимі: todo, in-progress, done, blocked, cancelled
tasks-status-updated = Статус завдання #{ $id } оновлено на { $status }

# Команда видалення
tasks-delete-missing-id = Відсутній ID завдання. Використання: delete <id> [--force]
tasks-delete-invalid-id = Невірний ID завдання
tasks-delete-confirm = Видалити завдання #{ $id }: { $title }?
tasks-delete-confirm-hint = Використовуйте --force для підтвердження
tasks-delete-success = Видалено завдання #{ $id }: { $title }

# Команда залежності
tasks-depend-missing-args = Відсутні аргументи. Використання: depend <id-завдання> <id-залежності>
tasks-depend-invalid-task-id = Невірний ID завдання
tasks-depend-invalid-depends-id = Невірний ID залежності
tasks-depend-success = Завдання #{ $task_id } тепер залежить від завдання #{ $depends_on }

# Команда видалення залежності
tasks-undepend-missing-args = Відсутні аргументи. Використання: undepend <id-завдання> <id-залежності>
tasks-undepend-invalid-task-id = Невірний ID завдання
tasks-undepend-invalid-depends-id = Невірний ID залежності
tasks-undepend-success = Видалено залежність: #{ $task_id } -> #{ $depends_on }

# Команда графа
tasks-graph-title = Граф залежностей завдань
tasks-graph-empty = Завдань не знайдено
tasks-graph-depends-on = залежить від #{ $id }: { $title }

# Команда пошуку
tasks-search-missing-query = Відсутній запит. Використання: search <запит> [--limit <n>]
tasks-search-empty = Завдань не знайдено
tasks-search-results = Знайдено { $count } результатів для "{ $query }":

# Команда заблокованих
tasks-blocked-empty = Немає заблокованих завдань
tasks-blocked-title = Заблоковані завдання
tasks-blocked-by = заблоковано #{ $id }: { $title } ({ $status })

# Команда циклів
tasks-cycles-empty = Циклічних залежностей не виявлено
tasks-cycles-found = Виявлено { $count } циклічних залежностей:
tasks-cycles-item = Цикл { $number }:

# Команда статистики
tasks-stats-title = Статистика завдань
tasks-stats-total = Всього завдань: { $count }
tasks-stats-todo = Очікують: { $count }
tasks-stats-in-progress = В роботі: { $count }
tasks-stats-done = Завершено: { $count }
tasks-stats-blocked = Заблоковано: { $count }
tasks-stats-cancelled = Скасовано: { $count }
tasks-stats-dependencies = Залежностей: { $count }
tasks-stats-cycles-yes = Цикли: Так (виконайте 'cycles' для перегляду)
tasks-stats-cycles-no = Цикли: Немає

# Помилки
error-not-initialized = Завдання не ініціалізовано
error-task-not-found = Завдання { $id } не знайдено
