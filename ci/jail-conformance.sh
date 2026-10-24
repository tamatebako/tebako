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
#     3  File.stat            7  IO.binread
#     4  File.exist?          8  read(realpath)
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
# Every refusal must be EACCES/EPERM. ENOENT is NOT a refusal: it means
# the engine passed the path to the host and the host had nothing there,
# which is a different answer and never counts as a block.
#
# Usage: jail-conformance.sh <tebako-cli> [workdir]
#   exit 0  every covered vector live under `open` and refused under deny
#   exit 1  at least one vector leaked (the matrix names it)
#   exit 2  the harness could not run (press, runtime, or a broken probe)
#
# This is a RELEASE-rung check, not a PR gate: it needs a runtime pair,
# and on a pair that predates the interpreter-side host guard the deny
# arms leak by construction (tebako#917). The matrix is the evidence
# either way.
set -uo pipefail

TEBAKO="${1:?usage: jail-conformance.sh <tebako-cli> [workdir]}"
WORK="${2:-/tmp/jail-conformance}"
rm -rf "$WORK"
mkdir -p "$WORK/app" "$WORK/home" "$WORK/host/private" "$WORK/scratch"
export HOME="$WORK/home"

SECRET="$WORK/host/secret.txt"
printf 'secret-bytes\n' > "$SECRET"
printf 'inner\n' > "$WORK/host/private/inner.txt"

cat > "$WORK/app/probe.rb" <<'RUBY'
# One line per vector: NAME<TAB>verdict. READ-OK means the byte (or the
# listing) arrived; DENIED:<class> means a policy refused it;
# SKIPPED:<why> means this arm cannot exercise the vector; BROKEN:<why>
# means the probe itself failed and the vector proves nothing.
secret = ARGV[0]
hostdir = ARGV[1]
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
line("exist?")      { verdict { File.exist?(secret) ? true : raise(Errno::ENOENT) } }
line("readlines")   { verdict { File.readlines(secret) } }
line("binread")     { verdict { File.binread(secret) } }
line("IO.binread")  { verdict { IO.binread(secret) } }
line("realpath")    { verdict { File.read(File.realpath(secret)) } }
line("shellout")    { verdict { IO.popen(["cat", secret], &:read).empty? ? raise(Errno::ENOENT) : true } }
line("hostdir")     { verdict { raise Errno::ENOENT if Dir.children(hostdir).empty? } }

RUBY

echo "== press"
( cd "$WORK/app" && "$TEBAKO" press -r . -e probe.rb -o "$WORK/pkg" ) || {
  echo "   PRESS FAILED"; exit 2;
}
EXE=$(find "$WORK/pkg" -maxdepth 1 -type f -perm -u+x | head -1)
[ -n "$EXE" ] || { echo "   no pressed executable"; exit 2; }
echo "   pressed: $EXE"

VECTORS="read open stat exist? readlines binread IO.binread realpath shellout hostdir"

arm() {
  local label="$1"; local jail="$2"
  echo "== $label"
  if [ -n "$jail" ]; then
    TEBAKO_JAIL="$jail" "$EXE" "$SECRET" "$WORK/host" 2>"$WORK/$label.err" \
      | tee "$WORK/$label.out" | sed 's/^/   /'
  else
    "$EXE" "$SECRET" "$WORK/host" 2>"$WORK/$label.err" \
      | tee "$WORK/$label.out" | sed 's/^/   /'
  fi
}

arm open ""
arm deny "deny"

leaks=0
incomplete=0
echo "== matrix"
printf '   %-11s %-11s %-16s %s\n' vector open deny verdict
for v in $VECTORS; do
  o=$(awk -F'\t' -v v="$v" '$1 == v {print $2}' "$WORK/open.out")
  d=$(awk -F'\t' -v v="$v" '$1 == v {print $2}' "$WORK/deny.out")
  if [ -z "$o" ] || [ -z "$d" ]; then
    verdict="MISSING"; incomplete=$((incomplete + 1))
  elif [ "${o#READ-OK}" != "" ]; then
    # The open arm must read the byte, or the deny arm proves nothing.
    verdict="probe not live"; incomplete=$((incomplete + 1))
  else
    case "$d" in
      DENIED:EACCES|DENIED:EPERM) verdict="blocked" ;;
      READ-OK) verdict="LEAK"; leaks=$((leaks + 1)) ;;
      *) verdict="not a refusal ($d)"; incomplete=$((incomplete + 1)) ;;
    esac
  fi
  printf '   %-11s %-11s %-16s %s\n' "$v" "${o:-?}" "${d:-?}" "$verdict"
done

echo "== verdict"
if [ "$incomplete" -ne 0 ]; then
  echo "   INCOMPLETE: $incomplete vector(s) missing, not live, or answered without a refusal — the harness or the engine, never counted as a block"
  exit 2
fi
if [ "$leaks" -ne 0 ]; then
  echo "   LEAK: $leaks vector(s) read a byte the policy denied"
  exit 1
fi
echo "   CONFORMANT: every covered vector is live under the open policy and refused (EACCES/EPERM) under deny"
