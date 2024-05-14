#!/usr/bin/env ruby
# frozen_string_literal: true

# Copyright (c) 2021-2024 [Ribose Inc](https://www.ribose.com).
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

require "English"
require "tmpdir"
require "fileutils"
require "rbconfig"
require "pathname"
require "rubygems"
require "open3"
require "minitest/autorun"

# Tebako test set
# rubocop:disable Metrics/ClassLength
class TebakoTest < Minitest::Test
  # Path to test fixtures.
  FixturePath = File.expand_path(File.join(File.dirname(__FILE__), "fixtures"))
  Prefix = File.expand_path(File.join(File.dirname(__FILE__), ".."))
  Tebako = File.join(Prefix, "exe", "tebako")

  def initialize(*args)
    super(*args)
    @testnum = 0
  end

  # Sets up temporary environment variables and yields to the
  # block. When the block exits, the environment variables are set
  # back to their original values.
  def with_env(hash)
    old = {}
    hash.each do |k, v|
      old[k] = ENV.fetch(k, nil)
      ENV[k] = v
    end
    begin
      yield
    ensure
      hash.each_key { |k| ENV[k] = old[k] }
    end
  end

  # Sets up an directory with a copy of a fixture and yields to the
  # block, then cleans up everything. A fixture here is a hierachy of
  # files located in test/fixtures.
  def with_fixture(name, target_path = nil, &block)
    path = File.join(FixturePath, name)
    with_tmpdir([], target_path) do |tmpdirname|
      FileUtils.cp_r path, tmpdirname
      FileUtils.cd tmpdirname, &block
    end
  end

  # Temporary directory
  def tmpdir_name
    tdm = RUBY_PLATFORM =~ /msys|mingw|cygwin|mswin/ ? ENV.fetch("TEMP", nil) : "/tmp"
    File.join(tdm, "tebako-test-#{$PROCESS_ID}-#{rand 2**32}").tr("\\", "/")
  end

  # Creates temporary dir, copies files to it, cleans everything when the business is finished
  def with_tmpdir(files = [], path = nil)
    tempdirname = path || tmpdir_name
    FileUtils.mkdir_p tempdirname
    begin
      FileUtils.cp files, tempdirname
      yield(tempdirname)
    ensure
      # puts "(Not) Cleaning up #{tempdirname}"
      FileUtils.rm_rf tempdirname
    end
  end

  # Create a pristine environment to test built executables. Files are
  # copied and the PATH environment is set to the minimal.
  # yeilds the name for pristine temp dir (as opposed to temp dir used for packaging)
  def pristine_env(*files)
    with_tmpdir files do |tempdirname|
      # [TODO] need equivalent of pristine environment for Windows case
      # (considering test_101_launcher which needs ldd from mingw binutil)
      if RUBY_PLATFORM =~ /msys|mingw|cygwin|mswin/
        yield(tempdirname)
      else
        with_env "PATH" => "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:" do
          yield(tempdirname)
        end
      end
    end
  end

  def ruby_ver
    ENV.fetch("RUBY_VER", "3.1.5")
  end

  # Run 'tebako press ...'
  def press(tebako, name, package, prefix)
    cmd = "ruby #{tebako} press -R #{ruby_ver} -o #{package} -e #{name}.rb -r #{name} -p '#{prefix}'"
    out, st = Open3.capture2e(cmd)
    if st.exitstatus != 0
      puts "\"cmd\" failed with status #{st.exitstatus}"
      puts out
    end
    assert_equal 0, st.exitstatus
    package += ".exe" if RUBY_PLATFORM =~ /msys|mingw|cygwin|mswin/
    assert File.exist?(package)
    package
  end

  # A kind of standart creates names - tmp dir with fixture - press sequence
  def with_fixture_press_and_env(name)
    package = "#{name}-package"
    with_fixture name do
      pkg_file = press(Tebako, name, package, Prefix)
      pristine_env pkg_file do |tempdirname|
        yield "#{tempdirname}/#{pkg_file}"
      end
    end
  end

  # io/wait library extension shall work as expected
  def test_311_io_wait
    name = "lib-io-wait"
    print "\n#{name} "
    with_fixture_press_and_env name do |package|
      out, st = Open3.capture2(package)
      assert_equal 0, st.exitstatus
      assert_match(%r{Received: Hello from io/wait writer!}, out)
    end
  end

  # Specified gems should be usable in packaged app
  # def test_217_psych5
  #  name = "gems-psych-5"
  #  with_fixture_press_and_env name do |package|
  #    out, st = Open3.capture2(package)
  #    assert_equal 0, st.exitstatus
  #    assert_equal out, "Hello! Psych welcomes you to the magic world of ruby gems.\n"
  #    check_libs(package.to_s)
  #  end
  # end

  # byebug gem should be usable in packaged app
  def test_216_byebug
    name = "gems-byebug"
    print "\n#{name} "
    with_fixture_press_and_env name do |package|
      out, st = Open3.capture2(package)
      assert_equal 0, st.exitstatus
      assert_match(/Hello! Byebug welcomes you to the magic world of ruby gems./, out)
    end
  end

  # expressir gem should be automatically included and usable in packaged app
  def test_215_expressir
    print "\n#{name = "gems-expressir"} "
    if RUBY_PLATFORM =~ /msys|mingw|cygwin|mswin/
      print "Skipping expressir test on Windows"
      return
    end
    with_fixture_press_and_env name do |package|
      out, st = Open3.capture2(package)
      assert_equal 0, st.exitstatus
      assert_match(/Hello! Expressir gem welcomes you to the magic world of ruby gems./, out)
    end
  end

  # sassc gem should be automatically included and usable in packaged app
  def test_214_sassc
    print "\n#{name = "gems-sassc"} "
    if RUBY_PLATFORM =~ /msys|mingw|cygwin|mswin/
      print "Skipping sassc test on Windows"
      return
    end
    with_fixture_press_and_env name do |package|
      out, st = Open3.capture2(package)
      assert_equal 0, st.exitstatus
      assert_match(/Hello! SassC gem welcomes you to the magic world of ruby gems./, out)
    end
  end

  # libmspack gem should be automatically included and usable in packaged app
  def test_213_libmspack
    print "\n#{name = "gems-libmspack"} "
    if RUBY_PLATFORM =~ /msys|mingw|cygwin|mswin/
      print "Skipping libmspack test on Windows"
      return
    end
    with_fixture_press_and_env name do |package|
      out, st = Open3.capture2(package)
      assert_equal 0, st.exitstatus
      assert_match(/Hello! libmspack welcomes you to the magic world of ruby gems./, out)
    end
  end

  # seven_zip gem should be automatically included and usable in packaged app
  def test_212_seven_zip
    print "\n#{name = "gems-seven-zip"} "
    if RUBY_PLATFORM =~ /msys|mingw|cygwin|mswin/
      print "Skipping libmspack test on Windows"
      return
    end
    with_fixture_press_and_env name do |package|
      out, st = Open3.capture2(package)
      assert_equal 0, st.exitstatus
      assert_match(/Hello! SevenZipRuby welcomes you to the magic world of ruby gems./, out)
    end
  end

  # bundler gem should be automatically included and usable in packaged app
  def test_211_bundler
    name = "gems-bundler"
    print "\n#{name} "
    with_fixture_press_and_env name do |package|
      out, st = Open3.capture2(package)
      assert_equal 0, st.exitstatus
      assert_match(/Hello! Bundler welcomes you to the magic world of ruby gems./, out)
    end
  end

  # Test io.c and file.c patching
  def test_122_io_and_file
    name = "patches-io-and-file"
    print "\n#{name} "
    with_fixture_press_and_env name do |package|
      _, st = Open3.capture2(package)
      assert_equal 0, st.exitstatus
    end
  end

  # Test dir.c patching
  def test_122_dir
    name = "patches-dir"
    print "\n#{name} "
    FileUtils.mkdir_p File.join(FixturePath, name, "level-1/level-2/level-3")
    with_fixture_press_and_env name do |package|
      _, st = Open3.capture2(package)
      assert_equal 0, st.exitstatus
    end
  end

  # Test main.c patching as relates to scrip argument handling
  # Test that scripts can exit with a specific exit status code
  def test_121_main
    name = "patches-main"
    print "\n#{name} "
    with_fixture_press_and_env name do |package|
      _, st = Open3.capture2("#{package} foo \"bar baz \\\"quote\\\"\"")
      assert_equal 5, st.exitstatus
    end
  end

  # Test:
  #  -- that executable can write a file to the current working directory (io.c, file.c patching)
  #  -- short options without whitespaces
  def test_105_launcher_pwd
    name = "launcher-pwd"
    print "\n#{name} "
    with_fixture_press_and_env name do |package|
      _, st = Open3.capture2(package.to_s)
      assert_equal 0, st.exitstatus
      assert File.exist?("output.txt")
      assert_equal "output", File.read("output.txt")
    end
  end

  # Test that executable can use ruby standard libraries (i.e. cgi)
  def test_104_launcher_coreincl
    name = "launcher-coreincl"
    print "\n#{name} "
    with_fixture_press_and_env name do |package|
      _, st = Open3.capture2(package)
      assert_equal 0, st.exitstatus
      assert File.exist?("output.txt")
      assert_equal "3 &lt; 5", File.read("output.txt")
    end
  end

  # Test that the standard output from a script can be redirected to a file.
  def test_103_launcher_stdoutredir
    name = "launcher-stdoutredir"
    print "\n#{name} "
    with_fixture_press_and_env name do |package|
      system("#{package} > output.txt")
      assert File.exist?("output.txt")
      o = File.read("output.txt")
      assert ["Hello, World!\n", "Hello, World!\r\n"].include?(o)
    end
  end

  # Test that the standard input to a script can be redirected from a file.
  def test_102_launcher_stdinredir
    name = "launcher-stdinredir"
    print "\n#{name} "
    package = "#{name}-package"
    with_fixture name do
      pkg_file = press(Tebako, name, package, Prefix)
      pristine_env pkg_file, "#{name}/input.txt" do |tempdirname|
        _, st = Open3.capture2("#{tempdirname}/#{pkg_file} < #{tempdirname}/input.txt")
        assert_equal 104, st.exitstatus
      end
    end
  end

  # Test :
  # -- that we can build and run executables.
  # -- short options with whitespaces
  # -- that we are linking to known set of shared libraries (https://github.com/tamatebako/tebako/issues/42)

  def expected_libs(package) # rubocop:disable Metrics/MethodLength
    case RbConfig::CONFIG["target_os"]
    when /darwin/
      ["Security.framework", "Foundation.framework", "CoreFoundation.framework", "libSystem",
       "libc++", "libc++abi", "#{package}:"]
    # This is the test program itself: for example, 'launcher-package-package:'
    when /linux-musl/
      ["libc.musl-x86_64.so", "ld-musl-x86_64.so"]
    when /msys|mingw|cygwin|mswin/
      ["ntdll.dll", "kernel32.dll", "kernelbase.dll", "advapi32.dll", "msvcrt.dll",
       "sechost.dll", "rpcrt4.dll", "shlwapi.dll", "user32.dll", "win32u.dll", "gdi32.dll",
       "gdi32full.dll", "msvcp_win.dll", "ucrtbase.dll", "ws2_32.dll", "wsock32.dll",
       "shell32.dll", "crypt32.dll", "bcrypt.dll", "imagehlp.dll", "ole32.dll", "oleaut32.dll",
       "iphlpapi.dll", "hlpapi.dll", "combase.dll"]
    else # linux-gnu assumed
      ["linux-vdso.so", "libpthread.so", "libdl.so", "libc.so", "ld-linux-", "libm.so", "librt.so"]
    end
  end

  def actual_libs(package)
    out, st = if RbConfig::CONFIG["host_os"] =~ /darwin/
                Open3.capture2("otool", "-L", package)
              else # linux assumed
                Open3.capture2("ldd", package)
              end
    assert_equal 0, st.exitstatus
    out.lines.map(&:strip)
  end

  def check_libs(package)
    l = actual_libs(package.to_s)
    l.delete_if { |ln| expected_libs(package).any? { |lib| ln.downcase.include?(lib.downcase) } }
    assert_equal 0, l.size, "Unexpected references to shared libraries #{l}"
  end

  def test_101_launcher
    name = "launcher-package"
    print "\n#{name} "
    with_fixture_press_and_env name do |package|
      out, st = Open3.capture2(package)
      assert_equal 0, st.exitstatus
      assert_equal out, "Hello, World!\n"
      check_libs(package.to_s)
    end
  end
end
# rubocop:enable Metrics/ClassLength
