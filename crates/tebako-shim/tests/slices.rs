//! Extension slices at dispatch (spec 03 §2.8's reverse edge + spec 07
//! §2 step 3a's scan, §4's pins, §7's named errors): auto discovery from
//! the store, pin precedence, the journals, and the fail-hard pin path.

mod common;

use common::*;
use tebako_shim::dispatch;
use tebako_shim::runtime::RuntimeResolution;

// ---------------------------------------------------------------------
// fixtures
// ---------------------------------------------------------------------

fn pin_env(ctx: &mut tebako_shim::Ctx, tool: &str, version: &str) {
    let var = tebako_shim::resolve::version_env_var(tool);
    ctx.env.insert(var, version.to_string());
}

fn read_journal(home: &std::path::Path) -> String {
    std::fs::read_to_string(home.join("journal.log")).unwrap_or_default()
}

/// The BASE: kind app, one ruby entrypoint, two extension points
/// (flavors/gem-home + codelists/files), one gem (the overlap journal's
/// shared name).
fn base_manifest(tool: &str, version: &str) -> String {
    base_manifest_points(
        tool,
        version,
        "    - {name: flavors, mount: /flavors.d, layout: gem-home}\n    - {name: codelists, mount: /codelists.d, layout: files}\n",
    )
}

fn base_manifest_points(tool: &str, version: &str, points_yaml: &str) -> String {
    format!(
        "identity:\n  schema_version: 1\n  kind: app\n  name: {tool}\n  version: \"{version}\"\n  producer: {{tool: tebako-shim-tests, tool_version: \"1\"}}\n  created: \"2026-07-27T00:00:00Z\"\n  digest:\n    tree_hash: \"sha256:{tree}\"\n    blob_sha256: {blob}\n  signing: {{state: unsigned}}\n  encryption: {{state: none}}\nprovides:\n  entrypoints:\n    - name: {tool}\n      path: /app/bin/{tool}\n      runtime_requirement: {{engine: ruby, constraint: \">= 3.3, < 5.0\"}}\n  platforms: universal\n  extension_points:\n{points_yaml}  gems:\n    - {{name: nokogiri, version: \"1.18.9\"}}\n  capabilities: {{exec: true, read: true}}\n",
        tree = "a".repeat(64),
        blob = "b".repeat(64),
    )
}

/// A SLICE: kind data, a language edge (ruby `>= 3.3, < 5.0`), the
/// `augments:` reverse edge naming `base` at `point` under `constraint`.
fn slice_manifest(name: &str, version: &str, base: &str, constraint: &str, point: &str) -> String {
    slice_manifest_full(name, version, base, constraint, point, ">= 3.3, < 5.0", "")
}

/// `gems_yaml` rides at the provides-level indent ("  gems:\n    - …\n").
#[allow(clippy::too_many_arguments)]
fn slice_manifest_full(
    name: &str,
    version: &str,
    base: &str,
    constraint: &str,
    point: &str,
    lang_constraint: &str,
    gems_yaml: &str,
) -> String {
    format!(
        "identity:\n  schema_version: 1\n  kind: data\n  name: {name}\n  version: \"{version}\"\n  producer: {{tool: tebako-shim-tests, tool_version: \"1\"}}\n  created: \"2026-07-27T00:00:00Z\"\n  digest:\n    tree_hash: \"sha256:{tree}\"\n    blob_sha256: {blob}\n  signing: {{state: unsigned}}\n  encryption: {{state: none}}\nprovides:\n  mount_semantics: {{suggested: /slices.d/{name}}}\n{gems_yaml}  capabilities: {{exec: false, read: true}}\nrequires:\n  - kind: language\n    engine: ruby\n    constraint: \"{lang_constraint}\"\naugments:\n  - payload: {base}\n    constraint: \"{constraint}\"\n    extension_point: {point}\n",
        tree = "a".repeat(64),
        blob = "b".repeat(64),
    )
}

