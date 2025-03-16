# frozen_string_literal: true

# Copyright (c) 2021-2025, [Ribose Inc](https://www.ribose.com).
# All rights reserved.
# This file is a part of the Tebako project.
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

puts "===== File tests ====="

rt = RUBY_PLATFORM =~ /msys|mingw|cygwin|mswin/ ? "A:/__tebako_memfs__" : "/__tebako_memfs__"

# test 1  exist? (aka stat)
print "exist?(\"#{rt}/local/level-1/level-2/file-1.txt\") ... "
r = File.exist?("#{rt}/local/level-1/level-2/file-1.txt")
raise "exist? returned '#{r}' while 'true' was expected" unless r

puts "OK(success)"

# test 1+  readable? (aka access/eaccess)
print "readable?(\"#{rt}/local/level-1/level-2/file-1.txt\") ... "
r = File.readable?("#{rt}/local/level-1/level-2/file-1.txt")
raise "readable? returned '#{r}' while 'true' was expected" unless r

puts "OK(success)"

# test 2  executable_real? (aka access)
# X_OK is simulated as 'exist' on Windows ...
unless RUBY_PLATFORM =~ /msys|mingw|cygwin|mswin/
  print "executable_real?(\"#{rt}/local/level-1/level-2/file-2.txt\") ... "
  r = File.executable_real?("#{rt}/local/level-1/level-2/file-2.txt")
  raise "executable_real? returned '#{r}' while 'false' was expected" if r

  puts "OK(failure)"
end

# test 3  open - read - seak - rewind - close
print "open - seek - read - rewind - close  ... "
Dir.chdir("#{rt}/local/level-1/level-2")
File.open("file-2.txt", "r") do |f|
  r = f.read(18)
  raise "read returned '#{r}' while 'This is file-2.txt' was expected" unless r.eql? "This is file-2.txt"

  s = f.pos
  raise "pos returned '#{s}' while '18' was expected" unless s.eql? 18

  f.seek(5, :SET)
  s = f.pos
  raise "pos returned '#{s}' while '5' was expected" unless s.eql? 5

  r = f.read(13)
  raise "read returned '#{r}' while 'is file-2.txt' was expected" unless r.eql? "is file-2.txt"

  f.rewind
  s = f.pos
  raise "pos returned '#{s}' while '5' was expected" unless s.eql? 0

  r = f.read(4)
  raise "read returned '#{r}' while 'This' was expected" unless r.eql? "This"
end
puts "OK(match)"

# test 4  open - pread - close
# Windows => pread() function is unimplemented on this machine
unless RUBY_PLATFORM =~ /msys|mingw|cygwin|mswin/
  print "open - pread - close  ... "
  Dir.chdir("#{rt}/local/level-1/level-2")
  File.open("file-1.txt", "r") do |f|
    r = f.pread(13, 5)
    raise "read returned '#{r}' while 'is file-1.txt' was expected" unless r.eql? "is file-1.txt"
  end
  puts "OK(match)"
end

# test 5 lstat - readlink
# Not supprting file links on Windows
print "readlink  ... "
unless RUBY_PLATFORM =~ /msys|mingw|cygwin|mswin/
  Dir.chdir("#{rt}/local/level-1")
  s = File.lstat("#{rt}/local/level-1/link-3").size
  raise "lstat returned '#{s}' while '18' was expected" unless s.eql? 18

  r = File.readlink("link-3")
  raise "readlink returned '#{r}' while 'level-2/file-3.txt' was expected" unless r.eql? "level-2/file-3.txt"

  puts "OK(match)"
end
