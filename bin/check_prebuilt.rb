#!/usr/bin/env ruby
#
require "bundler/setup"
require "octokit"
require "thor"
require "json"
require "date"

class PrebuiltMatrix < Thor
  RUNTIME_REPO = "tamatebako/tebako-runtime-ruby"
  PLATFORMS = %w[macos ubuntu windows-msys alpine]

  desc "generate VERSION", "Generate build matrix for given version"
  method_option :force_rebuild, type: :boolean, default: false, desc: "Force rebuild all packages"
  def generate(version)
    builder = RuntimeBuilder.new(version, options[:force_rebuild])
    builder.build_matrix
  end
end

class RuntimeBuilder
  RUNTIME_REPO = PrebuiltMatrix::RUNTIME_REPO
  PLATFORMS = PrebuiltMatrix::PLATFORMS

  def initialize(version, force_rebuild = false)
    @client = Octokit::Client.new
    @force_rebuild = force_rebuild
    @tebako_version = version
  end

  def build_matrix
    matrix = { "include" => build_combinations }
    File.write("build-matrix.json", matrix.to_json)
  end

  private

  def read_matrix_file(platform)
    JSON.parse(File.read(".github/matrices/#{platform}.json"))["full"]
  end

  def find_release_info(filename)
    releases = @client.releases(RUNTIME_REPO)
    releases.each do |release|
      assets = @client.release_assets(RUNTIME_REPO, release.id)
      asset = assets.find { |a| a.name == filename }
      return { "url" => asset.browser_download_url } if asset
    end
    nil
  rescue Octokit::NotFound
    warn "Warning: Repository #{RUNTIME_REPO} not found or no access"
    nil
  end

  def build_combinations
    matrices = load_matrices
    ruby_versions = matrices.values.map { |m| m["ruby"] }.flatten.uniq
    env_configs = tag_environments_with_platforms(matrices)

    ruby_versions.each_with_object([]) do |ruby_ver, combinations|
      env_configs.each do |env_config|
        config = build_config(ruby_ver, env_config)
        config["release"] = find_release_info(config["filename"])
        combinations << config
      end
    end
  end

  def load_matrices
    PLATFORMS.map { |platform| [platform, read_matrix_file(platform)] }.to_h
  end

  def tag_environments_with_platforms(matrices)
    matrices.flat_map do |platform, data|
      data["env"].map { |env| env.merge("platform" => platform.sub("-msys", "")) }
    end
  end

  def build_config(ruby_ver, env_config)
    platform = env_config["platform"]
    os = env_config["os"]
    platform_name, arch = get_platform_info(platform, os, env_config)
    ext = platform == "windows" ? ".exe" : ""
    filename = "tebako-ruby-#{@tebako_version}-#{ruby_ver}-#{platform_name}-#{arch}#{ext}"

    {
      "ruby_ver" => ruby_ver,
      "env" => env_config,
      "platform" => platform,
      "platform_name" => platform_name,
      "arch" => arch,
      "filename" => filename
    }
  end

  def get_platform_info(platform, os, env_config)
    case platform
    when "macos"
      version = os.match(/macos-(\d+)/)[1]
      arch = os.include?("-arm64") ? "arm64" : "x86_64"
      ["macos#{version}", arch]
    when "windows"
      %w[windows x64]
    when "ubuntu"
      version = os.match(/ubuntu-(\d+\.\d+)/)[1]
      ["ubuntu#{version}", "x86_64"]
    when "alpine"
      ["alpine#{env_config["ALPINE_VER"]}", "x86_64"]
    end
  end
end

PrebuiltMatrix.start(ARGV) if __FILE__ == $PROGRAM_NAME
