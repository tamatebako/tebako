#!/bin/bash
# link-unit-floor-link-check.sh — prove the staged gnu link unit FOLDS
# under the floor's binutils (tebako#413): compile a trivial C consumer
# and link it against the unit's two scoped archives plus the closure
# with the floor's ld (2.34 — ubuntu:20.04 never bumps binutils).
# binutils < 2.35 reads a group-less STB_GNU_UNIQUE definition as STRONG
# and rejects a symtab whose local symbols trail sh_info, so any binding
# the scoper failed to demote — or any inconsistently filed symbol —
# fails THIS link with "multiple definition" / "local symbol at index N"
# before the probe ever runs. The zero-UNIQUE gate in
# lib/link-unit-stage.sh is the static half; this link is the dynamic
# half, exercising every archive member the consumer pulls.
#
# The link shape mirrors a factory consumer: demand-driven extraction
# from --start-group'd archives (no --whole-archive — the unit is not
# promised to be whole-archive-clean; the vendored codec copies bundle
# some strong C symbols twice, harmless under demand-driven links). The
# closure's C++ objects were compiled in this same container
# (gnu-floor-build.sh's ppa gcc-11), so the floor's c++ driver — gcc-11
# via update-alternatives — resolves their libstdc++ references itself.
#
# Runs INSIDE the ubuntu:20.04 floor container; called from
# ci/gnu-floor-build.sh after lib/link-unit-stage.sh.
# Required env: PLATFORM (tebako platform id).
set -euo pipefail

unit="out/link-unit-$PLATFORM"
[ -f "$unit/libtfs.a" ] || { echo "link-unit-floor-link-check: $unit missing (stage first)" >&2; exit 64; }

probe_dir="$(mktemp -d)"
trap 'rm -rf "$probe_dir"' EXIT

# The staged include/tebako/fs/c_api.h has a dangling include
# (tebako/fs/platform.h ships nowhere — a separate pre-existing gap), so
# the consumer declares the two exports it needs, exactly the ABI shape.
cat > "$probe_dir/consumer.c" <<'EOF'
#include <stdint.h>
#include <stdio.h>
uint32_t tebako_driver_contract_version(void);
int tebako_fs_init_from_file(const char *archive_path, const char *mount_point);

int main(void) {
    uint32_t cv = tebako_driver_contract_version();
    int rc = tebako_fs_init_from_file("/nonexistent.tfs", "/__tebako__");
    printf("probe: contract=%u init_from_file(missing)=%d\n", cv, rc);
    return (cv > 0 && rc != 0) ? 0 : 1;
}
EOF

echo "== floor toolchain =="
ld --version | head -1
c++ --version | head -1

echo "== consumer link (demand-driven extraction, floor ld) =="
cc -c "$probe_dir/consumer.c" -o "$probe_dir/consumer.o"
c++ -o "$probe_dir/probe" "$probe_dir/consumer.o" \
  -Wl,--start-group "$unit/libtebako_driver.a" "$unit/libtfs.a" "$unit"/closure/*.a -Wl,--end-group \
  -lpthread -ldl -lm

echo "== run the probe (exercises the extracted driver code) =="
"$probe_dir/probe"
echo "link-unit-floor-link-check: OK ($PLATFORM folds under $(ld --version | head -1))"
