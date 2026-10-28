#!/usr/bin/env bash
# Downloads what Sieve uses but does not keep in git, checks each file's SHA-256, and unpacks it into
# third_party/:
#   the llama.cpp server (b11429; CUDA 13 build on Windows, CPU build on Linux), GreenMail (the
#   local IMAP/SMTP test server).
#   scripts/fetch-deps.sh            the above
#   scripts/fetch-deps.sh --eval     also the Enron corpus shard the evaluation rebuilds its sets from
#   scripts/fetch-deps.sh --cpu      the CPU llama.cpp server on Windows (no NVIDIA GPU)
# The language model itself (a GGUF file, e.g. Qwen2.5-3B-Instruct-Q4_K_M) is yours to choose.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
dl="$root/third_party/downloads"
mkdir -p "$dl"
eval=0
cpu=0
for a in "$@"; do
  case "$a" in
    --eval) eval=1 ;;
    --cpu) cpu=1 ;;
  esac
done

fetch() { # url sha256 file
  local url="$1" sha="$2" file="$dl/$3"
  if [ ! -f "$file" ] || ! echo "$sha  $file" | sha256sum -c - >/dev/null 2>&1; then
    echo "downloading $url"
    curl -fsSL --retry 3 -o "$file.part" "$url" || { echo "download failed: $url" >&2; exit 1; }
    echo "$sha  $file.part" | sha256sum -c - >/dev/null || { echo "checksum mismatch: $url" >&2; rm -f "$file.part"; exit 1; }
    mv "$file.part" "$file"
  fi
}

llama="https://github.com/ggml-org/llama.cpp/releases/download/b11429"
case "$(uname -s)" in
  MINGW* | MSYS* | CYGWIN*)
    if [ $cpu = 1 ]; then
      fetch "$llama/llama-b11429-bin-win-cpu-x64.zip" 1283323272b04cd07905816a597a0da810918102de958f4ff6f7bbaa70ed2efe llama-b11429-bin-win-cpu-x64.zip
      zips="llama-b11429-bin-win-cpu-x64.zip"
    else
      fetch "$llama/llama-b11429-bin-win-cuda-13.4-x64.zip" 76ddc6eff2389570789ed608881efc6977751a722015d8e8c94f302224ff1a3a llama-b11429-bin-win-cuda-13.4-x64.zip
      fetch "$llama/cudart-llama-bin-win-cuda-13.4-x64.zip" 738f8c251ac22b70c3ae6f83a10cf222725df0395246a2cf58f32bdb85fbe668 cudart-llama-bin-win-cuda-13.4-x64.zip
      zips="llama-b11429-bin-win-cuda-13.4-x64.zip cudart-llama-bin-win-cuda-13.4-x64.zip"
    fi
    if [ ! -f "$root/third_party/llama/llama-server.exe" ]; then
      mkdir -p "$root/third_party/llama"
      for z in $zips; do unzip -qo "$dl/$z" -d "$root/third_party/llama"; done
    fi ;;
  Linux)
    fetch "$llama/llama-b11429-bin-ubuntu-x64.tar.gz" f6d25dde8f51133143d1453da4fd5f73b145127177612a283bf7995957af3392 llama-b11429-bin-ubuntu-x64.tar.gz
    if [ ! -x "$root/third_party/llama/llama-server" ]; then
      mkdir -p "$root/third_party/llama"
      tar -xzf "$dl/llama-b11429-bin-ubuntu-x64.tar.gz" -C "$root/third_party/llama" --strip-components=1
    fi ;;
esac

fetch https://repo1.maven.org/maven2/com/icegreen/greenmail-standalone/2.1.15/greenmail-standalone-2.1.15.jar \
  91d58798698cac357f015365240383b936ced2f2a1a9f7db37d9e3e1a31391b1 greenmail-standalone-2.1.15.jar

if [ $eval = 1 ]; then
  fetch https://huggingface.co/datasets/corbt/enron-emails/resolve/cfc06c758093d90993abce1a43668fb7357258a6/data/train-00000-of-00003.parquet \
    97165e587875e3e57715c14f63547fedbf664f748de1124122d4d2952c56a9fc enron-train-00000-of-00003.parquet
fi
echo "dependencies are in $root/third_party"
