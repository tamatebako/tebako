#! /bin/bash
#
# Copyright (c) 2021-2024, [Ribose Inc](https://www.ribose.com).
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
     "$DIR_BIN"/tebako press -D -R "$RUBY_VER" --root="$1" --entry-point="$2" --output="$3" --mode application 2>&1 | tee tebako_test.log
     assertEquals 0 "${PIPESTATUS[0]}"
     result="$( cat tebako_test.log )"
   else
     result=$( "$DIR_BIN"/tebako press -D -R "$RUBY_VER" --root="$1" --entry-point="$2" --output="$3" --mode application 2>&1 )
     assertEquals 0 $?
   fi

# Check the first and the last messages expected from CMake script
   assertContains "$result" "Running tebako press"
   assertContains "$result" "Created tebako package at"
}

package_runner() {
# Runs a package built by tebako
# Parameters:
# $1 -- runtime name
# $2 -- application name
# $3 -- expected output
   if [ "${VERBOSE}" == "yes" ]; then
     "$1" --tebako-run "$2" | tee tebako_test.log
     assertEquals 0 "${PIPESTATUS[0]}"
     result="$( cat tebako_test.log )"
   else
     result=$( "$1" --tebako-run "$2" )
     assertEquals 0 $?
   fi

   assertContains "$result" "$3"
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
     "$DIR_BIN"/tebako press -D -R "$RUBY_VER" --root="$1" --entry-point="$2" --output="$3" --mode application 2>&1 | tee tebako_test.log
     assertEquals "$4" "${PIPESTATUS[0]}"
     result="$( cat tebako_test.log )"
   else
     result=$( "$DIR_BIN"/tebako press -D -R "$RUBY_VER" --root="$1" --entry-point="$2" --output="$3" --mode application 2>&1 )
     assertEquals "$4" "${PIPESTATUS[0]}"
   fi

   assertContains "$result" "$5"
}

# ......................................................................
# Tests
#  AU. Build runtime (--mode runtime)
#  01. Simple Ruby script, absolute path to root, relative path to entry point
#  02. Simple Ruby script, absolute path to root, relative path to entry point, entry point does not exist  [Expected error at build step]
#  03. Simple Ruby script, absolute path to root, absolute path to entry point
#  04. Simple Ruby script, relative path to root, relative path to entry point
#  05. Simple Ruby script, absolute path to root absolute path to entry point, entry point not within root  [Expected error at configure step]
#  09. Ruby gem (xxx.gem, no gemspec, no gemfile)
#  10. Ruby gem (xxx.gem, no gemspec, no gemfile), entry point does not exist                               [Expected error at build step]
#  11. Ruby gem (no gemfile, with gemspec)
#  12. Ruby gem (no gemfile, with gemspec), multiple gemspecs  --- moved to RSpec tests                     [Expected error at configure step]
#  13. Ruby gem (no gemfile, with gemspec), gemspec error                                                   [Expected error at build step]
#  14. Ruby gem (no gemfile, with gemspec), project root does not exist                                     [Expected error at build step]
#  15. Ruby gem (with gemspec, with gemfile)
#  16. Ruby gem (with gemspec, with gemfile), gemfile with error                                            [Expected error at build step]
#  17. Ruby gem (with gemspec, with gemfile), entry point does not exist                                    [Expected error at build step]
#  18. Ruby project (no gemspec, with gemfile)
#  19. Ruby project (no gemspec, with gemfile, with native extension)
#  20. Net/http Ruby script [sits here and not on tests-2 in order to allow cross test MacOS x86_64 --> MacOS arm64]


# ......................................................................
#  AU. Build runtime (--mode runtime)
test_AU_runtime() {
   echo "==> Build tebako runtime"
   result=$( "$DIR_BIN"/tebako press -D -R "$RUBY_VER" --root=tests/test-01 --entry=tebako-test-run.rb --output=tebako-runtime --mode runtime 2>&1 )

   assertEquals 0 "${PIPESTATUS[0]}"
   assertContains "$result" "Running tebako press script"
   assertContains "$result" "Created tebako package at"
}

# ......................................................................
#  01. Simple Ruby script, absolute path to root, relative path to entry point
test_tebako_press_01() {
   echo "==> simple Ruby script, absolute path to root, relative path to entry point"
   press_runner "${DIR_TESTS}/test-01" "tebako-test-run.rb" "test-01-package"
   package_runner "./tebako-runtime" "test-01-package.tebako" "Hello!  This is test-01 talking from inside DwarFS"
}

# ......................................................................
# 02. Simple Ruby script, absolute path to root, relative path to entry point, entry point does not exist
test_tebako_press_02() {
   echo "==> simple Ruby script, absolute path to root, relative path to entry point, entry point does not exist"
   press_runner_with_error "${DIR_TESTS}/test-01" \
                           "test-does-not-exist.rb" \
                           "test-02-package" \
                           106 "Tebako script failed"
}

# ......................................................................
# 03. Simple Ruby script, absolute path to root, absolute path to entry point
test_tebako_press_03() {
   echo "==> simple Ruby script, absolute path to root, absolute path to entry point"
   press_runner "${DIR_TESTS}/test-01" "${DIR_TESTS}/test-01/tebako-test-run.rb" "test-03-package"
   package_runner "./tebako-runtime" "test-03-package.tebako" "Hello!  This is test-01 talking from inside DwarFS"
}

