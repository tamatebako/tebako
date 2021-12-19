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

require "tmpdir"
require "fileutils"
require "rbconfig"
require "pathname"
require "rubygems"
require "open3"
require "minitest/autorun"

include FileUtils

class TestTebako < MiniTest::Test
  # Path to test fixtures.
    FixturePath = File.expand_path(File.join(File.dirname(__FILE__), 'fixtures'))
    Prefix = File.expand_path(File.join(File.dirname(__FILE__), '..'))
    Tebako = File.join(Prefix, "bin", "tebako")

    def initialize(*args)
        super(*args)
        @testnum = 0
    end

  # Sets up temporary environment variables and yields to the
  # block. When the block exits, the environment variables are set
  # back to their original values.
    def with_env(hash)
        old = {}
        hash.each do |k,v|
            old[k] = ENV[k]
            ENV[k] = v
        end
        begin
            yield
            ensure
            hash.each do |k,v|
                ENV[k] = old[k]
            end
        end
    end

  # Sets up an directory with a copy of a fixture and yields to the
  # block, then cleans up everything. A fixture here is a hierachy of
  # files located in test/fixtures.
    def with_fixture(name, target_path = nil)
        path = File.join(FixturePath, name)
        with_tmpdir([], target_path) do |tmpdirname|
            cp_r path, tmpdirname
            cd tmpdirname do
                yield
            end
        end
    end

  # Creates temporary dir, copies files to it, cleans everything when the business is finished
    def with_tmpdir(files = [], path = nil)
        tempdirname = path || File.join("/tmp" , "tebako-test-#{$$}-#{rand 2**32}").tr('\\','/')
        mkdir_p tempdirname
        begin
            cp files, tempdirname
            yield(tempdirname)
            ensure
            FileUtils.rm_rf tempdirname
        end
     end

  # Create a pristine environment to test built executables. Files are
  # copied and the PATH environment is set to the minimal.
  # yeilds the name for pristine temp dir (as opposed to temp dir used for packaging)
    def pristine_env(*files)
        with_tmpdir files do |tempdirname|
            with_env "PATH" => "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:" do
                yield(tempdirname)
            end
        end
    end

  # A kind of standart creates names - tmp dir with fixture - press sequence
    def with_fixture_press_and_env(name)
        package = "#{name}-package"
        with_fixture name do
            assert system("#{Tebako} press --output=#{package} --entry-point=#{name}.rb --root=#{name} --prefix='#{Prefix}'")
            assert File.exist?(package)
            pristine_env package do |tempdirname|
                yield "#{tempdirname}/#{package}"
            end
        end
    end

  # Specified gems should be automatically included and usable in packaged app
    def test_212_seven_zip_ruby
        name = "gems-seven_zip_ruby"
        with_fixture_press_and_env name do |package|
            out, st = Open3.capture2(package)
            assert_equal 0, st.exitstatus
            assert_equal out, "Hello! SevenZipRuby welcomes you to the magic world of ruby gems.\n"
        end
    end

  # Specified gems should be automatically included and usable in packaged app
    def test_211_bundler
        name = "gems-bundler"
        with_fixture_press_and_env name do |package|
            out, st = Open3.capture2(package)
            assert_equal 0, st.exitstatus
            assert_equal out, "Hello! Bundler welcomes you to the magic world of ruby gems.\n"
        end
    end

  # Test: 
  #  -- that executable can write a file to the current working directory (io.c, file.c patching)
  #  -- short options without whitespaces
  def test_122_io_and_file
        name = "patches-io-and-file"
        package = "#{name}-package"
        with_fixture_press_and_env name do
            with_fixture name do
                assert system("#{Tebako} press -o#{package} -e#{name}.rb -r#{name} -p#{Prefix}")
                assert File.exist?(package)
                pristine_env package do |tempdirname|
                    assert system("#{tempdirname}/#{package}")
                    assert File.exist?("output.txt")
                    assert_equal "output", File.read("output.txt")
                end
            end
        end
    end

  # Test dir.c patching
    def test_122_dir
        name = "patches-dir"
        with_fixture_press_and_env name do |package|
            assert system(package)
        end
    end

  # Test main.c patching as relates to scrip argument handling
  # Test that scripts can exit with a specific exit status code
    def test_121_main
        name = "patches-main"
        with_fixture_press_and_env name do |package|
            out, st = Open3.capture2("#{package} foo \"bar baz \\\"quote\\\"\"")
            assert_equal 5, st.exitstatus
      end
    end

  # Test that executable can use ruby standard libraries (i.e. cgi)
    def test_104_launcher_coreincl
        name = "launcher-coreincl"
        with_fixture_press_and_env name do |package|
            assert system(package)
            assert File.exist?("output.txt")
            assert_equal "3 &lt; 5", File.read("output.txt")
        end
    end

  # Test that the standard output from a script can be redirected to a file.
    def test_103_launcher_stdoutredir
        name = "launcher-stdoutredir"
        with_fixture_press_and_env name do |package|
            system("#{package} > output.txt")
            assert File.exist?("output.txt")
            o = File.read("output.txt")
            assert o == "Hello, World!\n" || o == "Hello, World!\r\n"
        end
    end

  # Test that the standard input to a script can be redirected from a file.
    def test_102_launcher_stdinredir
        name = "launcher-stdinredir"
        package = "#{name}-package"
        with_fixture name do
            assert system("#{Tebako} press -o #{package} -e #{name}.rb -r #{name} -p '#{Prefix}'")
            assert File.exist?(package)
            pristine_env package, "#{name}/input.txt" do |tempdirname|
                out, st = Open3.capture2("#{tempdirname}/#{package} < #{tempdirname}/input.txt")
                assert_equal 104, st.exitstatus
            end
        end
    end

# Test :
  # -- that we can build and run executables.
  # -- short options with whitespaces
  # -- that we are linking to known set of shared libraries (https://github.com/tamatebako/tebako/issues/42)
  def test_101_launcher
    name = "launcher-package"
    package = "#{name}-package"
    with_fixture name do
        assert system("#{Tebako} press -o #{package} -e #{name}.rb -r #{name} -p '#{Prefix}'")
        assert File.exist?(package)
        pristine_env package do |tempdirname|
            out, st = Open3.capture2("#{tempdirname}/#{package}")
            assert_equal 0, st.exitstatus

            out, st = Open3.capture2("ldd #{tempdirname}/#{package}")
            assert_equal 0, st.exitstatus

#   Expecting no more then 7 references to shared libraries
#   linux-vdso.so
#   libpthread.so
#   libdl.so
#   libc.so
#   libm.so
#   librt.so
#   /lib64/ld-linux-x86-64.so

            libs = ["linux-vdso.so", "libpthread.so", "libdl.so", "libc.so", "ld-linux-x86-64.so", "libm.so", "librt.so"]
            l = out.lines.map(&:strip)
            l.delete_if {|ln| libs.any? { |lib| ln.include?(lib) } }
            assert_equal 0, l.size, "Unexpected references to shared libraries #{l}"

        end
    end
end

end