/// A base + a cached ruby runtime + the version pinned through the env.
/// Returns (tmp, home).
fn base_setup(tag: &str, tool: &str, version: &str) -> (TempDir, std::path::PathBuf) {
    let tmp = TempDir::new(tag);
    let home = tmp.path().join("home");
    write_payload(&home, tool, version, &base_manifest(tool, version));
    write_runtime(&home, "4.0.6", "0.16.0", true);
    (tmp, home)
}

fn dispatch_pinned(
    home: &std::path::Path,
    cwd: &std::path::Path,
    tool: &str,
    version: &str,
) -> Result<dispatch::ExecPlan, tebako_shim::ShimError> {
    let mut ctx = ctx(home, cwd);
    pin_env(&mut ctx, tool, version);
    dispatch::dispatch(tool, &[], &ctx)
}

/// The attached slice mounts of a plan (mounts below the base's points —
/// everything but the app's own `/`).
fn slice_mounts(plan: &dispatch::ExecPlan) -> Vec<(String, String)> {
    plan.mounts
        .iter()
        .filter(|m| m.mount != "/")
        .map(|m| {
            (
                m.image
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                m.mount.clone(),
            )
        })
        .collect()
}

// ---------------------------------------------------------------------
// auto discovery (spec 07 §2 step 3a)
// ---------------------------------------------------------------------

#[test]
fn auto_scan_attaches_the_newest_compatible() {
    let (tmp, home) = base_setup("slice-auto", "metanorma", "1.16.2");
    // Two installed versions, both compatible — the newest attaches.
    write_payload(
        &home,
        "metanorma-bsi",
        "1.0.0",
        &slice_manifest(
            "metanorma-bsi",
            "1.0.0",
            "metanorma",
            ">= 1.16, < 2.0",
            "flavors",
        ),
    );
    write_payload(
        &home,
        "metanorma-bsi",
        "1.1.0",
        &slice_manifest(
            "metanorma-bsi",
            "1.1.0",
            "metanorma",
            ">= 1.16, < 2.0",
            "flavors",
        ),
    );

    let plan = dispatch_pinned(&home, tmp.path(), "metanorma", "1.16.2").unwrap();

    assert!(matches!(plan.runtime, RuntimeResolution::Ready(_)));
    assert_eq!(
        slice_mounts(&plan),
        vec![(
            "1.1.0.tfs".to_string(),
            "/flavors.d/metanorma-bsi".to_string()
        )],
        "the newest compatible slice version attaches below the point"
    );
    // The slice triple rides argv AFTER the app's own (spec 07 §2 step 3a
    // + spec 17's multi-mount order).
    let argv = plan.argv.join("\n");
    let app_pos = argv.find(":0:/\n").expect("the app triple");
    let slice_pos = argv
        .find(":0:/flavors.d/metanorma-bsi")
        .expect("the slice triple");
    assert!(app_pos < slice_pos, "app triple precedes the slice: {argv}");

    let journal = read_journal(&home);
    assert!(
        journal.contains(
            "event=slice-augment slice=metanorma-bsi@1.1.0 base=metanorma mount=/flavors.d/metanorma-bsi"
        ),
        "{journal}"
    );
}

#[test]
fn auto_scan_skips_a_base_version_mismatch_loudly() {
    let (tmp, home) = base_setup("slice-skip", "metanorma", "1.16.2");
    write_payload(
        &home,
        "metanorma-bsi",
        "9.9.9",
        &slice_manifest("metanorma-bsi", "9.9.9", "metanorma", "= 1.99.0", "flavors"),
    );

    let plan = dispatch_pinned(&home, tmp.path(), "metanorma", "1.16.2").unwrap();

    assert!(slice_mounts(&plan).is_empty(), "nothing attaches");
    let journal = read_journal(&home);
    assert!(
        journal.contains("event=slice-skip slice=metanorma-bsi@9.9.9 reason=base-version"),
        "{journal}"
    );
    assert!(!journal.contains("event=slice-augment"), "{journal}");
}

