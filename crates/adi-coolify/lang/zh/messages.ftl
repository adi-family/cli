# ADI Coolify 插件 - 简体中文翻译

# 命令
cmd-status = 显示所有服务的状态
cmd-deploy = 部署服务
cmd-watch = 监视部署进度
cmd-logs = 显示部署日志
cmd-list = 列出最近的部署
cmd-services = 列出可用服务
cmd-config = 显示当前配置
cmd-config-set = 设置配置值

# 帮助
help-title = ADI Coolify - 部署管理
help-commands = 命令
help-services = 服务
help-config = 配置
help-usage = 用法: adi coolify <命令> [参数]

# 服务名称
svc-auth = 认证 API
svc-platform = 平台 API
svc-signaling = 信令服务器
svc-web = Web 界面
svc-analytics-ingestion = 分析数据采集
svc-analytics = 分析 API
svc-registry = 插件注册中心

# 状态
status-title = ADI 部署状态
status-service = 服务
status-name = 名称
status-status = 状态
status-healthy = 健康
status-unhealthy = 不健康
status-unknown = 未知
status-building = 构建中
status-running = 运行中
status-queued = 排队中
status-finished = 已完成
status-failed = 失败
status-error = 错误

# 部署
deploy-starting = 正在部署服务...
deploy-started = 已启动
deploy-failed = 失败
deploy-uuid = 部署 UUID
deploy-use-watch = 使用 'adi coolify watch <服务>' 监视进度
deploy-service-required = 需要服务名称。用法: deploy <服务|all> [--force]
deploy-unknown-service = 未知服务 '{ $service }'。可用: { $available }

# 监视
watch-title = 正在监视 { $service } 的部署...
watch-latest = 最新部署
watch-uuid = UUID
watch-status = 状态
watch-commit = 提交
watch-no-deployments = 未找到 { $service } 的部署
watch-live-tip = 注意: 如需实时监视，请使用: ./scripts/deploy.sh watch { $service }
watch-service-required = 需要服务名称。用法: watch <服务>

# 日志
logs-title = { $service } 的部署日志
logs-deployment = 部署
logs-no-logs = 没有可用的日志
logs-service-required = 需要服务名称。用法: logs <服务>

# 列表
list-title = { $service } 的最近部署
list-created = 创建时间
list-commit = 提交
list-service-required = 需要服务名称。用法: list <服务> [数量]

# 服务列表
services-title = 可用服务
services-id = ID
services-uuid = UUID

# 配置
config-title = ADI Coolify 配置
config-current = 当前值
config-files = 配置文件
config-user = 用户
config-project = 项目
config-env-vars = 环境变量
config-set-usage = 设置配置
config-encryption = 加密
config-encrypted-at-rest = (密钥，已加密存储)
config-encrypted = (已加密)
config-not-set = (未设置)
config-unavailable = (不可用)
config-no-project = (无项目)
config-encryption-algo = 密钥使用 ChaCha20-Poly1305 加密。
config-master-key = 主密钥存储于: ~/.config/adi/secrets.key

# 配置设置
config-set-success = 已在 { $level } 配置中设置 { $key } = { $value }
config-set-file = 文件: { $path }
config-set-usage-full = 用法: config set <键> <值> [--user|--project]
config-unknown-key = 未知配置键: '{ $key }'。有效键: url, api_key
config-no-project-dir = 未设置项目目录。请从项目目录运行。
config-save-failed = 保存配置失败: { $error }

# 错误
error-api-key-not-set = API 密钥未配置。请通过以下方式设置:
error-api-key-env = - 环境变量: ADI_PLUGIN_ADI_COOLIFY_API_KEY=<密钥>
error-api-key-user = - 用户配置: adi coolify config set api_key <密钥>
error-api-key-project = - 项目配置: adi coolify config set api_key <密钥> --project
error-request-failed = 请求失败: { $error }
error-json-parse = JSON 解析错误: { $error }
error-unknown-command = 未知命令: { $command }
error-invalid-context = 无效上下文: { $error }
error-invalid-response = 无效响应格式
error-no-deployment-uuid = 没有部署 UUID
error-unknown-service = 未知服务: { $service }
