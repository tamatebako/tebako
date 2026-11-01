#!/usr/bin/env ruby
# frozen_string_literal: true

# sync.rb — drift gate for the Tebako Specifications series
# (docs/spec-site/). Each rendered document records the blob SHA of the
# docs/spec/ Markdown file it was converted from, in a comment line:
#
#   // source: docs/spec/NN-name.md @ <40-hex blob sha>
#
# The gate recomputes the blob SHA of every referenced Markdown source
# (git hash-object over the working tree, so an uncommitted edit also
# trips the gate) and compares it with the recorded value. The Markdown
# is the source of truth and the AsciiDoc is its rendered twin, so a
# specification edit that lands without the twin being updated leaves
# the recorded SHA stale and the gate goes red.
#
# Usage:
#   ruby docs/spec-site/tools/sync.rb          # check (the CI gate)
#   ruby docs/spec-site/tools/sync.rb --sync   # bless a re-synced state
#
# The conversion is hand-tuned: this tool never regenerates document
# content. --sync only rewrites the recorded source markers after the
# twins have been updated by hand and reviewed.

require "open3"
require "yaml"

ROOT = File.expand_path("../../..", __dir__)
SITE_DIR = File.join(ROOT, "docs", "spec-site")
MANIFEST = File.join(SITE_DIR, "metanorma.yml")
MARKER = %r{\A// source: (\S+) @ ([0-9a-f]{40})\s*\z}.freeze

def blob_sha(relative_path)
  out, status = Open3.capture2("git", "-C", ROOT, "hash-object", "--", relative_path)
  status.success? ? out.strip : nil
end

sync = ARGV.delete("--sync")
abort "usage: sync.rb [--sync]" unless ARGV.empty?

files = YAML.safe_load_file(MANIFEST).dig("metanorma", "source", "files")
abort "sync.rb: #{MANIFEST} has no metanorma.source.files list" unless files.is_a?(Array)

drifted = []
updated = []
checked = 0

files.each do |entry|
  adoc = File.join(SITE_DIR, entry)
  abort "sync.rb: #{entry} is listed in metanorma.yml but does not exist" unless File.exist?(adoc)

  marker = File.readlines(adoc, chomp: true).map(&:strip).find { |l| l.match?(MARKER) }
  next unless marker # documents without a source marker (the series landing) are not gated

  checked += 1
  src, recorded = marker.match(MARKER).captures
  unless File.exist?(File.join(ROOT, src))
    drifted << "#{entry}: source #{src} does not exist"
    next
  end

  current = blob_sha(src)
  next if current == recorded

  if sync
    lines = File.readlines(adoc, chomp: true)
    lines.map! { |l| l.strip.match?(MARKER) ? "// source: #{src} @ #{current}" : l }
    File.write(adoc, lines.join("\n") + "\n")
    updated << "#{entry}: #{recorded[0, 12]} -> #{current[0, 12]}"
  else
    drifted << "#{entry}: #{src} is at #{current[0, 12]}…, twin records #{recorded[0, 12]}…"
  end
end

if sync
  updated.each { |u| puts "updated #{u}" }
  puts "sync.rb: #{checked} documents gated, #{updated.size} marker(s) updated"
  exit 0
end

if drifted.empty?
  puts "sync.rb: #{checked} documents gated, all twins current"
  exit 0
end

warn "sync.rb: #{drifted.size} document(s) drifted from their Markdown source:"
drifted.each { |d| warn "  #{d}" }
warn "sync.rb: update the twin(s) under docs/spec-site/sources/, then bless with"
warn "sync.rb:   ruby docs/spec-site/tools/sync.rb --sync"
exit 1