#[test]
fn auto_scan_skips_an_unknown_point_loudly() {
    let (tmp, home) = base_setup("slice-unknown", "metanorma", "1.16.2");
    write_payload(
        &home,
        "metanorma-bsi",
        "1.2.0",
        &slice_manifest(
            "metanorma-bsi",
            "1.2.0",
            "metanorma",
            ">= 1.16",
            "no-such-point",
        ),
    );

    let plan = dispatch_pinned(&home, tmp.path(), "metanorma", "1.16.2").unwrap();

    assert!(slice_mounts(&plan).is_empty());
    let journal = read_journal(&home);
    assert!(
        journal.contains("event=slice-skip slice=metanorma-bsi@1.2.0 reason=unknown-point"),
        "{journal}"
    );
}

#[test]
fn auto_scan_skips_a_runtime_mismatch_loudly() {
    let (tmp, home) = base_setup("slice-runtime", "metanorma", "1.16.2");
    write_payload(
        &home,
        "metanorma-bsi",
        "1.2.0",
        &slice_manifest_full(
            "metanorma-bsi",
            "1.2.0",
            "metanorma",
            ">= 1.16",
            "flavors",
            ">= 9.0", // the cached ruby is 4.0.6
            "",
        ),
    );

    let plan = dispatch_pinned(&home, tmp.path(), "metanorma", "1.16.2").unwrap();

    assert!(slice_mounts(&plan).is_empty());
    let journal = read_journal(&home);
    assert!(
        journal.contains("event=slice-skip slice=metanorma-bsi@1.2.0 reason=runtime"),
        "{journal}"
    );
}

#[test]
fn auto_scan_ignores_slices_of_other_bases_and_non_slices() {
    let (tmp, home) = base_setup("slice-foreign", "metanorma", "1.16.2");
    // A slice of ANOTHER base: never a candidate here.
    write_payload(
        &home,
        "other-slice",
        "1.0.0",
        &slice_manifest("other-slice", "1.0.0", "other-base", ">= 1.0", "flavors"),
    );
    // A plain data payload (no augments): never a candidate.
    write_payload(
        &home,
        "plain-data",
        "1.0.0",
        &data_manifest("plain-data", "1.0.0"),
    );

    let plan = dispatch_pinned(&home, tmp.path(), "metanorma", "1.16.2").unwrap();

    assert!(slice_mounts(&plan).is_empty());
    let journal = read_journal(&home);
    assert!(!journal.contains("event=slice-"), "{journal}");
}

// ---------------------------------------------------------------------
// pins (spec 07 §4)
// ---------------------------------------------------------------------

#[test]
fn pin_overrides_auto_discovery_per_name() {
    let (tmp, home) = base_setup("slice-pin", "metanorma", "1.16.2");
    write_payload(
        &home,
        "metanorma-bsi",
        "1.0.0",
        &slice_manifest(
            "metanorma-bsi",
            "1.0.0",
            "metanorma",
            ">= 1.16, < 2.0",
            "flavors",
        ),
    );
    write_payload(
        &home,
        "metanorma-bsi",
        "1.1.0",
        &slice_manifest(
            "metanorma-bsi",
            "1.1.0",
            "metanorma",
            ">= 1.16, < 2.0",
            "flavors",
        ),
    );
    // The user default pins the OLDER slice version — the pin wins over
    // the auto pick (which would be 1.1.0).
    write_config(
        &home,
        "defaults:\n  metanorma:\n    version: \"1.16.2\"\n    slices: [metanorma-bsi@1.0.0]\n",
    );

    let ctx = ctx(&home, tmp.path());
    let plan = dispatch::dispatch("metanorma", &[], &ctx).unwrap();

    assert_eq!(
        slice_mounts(&plan),
        vec![(
            "1.0.0.tfs".to_string(),
            "/flavors.d/metanorma-bsi".to_string()
        )],
        "the pinned version attaches, not the auto pick"
    );
    let journal = read_journal(&home);
    assert!(
        journal.contains("event=slice-augment slice=metanorma-bsi@1.0.0"),
        "{journal}"
    );
}

