# frozen_string_literal: true

puts "Hello!  This is test-01 talking from inside DwarFS"

puts "Gem path: #{Gem.path}"
if defined?(TebakoRuntime::VERSION)
  puts "Using tebako-runtime v#{TebakoRuntime::VERSION}"
else
  puts "Tebako runtime not loaded"
end
