# frozen_string_literal: true

require "irb"
require "yaml"

puts "Hello!  This is test-01 talking from inside DwarFS"

puts "Load path: #{$LOAD_PATH}"
puts "Gem path: #{Gem.path}"

exit 255 unless Gem.path.is_a?(Array)
exit 255 unless Gem.path.all? { |element| element.include?("__tebako_memfs__") }

puts "Gem home: #{Gem.dir}"
exit 255 unless Gem.dir.include?("__tebako_memfs__")

puts "Rubygems version: #{Gem.rubygems_version}"
if defined?(TebakoRuntime::VERSION)
  puts "Using tebako-runtime v#{TebakoRuntime::VERSION}"
else
  puts "Tebako runtime is not loaded"
end

puts "Current pwd: #{Dir.pwd}"
puts "Original pwd: #{$tebako_original_pwd}"

puts "$PROGRAM_NAME: #{$PROGRAM_NAME}"
puts "$0: #{$0}"