#[test]
fn env_slices_prepend_and_win_per_name() {
    let (tmp, home) = base_setup("slice-env", "metanorma", "1.16.2");
    write_payload(
        &home,
        "metanorma-bsi",
        "1.0.0",
        &slice_manifest(
            "metanorma-bsi",
            "1.0.0",
            "metanorma",
            ">= 1.16, < 2.0",
            "flavors",
        ),
    );
    write_payload(
        &home,
        "metanorma-bsi",
        "1.1.0",
        &slice_manifest(
            "metanorma-bsi",
            "1.1.0",
            "metanorma",
            ">= 1.16, < 2.0",
            "flavors",
        ),
    );
    // The config pins 1.0.0; the env PREPENDS and wins the name (§4).
    write_config(
        &home,
        "defaults:\n  metanorma:\n    version: \"1.16.2\"\n    slices: [metanorma-bsi@1.0.0]\n",
    );

    let mut ctx = ctx(&home, tmp.path());
    ctx.env.insert(
        tebako_shim::resolve::slices_env_var("metanorma"),
        "metanorma-bsi@1.1.0".to_string(),
    );
    let plan = dispatch::dispatch("metanorma", &[], &ctx).unwrap();

    assert_eq!(
        slice_mounts(&plan),
        vec![(
            "1.1.0.tfs".to_string(),
            "/flavors.d/metanorma-bsi".to_string()
        )],
        "the env pin wins the name over the config pin"
    );
}

#[test]
fn project_file_slices_and_the_version_shadow_rule() {
    let (tmp, home) = base_setup("slice-proj", "metanorma", "1.16.2");
    write_payload(
        &home,
        "metanorma-bsi",
        "1.0.0",
        &slice_manifest(
            "metanorma-bsi",
            "1.0.0",
            "metanorma",
            ">= 1.16, < 2.0",
            "flavors",
        ),
    );
    // The nearer project file carries ONLY slices (the map form without
    // `version:` does NOT pin the version — the shadow rule per key);
    // the farther one pins the version.
    std::fs::write(
        tmp.path().join(".tebako-tools.yaml"),
        "metanorma: \"1.16.2\"\n",
    )
    .unwrap();
    let sub = tmp.path().join("sub");
    std::fs::create_dir_all(&sub).unwrap();
    std::fs::write(
        sub.join(".tebako-tools.yaml"),
        "metanorma:\n  slices: [metanorma-bsi@1.0.0]\n",
    )
    .unwrap();

    let ctx = ctx(&home, &sub);
    let plan = dispatch::dispatch("metanorma", &[], &ctx).unwrap();

    assert_eq!(
        slice_mounts(&plan),
        vec![(
            "1.0.0.tfs".to_string(),
            "/flavors.d/metanorma-bsi".to_string()
        )],
        "the nearer file's slices ride the farther file's version pin"
    );
    assert!(matches!(plan.runtime, RuntimeResolution::Ready(_)));
}

