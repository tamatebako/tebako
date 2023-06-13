# frozen_string_literal: true

require_relative "lib/tebako/bundle/test/version"

Gem::Specification.new do |spec|
  spec.name          = "tebako-bundle-test"
  spec.version       = Tebako::Bundle::Test::VERSION
  spec.authors       = ["Ribose"]
  spec.email         = open.source @ribose.com

  spec.summary       = "A simple gem for tebako testing"
  spec.homepage      = "https://github.com/tamitebako"
  spec.license       = "Unlicense"
  spec.required_ruby_version = Gem::Requirement.new(">= 2.7.0")
  spec.metadata["homepage_uri"] = spec.homepage

  spec.files = Dir["bin/**/*.rb"] + Dir["lib/**/*.rb"]

  spec.bindir = "bin"
  spec.executables << "tebako-test-run.rb"
  spec.require_paths = %w[bin lib]

  spec.add_dependency "text-table", "~> 1.2.4"
end
