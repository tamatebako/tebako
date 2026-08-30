#!/usr/bin/env ruby
# frozen_string_literal: true

# Generate the benchmark workflow's matrix FROM benchmarks/platforms.yaml
# (spec 27 §3: the workflow never hardcodes a triplet — the document is
# the SSOT; adding a platform edits platforms.yaml, never the workflow).
# Emits `matrix=<json>` to $GITHUB_OUTPUT. Runner-baked ruby; psych/JSON
# are stdlib.

require "json"
require "yaml"

doc = YAML.load_file(ARGV.fetch(0))
legs = doc.fetch("triplets").map do |triplet, cfg|
  leg = { "triplet" => triplet, "runner" => cfg.fetch("runner") }
  leg["container"] = cfg["container"] if cfg.key?("container")
  leg
end

File.open(ENV.fetch("GITHUB_OUTPUT"), "a") do |f|
  f.puts "matrix=#{JSON.generate({ "include" => legs })}"
end
