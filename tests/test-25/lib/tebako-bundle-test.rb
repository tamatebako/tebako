# frozen_string_literal: true

require "bundler"
Bundler.require

require "maxirmx_test_gem.rb"
require_relative "tebako-bundle-test/version"

module TebakoBundleTest
  # Tebako test class
  class TebakoTestClass
    def run!
      puts "Hello! This is test-24 talking from inside DwarFS"
      # Make sure the constant exists
      puts "TestGem version: #{TestGem::VERSION}"
    end
  end
end
