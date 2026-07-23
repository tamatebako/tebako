# frozen_string_literal: true

# Copyright (c) 2026 [Ribose Inc](https://www.ribose.com).
# All rights reserved.
# This file is a part of the Tebako project.
#
# Redistribution and use in source and binary forms, with or without
# modification, are permitted provided that the following conditions
# are met:
# 1. Redistributions of source code must retain the above copyright
#    notice, this list of conditions and the following disclaimer.
# 2. Redistributions in binary form must reproduce the above copyright
#    notice, this list of conditions and the following disclaimer in the
#    documentation and/or other materials provided with the distribution.
#
# THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
# ``AS IS'' AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED
# TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR
# PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDERS OR CONTRIBUTORS
# BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
# CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
# SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
# INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
# CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
# ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
# POSSIBILITY OF SUCH DAMAGE.

module Tebako
  # Launcher ABI v1 — the bootstrap → runtime handoff contract of the
  # three-part package model (spec §4.4).
  #
  # A lean package is [tebako-bootstrap][.tfs image slots][tpkg trailer].
  # A fat package has the same layout plus the runtime package itself as a
  # payload slot (format_id TPKG_FORMAT_RUNTIME, empty mount point,
  # ";sha256=<hex>" appended to the trailer's runtime_ref): at first run the
  # bootstrap installs the payload into the shared cache instead of
  # downloading it. Payload slots are never handed over as --tebako-image.
  #
  # The bootstrap (tamatebako/tebako-bootstrap) parses the trailer of its own
  # executable, resolves the language runtime into the shared cache, and
  # execs it as:
  #
  #   <runtime> --tebako-image <file>:<slot>:<mount-point> ...
  #             --tebako-entry <argv0> <user args...>
  #
  # The runtime (a tebako-packaged interpreter built from this repo) consumes
  # that handoff in src/tebako-main.cpp:
  #
  #   --tebako-image <file>:<slot>:<mount-point>
  #       Repeatable, one per image slot, in slot order. <file> is a package
  #       file carrying a tpkg manifest trailer (spec §4.3); <slot> is the
  #       0-based index into that file's slot table; <mount-point> is the
  #       memfs path the image is mounted at. The runtime reads the trailer
  #       of <file>, takes the slot's recorded offset/size, and mounts the
  #       image directly out of <file> — no extraction, no temp copies.
  #       The image whose mount point equals the runtime's compiled-in
  #       fs_mount_point becomes the root filesystem (fallback: the first
  #       image given); the remaining images are mounted as extra slots.
  #
  #   --tebako-entry <argv0>
  #       Not repeatable. <argv0> is the lean package's original argv[0] (a
  #       host path as invoked by the user — not a path inside the memfs);
  #       the runtime presents it as the application program name. Every
  #       argument after <argv0> is application argv and is passed through
  #       to the interpreter untouched; tebako option scanning stops here.
  #       The runtime then runs its compiled-in entry point (for prebuilt
  #       runtimes, the /local/stub.rb dispatcher inside the root image).
  #
  #   --tebako-launcher-abi <n>
  #       Not repeatable, optional. States the launcher ABI version the
  #       bootstrap speaks; the runtime refuses the handoff when <n> exceeds
  #       the ABI it supports, naming both versions. tebako-bootstrap v1 does
  #       not emit this option — it validates the trailer's launcher_abi
  #       field itself and encodes the ABI in the runtime name it resolves
  #       (runtime_ref "...;tebako=<abi>") — but the runtime accepts the
  #       option so the check exists on both sides of the handoff.
  #
  # The three option spellings above are exactly what tebako-bootstrap
  # v1 emits (space-separated values, images before --tebako-entry); the
  # runtime additionally accepts the --opt=value form of each.
  #
  # Versioning: VERSION is bumped whenever the handoff changes incompatibly.
  # This file is the Ruby-side constant set; src/tebako-main.cpp carries the
  # matching C++ constants — keep the two in sync.
  module LauncherAbi
    # Launcher ABI version implemented by this gem and by tebako-main.cpp
    VERSION = 1

    # Repeatable: hand one image slot of a package file to the runtime
    IMAGE_ARG = "--tebako-image"

    # Separator: value is the package argv[0]; the rest is application argv
    ENTRY_ARG = "--tebako-entry"

    # Optional: launcher ABI version the bootstrap speaks
    VERSION_ARG = "--tebako-launcher-abi"

    # Format one --tebako-image value per the ABI: "<file>:<slot>:<mount-point>".
    # (The parser splits on the last two colons, so <file> may itself contain
    # colons — e.g. Windows drive prefixes.)
    def self.image_spec(file, slot, mount_point)
      "#{file}:#{slot}:#{mount_point}"
    end
  end
end
