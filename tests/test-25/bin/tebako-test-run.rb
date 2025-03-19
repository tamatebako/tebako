#!/home/tebako/bin/ruby
# frozen_string_literal: true

require "tebako-bundle-test"

begin
  instance = TebakoBundleTest::TebakoTestClass.new
  instance.run!
end
