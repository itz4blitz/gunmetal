#!/usr/bin/env bash
# Runs Qodana for Rust on this repo. A missing Cargo project, a sanity
# failure, or any reported problem fails the run. Findings are fixed in the
# code or in this configuration. They are not parked in a baseline.
#
# The project token is QODANA_TOKEN, or the first line of ~/.config/qodana/token.
# It is never read from the tree. Docker is required. Membership is read from
# the group database, so a session whose credentials do not include docker yet
# still switches with sg.
set -euo pipefail
cd "$(dirname "$0")/.."

export PATH="${HOME}/.local/bin:${PATH}"

if [[ "${QODANA_IN_DOCKER_GROUP:-}" != 1 ]] && ! docker info >/dev/null 2>&1; then
  user="$(id -un)"
  # getent sees the account even when this login's id -nG does not.
  members="$(getent group docker 2>/dev/null | awk -F: '{ print $4 }' || true)"
  if [[ -n "${members}" && ",${members}," == *",${user},"* ]]; then
    quoted="$(printf '%q' "$0")"
    if (($# > 0)); then
      quoted+=" $(printf '%q ' "$@")"
    fi
    exec sg docker -c "QODANA_IN_DOCKER_GROUP=1 ${quoted}"
  fi
  echo "Docker is installed but this login cannot use /var/run/docker.sock." >&2
  echo "Log out and back in so the docker group applies, then run scripts/qodana.sh again." >&2
  exit 1
fi

token="${QODANA_TOKEN:-}"
token_file="${HOME}/.config/qodana/token"
if [[ -z "${token}" && -f "${token_file}" ]]; then
  token="$(head -n 1 "${token_file}")"
fi
if [[ -z "${token}" ]]; then
  echo "No Qodana project token. Export QODANA_TOKEN or write it to ${token_file}." >&2
  exit 1
fi
export QODANA_TOKEN="${token}"

channel="$(awk -F'"' '/^channel[[:space:]]*=/ { print $2; exit }' rust-toolchain.toml)"
if [[ -z "${channel}" ]]; then
  echo "rust-toolchain.toml has no channel." >&2
  exit 1
fi

rustup_dir="${HOME}/.cache/qodana/rustup"
registry_dir="${HOME}/.cache/qodana/cargo-registry"
git_dir="${HOME}/.cache/qodana/cargo-git"
results_dir="${HOME}/.cache/qodana/gunmetal/results"
cache_dir="${HOME}/.cache/qodana/gunmetal/cache"
mkdir -p "${rustup_dir}" "${registry_dir}" "${git_dir}" "${results_dir}" "${cache_dir}"

if ! find "${rustup_dir}/toolchains" -type f -name rustc -print -quit 2>/dev/null | grep -q .; then
  echo "==> install Rust ${channel} into the Qodana toolchain cache"
  docker run --rm --user 0:0 \
    -e RUSTUP_HOME=/usr/local/rustup \
    -v "${rustup_dir}:/usr/local/rustup" \
    --entrypoint bash \
    jetbrains/qodana-rust:2026.2-eap \
    -lc "rustup toolchain install ${channel} --profile minimal --component clippy,rustfmt && rustup default ${channel}"
fi
# The linter ships a bundled stdlib for this channel. Installing rust-src
# makes it switch to the sysroot copy, and that copy left `bool: Copy` and
# `AtomicUsize` unresolved. Leave rust-src uninstalled.
if [[ -d "${rustup_dir}/toolchains/${channel}-x86_64-unknown-linux-gnu/lib/rustlib/src" ]]; then
  echo "==> remove rust-src so Qodana keeps its bundled standard library"
  docker run --rm --user 0:0 \
    -e RUSTUP_HOME=/usr/local/rustup \
    -v "${rustup_dir}:/usr/local/rustup" \
    --entrypoint bash \
    jetbrains/qodana-rust:2026.2-eap \
    -lc "rustup component remove rust-src --toolchain ${channel}"
fi
# cargo metadata refreshes the pinned channel and writes rustup's temp files
# as whichever uid the linter uses. The directory has to be writable by that uid.
mkdir -p "${rustup_dir}/tmp"
chmod 1777 "${rustup_dir}" "${rustup_dir}/tmp" || true

# The Rust loader logs a non-empty task queue and starts analysis anyway.
# qodana/rust-wait is an activity tracker that keeps the opening stage open
# until Cargo, DefMaps, and that queue have been quiet.
plugin_src="${PWD}/qodana/rust-wait"
plugin_root="${cache_dir}/rust-wait-plugin"
plugin_jar="${plugin_root}/lib/gunmetal-rust-wait.jar"
src_file="${plugin_src}/src/local/gunmetal/qodana/RustModelTracker.java"
if [[ ! -f "${plugin_jar}" || "${src_file}" -nt "${plugin_jar}" || "${plugin_src}/META-INF/plugin.xml" -nt "${plugin_jar}" ]]; then
  echo "==> compile the Rust model wait plugin"
  build_dir="$(mktemp -d)"
  docker run --rm --user 0:0 \
    -v "${plugin_src}:/src:ro" \
    -v "${build_dir}:/out" \
    --entrypoint bash \
    jetbrains/qodana-rust:2026.2-eap \
    -lc 'rm -rf /out/classes && mkdir -p /out/classes && /opt/idea/jbr/bin/javac --release 25 -encoding UTF-8 -cp "/opt/idea/lib/*:/opt/idea/plugins/intellij-rust/lib/modules/intellij.rustrover.core.jar" -d /out/classes /src/src/local/gunmetal/qodana/RustModelTracker.java && chmod -R a+rwX /out'
  mkdir -p "${plugin_root}/lib"
  python3 - "${build_dir}/classes" "${plugin_src}/META-INF/plugin.xml" "${plugin_jar}" <<'PY'
import pathlib, sys, zipfile
classes, xml, jar = map(pathlib.Path, sys.argv[1:])
jar.parent.mkdir(parents=True, exist_ok=True)
with zipfile.ZipFile(jar, "w") as zipped:
    zipped.write(xml, "META-INF/plugin.xml")
    for path in classes.rglob("*.class"):
        zipped.write(path, path.relative_to(classes).as_posix())
PY
  docker run --rm --user 0:0 \
    -v "${build_dir}:/out" \
    --entrypoint bash \
    jetbrains/qodana-rust:2026.2-eap \
    -lc 'rm -rf /out/classes'
  rm -rf "${build_dir}"
fi

log="$(mktemp)"
trap 'rm -f "${log}"' EXIT
set +e
qodana scan \
  --linter qodana-rust \
  --within-docker=true \
  --project-dir "${PWD}" \
  --repository-root "${PWD}" \
  --user 0:0 \
  --results-dir "${results_dir}" \
  --cache-dir "${cache_dir}" \
  --volume "${rustup_dir}:/usr/local/rustup" \
  --volume "${registry_dir}:/usr/local/cargo/registry" \
  --volume "${git_dir}:/usr/local/cargo/git" \
  --volume "${plugin_root}:/opt/idea/plugins/gunmetal-rust-wait:ro" \
  --fail-threshold 0 \
  --print-problems \
  >"${log}" 2>&1
code=$?
set -e
cat "${log}"

# The linter runs as root so it can use the image's rustup. Hand the report
# files back to the user who started the scan.
docker run --rm --user 0:0 \
  -v "${results_dir}:/results" \
  -v "${cache_dir}:/cache" \
  --entrypoint chown \
  jetbrains/qodana-rust:2026.2-eap \
  -R "$(id -u):$(id -g)" /results /cache >/dev/null

# The rule the GitHub workflow applies to its own run: the log has to show
# loaded Cargo packages and a quiet sanity check. A scan that opened no
# packages exits 0 and is not a pass. scripts/check-scan-log.sh says which
# of its rules have been seen to fail a run.
scripts/check-scan-log.sh "${log}"
exit "${code}"
