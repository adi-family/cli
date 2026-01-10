# ADI Coolify プラグイン - 日本語翻訳

# コマンド
cmd-status = すべてのサービスのステータスを表示
cmd-deploy = サービスをデプロイ
cmd-watch = デプロイの進行状況を監視
cmd-logs = デプロイログを表示
cmd-list = 最近のデプロイを一覧表示
cmd-services = 利用可能なサービスを一覧表示
cmd-config = 現在の設定を表示
cmd-config-set = 設定値を設定

# ヘルプ
help-title = ADI Coolify - デプロイ管理
help-commands = コマンド
help-services = サービス
help-config = 設定
help-usage = 使用法: adi coolify <コマンド> [引数]

# サービス名
svc-auth = 認証 API
svc-platform = プラットフォーム API
svc-signaling = シグナリングサーバー
svc-web = Web インターフェース
svc-analytics-ingestion = 分析データ取り込み
svc-analytics = 分析 API
svc-registry = プラグインレジストリ

# ステータス
status-title = ADI デプロイステータス
status-service = サービス
status-name = 名前
status-status = ステータス
status-healthy = 正常
status-unhealthy = 異常
status-unknown = 不明
status-building = ビルド中
status-running = 実行中
status-queued = 待機中
status-finished = 完了
status-failed = 失敗
status-error = エラー

# デプロイ
deploy-starting = サービスをデプロイ中...
deploy-started = 開始
deploy-failed = 失敗
deploy-uuid = デプロイ UUID
deploy-use-watch = 'adi coolify watch <サービス>' で進行状況を監視
deploy-service-required = サービス名が必要です。使用法: deploy <サービス|all> [--force]
deploy-unknown-service = 不明なサービス '{ $service }'。利用可能: { $available }

# 監視
watch-title = { $service } のデプロイを監視中...
watch-latest = 最新のデプロイ
watch-uuid = UUID
watch-status = ステータス
watch-commit = コミット
watch-no-deployments = { $service } のデプロイが見つかりません
watch-live-tip = 注意: ライブ監視には: ./scripts/deploy.sh watch { $service } を使用
watch-service-required = サービス名が必要です。使用法: watch <サービス>

# ログ
logs-title = { $service } のデプロイログ
logs-deployment = デプロイ
logs-no-logs = 利用可能なログがありません
logs-service-required = サービス名が必要です。使用法: logs <サービス>

# 一覧
list-title = { $service } の最近のデプロイ
list-created = 作成日時
list-commit = コミット
list-service-required = サービス名が必要です。使用法: list <サービス> [件数]

# サービス一覧
services-title = 利用可能なサービス
services-id = ID
services-uuid = UUID

# 設定
config-title = ADI Coolify 設定
config-current = 現在の値
config-files = 設定ファイル
config-user = ユーザー
config-project = プロジェクト
config-env-vars = 環境変数
config-set-usage = 設定を行う
config-encryption = 暗号化
config-encrypted-at-rest = (シークレット、暗号化して保存)
config-encrypted = (暗号化済み)
config-not-set = (未設定)
config-unavailable = (利用不可)
config-no-project = (プロジェクトなし)
config-encryption-algo = シークレットは ChaCha20-Poly1305 で暗号化されます。
config-master-key = マスターキーの保存場所: ~/.config/adi/secrets.key

# 設定の変更
config-set-success = { $level } 設定で { $key } = { $value } を設定
config-set-file = ファイル: { $path }
config-set-usage-full = 使用法: config set <キー> <値> [--user|--project]
config-unknown-key = 不明な設定キー: '{ $key }'。有効なキー: url, api_key
config-no-project-dir = プロジェクトディレクトリが設定されていません。プロジェクトディレクトリから実行してください。
config-save-failed = 設定の保存に失敗: { $error }

# エラー
error-api-key-not-set = API キーが設定されていません。以下で設定してください:
error-api-key-env = - 環境変数: ADI_PLUGIN_ADI_COOLIFY_API_KEY=<キー>
error-api-key-user = - ユーザー設定: adi coolify config set api_key <キー>
error-api-key-project = - プロジェクト設定: adi coolify config set api_key <キー> --project
error-request-failed = リクエスト失敗: { $error }
error-json-parse = JSON 解析エラー: { $error }
error-unknown-command = 不明なコマンド: { $command }
error-invalid-context = 無効なコンテキスト: { $error }
error-invalid-response = 無効なレスポンス形式
error-no-deployment-uuid = デプロイ UUID がありません
error-unknown-service = 不明なサービス: { $service }