#[test]
fn auto_slices_false_kills_the_scan_but_pins_attach() {
    let (tmp, home) = base_setup("slice-noauto", "metanorma", "1.16.2");
    write_payload(
        &home,
        "metanorma-bsi",
        "1.1.0",
        &slice_manifest(
            "metanorma-bsi",
            "1.1.0",
            "metanorma",
            ">= 1.16, < 2.0",
            "flavors",
        ),
    );
    write_payload(
        &home,
        "metanorma-iso",
        "2.0.0",
        &slice_manifest(
            "metanorma-iso",
            "2.0.0",
            "metanorma",
            ">= 1.16, < 2.0",
            "codelists",
        ),
    );
    write_config(
        &home,
        "auto_slices: false\ndefaults:\n  metanorma: \"1.16.2\"\n",
    );

    // The scan is dead: nothing attaches although both slices qualify.
    let ctx = ctx(&home, tmp.path());
    let plan = dispatch::dispatch("metanorma", &[], &ctx).unwrap();
    assert!(
        slice_mounts(&plan).is_empty(),
        "auto_slices: false kills the scan"
    );
    assert!(
        !read_journal(&home).contains("event=slice-augment"),
        "{}",
        read_journal(&home)
    );

    // …but an explicit pin still attaches (spec 07 §4: the flag gates the
    // scan, never the pins).
    write_config(
        &home,
        "auto_slices: false\ndefaults:\n  metanorma:\n    version: \"1.16.2\"\n    slices: [metanorma-iso@2.0.0]\n",
    );
    let ctx2 = common::ctx(&home, tmp.path());
    let plan = dispatch::dispatch("metanorma", &[], &ctx2).unwrap();
    assert_eq!(
        slice_mounts(&plan),
        vec![(
            "2.0.0.tfs".to_string(),
            "/codelists.d/metanorma-iso".to_string()
        )],
        "the pin attaches with the scan off; the auto-discovered bsi stays out"
    );
}

// ---------------------------------------------------------------------
// the fail-hard pin path (spec 07 §7)
// ---------------------------------------------------------------------

#[test]
fn offline_pin_miss_is_the_named_cache_or_error() {
    let (tmp, home) = base_setup("slice-offline", "metanorma", "1.16.2");
    write_config(
        &home,
        "defaults:\n  metanorma:\n    version: \"1.16.2\"\n    slices: [metanorma-bsi@1.2.0]\n",
    );

    let mut ctx = ctx(&home, tmp.path());
    ctx.env
        .insert("TEBAKO_OFFLINE".to_string(), "1".to_string());
    let err = dispatch::dispatch("metanorma", &[], &ctx).unwrap_err();

    assert_eq!(err.code, tebako_shim::EX_TEBAKO_UNAVAILABLE, "{err:?}");
    assert!(
        err.message.contains("metanorma-bsi@1.2.0"),
        "{}",
        err.message
    );
    assert!(err.message.contains("TEBAKO_OFFLINE"), "{}", err.message);
    assert!(
        err.message.contains("tebako install metanorma-bsi@1.2.0"),
        "{}",
        err.message
    );
}

#[test]
fn pinned_incompatible_slice_is_slice_incompatible() {
    let (tmp, home) = base_setup("slice-bad-pin", "metanorma", "1.16.2");
    // Installed, but its constraint refuses the resolved base version.
    write_payload(
        &home,
        "metanorma-bsi",
        "1.2.0",
        &slice_manifest("metanorma-bsi", "1.2.0", "metanorma", "= 1.99.0", "flavors"),
    );
    write_config(
        &home,
        "defaults:\n  metanorma:\n    version: \"1.16.2\"\n    slices: [metanorma-bsi@1.2.0]\n",
    );

    let ctx = ctx(&home, tmp.path());
    let err = dispatch::dispatch("metanorma", &[], &ctx).unwrap_err();

    assert_eq!(err.code, tebako_shim::EX_TEBAKO_MANIFEST, "{err:?}");
    assert!(err.message.contains("SliceIncompatible"), "{}", err.message);
    // §7: the error names the slice, the pin's source link, and the
    // failed check.
    assert!(
        err.message.contains("metanorma-bsi@1.2.0"),
        "{}",
        err.message
    );
    assert!(err.message.contains("user default"), "{}", err.message);
    assert!(
        err.message
            .contains("\"= 1.99.0\" does not match the resolved metanorma 1.16.2"),
        "{}",
        err.message
    );
}

