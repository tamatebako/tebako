#!/home/tebako/bin/ruby

$LOAD_PATH.unshift File.expand_path("../../lib", __FILE__)
require "tebako_test"

begin
  instance = tebako_test.new
  instance.run!
end 

