# frozen_string_literal: true


puts "Hello! This is test-20 talking from inside DwarFS"

require "net/http"

uri = URI("https://github.com/tamatebako/tebako/archive/refs/tags/v0.1.3.tar.gz")
http = Net::HTTP.new(uri.host, uri.port)
http.use_ssl = true

http.start do |h|
  puts "Request URI: #{uri}"
  request = Net::HTTP::Head.new(uri)
  response = h.request(request)

  puts "Response: #{response.code} #{response.message}"
end
