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
#
#   not covered by this cut, named rather than implied
#     a `..` escape out of a grant (the jail's namespace is lexical: an
#     escape lands in VFS space, where only mounts exist, so the vector
#     measures path arithmetic rather than the policy)
#     a payload-made link pointing at a host path (its target is judged by
#     the host guard, which needs its own arm and a spelling the policy
#     answers EPERM on rather than ENOENT — the two vectors disagree today
#     and neither is evidence until that is settled)
#     case folding (needs a case-insensitive host file system: macOS)
#     alternate data streams (needs NTFS)
#     TOCTOU on materialization (needs a race harness)
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
printf 'other\n' > "$WORK/otherdir/one.txt"

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
  TEBAKO_RUNTIME_IMAGE="$PAIR.tfs"
  export TEBAKO_RUNTIME_IMAGE
  ENTRY=(--tebako-image "$WORK/app.tfs:0:/app" --tebako-entry /probe.rb)
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

VECTORS="read open stat exist? readlines binread IO.binread realpath shellout hostdir granted outside"

arm() {
  local label="$1"; local jail="$2"
  local granted="" outside=""
  if [ "$label" = "deny-grant" ]; then
    granted="$WORK/host/granted/ok.txt"
    outside="$WORK/host/secret.txt"
  fi
  echo "== $label"
  local -a envs=()
  if [ -n "$jail" ]; then
    envs+=("TEBAKO_JAIL=$jail")
  fi
  if [ "$label" = "record" ]; then
    envs+=("TEBAKO_JAIL_JOURNAL=$WORK/journal.log")
  fi
  env "${envs[@]}" "$EXE" "${ENTRY[@]}" "$SECRET" "$WORK/otherdir" "$granted" "$outside" \
    2>"$WORK/$label.err" | tee "$WORK/$label.out" | sed 's/^/   /'
}

arm open ""
arm deny "deny"
arm deny-grant "deny;$WORK/host/granted:/grant:ro"
arm record "record"

leaks=0
incomplete=0
echo "== matrix"
printf '   %-11s %-11s %-16s %-16s %-9s %s\n' vector open deny deny+grant record verdict
for v in $VECTORS; do
  o=$(awk -F'\t' -v v="$v" '$1 == v {print $2}' "$WORK/open.out")
  d=$(awk -F'\t' -v v="$v" '$1 == v {print $2}' "$WORK/deny.out")
  g=$(awk -F'\t' -v v="$v" '$1 == v {print $2}' "$WORK/deny-grant.out")
  r=$(awk -F'\t' -v v="$v" '$1 == v {print $2}' "$WORK/record.out")
  case "$v" in
    granted|outside)
      # Grant-arm-only vectors: the other arms skip them by design.
      if [ "$v" = "granted" ]; then
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
          READ-OK) verdict="LEAK"; leaks=$((leaks + 1)) ;;
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
if [ "$incomplete" -ne 0 ]; then
  echo "   INCOMPLETE: $incomplete vector(s) missing, not live, refused under record, or answered without a refusal — the harness or the engine, never counted as a block"
  exit 2
fi
echo "   CONFORMANT: every covered vector live under the open policy, refused under deny, the grant admits its own directory while refusing its neighbour, and record allows while journaling"
