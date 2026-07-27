require "nokogiri"

doc = Nokogiri::XML("<root><hi>there</hi></root>")
raise "nokogiri parse failed" unless doc.at_xpath("//hi").text == "there"

puts "Hello from nokogiri app with nokogiri #{Nokogiri::VERSION}"
