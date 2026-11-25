#!/bin/bash
# jail-conformance.sh — the jail's completeness proof, run per release.
#
# The jail shipped; SOTA is airtightness that is PROVEN. This harness
# attacks a pressed package from the vectors a payload can actually reach,
# and it names the ones it does not cover yet instead of implying them:
#
#   covered
#     1  File.read            5  File.readlines        9  shell-out read
#     2  File.open            6  File.binread         10  host dir listing
#     3  File.stat            7  IO.binread           11  a GRANT read
#     4  File.exist?          8  read(realpath)       12  a path outside it
#     5  File.readlines        9  shell-out read      13  a payload-MADE link
#     6  File.binread         10  host dir listing
#
#   the exemption vectors (spec 08 §1's deny-rule amendment, tebako#1128)
#     devnull       a File::NULL open/write/read — the fixed device set is
#                   kernel surface, not host content: READ-OK under EVERY
#                   arm, and a refusal is an over-denial failure, never a
#                   block
#     devnullspawn  system(…, out: File::NULL) — the metanorma failing
#                   shape (run 45585078751), the spawn target a host-floor
#                   binary the jail admits under deny, so the only
#                   judgement in play is the device open; on windows the
#                   tebako#1021 spawn boundary owns the non-trivial-policy
#                   refusal
#   the suite runs by contract against the PUBLISHED pinned pair: when that
#   pair predates the exemption, the deny arm answers DENIED:EPERM while
#   the record arm (allow-all) reads — the era's signature, and the rows
#   report armed-not-applicable (the resource axis's convention), never a
#   silent pass; a pair carrying the exemption answers to the strict rule
#
#   not covered by this cut, named rather than implied
#     a `..` escape out of a grant (the jail's namespace is lexical: an
#     escape lands in VFS space, where only mounts exist, so the vector
#     measures path arithmetic rather than the policy)
#     case folding (needs a case-insensitive host file system: macOS)
#     TOCTOU on materialization (needs a race harness)
#
#   platform-gated
#     ads  alternate data streams (NTFS): the harness arms the vector on
#          Windows hosts and the probe SKIPs it elsewhere — a skip in the
#          open arm marks the vector inapplicable, never incomplete.
#
# Three arms, because one is not enough to prove anything:
#
#   open          no policy at all: EVERY vector must read the secret.
#                 A vector that cannot read it here proves nothing later.
#   deny          `TEBAKO_JAIL=deny`, no grants: the plain vectors and the
#                 shell-out must be refused.
#
# A third arm runs `deny` WITH a grant, because a policy that refuses
# everything is not a jail — it is a wall. The granted directory must be
# READABLE there and a path outside it must be refused, so the suite
# proves the policy DISCRIMINATES rather than merely closing. A fourth arm
# runs `record`: every vector must READ (record allows and journals) and
# the journal must carry lines — a record mode that denies, or that allows
# silently, is broken in the direction the audit trail depends on.
#
# The resource axis (spec 08 §2.2) adds a fifth section — same philosophy,
# different judgement: the consumption bounds must HOLD, not just parse.
# The arms run --pair only (they exercise the driver baked into the
# candidate pair directly; press mode resolves a PUBLISHED pair, a
# different moment):
#
#   wall      a busy looper under wall_seconds=3 dies at the deadline —
#             the watchdog's named stderr line + the event=jail-resource
#             journal line (POSIX: the process group's SIGKILL; windows:
#             TerminateJobObject — one observed exit, 137, on both)
#   cpu       the looper under cpu_seconds=2 dies at the CPU ceiling
#             (POSIX: SIGXCPU; windows: the kernel terminates the process
#             at its per-process user-time ceiling)
#   bomber    (POSIX) a fork bomb under processes=<uid count>+30 hits
#             EAGAIN, exits 42, and the watchdog reaps its sleeper
#             children with the root (the orphan guard proves the group
#             emptied); (windows) a SPAWN bomb under processes=8 — ruby
#             has no fork here — stops at the job's ACTIVE_PROCESS
#             ceiling, its over-ceiling children terminated at creation
#   memory    (Linux) a memory hog under memory=1G stops at the
#             RLIMIT_AS ceiling — NoMemoryError or the VM's fatal
#             allocation abort, never a silent overshoot; (windows) the
#             same hog stops at the job's per-process commit ceiling
#   memory-refusal (macOS) a declared memory bound fails the boot by
#             name (exit 73) — the kernel's RLIMIT_AS floor, never a
#             silent clamp
#   control   generous bounds, nothing stressed: the run is
#             byte-identical to the open arm
#
# A pair that predates the axis answers the gate probe with the old
# parser's "invalid jail spec" — the arms then SKIP by name (armed, not
# applicable to this pair), never silently absent.
#
# Every refusal must be EACCES/EPERM. ENOENT is NOT a refusal: it means
# the engine passed the path to the host and the host had nothing there,
# which is a different answer and never counts as a block.
#
# Usage: jail-conformance.sh [--pair <exe-prefix> --tfs <tfs>] <tebako-cli> [workdir]
#
#   default        press the probe with the CLI (the pair it resolves)
#   --pair P       run a CANDIDATE pair directly — `P` is the runtime
#                  executable, `P.tfs` its env image — which is how a
#                  release gate uses this suite before a pair is published
#   exit 0  every covered vector live under `open` and refused under deny
#   exit 1  at least one vector leaked, or an exempt device was refused
#           (the matrix names it)
#   exit 2  the harness could not run (press, runtime, or a broken probe)
#
# This is a RELEASE-rung check, not a PR gate: it needs a runtime pair,
# and on a pair that predates the interpreter-side host guard the deny
# arms leak by construction (tebako#917). The matrix is the evidence
# either way.
set -uo pipefail

# --- the deadline + the orphan guard (the bounded-resources cell of
# TODO.sota/05, the 2026-10-26 incident: a probe child spun at ~97 % CPU
# for 45 h because nothing reaped it). Every arm runs in its OWN process
# group (perl setpgid — the portable primitive, present on macOS and the
# Linux runners) under a per-arm deadline; on ANY exit the group is torn
# down, so a payload's orphaned child dies with its probe. A deadline
# hit or a reaped group is a LOUD harness failure (the arm's rows go
# missing and the matrix counts them), never a silent pass.
DEADLINE="${DEADLINE:-600}"   # seconds per arm; DEADLINE=8 smoke-tests the guard itself
ORPHANS=0

run_bounded() {
  local label="$1"; shift
  if [ "$WIN" = "1" ]; then
    # The deadline/orphan guard is POSIX machinery (perl setpgid + the
    # process-group kill) — Git-bash's MSYS layer has neither reliably;
    # the arms run bare on Windows until the guard has a Windows answer
    # (named in the log, never a silent skip).
    echo "   note: the deadline/orphan guard is POSIX-only — running bare" >&2
    "$@"
    return $?
  fi
  perl -MPOSIX=setpgid -e 'setpgid(0, 0) or die "setpgid: $!\n"; exec @ARGV or die "exec: $!\n"' -- "$@" &
  wait_bounded "$label" "$DEADLINE" "$!"
}

