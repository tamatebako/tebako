#! /bin/bash
#
# Copyright (c) 2022, [Ribose Inc](https://www.ribose.com).
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
     "$DIR_BIN/tebako" press --root="$1" --entry-point="$2" --output="$3" --target="$TARGET" \
                             --target-homebrew="$DIR_HOMEBREW" 2>&1 | tee tebako_test.log
      assertEquals 0 "${PIPESTATUS[0]}"
      result="$( cat tebako_test.log )"
   else
      result="$( "$DIR_BIN/tebako" press -r "$1" -e "$2" -o "$3" -t "$TARGET" -b "$DIR_HOMEBREW" 2>&1 )"
      assertEquals 0 $?
   fi

# Check the first and the last messages expected from CMake script
   assertContains "$result" "Running tebako press script"
   assertContains "$result" "packaging has completed"
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
     "$DIR_BIN/tebako" press --root="$1" --entry-point="$2" --output="$3" --target="$TARGET" \
                             --target-homebrew="$DIR_HOMEBREW" 2>&1 | tee tebako_test.log
      assertEquals "$4" "${PIPESTATUS[0]}"
      result="$( cat tebako_test.log )"
   else
      result="$( "$DIR_BIN/tebako" press -r "$1" -e "$2" -o "$3" -t "$TARGET" -b "$DIR_HOMEBREW" 2>&1 )"
      assertEquals "$4" "${PIPESTATUS[0]}"
   fi

   assertContains "$result" "Running tebako press script"
   assertContains "$result" "$5"
}

# ......................................................................
# Tests
#  --. Very basic tebako CLI tests (error handling)
#  --  tebako setup
#  01. Simple Ruby script, absolute path to root, relative path to entry point                              [Positive scenario -- build only]
#  02. Simple Ruby script, absolute path to root, relative path to entry point, entry point does not exist  [Expected error at configure step]
#  03. Simple Ruby script, absolute path to root, absolute path to entry point                              [Positive scenario -- build only]
#  04. Simple Ruby script, relative path to root, relative path to entry point                              [Positive scenario -- build only]
#  05. Simple Ruby script, absolute path to root absolute path to entry point, entry point not within root  [Expected error at configure step]
#  -06. Rails project                                                               [TODO: this test is failing]
#  -07. Rails project, ruby and bundler version mismatch                                                     [Expected error at build step]
#  -08. Rails project, entry point does not exists                                                           [Expected error at configure step]
#  09. Ruby gem (xxx.gem, no gemspec, no gemfile)                                                           [Positive scenario -- build only]
#  10. Ruby gem (xxx.gem, no gemspec, no gemfile), entry point does not exist                               [Expected error at build step]
#  11. Ruby gem (no gemfile, with gemspec)                                                                  [Positive scenario -- build only]
#  12. Ruby gem (no gemfile, with gemspec), multiple gemspecs                                               [Expected error at configure step]
#  13. Ruby gem (no gemfile, with gemspec), gemspec error                                                   [Expected error at build step]
#  14. Ruby gem (no gemfile, with gemspec), entry point does not exist                                      [Expected error at configure step]
#  15. Ruby gem (with gemspec, with gemfile)                                                                [Positive scenario -- build only]
#  16. Ruby gem (with gemspec, with gemfile), gemfile with error                                            [Expected error at build step]
#  17. Ruby gem (with gemspec, with gemfile), entry point does not exist                                    [Expected error at build step]
#  18. Ruby project (no gemspec, with gemfile)                                                              [Positive scenario -- build only]
#  19. Ruby project (no gemspec, with gemfile, with native extension)                                       [Positive scenario -- build only]
#  -20. AUC. Check that it is possible to verify content of package fs              [TODO: this test is failing]

# ......................................................................
# 00. Very basic tebako CLI tests (error handling)
test_CLI_help() {
  if [ "${VERBOSE}" == "yes" ]; then
    "$DIR_BIN/tebako" --help | tee tebako_test.log
    assertEquals 0 "${PIPESTATUS[0]}"
    result="$( cat tebako_test.log )"
  else
    result="$( "$DIR_BIN/tebako" --help )"
    assertEquals 0 "${PIPESTATUS[0]}"
  fi
  assertContains "$result" "Usage:"
}

