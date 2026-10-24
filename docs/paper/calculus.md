# A deployment calculus for hermetic software distribution

This paper states the model behind tebako as a short formal system: the
primitives, the well-formedness conditions over them, the dispatch and
composition propositions that the runtime relies on, and the audience law
written as a complexity tiering. It is written for a reader who wants to
audit the claims without reading a chat log, and it states its own limits
in section 8.

One convention governs the whole paper. Each proposition is accompanied
by its **falsifier**: the executed test that fails when the proposition
is violated. The propositions are not machine-checked proofs, and the
paper does not claim they are. They are design invariants with an
enforcement path, which is the strongest honest claim available to a
system of this size.

## 1. The problem

Deploying software is not a function today. Its inputs include the host's
package mirror at the moment of installation, the order in which
libraries were installed, the user's `PATH`, and the machine's kernel.
Two installations of the same program therefore produce different
programs, and the difference is invisible until it fails. The industry
has three established escapes, and each purchases determinism with a
different currency.

**Static linking** makes the program closed over its libraries by
rewriting its links, at the cost of the program's own design: a
dynamically loaded extension, a plugin, a script interpreter inside the
program all become impossible or must be reimplemented.

**Virtualization and containers** make the runtime environment closed by
reproducing a machine, at the cost of a kernel, a daemon, a filesystem
tree, and a Linux-only ABI for the cheap form. The program becomes a
service that a foreign engine starts.

**Single-file bundlers** make the *artifact* closed by embedding an
interpreter and extracting it, at the cost of breadth: the runtime is
fixed to the bundler's own, the extraction directory is host state, and
nothing composes.

The calculus in this paper takes a fourth position. It makes the
**artifact** closed over a runtime and a set of payloads, and it does so
by mounting images instead of extracting them, which keeps the closure
real while leaving the program an ordinary process on the host.

## 2. Primitives

The model has five primitives. Everything else in the ecosystem is a
composition of them.

**Definition 1 (image).** An image `I` is a finite byte string tagged
with a format identifier `φ(I) ∈ {1, 2, 3, 5}`, where the values name the
dwarfs, squashfs, zip and LimniFS backends respectively. The value `4` is
reserved and names a legacy runtime-role wart; no new artifact carries
it. An image has a digest `D(I)`, the lowercase hexadecimal SHA-256 of
its bytes, and a size `|I|`.

**Definition 2 (manifest).** A payload manifest `M` is a document
carrying four fields: an identity `id(M)`, a set of provided capabilities
`prov(M)`, a set of required edges `req(M)`, and a platform set
`plat(M)`. An edge `e ∈ req(M)` names a target, a constraint in the
constraint grammar, and optionally a platform set and a mount point. The
manifest lives at the path `/__tpkg__/manifest.yaml` inside the image it
describes, so an image is self-describing and the authority over the
manifest is the image rather than a registry row.

**Definition 3 (payload).** A payload `P = (I, M)` is an image together
with the manifest it holds, such that `M` describes `I`.

**Definition 4 (mount).** A mount is a pair `(p, P)` of a path `p` and a
payload. The path is normalized, absolute and component-wise; a mount
point is compared as a sequence of components, never as a string.

**Definition 5 (mount table).** A mount table `T = {(p₁, P₁), …,
(pₙ, Pₙ)}` is a finite set of mounts whose path components are pairwise
distinct.

**Definition 6 (package).** A package `K = (B, σ₁ … σₙ, τ)` is a loader
`B`, a sequence of slots `σᵢ = (Iᵢ, pᵢ)` each carrying an image and its
mount point, and a trailer `τ`. The trailer records, for each slot, the
offset, the size, the format identifier and the mount point; and it
records the runtime reference, the launcher ABI version and a flag word.
The trailer sits at end of file, so a package is a loader, some bytes,
and a description of those bytes.

**Definition 7 (environment).** An environment `E = (T, R, J, S)` is a
mount table, a runtime pair `R = (interpreter, env image)`, a jail policy
`J`, and a store `S` that maps digests to cached bytes.

## 3. Well-formedness

The primitives are unconstrained until conditions are imposed. The
conditions are the part of this paper that a reviewer can check against
an artifact.

**WF-1 (image).** `φ(I)` is a supported backend and `I` parses under that
backend. The runtime is read-only over images: creation is a writer-side
activity, and no shipped reader learns to write.

**WF-2 (manifest).** `id(M)`, every provided name and every target name
match the locked name grammar; every constraint parses in the constraint
grammar; the schema era declared by the document is one the reader knows;
and the document's `schema_minor` is not newer than the reader's.

