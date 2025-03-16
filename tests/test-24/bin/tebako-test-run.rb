#!/home/tebako/bin/ruby
# frozen_string_literal: true

require "test"

begin
  instance = Tebako::Test::TebakoTest.new
  instance.run!
end
