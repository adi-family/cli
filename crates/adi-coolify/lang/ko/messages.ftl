# ADI Coolify 플러그인 - 한국어 번역

# 명령어
cmd-status = 모든 서비스 상태 표시
cmd-deploy = 서비스 배포
cmd-watch = 배포 진행 상황 모니터링
cmd-logs = 배포 로그 표시
cmd-list = 최근 배포 목록
cmd-services = 사용 가능한 서비스 목록
cmd-config = 현재 구성 표시
cmd-config-set = 구성 값 설정

# 도움말
help-title = ADI Coolify - 배포 관리
help-commands = 명령어
help-services = 서비스
help-config = 구성
help-usage = 사용법: adi coolify <명령어> [인수]

# 서비스 이름
svc-auth = 인증 API
svc-platform = 플랫폼 API
svc-signaling = 시그널링 서버
svc-web = 웹 인터페이스
svc-analytics-ingestion = 분석 데이터 수집
svc-analytics = 분석 API
svc-registry = 플러그인 레지스트리

# 상태
status-title = ADI 배포 상태
status-service = 서비스
status-name = 이름
status-status = 상태
status-healthy = 정상
status-unhealthy = 비정상
status-unknown = 알 수 없음
status-building = 빌드 중
status-running = 실행 중
status-queued = 대기 중
status-finished = 완료
status-failed = 실패
status-error = 오류

# 배포
deploy-starting = 서비스 배포 중...
deploy-started = 시작됨
deploy-failed = 실패
deploy-uuid = 배포 UUID
deploy-use-watch = 'adi coolify watch <서비스>'로 진행 상황 모니터링
deploy-service-required = 서비스 이름이 필요합니다. 사용법: deploy <서비스|all> [--force]
deploy-unknown-service = 알 수 없는 서비스 '{ $service }'. 사용 가능: { $available }

# 모니터링
watch-title = { $service } 배포 모니터링 중...
watch-latest = 최신 배포
watch-uuid = UUID
watch-status = 상태
watch-commit = 커밋
watch-no-deployments = { $service }의 배포를 찾을 수 없습니다
watch-live-tip = 참고: 실시간 모니터링은: adi workflow deploy { $service } 사용
watch-service-required = 서비스 이름이 필요합니다. 사용법: watch <서비스>

# 로그
logs-title = { $service }의 배포 로그
logs-deployment = 배포
logs-no-logs = 사용 가능한 로그가 없습니다
logs-service-required = 서비스 이름이 필요합니다. 사용법: logs <서비스>

# 목록
list-title = { $service }의 최근 배포
list-created = 생성일
list-commit = 커밋
list-service-required = 서비스 이름이 필요합니다. 사용법: list <서비스> [개수]

# 서비스 목록
services-title = 사용 가능한 서비스
services-id = ID
services-uuid = UUID

# 구성
config-title = ADI Coolify 구성
config-current = 현재 값
config-files = 구성 파일
config-user = 사용자
config-project = 프로젝트
config-env-vars = 환경 변수
config-set-usage = 구성 설정
config-encryption = 암호화
config-encrypted-at-rest = (비밀, 암호화되어 저장됨)
config-encrypted = (암호화됨)
config-not-set = (설정되지 않음)
config-unavailable = (사용 불가)
config-no-project = (프로젝트 없음)
config-encryption-algo = 비밀은 ChaCha20-Poly1305로 암호화됩니다.
config-master-key = 마스터 키 저장 위치: ~/.config/adi/secrets.key

# 구성 설정
config-set-success = { $level } 구성에서 { $key } = { $value } 설정
config-set-file = 파일: { $path }
config-set-usage-full = 사용법: config set <키> <값> [--user|--project]
config-unknown-key = 알 수 없는 구성 키: '{ $key }'. 유효한 키: url, api_key
config-no-project-dir = 프로젝트 디렉토리가 설정되지 않았습니다. 프로젝트 디렉토리에서 실행하세요.
config-save-failed = 구성 저장 실패: { $error }

# 오류
error-api-key-not-set = API 키가 구성되지 않았습니다. 다음을 통해 설정하세요:
error-api-key-env = - 환경 변수: ADI_PLUGIN_ADI_COOLIFY_API_KEY=<키>
error-api-key-user = - 사용자 구성: adi coolify config set api_key <키>
error-api-key-project = - 프로젝트 구성: adi coolify config set api_key <키> --project
error-request-failed = 요청 실패: { $error }
error-json-parse = JSON 파싱 오류: { $error }
error-unknown-command = 알 수 없는 명령어: { $command }
error-invalid-context = 잘못된 컨텍스트: { $error }
error-invalid-response = 잘못된 응답 형식
error-no-deployment-uuid = 배포 UUID가 없습니다
error-unknown-service = 알 수 없는 서비스: { $service }