#[test]
fn pinned_unknown_point_is_slice_incompatible() {
    let (tmp, home) = base_setup("slice-bad-point", "metanorma", "1.16.2");
    write_payload(
        &home,
        "metanorma-bsi",
        "1.2.0",
        &slice_manifest(
            "metanorma-bsi",
            "1.2.0",
            "metanorma",
            ">= 1.16",
            "no-such-point",
        ),
    );

    let mut ctx = ctx(&home, tmp.path());
    pin_env(&mut ctx, "metanorma", "1.16.2");
    ctx.env.insert(
        tebako_shim::resolve::slices_env_var("metanorma"),
        "metanorma-bsi@1.2.0".to_string(),
    );
    let err = dispatch::dispatch("metanorma", &[], &ctx).unwrap_err();

    assert_eq!(err.code, tebako_shim::EX_TEBAKO_MANIFEST, "{err:?}");
    assert!(err.message.contains("SliceIncompatible"), "{}", err.message);
    assert!(
        err.message.contains("env TEBAKO_METANORMA_SLICES"),
        "{}",
        err.message
    );
    assert!(err.message.contains("no-such-point"), "{}", err.message);
}

#[test]
fn slice_pin_grammar_requires_the_payload_part() {
    let (tmp, home) = base_setup("slice-bare-pin", "metanorma", "1.16.2");

    let mut ctx = ctx(&home, tmp.path());
    pin_env(&mut ctx, "metanorma", "1.16.2");
    ctx.env.insert(
        tebako_shim::resolve::slices_env_var("metanorma"),
        "1.2.0".to_string(), // bare version: no slice name
    );
    let err = dispatch::dispatch("metanorma", &[], &ctx).unwrap_err();

    assert_eq!(err.code, tebako_shim::EX_TEBAKO_MANIFEST, "{err:?}");
    assert!(err.message.contains("<slice>@<version>"), "{}", err.message);
    assert!(
        err.message.contains("env TEBAKO_METANORMA_SLICES"),
        "{}",
        err.message
    );
}

// ---------------------------------------------------------------------
// NestedAugment (spec 07 §7)
// ---------------------------------------------------------------------

#[test]
fn root_manifest_augments_is_nested_augment() {
    let tmp = TempDir::new("slice-root");
    let home = tmp.path().join("home");
    // A toolkit root carrying `augments:` — the one kind that parses with
    // the key (content-only, exec: false) AND dispatches far enough to
    // hit the check (its executables are dispatchable commands).
    let manifest = format!(
        "identity:\n  schema_version: 1\n  kind: toolkit\n  name: tk\n  version: \"1.0.0\"\n  producer: {{tool: tebako-shim-tests, tool_version: \"1\"}}\n  created: \"2026-07-27T00:00:00Z\"\n  digest:\n    tree_hash: \"sha256:{tree}\"\n    blob_sha256: {blob}\n  signing: {{state: unsigned}}\n  encryption: {{state: none}}\nprovides:\n  executables:\n    - {{name: tk, path: /bin/tk}}\n  platforms: universal\n  capabilities: {{exec: false, read: true}}\naugments:\n  - payload: metanorma\n    constraint: \">= 1.0\"\n    extension_point: flavors\n",
        tree = "a".repeat(64),
        blob = "b".repeat(64),
    );
    write_payload(&home, "tk", "1.0.0", &manifest);

    let mut ctx = ctx(&home, tmp.path());
    pin_env(&mut ctx, "tk", "1.0.0");
    let err = dispatch::dispatch("tk", &[], &ctx).unwrap_err();

    assert_eq!(err.code, tebako_shim::EX_TEBAKO_MANIFEST, "{err:?}");
    assert!(err.message.contains("NestedAugment"), "{}", err.message);
    assert!(err.message.contains("tk"), "{}", err.message);
}

