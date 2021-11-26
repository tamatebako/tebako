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
# Helper functions
press_runner() {
# Runs 'tabako press'
# Parameters:
# $1 -- project root
# $2 -- entry point
# $3 -- tebako package name
   if [ "${VERBOSE}" == "yes" ]; then
     $DIR_BIN/tebako press --root="$1" --entry-point="$2" --package-name="$3" 2>&1 | tee tebako_test.log
     assertEquals 0 ${PIPESTATUS[0]}
     result="$( cat tebako_test.log )"
   else
     result="$( $DIR_BIN/tebako press --root=$1 --entry-point=$2 --package-name=$3 2>&1 )"
     assertEquals 0 $?
   fi

# Check the first and the last messages expected from CMake script
   assertContains "$result" "Running tebako press script"
   assertContains "$result" "Tebako packaging has completed"

# Check that packaged executable file is not a dynamic executable
   result="$( ldd $3 2>&1 )"
   assertEquals 1 $?
   assertContains "$result" "not a dynamic executable"
}

package_runner() {
# Runs a package built by tebako
# Parameters:
# $1 -- file name
# $2 -- expected output
   if [ "${VERBOSE}" == "yes" ]; then
     $1 | tee tebako_test.log
     assertEquals 0 ${PIPESTATUS[0]}
     result="$( cat tebako_test.log )"
   else
     result="$( $1 )"
     assertEquals 0 $?
   fi

   assertContains "$result" "$2"
}

press_runner_with_error() {
# Runs 'tabako press' expecting failure
# Parameters:
# $1 -- project root
# $2 -- entry point
# $3 -- tebako package name
# $4 -- expected error code
# $5 -- expected error message
   if [ "${VERBOSE}" == "yes" ]; then
     $DIR_BIN/tebako press --root="$1" --entry-point="$2" --package-name="$3" 2>&1 | tee tebako_test.log
     assertEquals $4 ${PIPESTATUS[0]}
     result="$( cat tebako_test.log )"
   else
     result="$( $DIR_BIN/tebako press --root=$1 --entry-point=$2 --package-name=$3 2>&1 )"
     assertEquals $4 $?
   fi

   assertContains "$result" "Running tebako press script"
   assertContains "$result" "$5"
}

# ......................................................................
# Tests
#  --. Very basic tebako CLI tests (error handling)
#  --  tebako setup
#  01. Simple Ruby script, absolute path to root, relative path to entry point
#  02. Simple Ruby script, absolute path to root, relative path to entry point, entry point does not exist  [Expected error at configure step]
#  03. Simple Ruby script, absolute path to root, absolute path to entry point
#  04. Simple Ruby script, relative path to root, relative path to entry point
#  05. Simple Ruby script, absolute path to root absolute path to entry point, entry point not within root  [Expected error at configure step]
#  -06. Rails project                                                               [TODO: this test is failing]
#  07. Rails project, ruby and bundler version mismatch                                                     [Expected error at build step]
#  08. Rails project, entry point does not exists                                                           [Expected error at configure step]
#  09. Ruby gem (xxx.gem, no gemspec, no gemfile)
#  10. Ruby gem (xxx.gem, no gemspec, no gemfile), entry point does not exist                               [Expected error at build step]
#  11. Ruby gem (no gemfile, with gemspec)
#  12. Ruby gem (no gemfile, with gemspec), multiple gemspecs                                               [Expected error at configure step]
#  13. Ruby gem (no gemfile, with gemspec), gemspec error                                                   [Expected error at build step]
#  14. Ruby gem (no gemfile, with gemspec), entry point does not exist                                      [Expected error at configure step]
#  -15. Ruby gem, gemfile, gemspec                                                  [TODO: this test is failing]
#  16. Ruby gem (with gemspec, with gemfile), gemfile with error                                            [Expected error at build step]
#  17. Ruby gem (with gemspec, with gemfile), entry point dows not exist                                    [Expected error at build step]
#  18 - 19  -- reserved
#  -20. AUC. Check that it is possible to verify content of package fs              [TODO: this test is failing]

# ......................................................................
# 00. Very basic tebako CLI tests (error handling)
test_CLI_help() {
  if [ "${VERBOSE}" == "yes" ]; then
    $DIR_BIN/tebako --help | tee tebako_test.log
    assertEquals 0 ${PIPESTATUS[0]}
    result="$( cat tebako_test.log )"
  else
    result="$( $DIR_BIN/tebako --help )"
    assertEquals 0 $?
  fi
  assertContains "$result" "Usage:"
}

