require_relative "lib/tebako/bundle/test/version"

Gem::Specification.new do |spec|
  spec.name          = "tebako-bundle-test"
  spec.version       = Tebako::Bundle::Test::VERSION
  spec.authors       = ["Ribose"]
  spec.email         = ["open.source@ribose.com"]

  spec.summary       = "A simple gem for tebako testing"
  spec.homepage      = "https://github.com/tamitebako"
  spec.license       = "Unlicense"
  spec.required_ruby_version = Gem::Requirement.new(">= 2.3.0")
  spec.metadata["homepage_uri"] = spec.homepage

  # Specify which files should be added to the gem when it is released.
  # The `git ls-files -z` loads the files in the RubyGem that have been added into git.
  #  spec.files = Dir.chdir(File.expand_path(__dir__)) do
  #    `git ls-files -z`.split("\x0").reject { |f| f.match(%r{\A(?:test|spec|features)/}) }
  #  end

  spec.files = Dir["bin/**/*.rb"] + Dir["lib/**/*.rb"]

  spec.bindir = "bin"
  spec.executables << "tebako-test-run.rb"
  spec.require_paths = %w[bin lib]

  spec.add_dependency "text-table", "~> 1.2.4"
end