**WF-3 (package).** The slots of `K` are pairwise disjoint in
`[0, |K|)`; every slot's recorded size matches the region it names; no
slot carries a format identifier of `4`; and no two slots name the same
mount point. A package that violates any clause fails at parse, before a
byte is mapped.

**WF-4 (environment).** The mounts of `T` have pairwise distinct path
components; the env image of `R` is mounted at the runtime root; and every
need declared by a manifest in `T` is covered by `J` together with the
system floor. `WF-4` is the condition that turns the jail from a policy
into a *check*: an uncovered need is a named refusal before `exec`,
never a denial at first access.

The audience law below adds a fifth condition, and it is the one that
distinguishes this calculus from a packaging format.

## 4. Dispatch

The runtime answers every path lookup through one function, and the
properties of that function are what make composition predictable.

**Definition 8 (dispatch).** For a mount table `T` and a path `q`, let
`M(T, q) = {p ∈ T : p is a component-wise prefix of q}`. The dispatch is
`δ(T, q) = argmax_{p ∈ M(T,q)} |p|`, the longest matching prefix, and is
undefined when `M(T, q)` is empty.

**Proposition 1 (determinism and shadowing).** For a mount table `T`
satisfying WF-4 and a path `q`, `δ(T, q)` is unique when defined, and a
mount at `p` shadows every mount whose point is a proper prefix of `p`.

*Proof sketch.* Component-wise equality plus the distinctness clause of
WF-4 gives uniqueness. Shadowing follows because a proper prefix of `p`
has strictly smaller length, so the maximization cannot select it.

*Falsifier.* `crates/tfs/tests/nested_mounts.rs`, five cases:
`five_nested_mounts_dispatch_by_longest_prefix` (each of five nested levels
serves its own content, and the deepest path is not shadowed),
`a_mount_point_is_namespace_not_a_path_in_the_parent_image`,
`a_duplicate_mount_point_is_eexist`,
`unmounting_a_middle_level_does_not_cascade`, and
`unmounting_the_deepest_mount_falls_through_to_the_parents_own_tree`.

**Proposition 2 (coverage is not presence).** A path that a mount covers
but whose image does not hold resolves to the host answer, subject to the
jail, and not to a not-found error.

This proposition is what keeps a composed environment usable. With an
application mounted at `/`, every path the image does not hold would
otherwise become invisible, including the host's own libraries and
devices. The formal content is that dispatch selects a *backend*, and a
backend's answer for an absent entry is a second decision, not a third.

*Falsifier.*
`crates/tfs/tests/jails.rs::covered_but_not_held_paths_fall_through_to_the_host_decision`
asserts the first half — a path a mount covers but does not hold takes the
host decision rather than the backend's `ENOENT` — and
`crates/tfs/tests/jails.rs::deny_all_cannot_enumerate_or_read_but_memfs_is_unaffected`
asserts the second: the same path is refused under a deny policy, by
`EPERM` and not by `ENOENT`, which is the distinction a user-visible
diagnostic depends on. A per-release conformance suite extends the same
two arms over a matrix of path spellings; it is held on a branch and is
named here as an extension rather than as a shipped proof.

## 5. Composition

**Definition 9 (composition).** A composition of packages `K₁ … Kₙ` is
the environment whose mount table is the union of their slots and whose
runtime pair is the one the entry package requires; the union is defined
only when the mount points remain pairwise distinct.

**Proposition 3 (composition preserves well-formedness).** If every
composing payload satisfies WF-1 and WF-2, every package satisfies WF-3,
and the union of their mount points is component-wise distinct, then the
composed environment satisfies WF-4 whenever its jail covers the union of
the declared needs.

*Proof sketch.* WF-1 and WF-2 are properties of individual payloads and
are preserved by union. WF-4's distinctness is the union's stated
precondition. The coverage clause is a union of need sets, and a policy
covering the union covers each member; the converse fails, which is why
the condition is stated over the union rather than over the members.

*Falsifier.* The fixture package that mounts five payloads at five nested
points and prints from each level, run as one binary.

**Proposition 4 (entry resolution).** With an ordered sequence of image
specifications, the entry path resolves against the mount of the FIRST
specification; with no specification, it resolves against the runtime
root.

*Falsifier.*
`crates/tebako-driver/tests/boot.rs::five_image_triples_mount_in_order_and_dispatch_by_longest_prefix`
parses five image triples in one process, asserts the mount order, and
asserts the entry's resolution against the first image's mount.

