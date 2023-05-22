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
require 'open3'

require_relative 'pass1_patch_map'

# Tebako errors
class TebakoError < StandardError
  def initialize(msg = 'Unspecified error', code = 255)
    @error_code = code
    super(msg)
  end
  attr_accessor :error_code
end

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

FILES_TO_RESTORE = [
  'main.c',     'dir.c',     'dln.c',
  'file.c',     'io.c',      'tool/mkconfig.rb'
].freeze

FILES_TO_RESTORE_MSYS = [
  'ruby.c',        'win32/win32.c',
  'win32/file.c',  'win32/dir.h'
].freeze

def pass1(ostype, ruby_source_dir, mount_point, src_dir)
  puts '-- Running pass1 script'

  recreate(src_dir)
  do_patch(get_pass1_patch_map(mount_point), ruby_source_dir)

  # Roll back pass2 patches
  # Just in case we are recovering after some error
  FILES_TO_RESTORE.each do |fname|
    restore_and_save "#{ruby_source_dir}/#{fname}"
  end

  return unless ostype == /msys/

  FILES_TO_RESTORE_MSYS.each do |fname|
    restore_and_save "#{ruby_source_dir}/#{fname}"
  end
end

def get_prefix(package)
  out, st = capture2("brew --prefix #{package}")
  raise TebakoError, "brew --prefix #{package} failed with code #{st.exitstatus}" unless st.exitstatus.zero?

  out
end

def patch_c_file(pattern)
  c_file_subst = <<~SUBST
    /* -- Start of tebako patch -- */
    #ifndef NO_TEBAKO_INCLUDES
    #include <tebako/tebako-config.h>
    #include <tebako/tebako-defines.h>
    #include <tebako/tebako-io-rb-w32.h>
    #include <tebako/tebako-io.h>
    #endif
    /* -- End of tebako patch -- */
  SUBST

  {
    pattern => "#{c_file_subst}\n#{pattern}"
  }
end

LINUX_GNU_LIBS = <<~SUBST
  # -- Start of tebako patch --
  MAINLIBS = -l:libtebako-fs.a -l:libdwarfs-wr.a -l:libdwarfs.a -l:libfolly.a -l:libfsst.a -l:libmetadata_thrift.a -l:libthrift_light.a -l:libxxhash.a \
  -l:libfmt.a -l:libdouble-conversion.a -l:libglog.a -l:libgflags.a -l:libevent.a -l:libiberty.a -l:libacl.a -l:libssl.a -l:libcrypto.a -l:liblz4.a -l:libz.a \
  -l:libzstd.a -l:libgdbm.a -l:libreadline.a -l:libtinfo.a -l:libffi.a -l:libncurses.a -l:libjemalloc.a -l:libunwind.a -l:libcrypt.a -l:libanl.a -l:liblzma.a \
  -l:libboost_system.a -l:libstdc++.a -l:librt.a -ldl
  # -- End of tebako patch --
SUBST

LINUX_MUSL_LIBS = <<~SUBST
  # -- Start of tebako patch --
  MAINLIBS = -l:libtebako-fs.a -l:libdwarfs-wr.a -l:libdwarfs.a -l:libfolly.a -l:libfsst.a -l:libmetadata_thrift.a -l:libthrift_light.a -l:libxxhash.a \
  -l:libfmt.a -l:libdouble-conversion.a -l:libglog.a -l:libgflags.a -l:libevent.a -l:libiberty.a -l:libacl.a -l:libssl.a -l:libcrypto.a -l:liblz4.a -l:libz.a \
  -l:libzstd.a -l:libgdbm.a -l:libreadline.a -l:libffi.a -l:libncurses.a -l:libjemalloc.a -l:libunwind.a -l:libcrypt.a -l:liblzma.a \
  -l:libboost_system.a -l:libstdc++.a -l:librt.a -ldl
  # -- End of tebako patch --
SUBST

MSYS_LIBS = <<~SUBST
  # -- Start of tebako patch --
  MAINLIBS = -l:libtebako-fs.a -l:libdwarfs-wr.a -l:libdwarfs.a -l:libfolly.a -l:libfsst.a -l:libmetadata_thrift.a -l:libthrift_light.a -l:libxxhash.a \
  -l:libfmt.a -l:libdouble-conversion.a -l:libglog.a -l:libgflags.a -l:libevent.a -l:libssl.a -l:libcrypto.a -l:liblz4.a -l:libz.a \
  -l:libzstd.a -l:libffi.a -l:libgdbm.a -l:libncurses.a -l:libjemalloc.a -l:libunwind.a -l:liblzma.a -l:libiberty.a \
  -l:libstdc++.a -l:libdl.a -lole32 -loleaut32 -luuid
  # -- End of tebako patch --
