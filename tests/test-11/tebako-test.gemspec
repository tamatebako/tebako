# frozen_string_literal: true

Gem::Specification.new do |s|
  s.name        = "tebako-test"
  s.version     = "0.0.1"
  s.summary     = "A simple gem for tebako testing"
  s.authors     = ["Ribose"]
  s.email       = ["open.source@ribose.com"]
  s.files       = ["bin/tebako-test-run.rb", "lib/tebako-test.rb"]
  s.homepage    = "https://github.com/tamitebako"
  s.license     = "Unlicense"
  s.bindir      = "bin"
  s.required_ruby_version = Gem::Requirement.new(">= 2.7.0")
  s.add_dependency "text-table", "~> 1.2.4"
  s.executables << "tebako-test-run.rb"
end
