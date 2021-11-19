#! /bin/bash
#
# Copyright (c) 2021, [Ribose Inc](https://www.ribose.com).
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


# ......................................................................
# Very simple CLI tests
test_CLI_help() {
  $DIR_BIN/tebako --help | tee tebako_test.log
  assertEquals 0 ${PIPESTATUS[0]}
  
  result="$( cat tebako_test.log )"
  assertContains "$result" "Usage:"
}

test_CLI_missing_command() {
  result="$( $DIR_BIN/tebako )"
  assertEquals 4 $?
  assertContains "$result" "Missing command"
  assertContains "$result" "Usage:"
}

test_CLI_unknown_command() {
  result="$( $DIR_BIN/tebako jump )"
  assertEquals 5 $?
  assertContains "$result" "Unknown command"
  assertContains "$result" "Usage:"
}

# ......................................................................
# tebako setup test
test_tebako_setup() {
  echo "Running tebako setup. Patience ... the output is logged and this step may take up to 1 hour"	

  result="$( $DIR_BIN/tebako setup 2>&1 )"
  assertEquals 0 $?

  echo $result

# Check the first and the last messages expected from CMake script
  assertContains "$result" "Running tebako packager setup script"
  assertContains "$result" "tebako setup completed"

# Check that ruby is not a dynamic executable
  result="$( ldd ${DIR_DEPS}/bin/ruby 2>&1 )"
  assertEquals 1 $?
  assertContains "$result" "not a dynamic executable"
}

# ......................................................................
# main
DIR0="$( cd "$( dirname "$0" )" && pwd )"
DIR_BIN="$( cd $DIR0/../../bin && pwd )"
DIR_DEPS="$( cd $DIR0/../../deps && pwd )"
echo "Running tebako CLI tests at $DIR"
. $DIR0/../shunit2/shunit2
