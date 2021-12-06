require 'rubygems'
require 'bundler/setup'
require 'openssl'
require 'open-uri'
require 'net/https'
require 'tempfile'

COMPILER_MEMFS = '/__tebako_memfs__'

# https://github.com/pmq20/ruby-packer/blob/master/lib/compiler/constants.rb#L10
COMPILER_MEMFS_LIB_CACHE = Pathname.new(Dir.mktmpdir("packed-mn-"))
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

 puts "Hello, it looks like the test has passed"