# The unisolated variant for the resource arms: no harness setpgid, so
# the DRIVER's own watchdog arm can create its group (pgid == the arm's
# pid — the group ops below still address it). A perl-wrapped arm would
# make the payload a group leader already, and the watchdog's root-exit
# reap would stay out by design (spec 08 §2.2's job-control caveat) —
# exactly the mechanism the bomber arm proves.
run_bounded_bare() {
  local label="$1" deadline="$2"; shift 2
  "$@" &
  wait_bounded "$label" "$deadline" "$!"
}

wait_bounded() {
  local label="$1" deadline="$2" pid="$3"
  local waited=0
  while kill -0 "$pid" 2>/dev/null; do
    if [ "$waited" -ge "$deadline" ]; then
      echo "   DEADLINE: $label exceeded ${deadline}s — tearing down the process group" >&2
      kill -TERM -- -"$pid" 2>/dev/null
      kill -TERM "$pid" 2>/dev/null
      sleep 3
      kill -KILL -- -"$pid" 2>/dev/null
      kill -KILL "$pid" 2>/dev/null
      wait "$pid" 2>/dev/null
      return 124
    fi
    sleep 2
    waited=$((waited + 2))
  done
  wait "$pid"
  local rc=$?
  # The orphan guard: the arm's group must be EMPTY once its leader is
  # gone — but poll briefly first: a watchdog-armed arm's watchdog reaps
  # its own group within ~100 ms of the leader's exit (spec 08 §2.2's
  # die-with-root), and the guard must not mistake that reap for an
  # orphan. A member still live after 3 s is a reaped-and-failed arm,
  # named in the matrix.
  local polls=0
  while kill -0 -- -"$pid" 2>/dev/null; do
    if [ "$polls" -ge 15 ]; then
      echo "   ORPHAN: $label left a live process group — tearing it down" >&2
      ps -eo pid,pgid,etime,command 2>/dev/null | awk -v g="$pid" '$2 == g' | sed 's/^/   /' >&2
      kill -KILL -- -"$pid" 2>/dev/null
      ORPHANS=$((ORPHANS + 1))
      return 125
    fi
    sleep 0.2
    polls=$((polls + 1))
  done
  return "$rc"
}

PAIR=""
TFS_BIN=""
while [ $# -gt 0 ]; do
  case "$1" in
    --pair) PAIR="${2:?--pair needs the runtime executable}"; shift 2 ;;
    --tfs) TFS_BIN="${2:?--tfs needs a tfs binary}"; shift 2 ;;
    *) break ;;
  esac
done
TEBAKO="${1:?usage: jail-conformance.sh [--pair <exe> --tfs <tfs>] <tebako-cli> [workdir]}"
WORK="${2:-/tmp/jail-conformance}"
rm -rf "$WORK"
mkdir -p "$WORK/app" "$WORK/home" "$WORK/host/private" "$WORK/scratch"
export HOME="$WORK/home"

SECRET="$WORK/host/secret.txt"
printf 'secret-bytes\n' > "$SECRET"
printf 'inner\n' > "$WORK/host/private/inner.txt"
mkdir -p "$WORK/host/granted"
printf 'granted-bytes\n' > "$WORK/host/granted/ok.txt"
# The listing vector's target: a sibling of the grant, so the answer is
# about the policy and not about traversal into the granted directory.
mkdir -p "$WORK/otherdir"
printf 'link-target-bytes\n' > "$WORK/otherdir/target.txt"
printf 'other\n' > "$WORK/otherdir/one.txt"

# Alternate data streams (NTFS only): the secret lives in a stream hung
# off an ordinary host file, and the vector reads it back. The creation
# is verified by reading the stream on the host — a non-NTFS volume or a
# missing PowerShell leaves ADS_BASE empty and the probe SKIPs the vector
# (a skip in the open arm is "not applicable", never incomplete).
ADS_BASE=""
WIN=0
case "$(uname -s)" in
  MINGW*|MSYS*|CYGWIN*)
    WIN=1
    ADS_CANDIDATE="$WORK/host/adsbase.txt"
    printf 'base\n' > "$ADS_CANDIDATE"
    ADS_WIN=$(cygpath -w "$ADS_CANDIDATE" 2>/dev/null || echo "")
    if [ -n "$ADS_WIN" ] && command -v powershell >/dev/null 2>&1; then
      if powershell -NoProfile -NonInteractive -Command \
           "Set-Content -Path '$ADS_WIN' -Stream sota -Value 'secret-bytes'" >/dev/null 2>&1 \
         && powershell -NoProfile -NonInteractive -Command \
           "Get-Content -Path '$ADS_WIN' -Stream sota" 2>/dev/null | grep -q secret-bytes; then
        ADS_BASE="$ADS_CANDIDATE"
      fi
    fi
    ;;
esac

# Native Windows exes need Windows path spellings, and the driver's
# `--tebako-image <path>:<slot>:<mount>` triple carries colons of its own —
# MSYS's argument heuristic would read it as a path LIST and mangle it. So
# the invocation disables conversion wholesale (MSYS2_ARG_CONV_EXCL) and
# every host path crosses explicitly through here. The jail grammar splits
# mount directives from the right, so a drive letter survives (tpkg's
# parse_mount_directive).
hostpath() {
  if [ -z "$1" ] || [ "$WIN" != "1" ]; then
    printf '%s' "$1"
  else
    cygpath -w "$1"
  fi
}

cat > "$WORK/app/probe.rb" <<'RUBY'
# One line per vector: NAME<TAB>verdict. READ-OK means the byte (or the
# listing) arrived; DENIED:<class> means a policy refused it;
# SKIPPED:<why> means this arm cannot exercise the vector; BROKEN:<why>
# means the probe itself failed and the vector proves nothing.
secret = ARGV[0]
hostdir = ARGV[1]
granted = ARGV[2].to_s
outside = ARGV[3].to_s
def verdict
  yield
  "READ-OK"
rescue SystemCallError => e
  "DENIED:#{e.class.name.split('::').last.sub('Errno', 'E')}"
rescue StandardError => e
  "BROKEN:#{e.class}"
end

def line(name)
  puts "#{name}\t#{yield}"
end

line("read")        { verdict { File.read(secret) } }
line("open")        { verdict { File.open(secret) { |f| f.read } } }
line("stat")        { verdict { File.stat(secret) } }
# `exist?` cannot raise: a denied stat comes back as false. Rendering that
# as EACCES is the POSIX reading of the same judgement (and the only one
# the API offers), so the harness's single refusal rule still applies.
line("exist?")      { verdict { File.exist?(secret) ? true : raise(Errno::EACCES) } }
line("readlines")   { verdict { File.readlines(secret) } }
line("binread")     { verdict { File.binread(secret) } }
line("IO.binread")  { verdict { IO.binread(secret) } }
line("realpath")    { verdict { File.read(File.realpath(secret)) } }
# The shell-out vector reads through a CHILD process: `cat` prints
# nothing when its own read is refused, and the payload only sees an empty
# string. An empty read with a failed child is the refusal, rendered as
# EACCES; an empty read with a successful child would be a broken probe.
line("shellout") do
  verdict do
    out = IO.popen(["cat", secret], &:read)
    status = $?
    next true unless out.to_s.empty?

    raise Errno::EACCES if status && !status.success?

    raise "cat succeeded but printed nothing"
  end
