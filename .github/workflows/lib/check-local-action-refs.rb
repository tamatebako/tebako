#!/usr/bin/env ruby
# frozen_string_literal: true

# check-local-action-refs.rb — the v2.8.74 attest-path regression net.
#
# release run 42507966270 failed on every leg at "Attest build
# provenance": the runner resolves a local action's `uses: ./…` against
# the workspace root, never the job's checkout path, and every build leg
# checks the repo out at tebako-rs/. The defect shipped green because the
# step's `if` disarms it on rehearsals and a skipped step's `uses:` is
# never resolved. This check runs on PR CI, where the step is disarmed,
# and proves the reference resolves on the leg that carries it.
#
# For each workflow in .github/workflows/, for each job, for each
# `uses: ./…` step: the path is resolved against the job's FIRST
# actions/checkout `path:` (the runner's convention is the workspace
# root; a job that checks the repo out at a prefix needs the reference
# prefixed the same way), and the target must carry action.yml or
# action.yaml on disk.

require "yaml"

ROOT = File.expand_path("../../..", __dir__)
WORKFLOWS = File.join(ROOT, ".github", "workflows")

failures = []
checked = 0

Dir[File.join(WORKFLOWS, "*.{yml,yaml}")].sort.each do |wf|
  doc = YAML.unsafe_load_file(wf)
  jobs = doc["jobs"] or next
  jobs.each do |job_name, job|
    steps = job["steps"] or next
    checkout_path = nil
    steps.each do |step|
      uses = step["uses"] or next
      if uses.start_with?("actions/checkout@")
        checkout_path ||= step.dig("with", "path")
        next
      end
      next unless uses.start_with?("./")

      # The runner resolves ./ against the workspace root; the action
      # must exist at <workspace>/<checkout_path>/<ref>. On disk here the
      # checkout path is the repo root itself, so the action must exist at
      # <ref> with the job's checkout prefix REMOVED if the prefix is the
      # repo's own checkout dir — the reference spells the prefix because
      # the runner does not add it.
      rel = uses.delete_prefix("./")
      on_disk =
        if checkout_path && rel.start_with?("#{checkout_path}/")
          rel.delete_prefix("#{checkout_path}/")
        else
          rel
        end
      checked += 1
      has_action = %w[action.yml action.yaml].any? do |f|
        File.file?(File.join(ROOT, on_disk, f))
      end
      next if has_action

      failures << "#{File.basename(wf)}:#{job_name}: `uses: #{uses}` does not resolve to an action " \
                  "(checkout path #{checkout_path || '<root>'} → repo file #{on_disk}/{action.yml,action.yaml} missing)"
    end
  end
end

if failures.empty?
  puts "local-action-refs: #{checked} reference(s) resolve"
else
  failures.each { |f| warn "::error::#{f}" }
  exit 1
end
