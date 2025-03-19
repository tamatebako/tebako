# frozen_string_literal: true

begin
  require_relative "lib/tebako-bundle-test/version"
rescue LoadError
  require "tebako-bundle-test/version"
end

Gem::Specification.new do |spec|
    spec.name          = "tebako-bundle-test"
    spec.version       = TebakoBundleTest::VERSION
    spec.authors       = ["Ribose"]
    spec.email         = ["open.source@ribose.com"]

    spec.summary       = "A simple gem for tebako testing"
    spec.homepage      = "https://github.com/tamatebako"
    spec.license       = "Unlicense"
    spec.required_ruby_version = Gem::Requirement.new(">= 2.7.0")
    spec.metadata["homepage_uri"] = spec.homepage

    spec.files = Dir["bin/*.rb"] + Dir["lib/**/*.rb"] + Dir["*.gemspec"]
    spec.files << "Gemfile" << "Gemfile.lock"

    spec.bindir = "bin"
    spec.executables << "tebako-test-run.rb"
    spec.require_paths = %w[bin lib]
  end
