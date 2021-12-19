# Copyright (c) 2021, [Ribose Inc](https://www.ribose.com).
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

# This test is inspired packed-mn
# https://github.com/metanorma/packed-mn 

require 'rubygems'
require 'bundler/setup'
require 'tempfile'

COMPILER_MEMFS = '/__tebako_memfs__'

# https://github.com/pmq20/ruby-packer/blob/master/lib/compiler/constants.rb#L10
COMPILER_MEMFS_LIB_CACHE = Pathname.new(Dir.mktmpdir("tebako-tests-"))
at_exit {
  FileUtils.remove_dir(COMPILER_MEMFS_LIB_CACHE.to_path, true)
}

class String
  def is_quoted
    self.start_with?('"') && self.end_with?('"')
  end

  def unquote
    self.chomp('"').reverse.chomp('"').reverse
  end

  def quote
    "\"#{self}\""
  end
end

def extract_memfs(file, wild=false, extract_path=COMPILER_MEMFS_LIB_CACHE)
  is_quoted = file.is_quoted
  file = file.unquote if is_quoted

  return file unless File.exist?(file) && file.start_with?(COMPILER_MEMFS)

  memfs_extracted_file = extract_path + File.basename(file)
  unless memfs_extracted_file.exist?
    files = if wild
      Dir.glob("#{File.dirname(file)}/*#{File.extname(file)}")
    else
      [file]
    end
    FileUtils.cp_r files, extract_path
  end

  is_quoted ? memfs_extracted_file.to_path.quote : memfs_extracted_file.to_path
end

# HACK: extract temp libraries to use with ffi
# Wrapper for FFI.map_library_name method
require 'ffi'

module FFI
  # https://stackoverflow.com/questions/29907157/how-to-alias-a-class-method-in-rails-model/29907207
  self.singleton_class.send(:alias_method, :map_library_name_orig, :map_library_name)

  # http://tech.tulentsev.com/2012/02/ruby-how-to-override-class-method-with-a-module/
  def self.map_library_name(lib)
    l = extract_memfs(lib)
    ll = map_library_name_orig(l)
    puts "#{lib} ==> #{ll}"
    ll
  end
end
# END of HACK

# HACK fix path for 7zip load
sevenz_lib = RUBY_PLATFORM.downcase.match(/mswin|mingw/) ? "7z*.dll" : "7z.so"
#  sevenz_path = File.join(Gem.loaded_specs['seven_zip_ruby'].full_gem_path, 'lib', 'seven_zip_ruby', sevenz_lib)
sevenz_path = "/__tebako_memfs__/lib/ruby/gems/2.7.0/gems/seven_zip_ruby-1.3.0/lib/seven_zip_ruby/7z.so"
FileUtils.mkdir_p(COMPILER_MEMFS_LIB_CACHE / 'seven_zip_ruby')
FileUtils.cp(sevenz_path, COMPILER_MEMFS_LIB_CACHE / 'seven_zip_ruby')
$LOAD_PATH.unshift(COMPILER_MEMFS_LIB_CACHE)
# END of HACK

# Just make sure the constant exists
require 'seven_zip_ruby'
SevenZipRuby
puts "Hello! SevenZipRuby welcomes you to the magic world of ruby gems."