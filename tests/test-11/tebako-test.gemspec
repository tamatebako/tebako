# frozen_string_literal: true
require_relative "lib/version"

Gem::Specification.new do |s|
  s.name        = "tebako-test"
  s.version     = Test::VERSION
  s.summary     = "A simple gem for tebako testing"
  s.authors     = ["Ribose"]
  s.email       = ["open.source@ribose.com"]
  s.files       = Dir.glob("lib/**/*") + Dir.glob("exe/**/*")
  s.homepage    = "https://github.com/tamitebako"
  s.license     = "Unlicense"
  s.bindir      = "exe"
  s.required_ruby_version = Gem::Requirement.new(">= 2.7.0")
  s.add_dependency "text-table", "~> 1.2.4"
  s.executables << "tebako-test-run.rb"
end
