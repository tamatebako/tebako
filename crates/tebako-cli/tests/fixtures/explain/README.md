# `tebako trace explain` replay fixtures (spec 25 §5/§7, phase T4)

One `<name>.jsonl` capture per signature of `explain-signatures.yaml`
(the §5 seed table) plus a clean run. The captures are synthetic
reproductions of the incident corpus classes, authored line-by-line
against `docs/spec/schemas/trace-event.yaml` (the envelope: v/ts/pid/
tid/op/path/verdict/detail/dur_us, errno when the verdict carries one).
§7's gate is the shape: a capture replays through `explain` and the
named red hop matches the incident's hand-derived answer — each fixture
names its expected verdict below, pinned by `tests/trace_explain.rs`.

| capture                        | red hop (exit 1)                                   |
|--------------------------------|----------------------------------------------------|
| `env-image-never-mounted`      | mount — env image never mounted (handoff env lost) |
| `os-bind-module-not-found`     | OS bind — the closure resolved, the loader refused |
| `policy-denial`                | policy — policy denial (the EACCES class)          |
| `materialize-error`            | materialize — exec-cache write failure             |
| `clean-run`                    | GREEN (exit 0); the tail line is a crashed write   |

The real incident-13 dogfood captures replay through the same verdicts
factory-side (the msys dogfood rides the bus since T1); converting and
archiving them here is follow-up, not a gate.
