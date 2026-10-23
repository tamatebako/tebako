# The composition calculus of tebako

*A draft for circulation. It describes the model as implemented, states the
tractable properties, and names the ones that are not tractable. Numbers are
measurements from this repository, cited where they appear.*

## 1. The problem

A program written in a managed language is not runnable on a host that lacks
its runtime. The usual answers are a container image, which ships an operating
system with the program, or a bundled interpreter tree, which extracts a private
copy of the runtime beside it. Both answers work. Both make a choice the author
may not want: the first requires a container runtime on the host and a daemon to
supervise it, and the second makes every installation pay for a full extraction
and leaves the extracted files unverified after they land.

This paper describes a third shape. The program and its runtime are packaged as
filesystem images, the images are mounted without extraction, and the process
reads its standard library through the mount. The shape makes one question
central that the other two answer implicitly: how are the pieces *composed*?
A mounted model has an algebra where an extraction model has a directory, and
the algebra is what this paper is about.

## 2. The model

A package has up to three independently distributable parts, and the separation
is load-bearing rather than cosmetic.

**A loader** is a small native executable that reads the package's own trailer,
resolves the runtime, verifies what it fetched, and hands off. It is the process
entry point of every package, which is why it is size-gated: a loader that
carried the runtime would be a bundled tree again.

**A runtime** is two artifacts. One is the interpreter executable; the other is
an image of the runtime's files, which the interpreter reads as its root. The
two are published together, pinned together, and downloaded once per machine.

**A payload** is an image of the program and whatever else it brings: code, data,
fonts, other runtimes. A payload declares what it is and what it needs in a
manifest that lives inside the image, so a payload describes itself without a
side channel.

The trailer that ties them together is a fixed-width structure at the end of the
file, carrying slot records with offsets, sizes, formats and mount points, and a
header naming the runtime reference and the ABI the loader speaks. The format
version is fixed forever; extensions ride flags rather than version bumps, so a
reader from an earlier era never breaks on a package from a later one.

## 3. The algebra of mounts

Mounting is a partial function from paths to images. The table resolves a lookup
by taking the longest mount point that prefixes the requested path, so a mount at
`/a/b` shadows a mount at `/a` for every path beneath it, and a path that only
matches `/a` falls through to it. Three properties follow, and each is proven by
the product's own suites:

1. **Order independence among disjoint mounts.** Two mounts whose points are
   disjoint cannot shadow each other, so the order in which they were added does
   not affect any lookup. The property is what makes multi-mount composition
   predictable.
2. **Depth is not a special case.** A mount nested inside another behaves
   exactly as a top-level mount does; the table is recursive, not a two-level
   map. Five levels are proven at the table and through the driver's own argv
   path, and a standing fixture presses a five-level package and asserts that
   every level answers from its own mount.
3. **A duplicate mount point is an error, never a merge.** Two images claiming
   the same point would make the result depend on insertion order, which the
   first property forbids, so the operation fails with `EEXIST` and the package
   does not mount partially.

Unmounting removes one level and does not cascade into its ancestors or its
children, which is what allows a mount to be replaced without rebuilding the
table beneath it.

## 4. The resolution calculus

A payload's manifest declares requirements: a language runtime with a constraint,
a toolkit at a name, data at a version, or a spawned runtime with an engine and
an implementation axis. Resolving one requirement in isolation is a scan; resolving
a tree is a constraint problem, and the two are different enough to deserve
different machinery.

The solver takes three inputs: the seed edges, a closure map that supplies each
candidate payload's own requirements, and the registry documents. The closure map
exists because the layer model forbids the alternative: the registry mirrors
resolution fields only, while the dependency declarations belong to the payload's
own manifest, so a solver reading the registry alone would have to invent an
empty dependency set. A candidate absent from the map is a named error, never an
implicit leaf.

Among the assignments that satisfy every edge, the solver returns the maximal
one under a stated order: higher version first for the earliest name in canonical
order, then the next, with the artifact digest breaking ties. Because the order
is total and the search backtracks, the answer is a function of the inputs alone
and not of iteration order or of which mirror served the document. A conflict is
reported as a named structure — the edge as declared, the candidate versions, the
reason each was excluded, and the smallest change that would satisfy it — rather
than as a failed mount discovered after the package was built.

The solved assignment serialises to a lockfile whose entries carry content
anchors and each registry's canonical snapshot root, never a URL. That choice is
what makes the same tree lock identically from any mirror. A press under a locked
mode refuses a different assignment line by line; the mode exists because an
automated build that silently re-solves is a build whose green result no longer
means what it did yesterday.

## 5. Trust as a fetch-time property

Verification happens when bytes are fetched, not when they are run. The anchor is
a digest published beside the artifact, and the loader checks the served bytes
against it before the runtime starts. Runtimes are fetched once per machine,
verified once, and mounted thereafter, so the per-run cost of trust is zero and
the per-fetch cost is one hash.

Signing is an opt-in axis, independent of the shape: an unsigned package is
accepted with a loud notice and a journal entry, and a requirement for signed
packages fails closed. The separation matters because the integrity check and the
provenance claim answer different questions, and a model that conflated them
would either over-claim for unsigned packages or refuse them.

A registry may publish a transparency log, and a reader can then prove that a row
was present at a stated tree size from a single root, and that the log only grew.
Silence is the remaining attack — a registry that publishes no log cannot be held
to completeness — and the model names that limitation rather than papering over
it.

## 6. What was measured

The composition model is cheap to state and expensive to get right; the numbers
below are the ones this repository can defend.

- A five-level nested composition, pressed and executed, serves every level from
  its own mount (reference-suite fixture, run 39907456088).
- A re-press of an unchanged project takes 4.6 to 5.0 seconds on a release build
  once the deploy driver's image is cached by content, against 9.7 to 10.5
  seconds before that cache existed (tebako#864).
- A cold press, including a 28.4 MB runtime download, takes 31.7 seconds on the
  same machine, of which the download is 7.2 seconds.
- A delta between two consecutive runtime images moved 100 percent of the bytes,
  because the images shared no identical block group. The negative result is
  reported because it is the measurement that redirected the work: lazy seeding,
  not delta, is the optimisation that pays on the current image framing.

## 7. What the model does not do

It does not isolate. A package runs as the user who starts it, and the jail
policy governs which host paths it may reach, not whether it may reach the
network. Isolation is a container's job.

It does not resolve dependencies inside a payload's own ecosystem. A payload that
carries a language-level lockfile is authoritative for what it vendors; the
solver composes payloads, not gems.

It does not make a registry honest. A registry that publishes no transparency log
can withdraw a row silently, and no reader-side mechanism detects that.

It does not yet solve runtime edges with its own machinery: those are resolved by
the dispatcher, and the solver refuses them by name rather than duplicating the
dispatcher's matcher.

## 8. Related work

Container images and their registries solved distribution at the granularity of a
userland, and the ecosystem's trust machinery — digests, signatures, and
transparency logs — is the model this paper borrows most from. Bundled
interpreters solved the "no runtime on the host" problem directly, at the cost of
extraction. Content-addressed stores, from Nix onward, made the case for
addressing bytes rather than names, which is why the lockfile in this model
records content and not locations. This paper's contribution is narrow: an
algebra for mounting composed images, and a resolution calculus that makes the
composition's answer deterministic and auditable.

## 9. Conclusion

Mounting rather than extracting turns packaging into a composition problem, and a
composition problem needs an algebra, a solver, and a lockfile before it can be
trusted. The model described here has all three, has them tested at the depth a
user can feel, and states plainly where it stops.
