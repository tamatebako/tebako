# frozen_string_literal: true

require "text-table"

# Tebako test class
class TebakoTest
  def msg
    table = Text::Table.new
    table.head = %w[A B]
    table.rows = [%w[a1 b1]]
    table.rows << %w[a2 b2]
    puts table
  end

  def run!
    puts "Hello! This is test-11 talking from inside DwarFS"
    puts "You will now see a nice table that will be drawn for you by text-table gem."
    msg
  end
end
