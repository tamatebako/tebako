# frozen_string_literal: true

require "text-table"

table = Text::Table.new
table.head = %w[A B]
table.rows = [%w[a1 b1]]
table.rows << %w[a2 b2]

puts "Hello! This is test-18 talking from inside DwarFS"
puts "You will now see a nice table that will be drawn for you by text-table gem."
puts table
