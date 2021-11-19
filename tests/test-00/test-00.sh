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

# Tests
#  00. Very basic tebako CLI tests (error handling)
#  --  tebako setup (baseline for tests 01-17)
#  01. Simple Ruby script, relative path to entry point  
#  02. Simple Ruby script, relative path to entry point, non exisitng entrance      Expected error at configure step
#  03. Simple Ruby script, absolute path to entry point 
#  04. Simple Ruby script, absolute path to entry point, relative path to root      
#  05. Simple Ruby script, absolute path to entry point, not within root            Expected error at configure step
#  06. Rails project                                                                                                       
#  07. Rails project, ruby and bundler version mismatch                             Expected error at build step
#  08. Rails project, no entry point                                                Expected error at build step           
#  09. Ruby gem (xxx.gem file)  
#  10. Ruby gem (xxx.gem file), no entry point                                      Expected error at build step
#  11. Ruby gem, no gemfile, with gemspec                                             
#  12. Ruby gem, no gemfile, multiple gemspecs                                      Expected error at configure step
#  13. Ruby gem, no gemfile, gemspec error                                          Expected error at build step  
#  14. Ruby gem, no gemfile, gemspec, no entry point                                Expected error at build step  
#  15. Ruby gem, gemfile, gemspec                                             
#  16. Ruby gem, gemfile, gemspec error                                             Expected error at build step  
#  17. Ruby gem, gemfile, gemspec, no entry point                                   Expected error at build step  
#  18 - 39  -- reserved
#  40. CLI.  Change output name                                                     [depends on Test-01] 
#  41 - 49  reserved
#  50. AUC. Check that it is possible to verify content of package fs               [depends on Test-01] 


# ......................................................................
# 00. Very basic tebako CLI tests (error handling)
test_CLI_help() {
  if [ "${VERBOSE}" == "1" ]; then 
    $DIR_BIN/tebako --help | tee tebako_test.log
    assertEquals 0 ${PIPESTATUS[0]}
  else 
    $DIR_BIN/tebako --help > tebako_test.log
    assertEquals 0 $?
  fi
  
  result="$( cat tebako_test.log )"
  assertContains "$result" "Usage:"
}

test_CLI_missing_command() {
  if [ "${VERBOSE}" == "1" ]; then 
    $DIR_BIN/tebako | tee tebako_test.log
    assertEquals 4 ${PIPESTATUS[0]}
  else 
    $DIR_BIN/tebako > tebako_test.log
    assertEquals 4 $?
  fi

  result="$( cat tebako_test.log )"
  assertContains "$result" "Missing command"
  assertContains "$result" "Usage:"
}

test_CLI_unknown_command() {
  if [ "${VERBOSE}" == "1" ]; then 
    $DIR_BIN/tebako jump | tee tebako_test.log
    assertEquals 5 ${PIPESTATUS[0]}
  else 
    $DIR_BIN/tebako jump > tebako_test.log
    assertEquals 5 $?
  fi

  result="$( cat tebako_test.log )"
  assertContains "$result" "Unknown command"
  assertContains "$result" "Usage:"
}

# ......................................................................
#  --  tebako setup (baseline for tests 01-17)
test_tebako_setup() {
  echo "tebako setup ... Patience, please. It may take up to 1 hour."
  if [ "${VERBOSE}" == "1" ]; then 
    $DIR_BIN/tebako setup 2>&1 | tee tebako_test.log
    assertEquals 0 ${PIPESTATUS[0]}
  else 
    $DIR_BIN/tebako setup 2>&1 > tebako_test.log
    assertEquals 0 $?
  fi

# Check the first and the last messages expected from CMake script
  result="$( cat tebako_test.log )"
  assertContains "$result" "Running tebako packager setup script"
  assertContains "$result" "tebako setup completed"

# Check that ruby is not a dynamic executable
  result="$( ldd ${DIR_DEPS}/bin/ruby 2>&1 )"
  assertEquals 1 $?
  assertContains "$result" "not a dynamic executable"
}


# ......................................................................
# Helper
press_runner() {
  if [ "${VERBOSE}" == "1" ]; then 
    $DIR_BIN/tebako press 2>&1 --root="${DIR_TESTS}/$1" --entry-point="$2" | tee tebako_test.log
    assertEquals 0 ${PIPESTATUS[0]}
  else 
    $DIR_BIN/tebako press 2>&1 --root="${DIR_TESTS}/$1" --entry-point="$2" > tebako_test.log
    assertEquals 0 $?
  fi

# Check the first and the last messages expected from CMake script
  result="$( cat tebako_test.log )"
  assertContains "$result" "Running tebako packager configuration script"
  assertContains "$result" "tebako packaging configuration created"

# Check that ruby is not a dynamic executable
  result="$( ldd ${DIR_DEPS}/bin/ruby 2>&1 )"
  assertEquals 1 $?
  assertContains "$result" "not a dynamic executable"
}

# ......................................................................
#  01. Simple Ruby script, relative path to entry point  
test_tebako_press_01() {
   echo "tebako press test-01: simple Ruby script, relative path to entry point"
   press_runner "test-01" "test.rb"
}

# ......................................................................
# main

DIR0="$( cd "$( dirname "$0" )" && pwd )"
DIR_ROOT="$( cd $DIR0/../.. && pwd )"
DIR_BIN="$( cd $DIR_ROOT/bin && pwd )"
DIR_DEPS="$( cd $DIR_ROOT/deps && pwd )"
DIR_TESTS="$( cd $DIR_ROOT/tests && pwd )"

VERBOSE="1"

echo "Running tebako tests"
. $DIR_TESTS/shunit2/shunit2
