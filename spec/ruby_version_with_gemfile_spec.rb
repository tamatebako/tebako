# frozen_string_literal: true

# Copyright (c) 2025 [Ribose Inc](https://www.ribose.com).
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

require "tebako/error"
require "tebako/ruby_version"

# rubocop:disable Metrics/BlockLength

RSpec.describe Tebako::RubyVersionWithGemfile do
  let(:valid_gemfile_path) { "spec/fixtures/Gemfile" }
  let(:ruby_version) { "3.2.6" }

  before do
    # Create test Gemfile
    FileUtils.mkdir_p("spec/fixtures")
  end

  after do
    FileUtils.rm_rf("spec/fixtures")
  end

  describe "#initialize" do
    context "with valid parameters" do
      it "initializes successfully with Ruby version and Gemfile path" do
        File.write(valid_gemfile_path, "source 'https://rubygems.org'")
        expect do
          described_class.new(ruby_version, valid_gemfile_path)
        end.not_to raise_error
      end

      it "uses default Ruby version when nil is provided" do
        File.write(valid_gemfile_path, "source 'https://rubygems.org'")
        instance = described_class.new(nil, valid_gemfile_path)
        expect(instance.ruby_version).to eq(Tebako::RubyVersion::DEFAULT_RUBY_VERSION)
      end
    end

    context "with invalid parameters" do
      it "raises error for non-existent Gemfile" do
        expect do
          described_class.new(ruby_version, "non_existent_gemfile")
        end.to raise_error(Tebako::Error)
      end
    end
  end

  describe "Ruby version processing" do
    context "with Ruby version in Gemfile" do
      it "accepts matching Ruby version" do
        File.write(valid_gemfile_path, <<~GEMFILE)
          source 'https://rubygems.org'
          ruby '3.2.6'
        GEMFILE

        expect do
          described_class.new("3.2.6", valid_gemfile_path)
        end.not_to raise_error
      end

      it "raises error for version conflict" do
        File.write(valid_gemfile_path, <<~GEMFILE)
          source 'https://rubygems.org'
          ruby '3.2.6'
        GEMFILE

        expect do
          described_class.new("3.1.0", valid_gemfile_path)
        end.to raise_error(Tebako::Error)
      end

      it "handles version requirements with operators" do
        File.write(valid_gemfile_path, <<~GEMFILE)
          source 'https://rubygems.org'
          ruby '~> 3.2.0'
        GEMFILE

        expect do
          described_class.new("3.2.6", valid_gemfile_path)
        end.not_to raise_error
      end
    end

    context "without Ruby version in Gemfile" do
      it "uses provided Ruby version" do
        File.write(valid_gemfile_path, "source 'https://rubygems.org'")
        instance = described_class.new(ruby_version, valid_gemfile_path)
        expect(instance.ruby_version).to eq(ruby_version)
      end
    end
    context "when ruby_version is nil" do
      context "with explicit ruby version in Gemfile" do
        it "uses minimum compatible version for ~> requirement" do
          File.write(valid_gemfile_path, <<~GEMFILE)
            source 'https://rubygems.org'
            ruby '~> 3.2.0'
          GEMFILE

          instance = described_class.new(nil, valid_gemfile_path)
          expect(instance.ruby_version).to eq("3.2.4")
        end

        it "uses exact version for = requirement" do
          File.write(valid_gemfile_path, <<~GEMFILE)
            source 'https://rubygems.org'
            ruby '= 3.2.6'
          GEMFILE

          instance = described_class.new(nil, valid_gemfile_path)
          expect(instance.ruby_version).to eq("3.2.6")
        end

        it "raises error for unsatisfiable version requirement" do
          File.write(valid_gemfile_path, <<~GEMFILE)
            source 'https://rubygems.org'
            ruby '>= 9.9.9'
          GEMFILE

          expect do
            described_class.new(nil, valid_gemfile_path)
          end.to raise_error(Tebako::Error, /No available Ruby version satisfies requirement/)
        end
      end
    end
  end

  describe "#extend_ruby_version" do
    before do
      File.write(valid_gemfile_path, "source 'https://rubygems.org'")
    end

    it "returns array with version and hash" do
      instance = described_class.new(ruby_version, valid_gemfile_path)
      version, hash = instance.extend_ruby_version
      expect(version).to eq(ruby_version)
      expect(hash).to eq(Tebako::RubyVersion::RUBY_VERSIONS[ruby_version])
    end
  end
end

# rubocop:enable Metrics/BlockLength
