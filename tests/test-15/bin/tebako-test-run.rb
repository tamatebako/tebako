#!/home/tebako/bin/ruby
# frozen_string_literal: true

$LOAD_PATH.unshift File.expand_path("../lib/tebako/bundle", __dir__)
require "test"

begin
  instance = Tebako::Bundle::Test::TebakoTest.new
  instance.run!
end