#[test]
fn slice_mount_nesting_under_another_slice_is_nested_augment() {
    let tmp = TempDir::new("slice-nest");
    let home = tmp.path().join("home");
    // The `deep` point lives BELOW where the first slice mounts — the
    // second slice's mount then nests under the first's (spec 07 §7).
    write_payload(
        &home,
        "metanorma",
        "1.16.2",
        &base_manifest_points(
            "metanorma",
            "1.16.2",
            "    - {name: flavors, mount: /flavors.d, layout: gem-home}\n    - {name: deep, mount: /flavors.d/xslice, layout: files}\n",
        ),
    );
    write_runtime(&home, "4.0.6", "0.16.0", true);
    write_payload(
        &home,
        "xslice",
        "1.0.0",
        &slice_manifest("xslice", "1.0.0", "metanorma", ">= 1.16", "flavors"),
    );
    write_payload(
        &home,
        "yslice",
        "1.0.0",
        &slice_manifest("yslice", "1.0.0", "metanorma", ">= 1.16", "deep"),
    );

    let plan = dispatch_pinned(&home, tmp.path(), "metanorma", "1.16.2");
    let err = plan.unwrap_err();
    assert_eq!(err.code, tebako_shim::EX_TEBAKO_MANIFEST, "{err:?}");
    assert!(err.message.contains("NestedAugment"), "{}", err.message);
    assert!(err.message.contains("/flavors.d/xslice"), "{}", err.message);
}

// ---------------------------------------------------------------------
// the overlap journal + the zero-runtime note
// ---------------------------------------------------------------------

#[test]
fn base_and_slice_sharing_a_gem_journals_slice_overlap() {
    let (tmp, home) = base_setup("slice-overlap", "metanorma", "1.16.2");
    // The base carries nokogiri 1.18.9 (base_manifest); the slice carries
    // its own nokogiri — informational, never an error (§2 step 3a).
    write_payload(
        &home,
        "metanorma-bsi",
        "1.2.0",
        &slice_manifest_full(
            "metanorma-bsi",
            "1.2.0",
            "metanorma",
            ">= 1.16, < 2.0",
            "flavors",
            ">= 3.3, < 5.0",
            "  gems:\n    - {name: nokogiri, version: \"1.18.8\"}\n    - {name: metanorma-bsi, version: \"1.2.0\"}\n",
        ),
    );

    let plan = dispatch_pinned(&home, tmp.path(), "metanorma", "1.16.2").unwrap();

    assert_eq!(slice_mounts(&plan).len(), 1, "the slice still attaches");
    let journal = read_journal(&home);
    assert!(
        journal.contains("event=slice-overlap slice=metanorma-bsi@1.2.0 gem=nokogiri"),
        "{journal}"
    );
    // The slice's own gem does not overlap — exactly one overlap line.
    assert_eq!(
        journal.matches("event=slice-overlap").count(),
        1,
        "{journal}"
    );
}

