#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 2 ]]; then
  printf 'Usage: %s <model.gguf> <output-directory>\n' "$0" >&2
  exit 2
fi

source_file=$1
output_dir=$2
expected_hash=094912bc26f5f684a3809f4615d08c63c80e508432b2d9215404e37a240ac31c
expected_size=751093920
expected_name=nemotron-3.5-asr-ptbr-Q8_0.gguf
script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
repo_dir=$(cd -- "$script_dir/.." && pwd)

[[ -f "$source_file" ]] || { printf 'Model file not found: %s\n' "$source_file" >&2; exit 1; }
actual_size=$(stat -c %s -- "$source_file")
[[ "$actual_size" == "$expected_size" ]] || { printf 'Unexpected model size: %s\n' "$actual_size" >&2; exit 1; }
actual_hash=$(sha256sum -- "$source_file" | cut -d' ' -f1)
[[ "$actual_hash" == "$expected_hash" ]] || { printf 'Unexpected model SHA-256: %s\n' "$actual_hash" >&2; exit 1; }

mkdir -p -- "$output_dir"
cp -- "$source_file" "$output_dir/$expected_name"
cp -- "$repo_dir/licenses/OpenMDW-1.1.txt" "$output_dir/OpenMDW-1.1.txt"
cp -- "$repo_dir/docs/models/NEMOTRON_PTBR_NOTICE.txt" "$output_dir/NEMOTRON_PTBR_NOTICE.txt"
(
  cd -- "$output_dir"
  sha256sum -- "$expected_name" OpenMDW-1.1.txt NEMOTRON_PTBR_NOTICE.txt > SHA256SUMS
)
printf 'Prepared verified model, license, origin notice and checksums in %s\n' "$output_dir"
