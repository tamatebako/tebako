#!/home/tebako/bin/ruby

$LOAD_PATH.unshift File.expand_path("../lib/tebako/bundle", __dir__)
require "test"

begin
  instance = TebakoTest.new
  instance.run!
end
