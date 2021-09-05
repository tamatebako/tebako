require 'text-table'

class tebako_test 
  def self.msg
    table = Text::Table.new
    table.head = ['A', 'B']
    table.rows = [['a1', 'b1']]
    table.rows << ['a2', 'b2']
    table.to_s
    puts table.to_s
  end

  def self.run!
    puts "Hello! This is test-11 talking from inside DwarFS"
    self.msg
  end
end