end
line("hostdir")     { verdict { raise Errno::ENOENT if Dir.children(hostdir).empty? } }

# The grant arm's two vectors: the granted directory must be readable
# (the positive control a deny-only suite cannot have) and a sibling
# outside it must be refused.
line("granted") do
  granted.empty? ? "SKIPPED:no-grant-arm" : verdict { File.read(granted) }
end
line("outside") do
  outside.empty? ? "SKIPPED:no-grant-arm" : verdict { File.read(outside) }
end

# A link the payload MAKES inside its grant, pointing at a file outside it:
# reading through the link must be judged by the TARGET's policy, not by
# where the link was made.
line("madetlink") do
  if outside.empty? || ARGV[4].to_s.empty?
    "SKIPPED:no-grant-arm"
  else
    verdict do
      link = File.join(ARGV[4], "escape")
      begin
        File.unlink(link)
      rescue SystemCallError
        nil
      end
      File.symlink(outside, link)
      File.read(link)
    end
  end
end

# Alternate data streams (NTFS): the stream hangs off a host file, so a
# policy that judges only the base name leaks the stream. ARGV[5] is the
# base path the harness armed — empty where ADS does not exist, and the
# skip marks the vector inapplicable rather than unproven.
line("ads") do
  base = ARGV[5].to_s
  base.empty? ? "SKIPPED:not-ntfs" : verdict { File.read("#{base}:sota") }
end

# The device-node exemption (spec 08 §1's deny-rule amendment,
# tebako#1128): /dev/null and its siblings (NUL on windows) are
# process-essential kernel devices, not host content — usable under EVERY
# policy, so these vectors must READ-OK in every arm. A refusal is the
# over-denial the issue names, never a block.
line("devnull") do
  verdict do
    File.write(File::NULL, "devnull\n")
    File.read(File::NULL)
  end
end
# The exact failing shape (run 45585078751 — leptris's adapter probe
# under a tight jail): a spawn redirecting to File::NULL. The spawn
# target must be a binary the jail admits under deny, so the only policy
# judgement in play is the device open itself. The runtime's own exe
# cannot serve: RbConfig.ruby spells the in-image ruby path
# (/__tfs__/bin/ruby), and the v2 env image carries no ruby binstub
# (exe + image split by design — the image's bin/ holds gem binstubs
# only), so system() of it never runs (nil child, exit 127) in ANY era.
# The host's floor binary is the answer: the platform floor admits the
# OS's own directories under every policy, so `true` (POSIX) and
# hostname.exe (windows System32) are spawnable under deny in every era.
# On windows the tebako#1021 fail-closed spawn boundary then refuses ANY
# child under a non-trivial policy — the matrix owns that carve-out; the
# exemption's windows proof rides `devnull`.
line("devnullspawn") do
  verdict do
    target =
      if RUBY_PLATFORM.match?(/mingw|mswin/i)
        "#{ENV["SystemRoot"] || "C:/Windows"}/System32/hostname.exe"
      else
        ["/usr/bin/true", "/bin/true"].find { |p| File.executable?(p) }
      end
    raise "no floor binary for the spawn target" if target.nil?
    ok = system(target, out: File::NULL, err: File::NULL)
    raise "system(#{target}, out: File::NULL) failed (status #{$?.inspect})" unless ok
  end
end

RUBY

# The resource arm's payloads ride the same image (spec 08 §2.2, §7 rows
# 23–26/30–31). What each dies of names the bound that held.
cat > "$WORK/app/looper.rb" <<'RUBY'
# The runaway vector (rows 24/25): pure CPU, no sleep, no allocation.
loop {}
RUBY
cat > "$WORK/app/bomber.rb" <<'RUBY'
# The fork-bomb vector (row 23): fork sleeping children until the kernel
# refuses. RLIMIT_NPROC holding surfaces as Errno::EAGAIN — or as a STALL:
# ruby retries a refused fork in a backoff instead of raising promptly
# (process.c), so a fork that has not returned in 8 s is the bound holding
# just the same; the timer thread names it and exits 42. Nothing holding
# prints UNBOUNDED at the cap and exits 44 — self-limiting, and the arm's
# wall backstop then reaps the tree. sync: the record must survive the
# watchdog's group kill whenever it lands.
$stdout.sync = true
children = 0
timer = Thread.new do
  sleep 8
  puts "BOUND:processes children=#{children} (stalled)"
  exit! 42
end
begin
  100.times do
    fork { sleep }
    children += 1
  end
rescue Errno::EAGAIN
  timer.kill
  puts "BOUND:processes children=#{children} (EAGAIN)"
  exit 42
end
timer.kill
puts "UNBOUNDED:processes children=#{children}"
exit 44
RUBY
cat > "$WORK/app/spawner.rb" <<'RUBY'
# The WINDOWS process-bomb vector (row 23): ruby has no fork on windows —
# spawn is the creation spelling, and the Job Object's ACTIVE_PROCESS
# ceiling is the kernel's answer: children created past the ceiling are
# terminated at creation. ARGV: <ceiling> <storm>. Spawn <storm> sleeping
# children, give the kernel a moment, then count the LIVING: at most
# ceiling-1 may survive (the interpreter itself holds one job slot).
# BOUND exits 42 (the POSIX bomber's number); more alive than the ceiling
# permits exits 44 (unbounded). The probe reaps its own spawn either way
# — the bounded case's job kills the tree with the root
# (KILL_ON_JOB_CLOSE), but an UNBOUNDED run has no such backstop and the
# CI runner must never leak the storm.
$stdout.sync = true
ceiling = Integer(ARGV[0])
storm = Integer(ARGV[1])
children = storm.times.map do
  Process.spawn(RbConfig.ruby, "-e", "sleep", out: File::NULL, err: File::NULL)
end
sleep 2
# waitpid(WNOHANG) answers pid for a dead child (reaping it) and nil for
# a live one — the windows-portable aliveness probe (kill(0) is not).
alive = children.count { |pid| Process.waitpid(pid, Process::WNOHANG).nil? }
children.each do |pid|
  begin
    Process.kill("KILL", pid)
  rescue SystemCallError
    nil
  end
end
if alive <= ceiling - 1
  puts "BOUND:processes children=#{alive} (ceiling #{ceiling})"
  exit 42
else
  puts "UNBOUNDED:processes children=#{alive} (ceiling #{ceiling})"
  exit 44
end
RUBY
cat > "$WORK/app/eater.rb" <<'RUBY'
# The memory-hog vector (row 31, Linux): grow address space until the
# kernel refuses. NoMemoryError at the ceiling prints BOUND and exits 43;
# a fatal-allocation abort (SIGABRT from the VM's own mmap path) is the
# same ceiling seen from inside — the arm accepts either, never silence.
chunks = []
begin
  loop { chunks << (" " * (16 * 1024 * 1024)) }