SUBST

def darwin_libs(deps_lib_dir)
  p_libssl = "#{get_prefix('openssl@1.1')}/lib/libssl.a"
  p_libcrypto = "#{get_prefix('openssl@1.1')}/lib/libcrypto.a"
  p_libz = "#{get_prefix('zlib')}/lib/libz.a"
  p_libgdbm = "#{get_prefix('gdbm')}/lib/libgdbm.a"
  p_libreadline = "#{get_prefix('readline')}/lib/libreadline.a"
  p_libffi = "#{get_prefix('libffi')}/lib/libffi.a"
  p_libncurses = "#{get_prefix('ncurses')}/lib/libncurses.a"
  p_libfmt = "#{get_prefix('fmt')}/lib/libfmt.a"
  p_liblz4 = "#{get_prefix('lz4')}/lib/liblz4.a"
  p_liblzma = "#{get_prefix('xz')}/lib/liblzma.a"
  p_libdc = "#{get_prefix('double-conversion')}/lib/libdouble-conversion.a"
  p_glog = "#{deps_lib_dir}/libglog.a"
  p_gflags = "#{deps_lib_dir}/libgflags.a"
  <<~SUBST
    # -- Start of tebako patch --
    MAINLIBS = -ltebako-fs -ldwarfs-wr -ldwarfs -lfolly -lfsst -lmetadata_thrift -lthrift_light -lxxhash \
    -lzstd #{p_glog} #{p_gflags} #{p_libfmt} #{p_liblz4} #{p_liblzma} #{p_libdc} #{p_libssl} #{p_libcrypto} #{p_libz} #{p_libgdbm} \
    #{p_libreadline} #{p_libffi} #{p_libncurses} -ljemalloc -lc++
    # -- End of tebako patch --
  SUBST
end

def pass2(ostype, ruby_source_dir, _mount_point, deps_lib_dir)
  puts '-- Running pass2 script'

  case ostype
  when /linux-gnu/
    m_libs = LINUX_GNU_LIBS
  when /linux-musl/
    m_libs = LINUX_MUSL_LIBS
  when /darwin/
    m_libs = darwin_libs(deps_lib_dir)
  when /msys/
    m_libs = MSYS_LIBS
  else
    raise TebakoError, "Unknown ostype #{ostype}"
  end

  pass2_patch_map = {
    'template/Makefile.in' => {
      'MAINLIBS = @MAINLIBS@' => m_libs,

      'LIBS = @LIBS@ $(EXTLIBS)' =>
        "# -- Start of tebako patch --\n" \
        "LIBS = \$(MAINLIBS) @LIBS@\n" \
        "# -- End of tebako patch --\n",

      "\t\t$(Q) $(PURIFY) $(CC) $(LDFLAGS) $(XLDFLAGS) $(MAINOBJ) $(EXTOBJS) $(LIBRUBYARG) $(MAINLIBS) $(LIBS) $(EXTLIBS) $(OUTFLAG)$@" =>
        "# -- Start of tebako patch --\n" \
        "\t\t$(Q) $(PURIFY) $(CC) $(LDFLAGS) $(XLDFLAGS) $(MAINOBJ) $(EXTOBJS) $(LIBRUBYARG_STATIC) $(LIBS) $(OUTFLAG)$@\n" \
        '# -- End of tebako patch --',
    },
    'main.c' => {
      "int\nmain(int argc, char **argv)" =>
      "#include <tebako-main.h>\n\nint\nmain(int argc, char **argv)",

      '    ruby_sysinit(&argc, &argv);' =>
      "    ruby_sysinit(&argc, &argv);\n" \
      "\/* -- Start of tebako patch -- *\/\n" \
      "    if (tebako_main(&argc, &argv) != 0) {\n" \
      "      return -1;\n" \
      "    }\n" \
      "\/* -- End of tebako patch -- *\/",
    },
    'tool/mkconfig.rb' => {
      '    if fast[name]' =>
      "# -- Start of tebako patch --\n" \
      "    v_head_comp = \"  CONFIG[\\\"prefix\\\"] \#{eq} \"\n" \
      "    if v_head_comp == v[0...(v_head_comp.length)]\n" \
      "      if win32\n" \
      "       v = \"\#{v[0...(v_head_comp.length)]}CONFIG[\\\"RUBY_EXEC_PREFIX\\\"] = '/__tebako_memfs__'\n\"\n" \
      "      else\n" \
      "        v = \"\#{v[0...(v_head_comp.length)]}'/__tebako_memfs__'\n\"\n" \
      "      end\n" \
      "    end\n" \
      "    v_head_comp = \"  CONFIG[\\\"RUBY_EXEC_PREFIX\\\"] \#{eq} \"\n" \
      "    if v_head_comp == v[0...(v_head_comp.length)]\n" \
      "      v = \"\#{v[0...(v_head_comp.length)]}'/__tebako_memfs__'\n\"\n" \
      "    end\n" \
      "# -- End of tebako patch --\n" \
      '    if fast[name]',
    }
  }

  # Compensate ruby incorrect processing of (f)getattrlist returning ENOTSUP
  # Note. We are not patching need_normalization function
  # In this function (f)getattrlist failure with ENOTSUP is processed correctly
  dir_c_patch = {
    '#if defined HAVE_GETATTRLIST && defined ATTR_DIR_ENTRYCOUNT' =>
    "#if defined HAVE_GETATTRLIST && defined ATTR_DIR_ENTRYCOUNT\n    \/* tebako patch *\/ if (!within_tebako_memfs(path))",

    "#if USE_NAME_ON_FS == USE_NAME_ON_FS_REAL_BASENAME\n	    plain = 1;" =>
      "#if USE_NAME_ON_FS == USE_NAME_ON_FS_REAL_BASENAME\n	    \/* tebako patch *\/ if (!within_tebako_memfs(path)) plain = 1; else magical = 1;",

    'if (is_case_sensitive(dirp, path) == 0)' =>
      "if (is_case_sensitive(dirp, path) == 0 \/* tebako patch *\/ && !within_tebako_memfs(path))",

    'if ((*cur)->type == ALPHA) {' =>
      "if ((*cur)->type == ALPHA \/* tebako patch *\/ && !within_tebako_memfs(buf)) {",

    'else if (e == EIO) {' =>
      "else if (e == EIO \/* tebako patch *\/ && !within_tebako_memfs(path)) {"
  }

  dir_c_patch.merge!(patch_c_file(ostype == /msys/ ? "\/* define system APIs *\/" : '#ifdef HAVE_GETATTRLIST'))

  pass2_patch_map.store('dln.c',
                        patch_c_file('static const char funcname_prefix[sizeof(FUNCNAME_PREFIX) - 1] = FUNCNAME_PREFIX;'))
  pass2_patch_map.store('file.c', patch_c_file('/* define system APIs */'))
  pass2_patch_map.store('io.c', patch_c_file('/* define system APIs */'))
  pass2_patch_map.store('util.c', patch_c_file('#ifndef S_ISDIR'))
  pass2_patch_map.store('dir.c', dir_c_patch)

  do_patch(pass2_patch_map, ruby_source_dir)
