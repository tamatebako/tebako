# frozen_string_literal: true

puts "Hello!  This is test-01 talking from inside DwarFS"

puts "Gem path: #{Gem.path}"

puts "Rubygems version: #{Gem.rubygems_version}"
if defined?(TebakoRuntime::VERSION)
  puts "Using tebako-runtime v#{TebakoRuntime::VERSION}"
else
  puts "Tebako runtime not loaded"
end

puts "Current pwd: #{Dir.pwd}"
puts "Original pwd: #{$tebako_original_pwd}"
