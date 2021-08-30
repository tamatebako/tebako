# frozen_string_literal: true

require 'text-table'
require_relative "test/version"

module Tebako
  module Bundle
    module Test
      class Error < StandardError; end

      class TebakoTest 
       def msg
        table = Text::Table.new
        table.head = ['A', 'B']
        table.rows = [['a1', 'b1']]
        table.rows << ['a2', 'b2']
       puts table.to_s
      end

      def run!
       puts "Hello! This is test-20 talking from inside DwarFS"
       puts "You will now see a nice table that will be drawn for you by text-table gem."
       self.msg
      end
end
    end
  end
end
