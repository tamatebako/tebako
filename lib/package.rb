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

require 'fileutils'

require_relative 'pass1'
require_relative 'pass2'

# Tebako errors
class TebakoError < StandardError
  def initialize(msg = 'Unspecified error', code = 255)
    @error_code = code
    super(msg)
  end
  attr_accessor :error_code
end

# Tebako packaging support
module Package
  class << self
    # Pass1
    # Executed before Ruby build, patching ensures that Ruby itself is linked statically
    def pass1(ostype, ruby_source_dir, mount_point, src_dir)
      puts '-- Running pass1 script'

      recreate(src_dir)
      do_patch(Pass1.get_patch_map(mount_point), ruby_source_dir)

      # Roll back pass2 patches
      # Just in case we are recovering after some error
      restore_and_save_files(FILES_TO_RESTORE, ruby_source_dir)
      restore_and_save_files(FILES_TO_RESTORE_MUSL, ruby_source_dir) if ostype =~ /linux-musl/
      restore_and_save_files(FILES_TO_RESTORE_MSYS, ruby_source_dir) if ostype =~ /msys/
    end

    # Pass2
    # Creates packaging environment, patching ensures that tebako package is linked statically
    def pass2(ostype, ruby_source_dir, deps_lib_dir)
      puts '-- Running pass2 script'

      do_patch(Pass2.get_patch_map(ostype, deps_lib_dir), ruby_source_dir)
    end

    # Stash
    # Saves pristine Ruby environment that is used to deploy applications for packaging
    def stash(ruby_source_dir, src_dir, stash_dir)
      puts '-- Running stash script'
      #  .... this code snippet is executed 'outdside' of Ruby scripts
      # shall be reconsidered
      #    FileUtils.cd ruby_source_dir do
      #        puts "   ... creating pristine ruby environment at #{src_dir} [patience, it will take some time]"
      #        out, st = Open3.capture2e("cmake", "-E", "chdir", ruby_source_dir, "make", "install")
      #        print out if st.exitstatus != 0 || verbose
      #        raise TebakoError.new("stash [make install] failed with code #{st.exitstatus}") if st.exitstatus != 0
      #    end

      puts "   ... saving pristine ruby environment to #{stash_dir}"
      recreate(stash_dir)
      FileUtils.cp_r "#{src_dir}/.", stash_dir
      puts '   ... resetting extinit'
      FileUtils.rm_f("#{ruby_source_dir}/ext/extinit.c")
    end

    # Deploy
    # To be extended
    # Now it just recreates Ruby prostine environment from stash
    def deploy(stash_dir, src_dir, pre_dir, bin_dir)
      puts '-- Running deploy script'

      puts "   ... creating packaging environment at #{src_dir}"
      recreate([src_dir, pre_dir, bin_dir])
      FileUtils.cp_r "#{stash_dir}/.", src_dir
    end

    private

    FILES_TO_RESTORE = [
      'main.c',     'dir.c',     'dln.c',
      'file.c',     'io.c',      'tool/mkconfig.rb'
    ].freeze

    FILES_TO_RESTORE_MSYS = [
      'ruby.c',        'win32/win32.c',
      'win32/file.c',  'win32/dir.h'
    ].freeze

    FILES_TO_RESTORE_MUSL = [
      'thread_pthread.c'
    ].freeze

    def recreate(dirname)
      FileUtils.rm_rf(dirname, noop: nil, verbose: nil, secure: true)
      FileUtils.mkdir(dirname)
    end

    def restore_and_save(fname)
      raise TebakoError, "Could not save #{fname} because it does not exist." unless File.exist?(fname)

      old_fname = "#{fname}.old"
      if File.exist?(old_fname)
        File.delete(fname) if File.exist?(fname)
        File.rename(old_fname, fname)
      end
      FileUtils.cp(fname, old_fname)
    end

    def restore_and_save_files(files, ruby_source_dir)
      files.each do |fname|
        restore_and_save "#{ruby_source_dir}/#{fname}"
      end
    end

    def patch_file(fname, mapping)
      raise TebakoError, "Could not patch #{fname} because it does not exist." unless File.exist?(fname)

      puts "   ... patching #{fname}"
      restore_and_save(fname)
      contents = File.read(fname)
      mapping.each { |pattern, subst| contents.sub!(pattern, subst) }
      File.open(fname, 'w') { |file| file << contents }
    end

    def do_patch(patch_map, root)
      patch_map.each { |fname, mapping| patch_file("#{root}/#{fname}", mapping) }
    end
  end
end

begin
  unless ARGV.length.positive?
    raise TebakoError, 'Tebako script needs at least 1 arguments (command), none has been provided.'
  end

  case ARGV[0]
  when 'pass1'
    #       ARGV[0] -- command
    #       ARGV[1] -- OSTYPE
    #       ARGV[2] -- RUBY_SOURCE_DIR
    #       ARGV[3] -- FS_MOUNT_POINT
    #       ARGV[4] -- DATA_SRC_DIR
    raise TebakoError, "pass1 script expects 5 arguments, #{ARGV.length} has been provided." unless ARGV.length == 5

    Package.pass1(ARGV[1], ARGV[2], ARGV[3], ARGV[4])
  when 'pass2'
    #       ARGV[0] -- command
    #       ARGV[1] -- OSTYPE
    #       ARGV[2] -- RUBY_SOURCE_DIR
    #       ARGV[3] -- DEPS_LIB_DIR
    #       ARGV[4] -- DATA_SRC_DIR
    #       ARGV[5] -- FS_STASH_DIR
    raise TebakoError, "pass1 script expects 6 arguments, #{ARGV.length} has been provided." unless ARGV.length == 6

    Package.stash(ARGV[2], ARGV[4], ARGV[5])
    Package.pass2(ARGV[1], ARGV[2], ARGV[3])
  when 'stash'
    #       ARGV[0] -- command
    #       ARGV[1] -- RUBY_SOURCE_DIR
    #       ARGV[2] -- DATA_SRC_DIR
    #       ARGV[3] -- FS_STASH_DIR
    raise TebakoError, "stash script expects 4 arguments, #{ARGV.length} has been provided." unless ARGV.length == 4

    Package.stash(ARGV[1], ARGV[2], ARGV[3])
  when 'deploy'
    #       ARGV[0] -- command
    #       ARGV[1] -- FS_STASH_DIR
    #       ARGV[2] -- DATA_SRC_DIR
    #       ARGV[3] -- DATA_PRE_DIR
    #       ARGV[4] -- DATA_BIN_DIR
    raise TebakoError, "deploy script expects 5 arguments, #{ARGV.length} has been provided." unless ARGV.length == 5

    Package.deploy(ARGV[1], ARGV[2], ARGV[3], ARGV[4])
  else
    raise TebakoError, "Tebako script cannot process #{ARGV[0]} command"
  end
rescue TebakoError => e
  puts "Tebako script failed: #{e.message} [#{e.error_code}]"
  exit(e.error_code)
end

exit(0)