# Copyright (c) 2021, 2022 [Ribose Inc](https://www.ribose.com).
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

puts "===== Dir test ====="

# test 1  chdir absolute - getwd
ExpectedCwd1 = "/__tebako_memfs__/local/"
print "chdir '#{File.dirname(__FILE__)}' ... "
Dir.chdir(File.dirname(__FILE__))
cwd = Dir.getwd
raise "getwd returned #{cwd} while #{ExpectedCwd1} was expected" unless cwd.eql? ExpectedCwd1
print "OK(success)\n"

# test 2  chdir relative - getwd
ExpectedCwd2 = "/__tebako_memfs__/local/level-1/"
print "chdir 'level-1' ... "
Dir.chdir("level-1")
cwd = Dir.getwd
raise "getwd returned #{cwd} while #{ExpectedCwd2} was expected" unless cwd.eql? ExpectedCwd2
print "OK(success)\n"

# test 3  chdir relative (does not exit) - getwd
print "chdir 'does-not-exists' ... "
failed = true
begin
    Dir.chdir("does-not-exist")
    failed = false
rescue Errno::ENOENT
end
raise "chdir succeeded while exception was expected" unless failed
cwd = Dir.getwd
raise "Getwd returned #{cwd} while #{ExpectedCwd2} was expected" unless cwd.eql? ExpectedCwd2
print "OK(failure)\n"

# test 4  chdir absolute (does not exit) - getwd
print "chdir '/bin/does-not-exists' ... "
failed = true
begin
    Dir.chdir("/bin/does-not-exist")
    failed = false
rescue Errno::ENOENT
end
raise "chdir succeeded while exception was expected" unless failed
cwd = Dir.getwd
raise "getwd returned #{cwd} while #{ExpectedCwd2} was expected" unless cwd.eql? ExpectedCwd2
print "OK(failure)\n"

# test 5  open - read - tell - seek - close - rewind
print "open - read - tell - seek - close @ '/__tebako_memfs__/local/level-1/level-2' ... "
aDir = Dir.new("/__tebako_memfs__/local/level-1/level-2")

r = aDir.read
p = aDir.tell
raise "Dir.read returned #{r} while '.' was expected" unless r.eql? '.'
raise "Dir.tell returned #{p} while 1 was expected" unless p.eql? 1

r = aDir.read
p = aDir.tell
raise "Dir.read returned #{r} while '..' was expected" unless r.eql? '..'
raise "Dir.tell returned #{p} while 2 was expected" unless p.eql? 2

re = Regexp.new("file-.\.txt").freeze
for i in 3..5 do
    r = aDir.read
    p = aDir.tell
    raise "Dir.read returned #{r} while 'file-?.txt' was expected" unless r =~ re
    raise "Dir.tell returned #{p} while #{i} was expected" unless p.eql? i
end

aDir.seek(1)
r = aDir.read
p = aDir.tell
raise "Dir.read returned #{r} while '..' was expected" unless r.eql? '..'
raise "Dir.tell returned #{p} while 2 was expected" unless p.eql? 2

aDir.rewind
p = aDir.tell
raise "Dir.tell returned #{p} while 1 was expected" unless p.eql? 0
r = aDir.read
p = aDir.tell
raise "Dir.read returned #{r} while '.' was expected" unless r.eql? '.'
raise "Dir.tell returned #{p} while 1 was expected" unless p.eql? 1

p = aDir.close
raise "Dir.close returned #{p} while nil was expected" unless p.nil?

print "OK(match)\n"

# test 7  glob
print "[\"**/*.txt\", base:\"/__tebako_memfs__/local/\"] ... "
fls = Dir["**/*.txt", base:"/__tebako_memfs__/local/"]
Exp1 = ["level-1/level-2/file-1.txt", "level-1/level-2/file-2.txt", "level-1/level-2/file-3.txt"]

raise "Dir[] returned #{fls} while #{Exp1} was expected" if fls.difference(Exp1).any? || Exp1.difference(fls).any?
print "OK(match)\n"

# test 8  glob
print "[\"/__tebako_memfs__/local/**/file-1.txt\"] ... "
fls = Dir["/__tebako_memfs__/local/**/file-1.txt"]
Exp2 = ["/__tebako_memfs__/local/level-1/level-2/file-1.txt"]

raise "Dir[] returned #{fls} while #{Exp2} was expected" if fls.difference(Exp2).any? || Exp2.difference(fls).any?
print "OK(match)\n"

# test 9 Dir.empty?
print "Dir.empty?(\"/__tebako_memfs__/local/level-1/level-2\") ..."
r = Dir.empty?("/__tebako_memfs__/local/level-1/level-2")
raise "Dir.empty? returned #{r} while 'false' was expected" if r
print "OK(match)\n"

# test 10 Dir.empty?
print "Dir.empty?(\"/__tebako_memfs__/local/level-1/level-2/level-3\") ..."
r = Dir.empty?("/__tebako_memfs__/local/level-1/level-2/level-3")
raise "Dir.empty? returned #{r} while 'true' was expected" unless r
print "OK(match)\n"