**Proposition 5 (resolution is total or named).** For a finite universe
of candidate versions, the solver either returns an assignment satisfying
every edge or returns a conflict naming the identity, the demands with
their sources, and the versions offered. No third outcome exists.

*Falsifier.* Thirteen cases in the solver suite, including a two-sided
conflict whose rendering must contain both constraints and both
consumers, and an identity no registry publishes.

The runtime-on-runtime case extends Proposition 3 rather than escaping
it. A payload of kind `runtime` may declare an owner edge, and the owner
then supplies the process; both manifests' needs union identically, so
the coverage condition is unchanged and the composition remains closed.

## 6. The audience law as a complexity theorem

The law is usually written as a product statement. This paper writes it as
a theorem about the operations each actor must perform, because that is
the form a reviewer can falsify.

**Definition 10 (tier).** Let `Ops(a)` be the set of operations an actor
`a` must perform to produce or run a package: `compile` (invoke a
compiler), `link` (invoke a linker), `fetch` (retrieve an artifact),
`mount` (make an image readable), `exec` (start a process).

The four tiers are:

- `T₁`, the runner: `Ops = { fetch, mount, exec }`;
- `T₂`, the data author: `Ops = { fetch, mount, exec }`, producing
  payloads that contain no compiled code;
- `T₃`, the application author: `Ops = { compile, fetch, mount, exec }`,
  where `compile` applies to the author's own sources only;
- `T₄`, the platform developer: `Ops = { compile, link, fetch, mount,
  exec }`, applying to the ecosystem's own machinery.

**Proposition 6 (tier separation).** `compile ∈ Ops(T₄)` only. No tier
below `T₄` requires a compiler, and a design that makes a tier below `T₄`
compile is ill-formed.

*Proof sketch.* A `T₁` actor receives a package whose loader is a
prebuilt binary per triplet and whose runtime is a prebuilt pair fetched
by digest; nothing in the run path invokes a compiler. A `T₂` author
presses images from data, which requires no compiler. A `T₃` author
compiles their own sources with their own toolchain, and receives the
runtime and the loaders as published artifacts. The falsifier is
therefore an artifact inspection rather than a runtime observation.

*Falsifier.* The freestanding-boundary record enumerates, per tier, the
toolchain each actor needs, and the release audits record that no shipped
artifact shells out to `curl`, `git` or a compiler.

**Corollary 7 (the complexity budget).** For each tier, the number of
ecosystem-specific concepts an actor must learn is bounded by the concepts
of that tier's operations: a `T₁` actor needs the package file, a `T₂`
author needs the image and the manifest, a `T₃` author adds the press and
the trailer, and only `T₄` needs the wire format.

## 7. Trust

**Definition 11 (verification point).** Bytes are verified at fetch and
at install. A run verifies nothing, and it verifies nothing because the
bytes it reads are the bytes that were verified.

**Proposition 8 (verification is monotone under composition).** If every
image in a mount table was verified against a pinned digest before it
entered the store, and the store returns those bytes unmodified, then
every run over that mount table reads verified bytes, for every
composition and every number of mounts.

*Proof sketch.* The store's contract is byte-identity: the artifact in
the store is the artifact the registry published, never a re-wrapped or
re-stitched form. The mount path reads the store's bytes. Induction over
the mount table therefore preserves the predicate, and the composed run
inherits it because composition adds mounts rather than rewriting bytes.

*Falsifier.* The store's byte-identity tests, the digest-sidecar
verification tests, and the signing suite's refusal cases. The property
is also the reason a delta fetch is safe: the assembler verifies every
group against the sidecar as it lands and the whole image against its
pin before the file is renamed into place, so a lying sidecar wastes
bandwidth and cannot install bytes.

Two consequences are worth stating because they are often conflated.
First, signing is opt-in and verification of a signed artifact is always
strict, so an unsigned artifact is first-class and a signed one cannot
degrade to unsigned. Second, integrity and authenticity are different
predicates: a digest proves what the bytes are, a signature proves who
published them, and a system that has only the first has distribution
integrity without distribution identity.

## 8. What the calculus does not claim

A paper of this kind earns its credibility by its exclusions.

1. **It does not claim build reproducibility for arbitrary software.**
   That is a property of derivations over declared inputs, and the
   systems that provide it — Nix, Guix — do something this calculus does
   not attempt. Tebako reproduces its own artifacts.
