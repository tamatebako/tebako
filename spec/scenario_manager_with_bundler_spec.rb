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

require "tmpdir"
require_relative "../lib/tebako/scenario_manager"
# rubocop:disable Metrics/BlockLength

RSpec.describe Tebako::ScenarioManagerWithBundler do
  describe "#lookup_files" do
    let(:tmp_dir) { Dir.mktmpdir }
    let(:scenario_manager) { described_class.new(tmp_dir, "dummy_entry.rb") }
    let(:mock_fetcher) { instance_double(Gem::SpecFetcher) }
    let(:mock_tuple) { Gem::NameTuple.new("bundler", Gem::Version.new("2.6.3"), "ruby") }

    before do
      allow(Gem::SpecFetcher).to receive(:fetcher).and_return(mock_fetcher)
      allow(mock_fetcher).to receive(:detect).and_return([[mock_tuple, nil]])
    end

    after do
      FileUtils.remove_entry(tmp_dir)
    end

    context "with Gemfile.lock" do
      before do
        File.write(File.join(tmp_dir, "Gemfile.lock"), <<~LOCKFILE
          BUNDLED WITH
             2.6.3
        LOCKFILE
        )
      end

      it "sets bundler version from lockfile" do
        scenario_manager.send(:lookup_files)
        expect(scenario_manager.instance_variable_get(:@bundler_version)).to eq("2.6.3")
      end
    end

    context "with only Gemfile" do
      before do
        File.write(File.join(tmp_dir, "Gemfile"), <<~GEMFILE
          source 'https://rubygems.org'
          gem 'bundler', '>= 2.6'
        GEMFILE
        )
      end

      it "determines bundler version from Gemfile requirements" do
        scenario_manager.send(:lookup_files)
        expect(scenario_manager.instance_variable_get(:@bundler_version)).to eq("2.6.3")
      end
    end

    context "with invalid version requirements" do
      before do
        File.write(File.join(tmp_dir, "Gemfile"), <<~GEMFILE)
          source 'https://rubygems.org'
          gem 'bundler', '>= 999.0'
        GEMFILE
        allow(mock_fetcher).to receive(:detect).and_return([])
      end

      it "raises error when no compatible version found" do
        expect do
          scenario_manager.send(:lookup_files)
        end.to raise_error(Tebako::Error)
      end
    end
  end

  describe "#store_compatible_bundler_version" do
    let(:tmp_dir) { Dir.mktmpdir }
    let(:scenario_manager) { described_class.new(tmp_dir, "dummy_entry.rb") }
    let(:mock_fetcher) { instance_double(Gem::SpecFetcher) }
    let(:requirement) { Gem::Requirement.new(">= 2.6.0") }

    after do
      FileUtils.remove_entry(tmp_dir)
    end

    before do
      allow(Gem::SpecFetcher).to receive(:fetcher).and_return(mock_fetcher)
    end

    context "when no compatible versions found" do
      before do
        allow(mock_fetcher).to receive(:detect).and_return([])
      end

      it "raises error 119" do
        expect do
          scenario_manager.send(:store_compatible_bundler_version, requirement)
        end.to raise_error(Tebako::Error) { |error|
          expect(error.error_code).to eq(119)
        }
      end
    end

    context "when single compatible version found" do
      let(:mock_tuple) do
        Gem::NameTuple.new("bundler", Gem::Version.new("2.6.3"), "ruby")
      end

      before do
        allow(mock_fetcher).to receive(:detect).and_return([[mock_tuple, nil]])
      end

      it "sets that version" do
        scenario_manager.send(:store_compatible_bundler_version, requirement)
        expect(scenario_manager.instance_variable_get(:@bundler_version)).to eq("2.6.3")
      end
    end

    context "when multiple compatible versions found" do
      let(:mock_tuples) do
        [
          [Gem::NameTuple.new("bundler", Gem::Version.new("2.6.1"), "ruby"), nil],
          [Gem::NameTuple.new("bundler", Gem::Version.new("2.6.3"), "ruby"), nil],
          [Gem::NameTuple.new("bundler", Gem::Version.new("2.6.2"), "ruby"), nil]
        ]
      end

      before do
        allow(mock_fetcher).to receive(:detect).and_return(mock_tuples)
      end

      it "sets the latest compatible version" do
        scenario_manager.send(:store_compatible_bundler_version, requirement)
        expect(scenario_manager.instance_variable_get(:@bundler_version)).to eq("2.6.3")
      end
    end
  end

  describe "#update_bundler_version_from_gemfile" do
    let(:tmp_dir) { Dir.mktmpdir }
    let(:scenario_manager) { described_class.new(tmp_dir, "dummy_entry.rb") }
    let(:gemfile_path) { File.join(tmp_dir, "Gemfile") }

    after do
      FileUtils.remove_entry(tmp_dir)
    end

    context "with bundler requirement in Gemfile" do
      before do
        File.write(gemfile_path, <<~GEMFILE
          source 'https://rubygems.org'
          gem 'bundler', '= 2.6.3'
        GEMFILE
        )
      end

      it "finds compatible bundler version" do
        scenario_manager.send(:update_bundler_version_from_gemfile, gemfile_path)
        expect(scenario_manager.instance_variable_get(:@bundler_version)).to eq("2.6.3")
      end
    end
  end

  describe "#update_bundler_version_from_lockfile" do
    let(:tmp_dir) { Dir.mktmpdir }
    let(:scenario_manager) { described_class.new(tmp_dir, "dummy_entry.rb") }
    let(:lockfile_path) { File.join(tmp_dir, "Gemfile.lock") }

    after do
      FileUtils.remove_entry(tmp_dir)
    end

    context "when lockfile exists with valid bundler version" do
      context "when version satisfies minimum requirement" do
        before do
          File.write(lockfile_path, <<~LOCKFILE)
            BUNDLED WITH
               #{Tebako::BUNDLER_VERSION}
          LOCKFILE
        end

        it "sets bundler version and needs_bundler flag" do
          scenario_manager.send(:update_bundler_version_from_lockfile, lockfile_path)
          expect(scenario_manager.instance_variable_get(:@bundler_version)).to eq(Tebako::BUNDLER_VERSION)
          expect(scenario_manager.instance_variable_get(:@needs_bundler)).to be true
        end
      end

      context "when version is below minimum requirement" do
        before do
          File.write(lockfile_path, <<~LOCKFILE
            BUNDLED WITH
               2.2.0
          LOCKFILE
          )
        end

        it "raises error 118 with version information" do
          expect do
            scenario_manager.send(:update_bundler_version_from_lockfile, lockfile_path)
          end.to raise_error(Tebako::Error) { |error|
            expect(error.error_code).to eq(118)
            expect(error.message).to include("2.2.0 requested")
            expect(error.message).to include("#{Tebako::BUNDLER_VERSION} minimum required")
          }
        end
      end
    end

    context "when lockfile does not exist" do
      it "raises error 117" do
        expect do
          scenario_manager.send(:update_bundler_version_from_lockfile, "nonexistent.lock")
        end.to raise_error(Tebako::Error) { |error|
          expect(error.error_code).to eq(117)
        }
      end
    end

    context "when lockfile has invalid content" do
      before do
        File.write(lockfile_path, "invalid content")
      end

      it "raises error 117" do
        expect do
          scenario_manager.send(:update_bundler_version_from_lockfile, lockfile_path)
        end.to raise_error(Tebako::Error) { |error|
          expect(error.error_code).to eq(117)
        }
      end
    end

    context "when lockfile has malformed BUNDLED WITH section" do
      before do
        File.write(lockfile_path, <<~LOCKFILE
          BUNDLED WITH
             invalid.version.string
        LOCKFILE
        )
      end

      it "raises error 117" do
        expect do
          scenario_manager.send(:update_bundler_version_from_lockfile, lockfile_path)
        end.to raise_error(Tebako::Error) { |error|
          expect(error.error_code).to eq(117)
        }
      end
    end
  end
end

# rubocop:enable Metrics/BlockLength