test_CLI_missing_command() {
  if [ "${VERBOSE}" == "yes" ]; then
    "$DIR_BIN/tebako" | tee tebako_test.log
    assertEquals 4 "${PIPESTATUS[0]}"
    result="$( cat tebako_test.log )"
  else
    result="$( "$DIR_BIN/tebako" )"
    assertEquals 4 "${PIPESTATUS[0]}"
  fi

  assertContains "$result" "Missing command"
  assertContains "$result" "Usage:"
}

test_CLI_unknown_command() {
  if [ "${VERBOSE}" == "yes" ]; then
    "$DIR_BIN/tebako" jump | tee tebako_test.log
    assertEquals 5 "${PIPESTATUS[0]}"
    result="$( cat tebako_test.log )"
  else
    result="$( "$DIR_BIN/tebako" jump )"
    assertEquals 5 "${PIPESTATUS[0]}"
  fi

  assertContains "$result" "Unknown command"
  assertContains "$result" "Usage:"
}

test_CLI_wrong_target() {
  if [ "${VERBOSE}" == "yes" ]; then
    "$DIR_BIN/tebako" setup --target=intel_x86_win32 | tee tebako_test.log
    assertEquals 10 "${PIPESTATUS[0]}"
    result="$( cat tebako_test.log )"
  else
    result="$( "$DIR_BIN/tebako" setup --target=intel_x86_win32 )"
    assertEquals 10 "${PIPESTATUS[0]}"
  fi

  assertContains "$result" "Unknown target setting"
}


# ......................................................................
#  --  tebako setup
test_tebako_setup() {
  echo "tebako setup ... patience, please, it may take up to 1 hour."
  if [ "${VERBOSE}" == "yes" ]; then
    "$DIR_BIN/tebako" setup --target="$TARGET" \
                      --target-homebrew="$DIR_HOMEBREW" 2>&1 | tee tebako_test.log
    assertEquals 0 "${PIPESTATUS[0]}"
    result="$( cat tebako_test.log )"
  else
    result="$( "$DIR_BIN/tebako" setup -t "$TARGET" -b "$DIR_HOMEBREW" 2>&1 )"
    assertEquals 0 "${PIPESTATUS[0]}"
  fi

# Check the first and the last messages expected from CMake script
  assertContains "$result" "Running tebako setup script"
  assertContains "$result" "Tebako setup has been completed"
}

# ......................................................................
#  01. Simple Ruby script, absolute path to root, relative path to entry point
test_tebako_press_01() {
   echo "==> simple Ruby script,  absolute path to root, relative path to entry point"
   press_runner "${DIR_TESTS}/test-01" "tebako-test-run.rb" "test-01-package"
}

# ......................................................................
# 02. Simple Ruby script, absolute path to root, relative path to entry point, entry point does not exist
test_tebako_press_02() {
   echo "==> simple Ruby script, absolute path to root, relative path to entry point, entry point does not exist"
   press_runner_with_error "${DIR_TESTS}/test-01" "test-does-not-exist.rb" "test-02-package" 103 "'tebako press' configure step failed"
}

# ......................................................................
# 03. Simple Ruby script, absolute path to root, absolute path to entry point
test_tebako_press_03() {
   echo "==> simple Ruby script, absolute path to root, absolute path to entry point"
   press_runner "${DIR_TESTS}/test-01" "${DIR_TESTS}/test-01/tebako-test-run.rb" "test-03-package"
}

# ......................................................................
#  04. Simple Ruby script, relative path to root, relative path to entry point
test_tebako_press_04() {
   echo "==> simple Ruby script, relative path to root, relative path to entry point"
   pushd "${DIR_ROOT}" > /dev/null || fail "pushd ${DIR_ROOT} failed"
   press_runner "tests/test-01" "tebako-test-run.rb" "test-04-package"
   popd > /dev/null || fail "popd failed"
}