test_CLI_missing_command() {
  if [ "${VERBOSE}" == "yes" ]; then
    $DIR_BIN/tebako | tee tebako_test.log
    assertEquals 4 ${PIPESTATUS[0]}
    result="$( cat tebako_test.log )"
  else
    result="$( $DIR_BIN/tebako )"
    assertEquals 4 $?
  fi

  assertContains "$result" "Missing command"
  assertContains "$result" "Usage:"
}

test_CLI_unknown_command() {
  if [ "${VERBOSE}" == "yes" ]; then
    $DIR_BIN/tebako jump | tee tebako_test.log
    assertEquals 5 ${PIPESTATUS[0]}
    result="$( cat tebako_test.log )"
  else
    result="$( $DIR_BIN/tebako jump )"
    assertEquals 5 $?
  fi

  assertContains "$result" "Unknown command"
  assertContains "$result" "Usage:"
}

# ......................................................................
#  --  tebako setup
test_tebako_setup() {
  echo "tebako setup ... patience, please, it may take up to 1 hour."
  if [ "${VERBOSE}" == "1" ]; then
    $DIR_BIN/tebako setup 2>&1 | tee tebako_test.log
    assertEquals 0 ${PIPESTATUS[0]}
    result="$( cat tebako_test.log )"
  else
    result="$( $DIR_BIN/tebako setup 2>&1 )"
    assertEquals 0 $?
  fi

# Check the first and the last messages expected from CMake script
  assertContains "$result" "Running tebako setup script"
  assertContains "$result" "Tebako setup has completed"

# Check that ruby is not a dynamic executable
#  This check is disabled (temporarily ?)  because of https://github.com/tamatebako/tebako/issues/38 
#  We are using dynamic build for packaging and static build for final integration of patched version
#  result="$( ldd ${DIR_DEPS}/src/_ruby-build/ruby 2>&1 )"
#  assertEquals 1 $?
#  assertContains "$result" "not a dynamic executable"
}

# ......................................................................
#  01. Simple Ruby script, absolute path to root, relative path to entry point
test_tebako_press_01() {
   echo "==> simple Ruby script,  absolute path to root, relative path to entry point"
   press_runner "${DIR_TESTS}/test-01" "tebako-test-run.rb" "test-01-package"
   package_runner "./test-01-package" "Hello!  This is test-1 talking from inside DwarFS"
}

# 02. Simple Ruby script, absolute path to root, relative path to entry point, entry point does not exist
test_tebako_press_02() {
   echo "==> simple Ruby script, absolute path to root, relative path to entry point, entry point does not exist"
   press_runner_with_error "${DIR_TESTS}/test-01" "test-does-not-exist.rb" "test-02-package" 103 "'tebako press' configure step failed"
}

# 03. Simple Ruby script, absolute path to root, absolute path to entry point
test_tebako_press_03() {
   echo "==> simple Ruby script, absolute path to root, absolute path to entry point"
   press_runner "${DIR_TESTS}/test-01" "${DIR_TESTS}/test-01/tebako-test-run.rb" "test-03-package"
   package_runner "./test-03-package" "Hello!  This is test-1 talking from inside DwarFS"
}

#  04. Simple Ruby script, relative path to root, relative path to entry point
test_tebako_press_04() {
   echo "==> simple Ruby script, relative path to root, relative path to entry point"
   pushd ${DIR_ROOT} > /dev/null
   press_runner "tests/test-01" "tebako-test-run.rb" "test-04-package"
   package_runner "./test-04-package" "Hello!  This is test-1 talking from inside DwarFS"
   popd > /dev/null
}

test_tebako_press_05() {
   echo "==> simple Ruby script, absolute path to root absolute path to entry point, not within root"
   press_runner_with_error "${DIR_TESTS}/test-01" "${DIR_TESTS}/test-00/test.rb" "test-05-package" 103 "'tebako press' configure step failed"
}

#    - name: Test06 - tebako press - [Rails project]
#      run: |
#        ${{github.workspace}}/bin/tebako press                    \
#              --root="${{github.workspace}}/tests/test-06"        \
#              --entry-point="rails"

#    - name: Test06 - Run packaged solution - [Rails project]
#      run:  ${{github.workspace}}/output/tebako

# ......................................................................
# 07. Rails project, ruby and bundler version mismatch
test_tebako_press_07() {
   echo "==> Rails project, ruby and bundler version mismatch"
   press_runner_with_error "${DIR_TESTS}/test-07" "rails" "test-07-package" 104 "'tebako press' build step failed"
}

