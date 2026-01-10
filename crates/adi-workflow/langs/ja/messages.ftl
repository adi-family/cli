# ============================================================================
# ADI WORKFLOW - JAPANESE TRANSLATIONS (日本語)
# ============================================================================

# Help and descriptions
workflow-description = TOMLファイルで定義されたワークフローを実行
workflow-help-title = ADI Workflow - TOMLファイルで定義されたワークフローを実行
workflow-help-commands = コマンド：
workflow-help-run = 名前でワークフローを実行
workflow-help-list = 利用可能なワークフローを一覧表示
workflow-help-show = ワークフローの定義を表示
workflow-help-locations = ワークフローの場所：
workflow-help-local = （ローカル、最優先）
workflow-help-global = （グローバル）
workflow-help-usage = 使用方法：

# List command
workflow-list-title = 利用可能なワークフロー：
workflow-list-empty = ワークフローが見つかりません。
workflow-list-hint-create = ワークフローを作成する場所：
workflow-list-scope-local = [ローカル]
workflow-list-scope-global = [グローバル]

# Show command
workflow-show-title = ワークフロー：{ $name }
workflow-show-description = 説明：{ $description }
workflow-show-path = パス：{ $path }
workflow-show-inputs = 入力：
workflow-show-input-options = オプション：{ $options }
workflow-show-input-default = デフォルト：{ $default }
workflow-show-steps = ステップ：
workflow-show-step-if = 条件：{ $condition }
workflow-show-step-run = 実行：{ $command }
workflow-show-error-missing-name = ワークフロー名がありません。使用方法：show <名前>
workflow-show-error-not-found = ワークフロー '{ $name }' が見つかりません

# Run command
workflow-run-title = ワークフローを実行中：{ $name }
workflow-run-collecting-inputs = 入力を収集中...
workflow-run-executing-steps = ステップを実行中...
workflow-run-step-running = ステップ { $number } を実行中：{ $name }
workflow-run-step-skipping = ステップ { $number } をスキップ：{ $name }（条件未達成）
workflow-run-success = ワークフロー '{ $name }' が正常に完了しました！
workflow-run-error-not-found = ワークフロー '{ $name }' が見つかりません
workflow-run-error-no-steps = ワークフローに実行するステップがありません

# Input prompts
workflow-input-error-tty = インタラクティブプロンプトにはTTYが必要です
workflow-input-error-options = { $type } 入力にはオプションが必要です
workflow-input-error-options-empty = { $type } 入力には少なくとも1つのオプションが必要です
workflow-input-error-validation = 無効な検証パターン：{ $error }
workflow-input-error-prompt = プロンプトエラー：{ $error }
workflow-input-validation-failed = 入力はパターンに一致する必要があります：{ $pattern }

# Execution
workflow-exec-error-spawn = コマンドの起動に失敗しました：{ $error }
workflow-exec-error-wait = コマンドの待機に失敗しました：{ $error }
workflow-exec-error-exit-code = コマンドが終了コードで失敗しました：{ $code }
workflow-exec-error-template = テンプレートエラー：{ $error }

# Common
workflow-common-error-parse = ワークフローの解析に失敗しました：{ $error }
workflow-common-error-read = ワークフローファイルの読み取りに失敗しました：{ $error }