end

def stash(ruby_source_dir, src_dir, stash_dir)
  puts '-- Running stash script'
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

def deploy(stash_dir, src_dir, pre_dir, bin_dir)
  puts '-- Running deploy script'

  puts "   ... creating packaging environment at #{src_dir}"
  recreate([src_dir, pre_dir, bin_dir])
  FileUtils.cp_r "#{stash_dir}/.", src_dir
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

    pass1(ARGV[1], ARGV[2], ARGV[3], ARGV[4])
  when 'pass2'
    #       ARGV[0] -- command
    #       ARGV[1] -- OSTYPE
    #       ARGV[2] -- RUBY_SOURCE_DIR
    #       ARGV[3] -- FS_MOUNT_POINT
    #       ARGV[4] -- DEPS_LIB_DIR
    raise TebakoError, "pass1 script expects 5 arguments, #{ARGV.length} has been provided." unless ARGV.length == 5

    pass2(ARGV[1], ARGV[2], ARGV[3], ARGV[4])
  when 'stash'
    #       ARGV[0] -- command
    #       ARGV[1] -- RUBY_SOURCE_DIR
    #       ARGV[2] -- DATA_SRC_DIR
    #       ARGV[3] -- FS_STASH_DIR
    raise TebakoError, "stash script expects 4 arguments, #{ARGV.length} has been provided." unless ARGV.length == 4

    stash(ARGV[1], ARGV[2], ARGV[3])
  when 'deploy'
    #       ARGV[0] -- command
    #       ARGV[1] -- FS_STASH_DIR
    #       ARGV[2] -- DATA_SRC_DIR
    #       ARGV[3] -- DATA_PRE_DIR
    #       ARGV[4] -- DATA_BIN_DIR
    raise TebakoError, "deploy script expects 5 arguments, #{ARGV.length} has been provided." unless ARGV.length == 5

    deploy(ARGV[1], ARGV[2], ARGV[3], ARGV[4])
  else
    raise TebakoError, "Tebako script cannot process #{ARGV[0]} command"
  end
rescue TebakoError => e
  puts "Tebako script failed: #{e.message} [#{e.error_code}]"
  exit(e.error_code)
end

exit(0)