rescue NoMemoryError
  puts "BOUND:memory chunks=#{chunks.size}"
  exit 43
end
RUBY

if [ -n "$PAIR" ]; then
  [ -f "$PAIR" ] || { echo "no runtime executable at $PAIR"; exit 2; }
  [ -f "$PAIR.tfs" ] || { echo "no env image at $PAIR.tfs"; exit 2; }
  [ -n "$TFS_BIN" ] || { echo "--pair needs --tfs (the image writer)"; exit 2; }
  echo "== pair"
  "$TFS_BIN" mkimage "$WORK/app" -o "$WORK/app.tfs" >/dev/null || {
    echo "   app image FAILED"; exit 2;
  }
  EXE="$PAIR"
  if [ "$WIN" = "1" ]; then
    TEBAKO_RUNTIME_IMAGE=$(cygpath -w "$PAIR.tfs")
    ENTRY=(--tebako-image "$(cygpath -w "$WORK/app.tfs"):0:/app" --tebako-entry /probe.rb)
  else
    TEBAKO_RUNTIME_IMAGE="$PAIR.tfs"
    ENTRY=(--tebako-image "$WORK/app.tfs:0:/app" --tebako-entry /probe.rb)
  fi
  export TEBAKO_RUNTIME_IMAGE
  echo "   pair: $PAIR (+ .tfs)"
else
  echo "== press"
  ( cd "$WORK/app" && "$TEBAKO" press -r . -e probe.rb -o "$WORK/pkg" ) || {
    echo "   PRESS FAILED"; exit 2;
  }
  EXE=$(find "$WORK/pkg" -maxdepth 1 -type f -perm -u+x | head -1)
  [ -n "$EXE" ] || { echo "   no pressed executable"; exit 2; }
  ENTRY=()
  echo "   pressed: $EXE"
fi

VECTORS="read open stat exist? readlines binread IO.binread realpath shellout hostdir granted outside madetlink ads devnull devnullspawn"

arm() {
  local label="$1"; local jail="$2"
  local granted="" outside="" linkdir=""
  if [ "$label" = "deny-grant" ]; then
    granted="$WORK/host/granted/ok.txt"
    outside="$WORK/host/secret.txt"
    linkdir="$WORK/host/granted"
  elif [ "$label" = "open" ]; then
    # No policy: the payload may write anywhere, so make the link in the
    # scratch dir the harness owns.
    outside="$WORK/otherdir/target.txt"
    linkdir="$WORK/scratch"
  fi
  echo "== $label"
  local -a envs=()
  if [ -n "$jail" ]; then
    envs+=("TEBAKO_JAIL=$jail")
  fi
  if [ "$label" = "record" ]; then
    envs+=("TEBAKO_JAIL_JOURNAL=$(hostpath "$WORK/journal.log")")
  fi
  [ "$WIN" = "1" ] && envs+=("MSYS2_ARG_CONV_EXCL=*")
  # The arm runs bounded (arc 05's cell): its own process group, the
  # per-arm deadline, the group torn down on any exit. A deadline hit or
  # a reaped orphan leaves the arm's rows ABSENT — the matrix counts
  # them incomplete, never a silent pass.
  if ! run_bounded "$label" env ${envs[@]+"${envs[@]}"} "$EXE" "${ENTRY[@]}" \
    "$(hostpath "$SECRET")" "$(hostpath "$WORK/otherdir")" \
    "$(hostpath "$granted")" "$(hostpath "$outside")" \
    "$(hostpath "$linkdir")" "$(hostpath "$ADS_BASE")" \
    2>"$WORK/$label.err" > "$WORK/$label.out.raw"; then
    : > "$WORK/$label.out.raw"
  fi
  sed 's/^/   /' "$WORK/$label.out.raw"
  mv "$WORK/$label.out.raw" "$WORK/$label.out"
}

arm open ""
arm deny "deny"
arm deny-grant "deny;$(hostpath "$WORK/host/granted"):/grant:ro"
arm record "record"

# --- the resource axis (spec 08 §2.2): the bounds must HOLD, not just
# parse. --pair only — every other shape is a named note, never
# a silent absence. The gate probe boots the pair under a wall-only
# spec: a pair whose driver predates the axis rejects `resources:` at
# parse ("invalid jail spec"), a windows pair predating the Job Object
# arm fails closed on the bound by name ("not enforceable on windows"),
# and the arms then report armed-not-applicable. rsrc_fail = a bound did
# not hold (leak class); rsrc_broken = the arms could not run (harness
# class).
rsrc_fail=0
rsrc_broken=0
rsrc_note=""
rsrc_rows=""
rsrc_row() {
  rsrc_rows="$rsrc_rows$(printf '   %-16s %s\n' "$1" "$2")"$'\n'
}
echo "== resources"
if [ -z "$PAIR" ]; then
  rsrc_note="press mode resolves a PUBLISHED pair — the resource arms exercise the candidate pair's driver directly, so they run under --pair only"
