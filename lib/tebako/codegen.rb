# frozen_string_literal: true

# Copyright (c) 2024 [Ribose Inc](https://www.ribose.com).
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

require "pathname"
require "fileutils"

require_relative "package_descriptor"

# Tebako - an executable packager
module Tebako
  # Code geberation
  module Codegen
    COMMON_C_HEADER = <<~SUBST
      /**
      * THIS FILE WAS GENERATED AUTOMATICALLY BY TEBAKO. DO NOT CHANGE IT, PLEASE
      **/

    SUBST

    COMMON_RUBY_HEADER = <<~SUBST
      #!/usr/bin/env ruby
      # frozen_string_literal: true

      # THIS FILE WAS GENERATED AUTOMATICALLY BY TEBAKO. DO NOT CHANGE IT, PLEASE

    SUBST

    class << self # rubocop:disable Metrics/ClassLength
      def deploy_crt_implib(opt, scm)
        crt = ""
        if scm.msys?
          crt = <<~SUBST
            Tebako::Packager.create_implib("#{opt.ruby_src_dir}", "#{opt.data_src_dir}",
                                           "#{opt.package}", rv)
          SUBST
        end
        crt
      end

      def deploy_mk(opt, scm)
        case opt.mode
        when "bundle"
          deploy_mk_bundle(opt, scm)
        when /runtime|both/
          deploy_mk_stub(opt)
        end
      end

      def deploy_mk_bundle(opt, scm)
        <<~SUBST
          Tebako::Packager.deploy("#{opt.data_src_dir}", "#{opt.data_pre_dir}",
                                  rv , "#{opt.root}", "#{scm.fs_entrance}", "#{opt.cwd}")
          Tebako::Packager.mkdwarfs("#{opt.deps_bin_dir}", "#{opt.data_bundle_file}",
                                    "#{opt.data_src_dir}")
        SUBST
      end

      def deploy_mk_stub(opt)
        <<~SUBST
          Tebako::Packager.deploy("#{opt.data_src_dir}", "#{opt.data_pre_dir}",
                                  rv, "#{File.join(opt.deps, "src", "tebako", "local")}", "stub.rb", nil)
          Tebako::Packager.mkdwarfs("#{opt.deps_bin_dir}", "#{opt.data_stub_file}", "#{opt.data_src_dir}")
        SUBST
      end

      def deploy_rb(opt, scm)
        <<~SUBST
          #{deploy_rq}

          begin
            #{deploy_rb_inner(opt, scm)}
          rescue Tebako::Error => e
            puts "deploy script failed: \#{e.message} [\#{e.error_code}]"
            exit(e.error_code)
          end
        SUBST
      end

      def deploy_rb_inner(opt, scm)
        <<~SUBST
          rv = Tebako::RubyVersion.new(ARGV[0])
          stash = File.join("#{opt.deps}", "stash_\#{ARGV[0]}")
          Tebako::Packager::init(stash.to_s, "#{opt.data_src_dir}",
                               "#{opt.data_pre_dir}", "#{opt.data_bin_dir}")
          #{deploy_crt_implib(opt, scm)}
          #{deploy_mk(opt, scm)}
        SUBST
      end

      def deploy_rq
        <<~SUBST
          require "#{File.join(__dir__, "error.rb")}"
          require "#{File.join(__dir__, "package_descriptor.rb")}"
          require "#{File.join(__dir__, "packager.rb")}"
          require "#{File.join(__dir__, "ruby_version.rb")}"
        SUBST
      end

      def stub_rb(opt)
        <<~SUBST
          puts "Copyright (c) 2024-2025 Ribose Inc (https://www.ribose.com)"
          puts "Tebako runtime stub v#{Tebako::VERSION}"
          puts "To run your application please call #{File.basename(opt.package)} --tebako-run <your tebako package>"
        SUBST
      end

      def generate_stub_rb(options_manager)
        puts "   ... stub.rb"

        fname = File.join(options_manager.deps, "src", "tebako", "local", "stub.rb")
        FileUtils.mkdir_p(File.dirname(fname))

        File.open(fname, "w") do |file|
          file.write(COMMON_RUBY_HEADER)
          file.write(stub_rb(options_manager))
        end
      end

      def generate_deploy_rb(options_manager, scenario_manager)
        puts "   ... deploy.rb"

        fname = File.join(options_manager.deps, "bin", "deploy.rb")
        FileUtils.mkdir_p(File.dirname(fname))

        File.open(fname, "w") do |file|
          file.write(COMMON_RUBY_HEADER)
          file.write(deploy_rb(options_manager, scenario_manager))
        end
      end

      def generate_package_descriptor(options_manager, scenario_manager)
        puts "   ... package_descriptor"
        fname = File.join(options_manager.deps, "src", "tebako", "package_descriptor")
        FileUtils.mkdir_p(File.dirname(fname))
        descriptor = Tebako::PackageDescriptor.new(options_manager.ruby_ver, Tebako::VERSION,
                                                   scenario_manager.fs_mount_point, scenario_manager.fs_entry_point,
                                                   options_manager.cwd)
        File.binwrite(fname, descriptor.serialize)
        fname
      end

      def generate_tebako_fs_cpp(options_manager, scenario_manager)
        puts "   ... tebako-fs.cpp"

        fname = File.join(options_manager.deps, "src", "tebako", "tebako-fs.cpp")
        FileUtils.mkdir_p(File.dirname(fname))

        File.open(fname, "w") do |file|
          file.write(COMMON_C_HEADER)
          file.write(tebako_fs_cpp(options_manager, scenario_manager))
        end
      end

      def generate_tebako_version_h(options_manager, v_parts)
        puts "   ... tebako-version.h"

        fname = File.join(options_manager.deps, "include", "tebako", "tebako-version.h")
        FileUtils.mkdir_p(File.dirname(fname))

        File.open(fname, "w") do |file|
          file.write(COMMON_C_HEADER)
          file.write(tebako_version_h(v_parts))
        end
      end

      def package_cwd(options_manager, scenario_manager)
        if options_manager.cwd.nil?
          "nullptr"
        else
          "\"#{scenario_manager.fs_mount_point}/#{options_manager.cwd}\""
        end
      end

      def tebako_fs_cpp(options_manager, scenario_manager)
        case options_manager.mode
        when "bundle"
          tebako_fs_cpp_bundle(options_manager, scenario_manager)
        when /runtime|both/
          tebako_fs_cpp_stub(options_manager, scenario_manager)
        end
      end

      def tebako_fs_cpp_bundle(options_manager, scenario_manager)
        <<~SUBST
          #include <limits.h>
          #include <incbin/incbin.h>

          namespace tebako {
            const  char * fs_log_level   = "#{options_manager.l_level}";
            const  char * fs_mount_point = "#{scenario_manager.fs_mount_point}";
            const  char * fs_entry_point = "#{scenario_manager.fs_entry_point}";
            const  char * package_cwd 	 = #{package_cwd(options_manager, scenario_manager)};
            char   original_cwd[PATH_MAX];

            INCBIN(fs, "#{options_manager.data_bundle_file}");
          }
        SUBST
      end

      def tebako_fs_cpp_stub(options_manager, scenario_manager)
        <<~SUBST
          #include <limits.h>
          #include <incbin/incbin.h>

          namespace tebako {
            const  char * fs_log_level   = "#{options_manager.l_level}";
            const  char * fs_mount_point = "#{scenario_manager.fs_mount_point}";
            const  char * fs_entry_point = "/local/stub.rb";
            const  char * package_cwd 	 = nullptr;
            char   original_cwd[PATH_MAX];

            INCBIN(fs, "#{options_manager.data_stub_file}");
          }
        SUBST
      end

      def tebako_version_h(v_parts)
        <<~SUBST
          #pragma once

          const unsigned int tebako_version_major = #{v_parts[0]};
          const unsigned int tebako_version_minor = #{v_parts[1]};
          const unsigned int tebako_version_teeny = #{v_parts[2]};
        SUBST
      end
    end
  end
end
