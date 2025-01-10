#!/home/tebako/bin/ruby
# frozen_string_literal: true

$LOAD_PATH.unshift File.expand_path("../lib", __dir__)
require "tebako-test"

instance = Test::TebakoTest.new
instance.run
