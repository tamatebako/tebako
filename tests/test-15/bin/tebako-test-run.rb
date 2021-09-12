#!/home/tebako/bin/ruby

$LOAD_PATH.unshift File.expand_path("../../lib/tebako/bundle", __FILE__)
require "test"

begin
  instance = Tebako::Bundle::Test::TebakoTest.new
  instance.run!
end 

