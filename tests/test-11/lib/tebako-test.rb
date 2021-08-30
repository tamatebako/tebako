require 'text-table'

class TebakoTest 
  def msg
    table = Text::Table.new
    table.head = ['A', 'B']
    table.rows = [['a1', 'b1']]
    table.rows << ['a2', 'b2']
    puts table.to_s
  end

  def run!
    puts "Hello! This is test-11 talking from inside DwarFS"
    puts "You will now see a nice table that will be drawn for you by text-table gem."
    self.msg
  end
end