elif [ "$WIN" = "1" ]; then
  # === the windows arm (Job Objects — spec 08 §2.2, §7 row 27) ===
  # Same judgements as the POSIX arm, windows spellings. The arms run
  # BARE and synchronous: the harness's deadline/orphan guard is POSIX
  # machinery (perl setpgid + the process-group kill — named in
  # run_bounded), and on windows the Job Object IS the guard: a bounded
  # run's tree dies with its root (KILL_ON_JOB_CLOSE) or at its wall
  # backstop, by the kernel, with no harness involvement. Every arm
  # declares its own wall_seconds backstop, so a bound failing open
  # still terminates (a wedged pair is the CI job timeout's case). The
  # invocations ride the suite's conversions: windows path spellings in
  # the image triple, MSYS argument conversion off (see arm()).
  RSRC_TRIPLE="$(hostpath "$WORK/app.tfs"):0:/app"
  MSYS2_ARG_CONV_EXCL='*' env \
    "TEBAKO_JAIL=open;resources:wall_seconds=600" \
    "TEBAKO_JAIL_JOURNAL=$(hostpath "$WORK/journal.log")" \
    "$EXE" --tebako-image "$RSRC_TRIPLE" --tebako-entry /probe.rb \
    "$(hostpath "$SECRET")" "$(hostpath "$WORK/otherdir")" "" \
    "$(hostpath "$WORK/otherdir/target.txt")" "$(hostpath "$WORK/scratch")" "" \
    >"$WORK/rsrc-gate.out" 2>"$WORK/rsrc-gate.err"
  gate_rc=$?
  if [ "$gate_rc" != "0" ] && grep -q "invalid jail spec" "$WORK/rsrc-gate.err" 2>/dev/null; then
    rsrc_note="the pair predates the resource axis (its driver rejected resources: at parse) — arms armed, not applicable to this pair"
  elif [ "$gate_rc" != "0" ] && grep -q "not enforceable on windows" "$WORK/rsrc-gate.err" 2>/dev/null; then
    rsrc_note="the pair predates the windows resource arm (its driver fails closed on any declared bound — the pre-Job-Object named refusal) — arms armed, not applicable to this pair"
  elif [ "$gate_rc" != "0" ]; then
    rsrc_note="the gate probe failed to boot (rc $gate_rc — see rsrc-gate.err) — the pair or the harness, never a silent pass"
    rsrc_broken=1
  else
    # wall (spec row 24): the watchdog thread fires at the deadline —
    # TerminateJobObject at exit 137 (the POSIX SIGKILL's number — one
    # observed exit for one verdict), the named stderr line, the
    # jail-resource journal line.
    start=$(date +%s)
    MSYS2_ARG_CONV_EXCL='*' env \
      "TEBAKO_JAIL=open;resources:wall_seconds=3" \
      "TEBAKO_JAIL_JOURNAL=$(hostpath "$WORK/journal.log")" \
      "$EXE" --tebako-image "$RSRC_TRIPLE" --tebako-entry /looper.rb \
      >"$WORK/rsrc-wall.out" 2>"$WORK/rsrc-wall.err"
    rc=$?
    elapsed=$(( $(date +%s) - start ))
    if [ "$rc" = 137 ] && [ "$elapsed" -ge 3 ] && [ "$elapsed" -le 30 ] \
       && grep -q "resource bound exceeded (wall_seconds=3)" "$WORK/rsrc-wall.err" 2>/dev/null \
       && grep -q "event=jail-resource .*cause=wall" "$WORK/journal.log" 2>/dev/null; then
      rsrc_row wall "held — the job terminated at ${elapsed}s (exit 137), the named stderr line, the jail-resource journal line"
    else
      rsrc_row wall "LEAK — rc=$rc after ${elapsed}s (want 137 at 3s + the stderr line + the journal line): $(tail -1 "$WORK/rsrc-wall.err")"
      rsrc_fail=$((rsrc_fail + 1))
    fi

    # cpu (spec row 25): the per-process user-time ceiling terminates
    # the looper at ~2s; the wall backstop (25s) must NOT fire first.
    # Windows has no signal spelling — the kernel chooses the exit
    # code — so the arm asserts death-in-window + no wall line and
    # names the observed code.
    start=$(date +%s)
    MSYS2_ARG_CONV_EXCL='*' env \
      "TEBAKO_JAIL=open;resources:cpu_seconds=2,wall_seconds=25" \
      "TEBAKO_JAIL_JOURNAL=$(hostpath "$WORK/journal.log")" \
      "$EXE" --tebako-image "$RSRC_TRIPLE" --tebako-entry /looper.rb \
      >"$WORK/rsrc-cpu.out" 2>"$WORK/rsrc-cpu.err"
    rc=$?
    elapsed=$(( $(date +%s) - start ))
    if [ "$rc" != "0" ] && [ "$elapsed" -ge 2 ] && [ "$elapsed" -le 20 ] \
       && ! grep -q "resource bound exceeded" "$WORK/rsrc-cpu.err" 2>/dev/null; then
      rsrc_row cpu "held — terminated at ${elapsed}s (kernel exit $rc; the wall backstop never fired)"
    else
      rsrc_row cpu "LEAK — rc=$rc after ${elapsed}s (want a kill at ~2s, no wall line): $(tail -1 "$WORK/rsrc-cpu.err")"
      rsrc_fail=$((rsrc_fail + 1))
    fi

    # bomber (spec row 23): ruby has no fork on windows — spawn is the
    # creation spelling, and the job's ACTIVE_PROCESS ceiling
    # terminates the over-ceiling children at creation. The probe
    # counts the LIVING: at most ceiling-1 survive (the interpreter
    # holds one slot).
    MSYS2_ARG_CONV_EXCL='*' env \
      "TEBAKO_JAIL=open;resources:processes=8,wall_seconds=60" \
      "$EXE" --tebako-image "$RSRC_TRIPLE" --tebako-entry /spawner.rb \
      8 16 \
      >"$WORK/rsrc-bomber.out" 2>"$WORK/rsrc-bomber.err"
    rc=$?
    if [ "$rc" = 42 ] && grep -q "BOUND:processes children=" "$WORK/rsrc-bomber.out" 2>/dev/null; then
      rsrc_row bomber "held — children died at creation past the job's ceiling ($(grep -o 'children=[0-9]*' "$WORK/rsrc-bomber.out" | head -1)), exit 42"
    else
      rsrc_row bomber "LEAK — rc=$rc (want 42 + the BOUND line; 44 = more children alive than the ceiling): $(tail -1 "$WORK/rsrc-bomber.out")"
      rsrc_fail=$((rsrc_fail + 1))
    fi

    # memory (spec row 31): commit growth fails at the job's
    # per-process ceiling — ruby's NoMemoryError (exit 43 + BOUND). The
    # macOS refusal class has no windows spelling: the limit ENFORCES
    # here, so the row proves it exactly like Linux's.
    MSYS2_ARG_CONV_EXCL='*' env \
      "TEBAKO_JAIL=open;resources:memory=1G,wall_seconds=45" \
      "TEBAKO_JAIL_JOURNAL=$(hostpath "$WORK/journal.log")" \
      "$EXE" --tebako-image "$RSRC_TRIPLE" --tebako-entry /eater.rb \
      >"$WORK/rsrc-memory.out" 2>"$WORK/rsrc-memory.err"
    rc=$?
    if [ "$rc" = 43 ] && grep -q "BOUND:memory chunks=" "$WORK/rsrc-memory.out" 2>/dev/null; then
      rsrc_row memory "held — NoMemoryError at the commit ceiling ($(grep -o 'chunks=[0-9]*' "$WORK/rsrc-memory.out" | head -1))"
    else
      rsrc_row memory "LEAK — rc=$rc (want 43 + BOUND; a run to the wall backstop means the ceiling failed open): $(tail -1 "$WORK/rsrc-memory.out" "$WORK/rsrc-memory.err" 2>/dev/null)"
      rsrc_fail=$((rsrc_fail + 1))
    fi

    # control (spec row 26): generous bounds, nothing stressed — the
    # run is byte-identical to the open arm. processes=64 is the
    # windows spelling of the POSIX generous ceiling (no per-uid count
    # exists here; the probe peaks at 2 — the interpreter + its
    # shellout child).
    MSYS2_ARG_CONV_EXCL='*' env \
      "TEBAKO_JAIL=open;resources:cpu_seconds=300,memory=2G,processes=64,wall_seconds=120" \
      "TEBAKO_JAIL_JOURNAL=$(hostpath "$WORK/journal.log")" \
      "$EXE" --tebako-image "$RSRC_TRIPLE" --tebako-entry /probe.rb \
      "$(hostpath "$SECRET")" "$(hostpath "$WORK/otherdir")" "" \
      "$(hostpath "$WORK/otherdir/target.txt")" "$(hostpath "$WORK/scratch")" "" \
      >"$WORK/rsrc-control.out" 2>"$WORK/rsrc-control.err"
    rc=$?
    if [ "$rc" = 0 ] && diff -q "$WORK/rsrc-control.out" "$WORK/open.out" >/dev/null 2>&1; then
      rsrc_row control "held — byte-identical to the open arm under generous bounds"
    else
      rsrc_row control "LEAK — rc=$rc and/or output drifted from the open arm: $(diff "$WORK/rsrc-control.out" "$WORK/open.out" 2>/dev/null | head -3 | tr '\n' ' ')"
      rsrc_fail=$((rsrc_fail + 1))
    fi
  fi