# ......................................................................
#  04. Simple Ruby script, relative path to root, relative path to entry point
test_tebako_press_04() {
   echo "==> simple Ruby script, relative path to root, relative path to entry point"
   pushd "${DIR_ROOT}" > /dev/null || fail "pushd ${DIR_ROOT} failed"
   press_runner "tests/test-01" "tebako-test-run.rb" "test-04-package"
   package_runner "${DIR_ROOT}/tebako-runtime" "test-04-package.tebako" "Hello!  This is test-01 talking from inside DwarFS"
   popd > /dev/null || fail "popd failed"
}

# ......................................................................
#  05. Simple Ruby script, absolute path to root absolute path to entry point, entry point not within root
test_tebako_press_05() {
   echo "==> simple Ruby script, absolute path to root absolute path to entry point, not within root"
   press_runner_with_error "${DIR_TESTS}/test-01" \
                           "${DIR_TESTS}/test-00/test.rb" "test-05-package" \
                            114 "Tebako script failed: Entry point is not within the project root"
}

# ......................................................................
#  09. Ruby gem (xxx.gem, no gemspec, no gemfile)
test_tebako_press_09() {
   echo "==> Ruby gem (xxx.gem, no gemspec, no gemfile)"
   press_runner "${DIR_TESTS}/test-09" "tebako-test-run.rb" "test-09-package"
   package_runner "./tebako-runtime" "test-09-package.tebako" "| a1 | b1 |"
}

# ......................................................................
#  10. Ruby gem (xxx.gem, no gemspec, no gemfile), entry point does not exist
test_tebako_press_10() {
   echo "==> Ruby gem (xxx.gem, no gemspec, no gemfile), entry point does not exist"
   press_runner_with_error "${DIR_TESTS}/test-09" \
                           "test-does-not-exist.rb" \
                           "test-10-package" \
                           106 "Tebako script failed"
}

# ......................................................................
#  11. Ruby gem (no gemfile, with gemspec)
test_tebako_press_11() {
   echo "==> Ruby gem (no gemfile, with gemspec)"
   press_runner "${DIR_TESTS}/test-11" "tebako-test-run.rb" "test-11-package"
   package_runner "./tebako-runtime" "test-11-package.tebako" "| a1 | b1 |"
}

# ......................................................................
#  13. Ruby gem (no gemfile, with gemspec), gemspec error
test_tebako_press_13() {
   echo "==>  Ruby gem (no gemfile, with gemspec), gemspec error"
   press_runner_with_error "${DIR_TESTS}/test-13" \
                            "tebako-test-run.rb" \
                            "test-13-package" \
                            255 "Tebako script failed"
}

# ......................................................................
#  14. Ruby gem (no gemfile, with gemspec), project root does not exist
test_tebako_press_14() {
   echo "==>  Ruby gem (no gemfile, with gemspec), project root does not exist"
   press_runner_with_error "${DIR_TESTS}/test-14" \
                           "test-does-not-exist.rb" \
                           "test-14-package" \
                           107 "Tebako script failed: Project root does not exist or is not accessible"
}

# ......................................................................
#  15. Ruby gem (with gemspec, with gemfile)
test_tebako_press_15() {
   echo "==> Ruby gem (with gemspec, with gemfile)"
   press_runner "${DIR_TESTS}/test-15" "tebako-test-run.rb" "test-15-package"
   package_runner "./tebako-runtime" "test-15-package.tebako" "| a1 | b1 |"
}

# ......................................................................
# 16. Ruby gem (with gemspec, with gemfile), gemfile with error
test_tebako_press_16() {
   echo "==> Ruby gem (with gemspec, with gemfile), gemfile with error"
   press_runner_with_error "${DIR_TESTS}/test-16" \
                           "tebako-test-run.rb" \
                           "test-16-package" \
                           255 "Tebako script failed"
}

# ......................................................................
# 17. Ruby gem (with gemspec, with gemfile), entry point dows not exist
test_tebako_press_17() {
   echo "==> Ruby gem (with gemspec, with gemfile), entry point does not exist"
   press_runner_with_error "${DIR_TESTS}/test-15" \
                           "test-does-not-exist.rb" \
                           "test-17-package" \
                           106 "Tebako script failed"
}

# ......................................................................
# 18. Ruby project (no gemspec, with gemfile)
test_tebako_press_18() {
   echo "==> Ruby project (no gemspec, with gemfile)"
   press_runner "${DIR_TESTS}/test-18" "tebako-test-run.rb" "test-18-package"
   package_runner "./tebako-runtime" "test-18-package.tebako" "| a1 | b1 |"
}

# ......................................................................
# 19. Ruby project (no gemspec, with gemfile, with native extension)
test_tebako_press_19() {
   echo "==> Ruby project (no gemspec, with gemfile, with native extension)"
   press_runner "${DIR_TESTS}/test-19" "tebako-test-run.rb" "test-19-package"
   package_runner "./tebako-runtime" "test-19-package.tebako" "Hello, World via libc puts using FFI on tebako package"
}

# ......................................................................
# 20. Net/http Ruby script
test_tebako_press_20() {
   echo "==> Net/http Ruby script"
   press_runner "${DIR_TESTS}/test-20" "tebako-test-run.rb" "test-20-package"
   package_runner "./tebako-runtime" "test-20-package.tebako" "Response: 302 Found"
}

# ......................................................................
# main

DIR0=$( dirname "$0" )
DIR_ROOT=$( cd "$DIR0"/../.. && pwd )
DIR_BIN=$( cd "$DIR_ROOT"/exe && pwd )
DIR_TESTS=$( cd "$DIR_ROOT"/tests && pwd )
RUBY_VER=${RUBY_VER:-3.2.6}

echo "Running tebako tests for Ruby $RUBY_VER"
# shellcheck source=/dev/null
. "$DIR_TESTS/shunit2/shunit2"
