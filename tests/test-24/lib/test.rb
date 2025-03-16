# frozen_string_literal: true

require "bundler"
Bundler.require

require 'seven_zip_ruby.rb'
require_relative "tebako/version"

module Tebako
  module Test
    class Error < StandardError; end

    # Tebako test class
    class TebakoTest
      def run!
        puts "Hello! This is test-24 talking from inside DwarFS"
        # Make sure the constant exists
        puts "SevenZipRuby version: #{SevenZipRuby::VERSION}"
      end
    end
  end
end