else
  # SIGXCPU's default action cores on some hosts; the arms prove death
  # by signal, and the suite leaves no core files behind.
  ulimit -c 0 2>/dev/null || true
  run_bounded_bare "rsrc-gate" 30 env \
    "TEBAKO_JAIL=open;resources:wall_seconds=600" \
    "TEBAKO_JAIL_JOURNAL=$(hostpath "$WORK/journal.log")" \
    "$EXE" --tebako-image "$WORK/app.tfs:0:/app" --tebako-entry /probe.rb \
    "$(hostpath "$SECRET")" "$(hostpath "$WORK/otherdir")" "" \
    "$(hostpath "$WORK/otherdir/target.txt")" "$(hostpath "$WORK/scratch")" "" \
    >"$WORK/rsrc-gate.out" 2>"$WORK/rsrc-gate.err"
  gate_rc=$?
  if [ "$gate_rc" != "0" ] && grep -q "invalid jail spec" "$WORK/rsrc-gate.err" 2>/dev/null; then
    rsrc_note="the pair predates the resource axis (its driver rejected resources: at parse) — arms armed, not applicable to this pair"
  elif [ "$gate_rc" != "0" ]; then
    rsrc_note="the gate probe failed to boot (rc $gate_rc — see rsrc-gate.err) — the pair or the harness, never a silent pass"
    rsrc_broken=1
  else
    UID_PROCS=$(ps -eo uid= 2>/dev/null | awk -v u="$(id -u)" '$1 == u' | wc -l | tr -d ' ')
    OS="$(uname -s)"

    # wall (spec row 24): the watchdog fires at the deadline — SIGKILL,
    # the named stderr line, the jail-resource journal line.
    start=$(date +%s)
    run_bounded_bare "rsrc-wall" 45 env \
      "TEBAKO_JAIL=open;resources:wall_seconds=3" \
      "TEBAKO_JAIL_JOURNAL=$(hostpath "$WORK/journal.log")" \
      "$EXE" --tebako-image "$WORK/app.tfs:0:/app" --tebako-entry /looper.rb \
      >"$WORK/rsrc-wall.out" 2>"$WORK/rsrc-wall.err"
    rc=$?
    elapsed=$(( $(date +%s) - start ))
    if [ "$rc" = 137 ] && [ "$elapsed" -ge 3 ] && [ "$elapsed" -le 30 ] \
       && grep -q "resource bound exceeded (wall_seconds=3)" "$WORK/rsrc-wall.err" 2>/dev/null \
       && grep -q "event=jail-resource .*cause=wall" "$WORK/journal.log" 2>/dev/null; then
      rsrc_row wall "held — SIGKILL at ${elapsed}s, the named stderr line, the jail-resource journal line"
    else
      rsrc_row wall "LEAK — rc=$rc after ${elapsed}s (want 137 at 3s + the stderr line + the journal line): $(tail -1 "$WORK/rsrc-wall.err")"
      rsrc_fail=$((rsrc_fail + 1))
    fi

    # cpu (spec row 25): SIGXCPU at the RLIMIT_CPU ceiling; the wall
    # backstop (25s) must NOT fire first.
    start=$(date +%s)
    run_bounded_bare "rsrc-cpu" 45 env \
      "TEBAKO_JAIL=open;resources:cpu_seconds=2,wall_seconds=25" \
      "TEBAKO_JAIL_JOURNAL=$(hostpath "$WORK/journal.log")" \
      "$EXE" --tebako-image "$WORK/app.tfs:0:/app" --tebako-entry /looper.rb \
      >"$WORK/rsrc-cpu.out" 2>"$WORK/rsrc-cpu.err"
    rc=$?
    elapsed=$(( $(date +%s) - start ))
    if [ "$rc" = 152 ] && [ "$elapsed" -ge 2 ] && [ "$elapsed" -le 30 ] \
       && ! grep -q "resource bound exceeded" "$WORK/rsrc-cpu.err" 2>/dev/null; then
      rsrc_row cpu "held — SIGXCPU at ${elapsed}s (the wall backstop never fired)"
    else
      rsrc_row cpu "LEAK — rc=$rc after ${elapsed}s (want 152 = 128+SIGXCPU at ~2s, no wall line): $(tail -1 "$WORK/rsrc-cpu.err")"
      rsrc_fail=$((rsrc_fail + 1))
    fi

    # bomber (spec row 23): fork stops at the RLIMIT_NPROC ceiling — the
    # kernel's refusal surfaces as EAGAIN or, through ruby's retry backoff,
    # as a stall the vector names on its own timer; the watchdog reaps the
    # sleeper children with the root (the orphan guard proves the group
    # emptied).
    run_bounded_bare "rsrc-bomber" 90 env \
      "TEBAKO_JAIL=open;resources:processes=$((UID_PROCS + 30)),wall_seconds=60" \
      "TEBAKO_JAIL_JOURNAL=$(hostpath "$WORK/journal.log")" \
      "$EXE" --tebako-image "$WORK/app.tfs:0:/app" --tebako-entry /bomber.rb \
      >"$WORK/rsrc-bomber.out" 2>"$WORK/rsrc-bomber.err"
    rc=$?
    if [ "$rc" = 42 ] && grep -q "BOUND:processes children=" "$WORK/rsrc-bomber.out" 2>/dev/null; then
      rsrc_row bomber "held — fork failed EAGAIN at the ceiling ($(grep -o 'children=[0-9]*' "$WORK/rsrc-bomber.out" | head -1)), exit 42, the tree reaped with the root"
    else
      rsrc_row bomber "LEAK — rc=$rc (want 42 + the BOUND line; 44 = unbounded at the 100-fork cap, 137 = the backstop fired): $(tail -1 "$WORK/rsrc-bomber.out")"
      rsrc_fail=$((rsrc_fail + 1))
    fi

    case "$OS" in
      Linux)
        # memory (spec row 31): growth stops at the RLIMIT_AS ceiling —
        # ruby's NoMemoryError (exit 43 + BOUND) or the VM's
        # fatal-allocation abort (SIGABRT), never a silent overshoot.
        run_bounded_bare "rsrc-memory" 75 env \
          "TEBAKO_JAIL=open;resources:memory=1G,wall_seconds=45" \
          "TEBAKO_JAIL_JOURNAL=$(hostpath "$WORK/journal.log")" \
          "$EXE" --tebako-image "$WORK/app.tfs:0:/app" --tebako-entry /eater.rb \
          >"$WORK/rsrc-memory.out" 2>"$WORK/rsrc-memory.err"
        rc=$?
        if [ "$rc" = 43 ] && grep -q "BOUND:memory chunks=" "$WORK/rsrc-memory.out" 2>/dev/null; then
          rsrc_row memory "held — NoMemoryError at the ceiling ($(grep -o 'chunks=[0-9]*' "$WORK/rsrc-memory.out" | head -1))"
        elif [ "$rc" = 134 ] && grep -qi "failed to allocate memory" "$WORK/rsrc-memory.err" 2>/dev/null; then
          rsrc_row memory "held — the VM's fatal-allocation abort at the ceiling"
        else
          rsrc_row memory "LEAK — rc=$rc (want 43 + BOUND, or 134 + the fatal line): $(tail -1 "$WORK/rsrc-memory.out" "$WORK/rsrc-memory.err" 2>/dev/null)"
          rsrc_fail=$((rsrc_fail + 1))
        fi
        ;;
      Darwin)
        # memory-refusal (spec row 30): the kernel's RLIMIT_AS floor
        # refuses every realistic bound — fail closed, exit 73, by name.
        run_bounded_bare "rsrc-memory-refusal" 30 env \
          "TEBAKO_JAIL=open;resources:memory=256M" \
          "$EXE" --tebako-image "$WORK/app.tfs:0:/app" --tebako-entry /probe.rb \
          "$(hostpath "$SECRET")" "$(hostpath "$WORK/otherdir")" "" \
          "$(hostpath "$WORK/otherdir/target.txt")" "$(hostpath "$WORK/scratch")" "" \
          >"$WORK/rsrc-memory-refusal.out" 2>"$WORK/rsrc-memory-refusal.err"
        rc=$?
        if [ "$rc" = 73 ] && grep -q "resources: memory:" "$WORK/rsrc-memory-refusal.err" 2>/dev/null \
           && grep -q "address-space floor" "$WORK/rsrc-memory-refusal.err" 2>/dev/null; then
          rsrc_row memory-refusal "held — refused by name (exit 73, the kernel's RLIMIT_AS floor)"
        else
          rsrc_row memory-refusal "LEAK — rc=$rc (want 73 + the named refusal): $(tail -1 "$WORK/rsrc-memory-refusal.err")"
          rsrc_fail=$((rsrc_fail + 1))
        fi
        ;;
      *)
        rsrc_row memory "note: no memory arm defined for $OS"
        ;;
    esac

    # control (spec row 26): generous bounds, nothing stressed — the run
    # is byte-identical to the open arm. Linux adds the memory bound;
    # macOS omits it (row 30's floor).
    if [ "$OS" = "Linux" ]; then
      control_spec="cpu_seconds=300,memory=2G,processes=$((UID_PROCS + 400)),wall_seconds=120"
    else
      control_spec="cpu_seconds=300,processes=$((UID_PROCS + 400)),wall_seconds=120"
    fi
    run_bounded_bare "rsrc-control" 150 env \
      "TEBAKO_JAIL=open;resources:$control_spec" \
      "TEBAKO_JAIL_JOURNAL=$(hostpath "$WORK/journal.log")" \
      "$EXE" --tebako-image "$WORK/app.tfs:0:/app" --tebako-entry /probe.rb \
      "$(hostpath "$SECRET")" "$(hostpath "$WORK/otherdir")" "" \
      "$(hostpath "$WORK/otherdir/target.txt")" "$(hostpath "$WORK/scratch")" "" \
      >"$WORK/rsrc-control.out" 2>"$WORK/rsrc-control.err"
    rc=$?
    if [ "$rc" = 0 ] && diff -q "$WORK/rsrc-control.out" "$WORK/open.out" >/dev/null 2>&1; then
      rsrc_row control "held — byte-identical to the open arm under generous bounds"
    else
      rsrc_row control "LEAK — rc=$rc and/or output drifted from the open arm: $(diff "$WORK/rsrc-control.out" "$WORK/open.out" 2>/dev/null | head -3 | tr '\n' ' ')"
      rsrc_fail=$((rsrc_fail + 1))
    fi
  fi
