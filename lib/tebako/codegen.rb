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

    class << self
      def deploy_crt_implib(opt, scm)
        crt = ""
        if scm.msys?
          crt = <<~SUBST
            Tebako::Packager.create_implib("#{opt.ruby_src_dir}", "#{opt.data_src_dir}",
                                           "#{File.basename(opt.package)}", rv)
          SUBST
        end
        crt
      end

      def deploy_cwd(opt)
        opt.cwd.nil? ? "nil" : "\"#{opt.cwd}\""
      end

      def deploy_rb(opt, scm)
        <<~SUBST
          #{deploy_rq}

          rv = Tebako::RubyVersion.new("#{opt.ruby_ver}")
          Tebako::Packager::init("#{opt.stash_dir}", "#{opt.data_src_dir}",
                               "#{opt.data_pre_dir}", "#{opt.data_bin_dir}")
          #{deploy_crt_implib(opt, scm)}
          Tebako::Packager.deploy("#{opt.data_src_dir}", "#{opt.data_pre_dir}",
                                  rv , "#{opt.root}",
                                  "#{scm.fs_entrance}", #{deploy_cwd(opt)})
          Tebako::Packager.mkdwarfs("#{opt.deps_bin_dir}", "#{opt.data_bin_file}",
                                    "#{opt.data_src_dir}")
        SUBST
      end

      def deploy_rq
        <<~SUBST
          require "#{File.join(__dir__, "packager.rb")}"
          require "#{File.join(__dir__, "ruby_version.rb")}"
        SUBST
      end

      def generate_deploy_rb(options_manager, scenario_manager)
        fname = File.join(options_manager.deps, "bin", "deploy.rb")
        FileUtils.mkdir_p(File.dirname(fname))

        File.open(fname, "w") do |file|
          file.write(COMMON_RUBY_HEADER)
          file.write(deploy_rb(options_manager, scenario_manager))
        end
      end

      def generate_tebako_fs_cpp(options_manager, scenario_manager)
        fname = File.join(options_manager.deps, "src", "tebako", "tebako-fs.cpp")
        FileUtils.mkdir_p(File.dirname(fname))

        File.open(fname, "w") do |file|
          file.write(COMMON_C_HEADER)
          file.write(tebako_fs_cpp(options_manager, scenario_manager))
        end
      end

      def generate_tebako_version_h(options_manager, v_parts)
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
        <<~SUBST
          #include <incbin/incbin.h>

          namespace tebako {
            const  char * fs_log_level   = "#{options_manager.l_level}";
            const  char * fs_mount_point = "#{scenario_manager.fs_mount_point}";
            const  char * fs_entry_point = "#{scenario_manager.fs_entry_point}";
            const  char * package_cwd 	 = #{package_cwd(options_manager, scenario_manager)};
            char   original_cwd[PATH_MAX];

            INCBIN(fs, "#{options_manager.output_folder}/p/fs.bin");
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
