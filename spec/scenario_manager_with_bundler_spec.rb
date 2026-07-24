# frozen_string_literal: true

# Copyright (c) 2025 [Ribose Inc](https://www.ribose.com).
# All rights reserved.
# This file is a part of the Tebako project.
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
# rubocop:disable Metrics/BlockLength

RSpec.describe Tebako::ScenarioManagerWithBundler do
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

  describe "bundler version resolution from Gemfile.lock" do
    context "with a valid lockfile" do
      before do
        File.write(File.join(tmp_dir, "Gemfile.lock"), <<~LOCKFILE
          BUNDLED WITH
             2.6.3
        LOCKFILE
        )
      end

      it "pins the bundler version from the lockfile" do
        scenario_manager.configure_scenario
        expect(scenario_manager.bundler_version).to eq("2.6.3")
        expect(scenario_manager.needs_bundler).to be true
      end
    end

    context "when the lockfile version is below the minimum requirement" do
      before do
        File.write(File.join(tmp_dir, "Gemfile.lock"), <<~LOCKFILE
          BUNDLED WITH
             2.2.0
        LOCKFILE
        )
      end

      it "raises error 118 with version information" do
        expect { scenario_manager.configure_scenario }.to raise_error(Tebako::Error) { |error|
          expect(error.error_code).to eq(118)
          expect(error.message).to include("2.2.0 requested")
          expect(error.message).to include("#{Tebako::BUNDLER_VERSION} minimum required")
        }
      end
    end

    context "when the lockfile has invalid content" do
      before do
        File.write(File.join(tmp_dir, "Gemfile.lock"), "invalid content")
      end

      it "raises error 117" do
        expect { scenario_manager.configure_scenario }.to raise_error(Tebako::Error) { |error|
          expect(error.error_code).to eq(117)
        }
      end
    end

    context "when the lockfile has a malformed BUNDLED WITH section" do
      before do
        File.write(File.join(tmp_dir, "Gemfile.lock"), <<~LOCKFILE
          BUNDLED WITH
             invalid.version.string
        LOCKFILE
        )
      end

      it "raises error 117" do
        expect { scenario_manager.configure_scenario }.to raise_error(Tebako::Error) { |error|
          expect(error.error_code).to eq(117)
        }
      end
    end

    context "when the lockfile disappears after being detected" do
      let(:lockfile_path) { File.join(File.realpath(tmp_dir), "Gemfile.lock") }

      before do
        File.write(lockfile_path, <<~LOCKFILE
          BUNDLED WITH
             2.6.3
        LOCKFILE
        )
        allow(File).to receive(:exist?).and_call_original
        allow(File).to receive(:exist?).with(lockfile_path).and_return(true, false)
      end

      it "raises error 117" do
        expect { scenario_manager.configure_scenario }.to raise_error(Tebako::Error) { |error|
          expect(error.error_code).to eq(117)
        }
      end
    end
  end

  describe "bundler version resolution from Gemfile" do
    context "with a compatible bundler requirement" do
      before do
        File.write(File.join(tmp_dir, "Gemfile"), <<~GEMFILE
          source 'https://rubygems.org'
          gem 'bundler', '>= 2.6'
        GEMFILE
        )
      end

      it "resolves the latest compatible bundler version" do
        scenario_manager.configure_scenario
        expect(scenario_manager.bundler_version).to eq("2.6.3")
        expect(scenario_manager.needs_bundler).to be true
      end

      context "when multiple compatible versions are released" do
        before do
          allow(mock_fetcher).to receive(:detect).and_return(
            [
              [Gem::NameTuple.new("bundler", Gem::Version.new("2.6.1"), "ruby"), nil],
              [Gem::NameTuple.new("bundler", Gem::Version.new("2.6.3"), "ruby"), nil],
              [Gem::NameTuple.new("bundler", Gem::Version.new("2.6.2"), "ruby"), nil]
            ]
          )
        end

        it "picks the latest one" do
          scenario_manager.configure_scenario
          expect(scenario_manager.bundler_version).to eq("2.6.3")
        end
      end
    end

    context "with an exact bundler requirement" do
      before do
        File.write(File.join(tmp_dir, "Gemfile"), <<~GEMFILE
          source 'https://rubygems.org'
          gem 'bundler', '= 2.6.3'
        GEMFILE
        )
      end

      it "resolves the pinned bundler version" do
        scenario_manager.configure_scenario
        expect(scenario_manager.bundler_version).to eq("2.6.3")
      end
    end

    context "with a requirement no released bundler satisfies" do
      before do
        File.write(File.join(tmp_dir, "Gemfile"), <<~GEMFILE
          source 'https://rubygems.org'
          gem 'bundler', '>= 999.0'
        GEMFILE
        )
        allow(mock_fetcher).to receive(:detect).and_return([])
      end

      it "raises error 119" do
        expect { scenario_manager.configure_scenario }.to raise_error(Tebako::Error) { |error|
          expect(error.error_code).to eq(119)
        }
      end
    end

    context "without a bundler dependency" do
      before do
        File.write(File.join(tmp_dir, "Gemfile"), <<~GEMFILE
          source 'https://rubygems.org'
          gem 'rake'
        GEMFILE
        )
      end

      it "keeps the default bundler version and does not require bundler" do
        scenario_manager.configure_scenario
        expect(scenario_manager.bundler_version).to eq(Tebako::BUNDLER_VERSION)
        expect(scenario_manager.needs_bundler).to be false
      end
    end
  end
end

# rubocop:enable Metrics/BlockLength