fi
if [ -n "$rsrc_note" ]; then
  echo "   note: $rsrc_note"
fi
[ -n "$rsrc_rows" ] && printf '%s' "$rsrc_rows"

leaks=0
overdenials=0
incomplete=0
knowngaps=0
dev_note=""
echo "== matrix"
printf '   %-11s %-11s %-16s %-16s %-9s %s\n' vector open deny deny+grant record verdict
for v in $VECTORS; do
  o=$(awk -F'\t' -v v="$v" '$1 == v {print $2}' "$WORK/open.out")
  d=$(awk -F'\t' -v v="$v" '$1 == v {print $2}' "$WORK/deny.out")
  g=$(awk -F'\t' -v v="$v" '$1 == v {print $2}' "$WORK/deny-grant.out")
  r=$(awk -F'\t' -v v="$v" '$1 == v {print $2}' "$WORK/record.out")
  case "$v" in
    devnull|devnullspawn)
      # spec 08 §1's deny-rule amendment (tebako#1128): the device nodes
      # are exempt from the deny stance — usable under EVERY arm, so a
      # refusal on a pair that CARRIES the exemption is the over-denial
      # the issue names, counted on its own failure line (a jail that
      # breaks a process-essential open fails; it has not "blocked"
      # anything). Two named carve-outs, never silent passes:
      #
      #   * the suite runs by contract against the PUBLISHED pair, which
      #     may predate the exemption. That era's signature is the deny
      #     arm answering DENIED:EPERM while the record arm (allow-all)
      #     reads — the resource axis's gate convention: the row reports
      #     armed-not-applicable with a note, and nothing is counted.
      #     A pair whose deny arm reads carries the exemption and
      #     answers to the strict rule.
      #   * on windows the tebako#1021 fail-closed spawn boundary refuses
      #     ANY child under a non-trivial policy, so a DENIED:EPERM from
      #     devnullspawn there is the boundary answering, not the device
      #     verdict — era-independent, and the exemption's windows proof
      #     rides `devnull`.
      if [ -z "$o" ] || [ -z "$d" ] || [ -z "$g" ] || [ -z "$r" ]; then
        verdict="MISSING"; incomplete=$((incomplete + 1))
      elif [ "${o#READ-OK}" != "" ]; then
        verdict="probe not live under open ($o)"; incomplete=$((incomplete + 1))
      elif [ "$v" = "devnullspawn" ] && [ "$WIN" = "1" ] \
           && [ "$d" = "DENIED:EPERM" ] && [ "$g" = "DENIED:EPERM" ] && [ "$r" = "DENIED:EPERM" ]; then
        verdict="exempt under open; the spawn boundary owns the non-trivial refusal (tebako#1021)"
      elif [ "$d" = "DENIED:EPERM" ] && [ "$r" = "READ-OK" ]; then
        verdict="the pair predates the exemption — armed, not applicable to this pair"
        dev_note="the pair predates the device-node exemption (tebako#1128) — the device vectors are armed, not applicable to this pair"
      elif [ "$d" = "READ-OK" ]; then
        bad=""
        [ "${g#READ-OK}" != "" ] && bad="grant=$g"
        [ "${r#READ-OK}" != "" ] && bad="$bad record=$r"
        if [ -z "$bad" ]; then
          verdict="exempt under every policy"
        else
          verdict="OVER-DENIED ($bad) — a device node must never be refused"
          overdenials=$((overdenials + 1))
        fi
      else
        verdict="unexpected verdicts (deny=$d grant=$g record=$r)"; incomplete=$((incomplete + 1))
      fi
      ;;
    ads)
      # NTFS-only: a skip in the open arm marks the vector inapplicable
      # on this host, never incomplete. Once live, it answers to the
      # same rule as every plain vector.
      if [ "$o" = "SKIPPED:not-ntfs" ]; then
        verdict="not applicable (no NTFS)"
      elif [ -z "$o" ] || [ -z "$d" ] || [ -z "$r" ]; then
        verdict="MISSING"; incomplete=$((incomplete + 1))
      elif [ "${o#READ-OK}" != "" ]; then
        verdict="probe not live ($o)"; incomplete=$((incomplete + 1))
      elif [ "${r#READ-OK}" != "" ]; then
        verdict="record did not allow ($r)"; incomplete=$((incomplete + 1))
      else
        case "$d" in
          READ-OK) verdict="LEAK"; leaks=$((leaks + 1)) ;;
          DENIED:EACCES|DENIED:EPERM) verdict="blocked" ;;
          *) verdict="not a refusal ($d)"; incomplete=$((incomplete + 1)) ;;
        esac
      fi
      ;;
    granted|outside|madetlink)
      # Grant-arm-only vectors: the other arms skip them by design.
      if [ "$v" = "madetlink" ]; then
        # Live under the open arm (the link is made and followed) and
        # refused in the grant arm — the target's policy, not the link's.
        if [ "${o#READ-OK}" != "" ]; then
          verdict="probe not live under open ($o)"; incomplete=$((incomplete + 1))
        else
          case "$g" in
            READ-OK) verdict="LEAK"; leaks=$((leaks + 1)) ;;
            DENIED:EACCES|DENIED:EPERM) verdict="link judged by its target" ;;
            *) verdict="not a refusal ($g)"; incomplete=$((incomplete + 1)) ;;
          esac
        fi
      elif [ "$v" = "granted" ]; then
        case "$g" in
          READ-OK) verdict="grant honours" ;;
          *) verdict="grant does not admit ($g)"; incomplete=$((incomplete + 1)) ;;
        esac
      else
        case "$g" in
          READ-OK) verdict="LEAK"; leaks=$((leaks + 1)) ;;
          DENIED:EACCES|DENIED:EPERM) verdict="grant boundary holds" ;;
          *) verdict="not a refusal ($g)"; incomplete=$((incomplete + 1)) ;;
        esac
      fi
      ;;
    *)
      if [ -z "$o" ] || [ -z "$d" ] || [ -z "$r" ]; then
        verdict="MISSING"; incomplete=$((incomplete + 1))
      elif [ "${o#READ-OK}" != "" ]; then
        verdict="probe not live"; incomplete=$((incomplete + 1))
      elif [ "${r#READ-OK}" != "" ] && { [ "$v" != "shellout" ] || [ "$WIN" != "1" ] || [ "$r" != "DENIED:EPERM" ]; }; then
        # record allows and journals: a vector it refuses is a broken mode.
        # Exception, the windows shellout (tebako#1039's boundary): the
        # child carries no preload, so record mode could never journal it —
        # the boundary refuses the child instead, and that refusal IS the
        # record contract holding (nothing escapes unrecorded). The row
        # then falls through to the deny-arm verdict below.
        verdict="record did not allow ($r)"; incomplete=$((incomplete + 1))
      else
        case "$d" in
          READ-OK)
            verdict="LEAK"; leaks=$((leaks + 1)) ;;
          DENIED:EACCES|DENIED:EPERM) verdict="blocked" ;;
          DENIED:*)
            case "$v" in
              shellout) verdict="blocked (spawn refused: $d)" ;;
              *) verdict="not a refusal ($d)"; incomplete=$((incomplete + 1)) ;;
            esac
            ;;
          *) verdict="not a refusal ($d)"; incomplete=$((incomplete + 1)) ;;
        esac
      fi
      ;;
  esac
  printf '   %-11s %-11s %-16s %-16s %-9s %s\n' "$v" "${o:-?}" "${d:-?}" "${g:-?}" "${r:-?}" "$verdict"
