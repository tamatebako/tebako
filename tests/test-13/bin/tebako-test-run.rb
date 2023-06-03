#!/home/tebako/bin/ruby

$LOAD_PATH.unshift File.expand_path("../lib", __dir__)
require "tebako-test"

begin
  instance = TebakoTest.new
  instance.run!
end