#[test]
fn zero_runtime_dispatch_notes_and_attaches_nothing() {
    let tmp = TempDir::new("slice-zero");
    let home = tmp.path().join("home");
    // A NATIVE base (no runtime_requirement → the Zero arm) declaring
    // extension points, plus an install-time materialized entrypoint.
    let manifest = format!(
        "identity:\n  schema_version: 1\n  kind: app\n  name: inkview\n  version: \"8.1.0\"\n  producer: {{tool: tebako-shim-tests, tool_version: \"1\"}}\n  created: \"2026-07-27T00:00:00Z\"\n  digest:\n    tree_hash: \"sha256:{tree}\"\n    blob_sha256: {blob}\n  signing: {{state: unsigned}}\n  encryption: {{state: none}}\nprovides:\n  entrypoints:\n    - name: inkview\n      path: /app/bin/inkview\n  platforms: universal\n  extension_points:\n    - {{name: codelists, mount: /codelists.d, layout: files}}\n  capabilities: {{exec: true, read: true}}\n",
        tree = "a".repeat(64),
        blob = "b".repeat(64),
    );
    write_payload(&home, "inkview", "8.1.0", &manifest);
    let entry_host = home
        .join("payloads")
        .join("inkview")
        .join("8.1.0.tree")
        .join("app/bin/inkview");
    std::fs::create_dir_all(entry_host.parent().unwrap()).unwrap();
    std::fs::write(&entry_host, b"#!/bin/sh\n").unwrap();
    write_payload(
        &home,
        "codelists-iso",
        "2.0.0",
        &slice_manifest("codelists-iso", "2.0.0", "inkview", ">= 8.0", "codelists"),
    );

    let mut ctx = ctx(&home, tmp.path());
    pin_env(&mut ctx, "inkview", "8.1.0");
    let plan = dispatch::dispatch("inkview", &[], &ctx).unwrap();

    assert!(matches!(plan.runtime, RuntimeResolution::Zero));
    assert_eq!(plan.mounts.len(), 1, "no VFS: the slice cannot attach");
    let journal = read_journal(&home);
    assert!(
        journal.contains("event=slice-skip base=inkview reason=zero-runtime"),
        "{journal}"
    );
    assert!(!journal.contains("event=slice-augment"), "{journal}");
}

// ---------------------------------------------------------------------
// the pin's fetch-on-miss (spec 07 §4 — parity with the runtime fetch)
// ---------------------------------------------------------------------

#[test]
fn pin_miss_fetches_through_the_registry_then_demands_the_mirror() {
    let (tmp, home) = base_setup("slice-fetch", "metanorma", "1.16.2");
    // The slice is NOT installed; a file:// registry serves its bytes.
    let mirror = tmp.path().join("mirror");
    std::fs::create_dir_all(&mirror).unwrap();
    let image_bytes = b"fake slice image bytes\n";
    std::fs::write(mirror.join("metanorma-bsi-1.2.0.tfs"), image_bytes).unwrap();
    let payload_ref = tebako_http::file_url(&mirror.join("metanorma-bsi-1.2.0.tfs"));
    let registry_yaml = format!(
        "schema_version: 1\npayloads:\n  - name: metanorma-bsi\n    kind: data\n    versions:\n      - version: 1.2.0\n        platforms: universal\n        release: {{ref: {payload_ref}}}\n"
    );
    std::fs::write(mirror.join("tpkg-registry.yaml"), &registry_yaml).unwrap();
    let registry_ref = tebako_http::file_url(&mirror.join("tpkg-registry.yaml"));
    write_config(
        &home,
        &format!(
            "registries:\n  - {registry_ref}\ndefaults:\n  metanorma:\n    version: \"1.16.2\"\n    slices: [metanorma-bsi@1.2.0]\n"
        ),
    );

    let ctx = ctx(&home, tmp.path());
    let err = dispatch::dispatch("metanorma", &[], &ctx).unwrap_err();

    // The bytes LAND (fetch + sha-free unsigned acceptance + cache
    // install)…
    let cached = home
        .join("payloads")
        .join("metanorma-bsi")
        .join("1.2.0.tfs");
    assert_eq!(std::fs::read(&cached).unwrap(), image_bytes);
    let journal = read_journal(&home);
    assert!(
        journal.contains("event=payload-installed name=metanorma-bsi version=1.2.0"),
        "{journal}"
    );
    assert!(
        journal.contains("event=legacy-unsigned-accepted"),
        "{journal}"
    );
    // …but the dispatch-time checks need the manifest mirror, which the
    // cache install never writes — the named 69 with the remedy (the
    // mirror gap: only `tebako install` writes it).
    assert_eq!(err.code, tebako_shim::EX_TEBAKO_UNAVAILABLE, "{err:?}");
    assert!(err.message.contains("manifest mirror"), "{}", err.message);
    assert!(
        err.message.contains("tebako install metanorma-bsi@1.2.0"),
        "{}",
        err.message
    );
}
