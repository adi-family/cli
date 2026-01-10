# ============================================================================
# ADI WORKFLOW - SPANISH TRANSLATIONS (Español)
# ============================================================================

# Help and descriptions
workflow-description = Ejecutar flujos de trabajo definidos en archivos TOML
workflow-help-title = ADI Workflow - Ejecutar flujos de trabajo definidos en archivos TOML
workflow-help-commands = Comandos:
workflow-help-run = Ejecutar un flujo de trabajo por nombre
workflow-help-list = Listar flujos de trabajo disponibles
workflow-help-show = Mostrar definición del flujo de trabajo
workflow-help-locations = Ubicaciones de flujos de trabajo:
workflow-help-local = (local, mayor prioridad)
workflow-help-global = (global)
workflow-help-usage = Uso:

# List command
workflow-list-title = Flujos de trabajo disponibles:
workflow-list-empty = No se encontraron flujos de trabajo.
workflow-list-hint-create = Crear flujos de trabajo en:
workflow-list-scope-local = [local]
workflow-list-scope-global = [global]

# Show command
workflow-show-title = Flujo de trabajo: { $name }
workflow-show-description = Descripción: { $description }
workflow-show-path = Ruta: { $path }
workflow-show-inputs = Entradas:
workflow-show-input-options = Opciones: { $options }
workflow-show-input-default = Por defecto: { $default }
workflow-show-steps = Pasos:
workflow-show-step-if = si: { $condition }
workflow-show-step-run = ejecutar: { $command }
workflow-show-error-missing-name = Falta el nombre del flujo de trabajo. Uso: show <nombre>
workflow-show-error-not-found = Flujo de trabajo '{ $name }' no encontrado

# Run command
workflow-run-title = Ejecutando flujo de trabajo: { $name }
workflow-run-collecting-inputs = Recopilando entradas...
workflow-run-executing-steps = Ejecutando pasos...
workflow-run-step-running = Ejecutando paso { $number }: { $name }
workflow-run-step-skipping = Omitiendo paso { $number }: { $name } (condición no cumplida)
workflow-run-success = ¡Flujo de trabajo '{ $name }' completado exitosamente!
workflow-run-error-not-found = Flujo de trabajo '{ $name }' no encontrado
workflow-run-error-no-steps = El flujo de trabajo no tiene pasos para ejecutar

# Input prompts
workflow-input-error-tty = Los prompts interactivos requieren un TTY
workflow-input-error-options = La entrada { $type } requiere opciones
workflow-input-error-options-empty = La entrada { $type } requiere al menos una opción
workflow-input-error-validation = Patrón de validación inválido: { $error }
workflow-input-error-prompt = Error de prompt: { $error }
workflow-input-validation-failed = La entrada debe coincidir con el patrón: { $pattern }

# Execution
workflow-exec-error-spawn = Error al iniciar comando: { $error }
workflow-exec-error-wait = Error al esperar comando: { $error }
workflow-exec-error-exit-code = Comando falló con código de salida: { $code }
workflow-exec-error-template = Error de plantilla: { $error }

# Common
workflow-common-error-parse = Error al analizar flujo de trabajo: { $error }
workflow-common-error-read = Error al leer archivo de flujo de trabajo: { $error }
