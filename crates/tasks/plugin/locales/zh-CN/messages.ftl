# ============================================================================
# ADI TASKS - 中文翻译
# ============================================================================

# 插件元数据
plugin-name = 任务
plugin-description = 带依赖关系的任务管理

# 命令描述
cmd-list-help = 列出所有任务
cmd-add-help = 添加新任务
cmd-show-help = 显示任务详情
cmd-status-help = 更新任务状态
cmd-delete-help = 删除任务
cmd-depend-help = 添加任务依赖
cmd-undepend-help = 移除任务依赖
cmd-graph-help = 显示依赖图
cmd-search-help = 搜索任务
cmd-blocked-help = 显示被阻塞的任务
cmd-cycles-help = 检测循环依赖
cmd-stats-help = 显示任务统计

# 帮助文本
tasks-help-title = ADI 任务 - 带依赖关系的任务管理
tasks-help-commands = 命令:
tasks-help-usage = 用法: adi tasks <命令> [参数]

# 列表命令
tasks-list-empty = 未找到任务
tasks-list-scope-global = [全局]
tasks-list-scope-project = [项目]

# 添加命令
tasks-add-missing-title = 缺少标题。用法: add <标题> [--description <描述>]
tasks-add-created = 创建任务 #{ $id }: { $title }

# 显示命令
tasks-show-missing-id = 缺少任务 ID。用法: show <id>
tasks-show-invalid-id = 无效的任务 ID
tasks-show-title = 任务 #{ $id }
tasks-show-field-title = 标题: { $title }
tasks-show-field-status = 状态: { $status }
tasks-show-field-description = 描述: { $description }
tasks-show-field-symbol = 关联符号: #{ $symbol_id }
tasks-show-field-scope = 范围: { $scope }
tasks-show-dependencies = 依赖:
tasks-show-dependents = 被依赖:

# 状态命令
tasks-status-missing-args = 缺少参数。用法: status <id> <状态>
tasks-status-invalid-id = 无效的任务 ID
tasks-status-invalid-status = 无效状态: { $status }。有效值: todo, in-progress, done, blocked, cancelled
tasks-status-updated = 任务 #{ $id } 状态已更新为 { $status }

# 删除命令
tasks-delete-missing-id = 缺少任务 ID。用法: delete <id> [--force]
tasks-delete-invalid-id = 无效的任务 ID
tasks-delete-confirm = 删除任务 #{ $id }: { $title }?
tasks-delete-confirm-hint = 使用 --force 确认删除
tasks-delete-success = 已删除任务 #{ $id }: { $title }

# 依赖命令
tasks-depend-missing-args = 缺少参数。用法: depend <任务ID> <依赖ID>
tasks-depend-invalid-task-id = 无效的任务 ID
tasks-depend-invalid-depends-id = 无效的依赖 ID
tasks-depend-success = 任务 #{ $task_id } 现在依赖于任务 #{ $depends_on }

# 移除依赖命令
tasks-undepend-missing-args = 缺少参数。用法: undepend <任务ID> <依赖ID>
tasks-undepend-invalid-task-id = 无效的任务 ID
tasks-undepend-invalid-depends-id = 无效的依赖 ID
tasks-undepend-success = 已移除依赖: #{ $task_id } -> #{ $depends_on }

# 图形命令
tasks-graph-title = 任务依赖图
tasks-graph-empty = 未找到任务
tasks-graph-depends-on = 依赖于 #{ $id }: { $title }

# 搜索命令
tasks-search-missing-query = 缺少查询条件。用法: search <查询> [--limit <n>]
tasks-search-empty = 未找到任务
tasks-search-results = 找到 { $count } 个 "{ $query }" 的结果:

# 阻塞命令
tasks-blocked-empty = 没有被阻塞的任务
tasks-blocked-title = 被阻塞的任务
tasks-blocked-by = 被 #{ $id } 阻塞: { $title } ({ $status })

# 循环检测命令
tasks-cycles-empty = 未检测到循环依赖
tasks-cycles-found = 发现 { $count } 个循环依赖:
tasks-cycles-item = 循环 { $number }:

# 统计命令
tasks-stats-title = 任务统计
tasks-stats-total = 任务总数: { $count }
tasks-stats-todo = 待办: { $count }
tasks-stats-in-progress = 进行中: { $count }
tasks-stats-done = 已完成: { $count }
tasks-stats-blocked = 已阻塞: { $count }
tasks-stats-cancelled = 已取消: { $count }
tasks-stats-dependencies = 依赖关系: { $count }
tasks-stats-cycles-yes = 循环: 是 (运行 'cycles' 查看)
tasks-stats-cycles-no = 循环: 无

# 错误
error-not-initialized = 任务未初始化
error-task-not-found = 找不到任务 { $id }
