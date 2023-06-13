#!/home/tebako/bin/ruby
# frozen_string_literal: true

$LOAD_PATH.unshift File.expand_path("../lib", __dir__)
require "tebako-test"

begin
  instance = TebakoTest.new
  instance.run!
end