# ......................................................................
#  05. Simple Ruby script, absolute path to root absolute path to entry point, entry point not within root
test_tebako_press_05() {
   echo "==> simple Ruby script, absolute path to root absolute path to entry point, not within root"
   press_runner_with_error "${DIR_TESTS}/test-01" "${DIR_TESTS}/test-00/test.rb" "test-05-package" 103 "'tebako press' configure step failed"
}

# ......................................................................
#  06. Rails project
#test_tebako_press_06() {
#   echo "==> Rails project"
#   press_runner "${DIR_TESTS}/test-06" "bin/rails" "test-06-package"
#   ???? package_runner "./test-06-package"  ????
#}

# ......................................................................
# 07. Rails project, ruby and bundler version mismatch
#test_tebako_press_07() {
#   echo "==> Rails project, ruby and bundler version mismatch"
#   press_runner_with_error "${DIR_TESTS}/test-07" "rails" "test-07-package" 103 "'tebako press' configure step failed"
#}

# ......................................................................
# 08. Rails project, entry point does not exists
#test_tebako_press_08() {
#   echo "==> Rails project, entry point does not exists"
#   press_runner_with_error "${DIR_TESTS}/test-08" "rails" "test-does-not-exist.rb" 103 "'tebako press' configure step failed"
#}

# ......................................................................
#  09. Ruby gem (xxx.gem, no gemspec, no gemfile)
test_tebako_press_09() {
   echo "==> Ruby gem (xxx.gem, no gemspec, no gemfile)"
   press_runner "${DIR_TESTS}/test-09" "tebako-test-run.rb" "test-09-package"
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

# ......................................................................
#  15. Ruby gem (with gemspec, with gemfile)
test_tebako_press_15() {
   echo "==> Ruby gem (with gemspec, with gemfile)"
   press_runner "${DIR_TESTS}/test-15" "tebako-test-run.rb" "test-15-package"
}

# ......................................................................
# 16. Ruby gem (with gemspec, with gemfile), gemfile with error
test_tebako_press_16() {
   echo "==> Ruby gem (with gemspec, with gemfile), gemfile with error"
   press_runner_with_error "${DIR_TESTS}/test-16" "tebako-test-run.rb" "test-16-package" 104 "'tebako press' build step failed"
}

# ......................................................................
# 17. Ruby gem (with gemspec, with gemfile), entry point dows not exist
test_tebako_press_17() {
   echo "==> Ruby gem (with gemspec, with gemfile), entry point dows not exist"
   press_runner_with_error "${DIR_TESTS}/test-15" "test-does-not-exist.rb" "test-17-package" 104 "'tebako press' build step failed"
}

# ......................................................................
# 18. Ruby project (no gemspec, with gemfile)
test_tebako_press_18() {
   echo "==> Ruby project (no gemspec, with gemfile)"
   press_runner "${DIR_TESTS}/test-18" "tebako-test-run.rb" "test-18-package"
}

# ......................................................................
# 19. Ruby project (no gemspec, with gemfile, with native extension)
#test_tebako_press_19() {
#   echo "==> Ruby project (no gemspec, with gemfile, with native extension)"
#   press_runner "${DIR_TESTS}/test-19" "tebako-test-run.rb" "test-19-package"
#}

#    - name: Test20 -AUC - Check that it is possible to verify content of packaged fs
#      run: |
#        ${{github.workspace}}/bin/tebako press                    \
#              --root="${{github.workspace}}/tests/test-01"        \
#              --entry-point="test.rb"
#        deps/bin/dwarfs output/tebako home/tebako

# ......................................................................
# main

DIR0="$( cd "$( dirname "$0" )" && pwd )"
DIR_ROOT="$( cd "$DIR0"/../.. && pwd )"
DIR_BIN="$( cd "$DIR_ROOT"/bin && pwd )"
DIR_TESTS="$( cd "$DIR_ROOT"/tests && pwd )"

DIR_HOMEBREW="$( cd "$DIR_HOMEBREW" && pwd )"

echo "Running tebako tests"
# shellcheck source=/dev/null
. "$DIR_TESTS/shunit2/shunit2"
