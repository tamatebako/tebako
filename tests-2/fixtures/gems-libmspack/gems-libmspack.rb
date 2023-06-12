# frozen_string_literal: true

# Copyright (c) 2022-2023, [Ribose Inc](https://www.ribose.com).
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

require "rubygems"
require "bundler/setup"
require "tempfile"

COMPILER_MEMFS = "/__tebako_memfs__"

# https://github.com/pmq20/ruby-packer/blob/master/lib/compiler/constants.rb#L10
COMPILER_MEMFS_LIB_CACHE = Pathname.new(Dir.mktmpdir("tebako-tests-"))
at_exit do
  FileUtils.remove_dir(COMPILER_MEMFS_LIB_CACHE.to_path, true)
end

# Extension for String class
# Operations with quotes
class String
  def quoted?
    start_with?('"') && end_with?('"')
  end

  def unquote
    chomp('"').reverse.chomp('"').reverse
  end

  def quote
    "\"#{self}\""
  end
end

def extract_memfs(file, wild: false, extract_path: COMPILER_MEMFS_LIB_CACHE)
  is_quoted = file.quoted?
  file = file.unquote if is_quoted

  return file unless File.exist?(file) && file.start_with?(COMPILER_MEMFS)

  memfs_extracted_file = extract_path + File.basename(file)
  unless memfs_extracted_file.exist?
    files = wild ? Dir.glob("#{File.dirname(file)}/*#{File.extname(file)}") : [file]
    FileUtils.cp_r files, extract_path
  end

  is_quoted ? memfs_extracted_file.to_path.quote : memfs_extracted_file.to_path
end

require "ffi"

# HACK: extract temp libraries to use with ffi
# Wrapper for FFI.map_library_name method
module FFI
  # https://stackoverflow.com/questions/29907157/how-to-alias-a-class-method-in-rails-model/29907207
  singleton_class.send(:alias_method, :map_library_name_orig, :map_library_name)

  # http://tech.tulentsev.com/2012/02/ruby-how-to-override-class-method-with-a-module/
  def self.map_library_name(lib)
    l = extract_memfs(lib)
    puts "#{lib} ==> #{l}"
    ll = map_library_name_orig(l)
    puts "#{lib} ==> #{l} ==> #{ll}"
    ll
  end
end
# END of HACK

require "libmspack"
puts "Hello! libmspack welcomes you to the magic world of ruby gems."
