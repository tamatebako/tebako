# frozen_string_literal: true

require_relative "../lib/cli"

if !ARGV.empty
  Tebako::TebakoCli.start
end
