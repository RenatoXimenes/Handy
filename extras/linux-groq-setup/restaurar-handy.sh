#!/usr/bin/env bash
set -euo pipefail

source_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
settings_dir="${XDG_DATA_HOME:-$HOME/.local/share}/com.pais.handy"
settings_file="$settings_dir/settings_store.json"
target_script="$HOME/.local/bin/handy-smart-paste"

for program in jq xdotool xprop xsel timeout; do
  if ! command -v "$program" >/dev/null 2>&1; then
    printf 'Falta o programa: %s\n' "$program" >&2
    printf 'No Ubuntu, instale as dependencias com:\n' >&2
    printf '  sudo apt install jq xdotool x11-utils xsel coreutils\n' >&2
    exit 1
  fi
done

if pgrep -x handy >/dev/null 2>&1; then
  printf 'Feche completamente o Handy e execute este restaurador novamente.\n' >&2
  exit 1
fi

if [[ ! -f "$settings_file" ]]; then
  printf 'Configuracao inexistente. Abra o Handy uma vez, feche-o e tente novamente.\n' >&2
  exit 1
fi

mkdir -p "$HOME/.local/bin"
install -m 0755 "$source_dir/handy-smart-paste" "$target_script"

backup_file="$settings_file.backup-$(date +%Y%m%d-%H%M%S)"
cp -a "$settings_file" "$backup_file"

temporary_file=$(mktemp "$settings_dir/settings_store.json.XXXXXX")
trap 'rm -f "$temporary_file"' EXIT

jq --arg script "$target_script" '
  .settings.always_on_microphone = true
  | .settings.active_transcription_endpoint_id = "groq"
  | .settings.selected_language = "pt"
  | .settings.selected_microphone = "fifine Microphone"
  | .settings.extra_recording_buffer_ms = 0
  | .settings.vad_enabled = true
  | .settings.push_to_talk = true
  | .settings.audio_feedback = true
  | .settings.audio_feedback_volume = 1.0
  | .settings.clipboard_handling = "dont_modify"
  | .settings.keyboard_implementation = "handy_keys"
  | .settings.typing_tool = "auto"
  | .settings.paste_method = "external_script"
  | .settings.external_script_path = $script
  | .settings.paste_delay_ms = 60
  | .settings.paste_delay_after_ms = 60
  | .settings.bindings.transcribe.current_binding = "ctrl_left+space"
  | (.settings.transcription_endpoints[] | select(.id == "groq")) |= (
      .model = "whisper-large-v3-turbo"
      | .send_language = true
      | .timeout_secs = 60
    )
' "$settings_file" >"$temporary_file"

chmod --reference="$settings_file" "$temporary_file"
mv "$temporary_file" "$settings_file"
trap - EXIT

printf 'Configuracao restaurada com sucesso.\n'
printf 'Backup da configuracao anterior: %s\n' "$backup_file"
printf 'Agora abra o Handy e informe novamente sua chave da Groq.\n'
