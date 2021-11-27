require "minitest/autorun"

require "tmpdir"
require "fileutils"
require "rbconfig"
require "pathname"
require "rubygems"

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
  # copied and the PATH environment is set to the minimal. Yields to
  # the block, then cleans up.
    def pristine_env(*files)
        with_tmpdir files do
            with_env "PATH" => "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:" do
                yield
            end 
        end
    end
 
  # Should be able to call tebako
    def test_smoke
        assert system(Tebako + " --help > /dev/null")
    end

  # Hello world test. Test that we can build and run executables.
    def test_helloworld
        name = "helloworld"
        package = "#{name}-package"
        with_fixture name do               
            assert system("#{Tebako} press -o #{package} -e #{name}.rb -r #{name} -p '#{Prefix}'")
            assert File.exist?(package)
            pristine_env package do
                assert system(package)
            end
        end
    end
  # Test that executable can write a file to the current working directory
    def test_writefile
        name = "writefile"
        package = "#{name}-package"
        with_fixture name do               
            assert system("#{Tebako} press -o #{package} -e #{name}.rb -r #{name} -p '#{Prefix}'")
            pristine_env package do
                assert File.exist?(package)
                assert system(package)
                assert File.exist?("output.txt")
                assert_equal "output", File.read("output.txt")
            end
        end
    end

  # Test that executable can use ruby standard libraries (i.e. cgi)
    def test_rubycoreincl
        name = "rubycoreincl"
        package = "#{name}-package"
        with_fixture name do
            assert system("#{Tebako} press --output=#{package} --entry-point=#{name}.rb --root=#{name} --prefix='#{Prefix}'")
            pristine_env package do
                assert File.exist?(package)
                assert system(package)
                assert File.exist?("output.txt")
                assert_equal "3 &lt; 5", File.read("output.txt")
            end
        end
    end
end



