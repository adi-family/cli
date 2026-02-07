# ============================================================================
# ADI WORKFLOW - KOREAN TRANSLATIONS (한국어)
# ============================================================================

# Help and descriptions
workflow-description = TOML 파일에 정의된 워크플로우 실행
workflow-help-title = ADI Workflow - TOML 파일에 정의된 워크플로우 실행
workflow-help-commands = 명령:
workflow-help-run = 이름으로 워크플로우 실행
workflow-help-list = 사용 가능한 워크플로우 목록
workflow-help-show = 워크플로우 정의 표시
workflow-help-locations = 워크플로우 위치:
workflow-help-local = (로컬, 최우선)
workflow-help-global = (전역)
workflow-help-usage = 사용법:

# List command
workflow-list-title = 사용 가능한 워크플로우:
workflow-list-empty = 워크플로우를 찾을 수 없습니다.
workflow-list-hint-create = 워크플로우 생성 위치:
workflow-list-scope-local = [로컬]
workflow-list-scope-global = [전역]

# Show command
workflow-show-title = 워크플로우: { $name }
workflow-show-description = 설명: { $description }
workflow-show-path = 경로: { $path }
workflow-show-inputs = 입력:
workflow-show-input-options = 옵션: { $options }
workflow-show-input-default = 기본값: { $default }
workflow-show-steps = 단계:
workflow-show-step-if = 조건: { $condition }
workflow-show-step-run = 실행: { $command }
workflow-show-error-missing-name = 워크플로우 이름이 없습니다. 사용법: show <이름>
workflow-show-error-not-found = 워크플로우 '{ $name }'를 찾을 수 없습니다

# Run command
workflow-run-title = 워크플로우 실행 중: { $name }
workflow-run-collecting-inputs = 입력 수집 중...
workflow-run-executing-steps = 단계 실행 중...
workflow-run-step-running = 단계 { $number } 실행 중: { $name }
workflow-run-step-skipping = 단계 { $number } 건너뜀: { $name } (조건 미충족)
workflow-run-success = 워크플로우 '{ $name }' 성공적으로 완료!
workflow-run-error-not-found = 워크플로우 '{ $name }'를 찾을 수 없습니다
workflow-run-error-no-steps = 워크플로우에 실행할 단계가 없습니다

# Input prompts
workflow-input-error-tty = 대화형 프롬프트에는 TTY가 필요합니다
workflow-input-error-options = { $type } 입력에는 옵션이 필요합니다
workflow-input-error-options-empty = { $type } 입력에는 최소 하나의 옵션이 필요합니다
workflow-input-error-validation = 잘못된 검증 패턴: { $error }
workflow-input-error-prompt = 프롬프트 오류: { $error }
workflow-input-validation-failed = 입력은 패턴과 일치해야 합니다: { $pattern }

# Execution
workflow-exec-error-spawn = 명령을 시작할 수 없습니다: { $error }
workflow-exec-error-wait = 명령을 기다릴 수 없습니다: { $error }
workflow-exec-error-exit-code = 명령이 종료 코드로 실패했습니다: { $code }
workflow-exec-error-template = 템플릿 오류: { $error }

# Common
workflow-common-error-parse = 워크플로우 파싱 실패: { $error }
workflow-common-error-read = 워크플로우 파일 읽기 실패: { $error }