done
if [ -n "$dev_note" ]; then
  echo "   note: $dev_note"
fi

echo "== record journal"
journal_bad=0
if [ -s "$WORK/journal.log" ]; then
  echo "   $(wc -l < "$WORK/journal.log" | tr -d ' ') line(s), $(wc -c < "$WORK/journal.log" | tr -d ' ') bytes"
else
  echo "   EMPTY — a record policy that allows silently is not an audit trail"
  journal_bad=1
fi

echo "== verdict"
# An over-denial outranks nothing but ranks with a leak: a device node
# the policy refused is the jail breaking a process-essential open
# (tebako#1128) — the same exit class, its own line.
if [ "$overdenials" -ne 0 ]; then
  echo "   OVER-DENIAL: $overdenials device vector(s) refused — the fixed device set must be usable under every policy (tebako#1128)"
  exit 1
fi
# A leak outranks an empty journal: bytes reached a payload the policy
# refused is the harm; the missing trail is how it went unseen.
if [ "$leaks" -ne 0 ]; then
  echo "   LEAK: $leaks vector(s) reached a byte the policy did not grant"
  [ "$journal_bad" -ne 0 ] && echo "   and the record journal stayed empty — the same blind spot, seen from the audit side"
  exit 1
fi
if [ "$rsrc_fail" -ne 0 ]; then
  echo "   LEAK: $rsrc_fail resource arm(s) failed — a declared bound did not hold (the resource rows name it)"
  exit 1
fi
if [ "$rsrc_broken" -ne 0 ]; then
  echo "   INCOMPLETE: the resource arms could not run — $rsrc_note"
  exit 2
fi
if [ "$journal_bad" -ne 0 ]; then
  echo "   INCOMPLETE: the record journal is empty — a record policy that allows silently is not an audit trail"
  exit 2
fi
if [ "$ORPHANS" -ne 0 ]; then
  echo "   ORPHANS: $ORPHANS arm(s) left a live process group — reaped by the guard; a runtime that orphans its children fails this suite on its own"
  exit 2
fi
if [ "$incomplete" -ne 0 ]; then
  echo "   INCOMPLETE: $incomplete vector(s) missing, not live, refused under record, or answered without a refusal — the harness or the engine, never counted as a block"
  exit 2
fi
if [ "$knowngaps" -ne 0 ]; then
  echo "   CONFORMANT with $knowngaps named known-gap(s) — filed and tracked, never counted as covered"
else
  echo "   CONFORMANT: every covered vector live under the open policy, refused under deny, the grant admits its own directory while refusing its neighbour, and record allows while journaling"
fi
[ -n "$rsrc_rows" ] && echo "   and the resource axis held — wall, cpu, processes, memory (per-platform), and the under-bounds control (the rows above)"
# Every failure family exited explicitly above; reaching the end is the
# conformant verdict — never the && list's status (an empty rsrc_rows on
# a pre-axis pair would otherwise read as exit 1).
exit 0
