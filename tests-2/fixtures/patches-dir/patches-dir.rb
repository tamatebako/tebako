# frozen_string_literal: true

# Copyright (c) 2021-2023 [Ribose Inc](https://www.ribose.com).
# All rights reserved.
# This file is a part of tebako
#
# Redistribution and use in source and binary forms, with or without
# modification, are permitted provided that the following conditions
# are met:
# 1. Redistributions of source code must retain the above copyright
#    notice, this list of conditions and the following disclaimer.
# 2. Redistributions in binary form must reproduce the above copyright
#    notice, this list of conditions and the following disclaimer in the
#    documentation and/or other materials provided with the distribution.
#
# THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
# ``AS IS'' AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED
# TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR
# PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDERS OR CONTRIBUTORS
# BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
# CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
# SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
# INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
# CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
# ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
# POSSIBILITY OF SUCH DAMAGE.

rt = RUBY_PLATFORM =~ /msys|mingw|cygwin|mswin/ ? "A:/__tebako_memfs__" : "/__tebako_memfs__"

puts "===== Dir test ====="

# test 1  chdir absolute - getwd
expected_cwd = "#{rt}/local/"
print "chdir '#{File.dirname(__FILE__)}' ... "
Dir.chdir(File.dirname(__FILE__))
cwd = Dir.getwd
raise "getwd returned #{cwd} while #{expected_cwd} was expected" unless cwd.eql? expected_cwd

puts "OK(success)"

# test 2  chdir relative - getwd
expected_cwd = "#{rt}/local/level-1/"
print "chdir 'level-1' ... "
Dir.chdir("level-1")
cwd = Dir.getwd
raise "getwd returned #{cwd} while #{expected_cwd} was expected" unless cwd.eql? expected_cwd

puts "OK(success)"

# test 3  chdir relative (does not exit) - getwd
print "chdir 'does-not-exists' ... "
begin
  Dir.chdir("does-not-exist")
  failed = false
rescue Errno::ENOENT
  failed = true
end
raise "chdir succeeded while exception was expected" unless failed

cwd = Dir.getwd
raise "Getwd returned #{cwd} while #{expected_cwd} was expected" unless cwd.eql? expected_cwd

puts "OK(failure)"

# test 4  chdir absolute (does not exit) - getwd
print "chdir '/bin/does-not-exists' ... "
begin
  Dir.chdir("/bin/does-not-exist")
  failed = false
rescue Errno::ENOENT, Errno::ENXIO
  failed = true
end
raise "chdir succeeded while exception was expected" unless failed

cwd = Dir.getwd
raise "getwd returned #{cwd} while #{expected_cwd} was expected" unless cwd.eql? expected_cwd

puts "OK(failure)"

# test 5  open - read - tell - seek - close - rewind
print "open - read - tell - seek - close @ '#{rt}/local/level-1/level-2' ... "
a_dir = Dir.new("#{rt}/local/level-1/level-2")

r = a_dir.read
p = a_dir.tell
raise "Dir.read returned #{r} while '.' was expected" unless r.eql? "."
raise "Dir.tell returned #{p} while 1 was expected" unless p.eql? 1

r = a_dir.read
p = a_dir.tell
raise "Dir.read returned #{r} while '..' was expected" unless r.eql? ".."
raise "Dir.tell returned #{p} while 2 was expected" unless p.eql? 2

re = Regexp.new("file-..txt").freeze
(3..5).each do |i|
  r = a_dir.read
  p = a_dir.tell
  raise "Dir.read returned #{r} while 'file-?.txt' was expected" unless r =~ re
  raise "Dir.tell returned #{p} while #{i} was expected" unless p.eql? i
end

a_dir.seek(1)
r = a_dir.read
p = a_dir.tell
raise "Dir.read returned #{r} while '..' was expected" unless r.eql? ".."
raise "Dir.tell returned #{p} while 2 was expected" unless p.eql? 2

a_dir.rewind
p = a_dir.tell
raise "Dir.tell returned #{p} while 1 was expected" unless p.eql? 0

r = a_dir.read
p = a_dir.tell
raise "Dir.read returned #{r} while '.' was expected" unless r.eql? "."
raise "Dir.tell returned #{p} while 1 was expected" unless p.eql? 1

p = a_dir.close
raise "Dir.close returned #{p} while nil was expected" unless p.nil?

puts "OK(match)"

# test 7  glob
print "[\"**/*.txt\", base:\"#{rt}/local/\"] ... "
fls = Dir["**/*.txt", base: "#{rt}/local/"]
EXPECTED_1 = ["level-1/level-2/file-1.txt", "level-1/level-2/file-2.txt", "level-1/level-2/file-3.txt"].freeze

if fls.difference(EXPECTED_1).any? || EXPECTED_1.difference(fls).any?
  raise "Dir[] returned #{fls} while #{EXPECTED_1} was expected"
end

puts "OK(match)"

# test 8  glob
print "[\"#{rt}/local/**/file-1.txt\"] ... "
fls = Dir["#{rt}/local/**/file-1.txt"]
EXPECTED_2 = ["#{rt}/local/level-1/level-2/file-1.txt"].freeze

if fls.difference(EXPECTED_2).any? || EXPECTED_2.difference(fls).any?
  raise "Dir[] returned #{fls} while #{EXPECTED_2} was expected"
end

puts "OK(match)"

# test 9 Dir.empty?
print "Dir.empty?(\"#{rt}/local/level-1/level-2\") ..."
r = Dir.empty?("#{rt}/local/level-1/level-2")
raise "Dir.empty? returned #{r} while 'false' was expected" if r

puts "OK(match)"

# test 10 Dir.empty?
print "Dir.empty?(\"#{rt}/local/level-1/level-2/level-3\") ..."
r = Dir.empty?("#{rt}/local/level-1/level-2/level-3")
raise "Dir.empty? returned #{r} while 'true' was expected" unless r

puts "OK(match)"