2. **It does not claim multi-process orchestration.** A package is one
   process tree, and a container platform is the right answer for a
   service mesh.
3. **It does not claim byte-level delta transfer on its own images.** A
   measurement over three consecutive runtime pairs found zero shared
   4 MiB groups: the images are compressed containers, so a rebuild
   re-lays out every byte. The assembler declines a delta that cannot
   pay, and the honest statement is that the mechanism is proven and its
   leverage on today's artifacts is zero.
4. **It does not claim uniform behaviour across all tiers on every
   platform.** A Windows host materializes a host tree instead of reading
   an image lazily, because its boot tier cannot read the seeded layout;
   the difference is recorded per platform rather than smoothed over.
5. **It does not claim the propositions are machine-checked.** Each is
   stated with the test that falsifies it, which is a weaker and more
   useful claim than an unverified proof.

## 9. The normative set as appendix

| this paper | normative source |
|---|---|
| Definition 1, WF-1 | Spec 11 (the VFS model), Spec 20 (the LimniFS backend) |
| Definition 2, WF-2 | Spec 03 (the payload manifest), Spec 05 (constraint semantics) |
| Definitions 3–6, WF-3 | Spec 02 (the wire format), Spec 17 (the driver contract) |
| Definition 7, WF-4 | Spec 08 (jails), Spec 23 §6 (the resolution algorithm) |
| Proposition 1, 2 | Spec 11 §2 (mount table and dispatch) |
| Proposition 3 | Spec 23 (declarative composition), Spec 33 (runtime on runtime) |
| Proposition 4 | Spec 17 (the launcher ABI and entry resolution) |
| Proposition 5 | Spec 42 (tree solving) |
| Proposition 6, Corollary 7 | Spec 00 (the invariants), the audience law |
| Proposition 8 | Spec 09 (trust and signing), Spec 16 (distribution and installation), Spec 39 (lazy range mount) |

The appendix is the specification set itself, not a restatement of it. A
reader who wants to audit a proposition follows the row to the document,
and then to the falsifier named beside it.

## 10. References and the submission shape

The paper's own claims are grounded in the specification set named in
section 9. The references below are the external works a reviewer will
expect it to engage, and they are listed because the comparison in
section 8 is only credible when the systems being compared are cited
rather than described.

1. E. Dolstra, A. Löh and N. Pierron, "NixOS: A Purely Functional Linux
   Distribution", *ACM SIGPLAN International Conference on Functional
   Programming (ICFP)*, 2008. The content-addressed store and the
   derivation model that section 9 compares against.
2. L. Courtès, "Functional Package Management with Guix", *European Lisp
   Symposium (ELS)*, 2013. The Scheme-embedded form of the same model,
   with the bootstrap chain that section 11 credits.
3. A. Mokhov, N. Mitchell and S. Peyton Jones, "Build Systems à la
   Carte", *Proceedings of the ACM on Programming Languages (ICFP)*,
   2018. The taxonomy of build systems that this paper's dispatch and
   resolution separation follows.
4. U. Drepper, "Static Linking Considered Harmful", 2003. The cost of the
   first escape in section 1, stated by the person who maintained the
   dynamic loader.
5. Open Container Initiative, "OCI Image Format Specification", current
   revision. The image and distribution vocabulary that section 1 and
   section 8 use as the point of contrast.
6. The Reproducible Builds project, `reproducible-builds.org`, and the
   in-toto framework (S. Torres-Arias et al., *USENIX Security*, 2019)
   for supply-chain attestation. Both are the standard against which
   Proposition 8's scope is drawn: tebako verifies what it fetched, and
   attestation of how it was built is a separate, complementary claim.
7. The AppImage documentation and the `conda` channel specification, for
   the two comparators in sections 9 and 10 whose behaviour is defined by
   their own documents rather than in the literature.

**Submission shape.** The draft is roughly three thousand words across ten
sections, which maps to a ten-to-twelve page workshop submission with the
specification set as an appendix and the falsifier table as an artifact
appendix. The
target venues are the ones a systems reader would look for this argument
in: a workshop adjacent to HotOS or SOSP, or the REBLS workshop for the
language-oriented framing. The choice between them turns on whether the
reviewers weight the algebra or the systems evidence, and the draft is
deliberately written to serve either.

**What a submission would still need.** A bibliography pass against each
venue's style, a review by a reader who has not seen the system, and a
decision on whether the falsifier table belongs in the paper or in an
artifact appendix. None of these change the content, and all of them are
outside what this queue can produce without an author who is not its
implementer.