# ......................................................................
# 08. Rails project, entry point does not exists
test_tebako_press_08() {
   echo "==> Rails project, entry point does not exists"
   press_runner_with_error "${DIR_TESTS}/test-08" "rails" "test-does-not-exist.rb" 103 "'tebako press' configure step failed"
}

# ......................................................................
#  09. Ruby gem (xxx.gem, no gemspec, no gemfile)
test_tebako_press_09() {
   echo "==> Ruby gem (xxx.gem, no gemspec, no gemfile)"
   press_runner "${DIR_TESTS}/test-09" "tebako-test-run.rb" "test-09-package"
   package_runner "./test-09-package" "| a1 | b1 |"
}

# ......................................................................
#  10. Ruby gem (xxx.gem, no gemspec, no gemfile), entry point does not exist
test_tebako_press_10() {
   echo "==> Ruby gem (xxx.gem, no gemspec, no gemfile), entry point does not exist"
   press_runner_with_error "${DIR_TESTS}/test-09" "test-does-not-exist.rb" "test-10-package" 104 "'tebako press' build step failed"
}

# ......................................................................
#  11. Ruby gem (no gemfile, with gemspec)
test_tebako_press_11() {
   echo "==> Ruby gem (no gemfile, with gemspec)"
   press_runner "${DIR_TESTS}/test-11" "tebako-test-run.rb" "test-11-package"
   package_runner "./test-11-package" "| a1 | b1 |"
}

# ......................................................................
#  12. Ruby gem (no gemfile, with gemspec), multiple gemspecs
test_tebako_press_12() {
   echo "==> Ruby gem (no gemfile, with gemspec), multiple gemspecs"
   press_runner_with_error "${DIR_TESTS}/test-12" "tebako-test-run.rb" "test-12-package" 103 "'tebako press' configure step failed"
}

# ......................................................................
#  13. Ruby gem (no gemfile, with gemspec), gemspec error
test_tebako_press_13() {
   echo "==>  Ruby gem (no gemfile, with gemspec), gemspec error"
   press_runner_with_error "${DIR_TESTS}/test-13" "tebako-test-run.rb" "test-13-package" 104 "'tebako press' build step failed"
}

# ......................................................................
#  14. Ruby gem (no gemfile, with gemspec), entry point does not exist
test_tebako_press_14() {
   echo "==>  Ruby gem (no gemfile, with gemspec), entry point does not exist"
   press_runner_with_error "${DIR_TESTS}/test-14" "test-does-not-exist.rb" "test-14-package" 103 "'tebako press' configure step failed"
}

#    - name: Test15 - tebako press - [Ruby gem, gemspec, gemfile]
#      run: |
#        ${{github.workspace}}/bin/tebako press                   \
#              --root="${{github.workspace}}/tests/test-15"       \
#              --entry-point="tebako-test-run.rb"

#    - name: Test15 - Run packaged solution - [Ruby gem, gemspec, gemfile]
#      run:  ${{github.workspace}}/output/tebako

# ......................................................................
# 16. Ruby gem (with gemspec, with gemfile), gemfile with error
test_tebako_press_16() {
   echo "==> Ruby gem (with gemspec, with gemfile), gemfile with error"
   press_runner_with_error "${DIR_TESTS}/test-16" "tebako-test-run.rb" "test-16-package" 104 "'tebako press' build step failed"
}

# ......................................................................
# 17. Ruby gem (with gemspec, with gemfile), entry point dows not exist
test_tebako_press_17() {
   echo "==> Ruby gem (with gemspec), entry point dows not exist"
   press_runner_with_error "${DIR_TESTS}/test-15" "test-does-not-exist.rb" "test-17-package" 104 "'tebako press' build step failed"
}

#    - name: Test20 -AUC - Check that it is possible to verify content of packaged fs
#      run: |
#        ${{github.workspace}}/bin/tebako press                    \
#              --root="${{github.workspace}}/tests/test-01"        \
#              --entry-point="test.rb"
#        deps/bin/dwarfs output/tebako home/tebako

# ......................................................................
# main

DIR0="$( cd "$( dirname "$0" )" && pwd )"
DIR_ROOT="$( cd $DIR0/../.. && pwd )"
DIR_BIN="$( cd $DIR_ROOT/bin && pwd )"
DIR_DEPS="$( cd $DIR_ROOT/deps && pwd )"
DIR_TESTS="$( cd $DIR_ROOT/tests && pwd )"

echo "Running tebako tests"
. $DIR_TESTS/shunit2/shunit2
