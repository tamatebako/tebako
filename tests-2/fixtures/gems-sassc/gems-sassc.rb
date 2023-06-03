# frozen_string_literal: true

# Copyright (c) 2021-2022, [Ribose Inc](https://www.ribose.com).
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

require "rubygems"
require "bundler/setup"
require "tempfile"

COMPILER_MEMFS = "/__tebako_memfs__"

# https://github.com/pmq20/ruby-packer/blob/master/lib/compiler/constants.rb#L10
COMPILER_MEMFS_LIB_CACHE = Pathname.new(Dir.mktmpdir("tebako-tests-"))
at_exit do
  FileUtils.remove_dir(COMPILER_MEMFS_LIB_CACHE.to_path, true)
end

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
require "ffi"

module FFI
  # https://stackoverflow.com/questions/29907157/how-to-alias-a-class-method-in-rails-model/29907207
  singleton_class.send(:alias_method, :map_library_name_orig, :map_library_name)

  # http://tech.tulentsev.com/2012/02/ruby-how-to-override-class-method-with-a-module/
  def self.map_library_name(lib)
    l = extract_memfs(lib)
    ll = map_library_name_orig(l)
    puts "#{lib} ==> #{ll}"
    ll
  end
end
# END of HACK

require "sassc"

module SassC
  class Engine
    alias load_paths_orig load_paths
    def load_paths
      paths = (@options[:load_paths] || []) + SassC.load_paths
      np = []
      paths.each do |p|
        if p.start_with?(COMPILER_MEMFS)
          m = p.sub(COMPILER_MEMFS, COMPILER_MEMFS_LIB_CACHE.to_s)
          FileUtils.cp_r(File.join(p, "."), m) if File.exist?(p)
          np << m
        else
          np << p
        end
      end
      pp = np.join(File::PATH_SEPARATOR) unless np.empty?
      puts "Using load_path @ '#{pp}'"
      pp
    end
  end
end

name = File.join(File.dirname(__FILE__), "styles")
SassC.load_paths << File.dirname(__FILE__)
SassC.load_paths << name
SassC.load_paths << "/usr/local/fun"

SassC::Engine.new("@import 'base_style/all.scss'", style: :compressed).render

puts "Hello! SassC gem welcomes you to the magic world of ruby gems."
