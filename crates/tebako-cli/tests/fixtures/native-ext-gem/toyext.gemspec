Gem::Specification.new do |spec|
  spec.name = "toyext"
  spec.version = "0.1.0"
  spec.authors = ["tebako-cli tests"]
  spec.summary = "Tiny native-extension fixture (one C function)"
  spec.files = Dir["lib/**/*"] + Dir["ext/**/*"]
  spec.require_paths = ["lib"]
  spec.extensions = ["ext/toyext/extconf.rb"]
end
