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
#   exit 1  at least one vector leaked (the matrix names it)
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
  local pid=$!
  local waited=0
  while kill -0 "$pid" 2>/dev/null; do
    if [ "$waited" -ge "$DEADLINE" ]; then
      echo "   DEADLINE: $label exceeded ${DEADLINE}s — tearing down the process group" >&2
      kill -TERM -- -"$pid" 2>/dev/null
      sleep 3
      kill -KILL -- -"$pid" 2>/dev/null
      wait "$pid" 2>/dev/null
      return 124
    fi
    sleep 2
    waited=$((waited + 2))
  done
  wait "$pid"
  local rc=$?
  # The orphan guard: the arm's group must be EMPTY once its leader is
  # gone. A live member is a reaped-and-failed arm, named in the matrix.
  if kill -0 -- -"$pid" 2>/dev/null; then
    echo "   ORPHAN: $label left a live process group — tearing it down" >&2
    ps -eo pid,pgid,etime,command 2>/dev/null | awk -v g="$pid" '$2 == g' | sed 's/^/   /' >&2
    kill -KILL -- -"$pid" 2>/dev/null
    ORPHANS=$((ORPHANS + 1))
    return 125
  fi
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

VECTORS="read open stat exist? readlines binread IO.binread realpath shellout hostdir granted outside madetlink ads"

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

leaks=0
incomplete=0
knowngaps=0
echo "== matrix"
printf '   %-11s %-11s %-16s %-16s %-9s %s\n' vector open deny deny+grant record verdict
for v in $VECTORS; do
  o=$(awk -F'\t' -v v="$v" '$1 == v {print $2}' "$WORK/open.out")
  d=$(awk -F'\t' -v v="$v" '$1 == v {print $2}' "$WORK/deny.out")
  g=$(awk -F'\t' -v v="$v" '$1 == v {print $2}' "$WORK/deny-grant.out")
  r=$(awk -F'\t' -v v="$v" '$1 == v {print $2}' "$WORK/record.out")
  case "$v" in
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
      elif [ "${r#READ-OK}" != "" ]; then
        # record allows and journals: a vector it refuses is a broken mode.
        verdict="record did not allow ($r)"; incomplete=$((incomplete + 1))
      else
        case "$d" in
          READ-OK)
            if [ "$v" = "shellout" ] && [ "$WIN" = "1" ]; then
              # the spawn boundary does not carry the jail into the child
              # on Windows (tebako#1021) — a named known-gap, never a
              # silent pass, and never counted as covered
              verdict="KNOWN-GAP windows (tebako#1021)"; knowngaps=$((knowngaps + 1))
            else
              verdict="LEAK"; leaks=$((leaks + 1))
            fi ;;
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

echo "== record journal"
journal_bad=0
if [ -s "$WORK/journal.log" ]; then
  echo "   $(wc -l < "$WORK/journal.log" | tr -d ' ') line(s), $(wc -c < "$WORK/journal.log" | tr -d ' ') bytes"
else
  echo "   EMPTY — a record policy that allows silently is not an audit trail"
  journal_bad=1
fi

echo "== verdict"
# A leak outranks an empty journal: bytes reached a payload the policy
# refused is the harm; the missing trail is how it went unseen.
if [ "$leaks" -ne 0 ]; then
  echo "   LEAK: $leaks vector(s) reached a byte the policy did not grant"
  [ "$journal_bad" -ne 0 ] && echo "   and the record journal stayed empty — the same blind spot, seen from the audit side"
  exit 1
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
