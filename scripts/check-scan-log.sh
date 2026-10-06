#!/usr/bin/env bash
# Decides whether a Qodana for Rust run really analysed the Cargo workspace.
# A run that opens the repository without its Cargo packages reports no
# problems and exits 0, so the exit code alone proves nothing.
#
# Both callers apply this one rule. scripts/qodana.sh passes the console
# output of a local run. .github/workflows/qodana.yml passes the linter's
# own log, log/idea.log in the results directory, to a copy of this script
# that it takes before the linter starts. On a pull request that log holds
# every stage (so far the base commit and then the head), and any stage
# that reports 0 packages fails it.
#
# The log fails when:
#   1. it reports "Analysis scope: 0 packages";
#   2. there is no log file, or the log never reports a scope of one
#      package or more;
#   3. the sanity check lists "Cargo project loading";
#   4. the sanity check counts one suspicious problem or more.
#
# What the runs on GitHub have shown, as of 2026-10-05:
#   - Rule 1 is the only rule that has failed a run. In run 37353079041, a
#     pull request, idea.log reported 0 packages four times, twice for each
#     stage, and this script exited 1.
#   - Rule 2 has never fired.
#   - Rules 3 and 4 have never fired. A pull request run gives them nothing
#     to match: runs 37258483044 and 37353079041 loaded no packages at all
#     and printed no sanity output, and in the second this script found
#     neither pattern in idea.log. The wording they match comes from the
#     console of one push run, 37258478134, which was green and was made
#     before this script existed. No push has run this script yet, and
#     idea.log is not uploaded, so nothing shows that it carries the sanity
#     lines. The scope line is the one console line it has been seen to
#     repeat.
#   - No local run has passed through this script: scripts/qodana.sh has
#     not been run since it started calling it.
#
# What still passes: a log in which one stage reports no scope while
# another reports one, and a workspace that loads only some of its
# packages. A single line with a count above zero satisfies rule 2. The
# count is Cargo packages, not files analysed.
#
# Usage: scripts/check-scan-log.sh <log file>
set -euo pipefail

if (($# != 1)); then
  echo "usage: $0 <log file>" >&2
  exit 2
fi
log="$1"

if [[ ! -f "${log}" ]]; then
  echo "No scan log at ${log}. Nothing shows that the linter opened the Cargo workspace." >&2
  nearest="$(dirname "${log}")"
  while [[ ! -d "${nearest}" ]]; do
    nearest="$(dirname "${nearest}")"
  done
  echo "The nearest directory that exists is ${nearest}:" >&2
  ls -la "${nearest}" >&2 || true
  exit 1
fi

echo "Analysis scope lines in ${log}:"
grep -o 'Analysis scope: .*' "${log}" | uniq -c || echo "  none"

failed=0
if grep -q 'Analysis scope: 0 packages' "${log}"; then
  echo "Qodana opened the repo and loaded no Cargo packages. The report is not a real analysis." >&2
  failed=1
elif ! grep -E -q 'Analysis scope: [1-9][0-9]* packages' "${log}"; then
  echo "The log reports no analysis scope. Nothing shows that the linter loaded Cargo packages." >&2
  failed=1
fi
if grep -q 'Cargo project loading' "${log}"; then
  echo "Cargo did not finish loading, so the analysis is incomplete." >&2
  failed=1
fi
# The count is singular for one problem ("1 suspicious problem was detected").
if grep -E -q '(^|[^0-9])[1-9][0-9]* suspicious problems?' "${log}"; then
  echo "Qodana's sanity check reports suspicious problems. The project model is incomplete." >&2
  failed=1
fi

if ((failed)); then
  # The linter's own log records each command the Rust plugin ran and how it
  # failed: the command, its directory and environment, then its output.
  # Console output has none of these lines.
  if grep -q -E 'Execution failed|Cargo project model loading finished' "${log}"; then
    echo "What the Rust plugin logged while it loaded Cargo:" >&2
    grep 'Cargo project model loading finished' "${log}" >&2 || true
    grep -A 14 'Execution failed' "${log}" | head -n 150 >&2 || true
  fi
  exit 1
fi
echo "The scan loaded the Cargo workspace."
