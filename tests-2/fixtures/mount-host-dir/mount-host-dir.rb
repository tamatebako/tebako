# frozen_string_literal: true

# Copyright (c) 2024, [Ribose Inc](https://www.ribose.com).
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

require "fileutils"

def create_test_folder(tebako_folder)
  # Step 1: Create the tebako folder
  puts "Creating folder: #{tebako_folder}"
  FileUtils.mkdir_p(tebako_folder)
end

def create_test_file(test_file, message)
  # Step 2: Create the test.txt file and write the message to it
  puts("Writing message to file: #{test_file}")
  File.write(test_file, message)
end

def check_test_file(test_file, message)
  # Step 3: Rewind the file, read the message, and check that it matches
  File.open(test_file, "r+") do |file|
    file.rewind
    read_message = file.read
    if read_message == message
      puts "Message matches: #{read_message}"
    else
      puts "Message does not match. Expected: #{message}, Got: #{read_message}"
    end
  end
end

def delete_test_folder(test_folder)
  # Step 4: Delete the test.txt file and the tebako folder
  puts("Deleting #{test_folder}")
  FileUtils.rm_rf(test_folder)
end

# Example usage
# base_folder relative to current folder which is (presumably) /__tebako_memfs__/local
puts("Running at: #{Dir.pwd}")

base_folder = "tmp"
tebako_folder = File.join(base_folder, "tebako")
create_test_folder(tebako_folder)

test_file = File.join(tebako_folder, "test.txt")
message = "Hello, Tebako!"

create_test_file(test_file, message)
check_test_file(test_file, message)

delete_test_folder(tebako_folder)
