# ============================================================================
# ADI WORKFLOW - CHINESE TRANSLATIONS (简体中文)
# ============================================================================

# Help and descriptions
workflow-description = 运行 TOML 文件中定义的工作流
workflow-help-title = ADI Workflow - 运行 TOML 文件中定义的工作流
workflow-help-commands = 命令：
workflow-help-run = 按名称运行工作流
workflow-help-list = 列出可用的工作流
workflow-help-show = 显示工作流定义
workflow-help-locations = 工作流位置：
workflow-help-local = （本地，最高优先级）
workflow-help-global = （全局）
workflow-help-usage = 用法：

# List command
workflow-list-title = 可用的工作流：
workflow-list-empty = 未找到工作流。
workflow-list-hint-create = 在以下位置创建工作流：
workflow-list-scope-local = [本地]
workflow-list-scope-global = [全局]

# Show command
workflow-show-title = 工作流：{ $name }
workflow-show-description = 描述：{ $description }
workflow-show-path = 路径：{ $path }
workflow-show-inputs = 输入：
workflow-show-input-options = 选项：{ $options }
workflow-show-input-default = 默认值：{ $default }
workflow-show-steps = 步骤：
workflow-show-step-if = 条件：{ $condition }
workflow-show-step-run = 运行：{ $command }
workflow-show-error-missing-name = 缺少工作流名称。用法：show <名称>
workflow-show-error-not-found = 未找到工作流 '{ $name }'

# Run command
workflow-run-title = 运行工作流：{ $name }
workflow-run-collecting-inputs = 收集输入...
workflow-run-executing-steps = 执行步骤...
workflow-run-step-running = 运行步骤 { $number }：{ $name }
workflow-run-step-skipping = 跳过步骤 { $number }：{ $name }（条件不满足）
workflow-run-success = 工作流 '{ $name }' 成功完成！
workflow-run-error-not-found = 未找到工作流 '{ $name }'
workflow-run-error-no-steps = 工作流没有可执行的步骤

# Input prompts
workflow-input-error-tty = 交互式提示需要 TTY
workflow-input-error-options = { $type } 输入需要选项
workflow-input-error-options-empty = { $type } 输入至少需要一个选项
workflow-input-error-validation = 无效的验证模式：{ $error }
workflow-input-error-prompt = 提示错误：{ $error }
workflow-input-validation-failed = 输入必须匹配模式：{ $pattern }

# Execution
workflow-exec-error-spawn = 无法启动命令：{ $error }
workflow-exec-error-wait = 无法等待命令：{ $error }
workflow-exec-error-exit-code = 命令失败，退出代码：{ $code }
workflow-exec-error-template = 模板错误：{ $error }

# Common
workflow-common-error-parse = 无法解析工作流：{ $error }
workflow-common-error-read = 无法读取工作流文件：{ $error }
