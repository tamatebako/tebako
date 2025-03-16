#! /bin/bash
#
# Copyright (c) 2021-2025, [Ribose Inc](https://www.ribose.com).
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

# ......................................................................
# Helper functions
press_runner_bundle() {
# Runs 'tabako press'
# Parameters:
# $1 -- project root
# $2 -- entry point
# $3 -- tebako package name
   if [ "${VERBOSE}" == "yes" ]; then
     "$DIR_BIN"/tebako press -D -R "$RUBY_VER" --root="$1" --entry-point="$2" --output="$3" 2>&1 | tee tebako_test.log
     assertEquals 0 "${PIPESTATUS[0]}"
     result="$( cat tebako_test.log )"
   else
     result=$( "$DIR_BIN"/tebako press -D -R "$RUBY_VER" --root="$1" --entry-point="$2" --output="$3" 2>&1 )
     assertEquals 0 $?
   fi

# Check the first and the last messages expected from CMake script
   assertContains "$result" "Running tebako press script"
   assertContains "$result" "Created tebako package at"
}

press_runner_app() {
# Runs 'tabako press'
# Parameters:
# $1 -- project root
# $2 -- entry point
# $3 -- tebako package name
   if [ "${VERBOSE}" == "yes" ]; then
     "$DIR_BIN"/tebako press -D -R "$RUBY_VER" --root="$1" --entry-point="$2" --output="$3" --mode=application 2>&1 | tee tebako_test.log
     assertEquals 0 "${PIPESTATUS[0]}"
     result="$( cat tebako_test.log )"
   else
     result=$( "$DIR_BIN"/tebako press -D -R "$RUBY_VER" --root="$1" --entry-point="$2" --output="$3" --mode=application 2>&1 )
     assertEquals 0 $?
   fi

# Check the first and the last messages expected from CMake script
   assertContains "$result" "Created tebako application package at"
}


package_runner_bundle() {
# Runs a package built by tebako
# Parameters:
# $1 -- file name
# $2 -- expected output
   if [ "${VERBOSE}" == "yes" ]; then
     $1 | tee tebako_test.log
     assertEquals 0 "${PIPESTATUS[0]}"
     result="$( cat tebako_test.log )"
   else
     result="$( $1 )"
     assertEquals 0 $?
   fi

   assertContains "$result" "$2"
}

package_runner_app() {
# Runs a package built by tebako
# Parameters:
# $1 -- file name
# $2 -- expected output
   if [ "${VERBOSE}" == "yes" ]; then
     ./tebako-runtime --tebako-run="$1".tebako | tee tebako_test.log
     assertEquals 0 "${PIPESTATUS[0]}"
     result=$( cat tebako_test.log )
   else
     result=$( ./tebako-runtime --tebako-run="$1".tebako )
     assertEquals 0 $?
   fi

   assertContains "$result" "$2"
}

press_runner_with_error_bundle() {
# Runs 'tabako press' expecting failure
# Parameters:
# $1 -- project root
# $2 -- entry point
# $3 -- tebako package name
# $4 -- expected error code
# $5 -- expected error message (optional)
   if [ "${VERBOSE}" == "yes" ]; then
     "$DIR_BIN"/tebako press -D -R "$RUBY_VER" --root="$1" --entry-point="$2" --output="$3" 2>&1 | tee tebako_test.log
     assertEquals "$4" "${PIPESTATUS[0]}"
     result="$( cat tebako_test.log )"
   else
     result=$( "$DIR_BIN"/tebako press -D -R "$RUBY_VER" --root="$1" --entry-point="$2" --output="$3" 2>&1 )
     assertEquals "$4" "${PIPESTATUS[0]}"
   fi

   if [ -n "$5" ]; then
     failed_msg="$5"
   else
     failed_msg="$PRESS_FAILED_MSG"
   fi
   assertContains "$result" "$failed_msg"
}

press_runner_with_error_app() {
# Runs 'tabako press' expecting failure
# Parameters:
# $1 -- project root
# $2 -- entry point
# $3 -- tebako package name
# $4 -- expected error code
   if [ "${VERBOSE}" == "yes" ]; then
     "$DIR_BIN"/tebako press -D -R "$RUBY_VER" --root="$1" --entry-point="$2" --output="$3" --mode="application" 2>&1 | tee tebako_test.log
     assertEquals "$4" "${PIPESTATUS[0]}"
     result="$( cat tebako_test.log )"
   else
     result=$( "$DIR_BIN"/tebako press -D -R "$RUBY_VER" --root="$1" --entry-point="$2" --output="$3" --mode="application" 2>&1 )
     assertEquals "$4" "${PIPESTATUS[0]}"
   fi

   assertContains "$result" "$PRESS_FAILED_MSG"
}

# ......................................................................
# Tests
#  AU1. Check that it is possible to extract image content (--tebako-extract option)
#  AU2. Build runtime (--mode runtime)
#  01. Simple Ruby script, absolute path to root, relative path to entry point
#  02. Simple Ruby script, absolute path to root, relative path to entry point, entry point does not exist  [Expected error at build step]
#  03. Simple Ruby script, absolute path to root, absolute path to entry point
#  04. Simple Ruby script, relative path to root, relative path to entry point
#  09. Ruby gem (xxx.gem, no gemspec, no gemfile)
#  10. Ruby gem (xxx.gem, no gemspec, no gemfile), entry point does not exist                               [Expected error at build step]
#  11. Ruby gem (no gemfile, with gemspec)
#  13. Ruby gem (no gemfile, with gemspec), gemspec error                                                   [Expected error at build step]
#  15. Ruby gem (with gemspec, with gemfile)
#  16. Ruby gem (with gemspec, with gemfile), gemfile with error                                            [Expected error at build step]
#  17. Ruby gem (with gemspec, with gemfile), entry point does not exist                                    [Expected error at build step]
#  18. Ruby project (no gemspec, with gemfile)
#  19. Ruby project (no gemspec, with gemfile, with ffi extension)
#  20. Net/http Ruby script [sits here and not on tests-2 in order to allow cross test MacOS x86_64 --> MacOS arm64]
#  21. Ruby gem (with gemspec, with gemfile, with lockfile)
#  22. Ruby project (no gemspec, with gemfile, with lockfile, with ffi extension)
#  23. Ruby gem (with gemspec, with gemfile, with bundler requirement in gemfile)
#  24. Ruby gem (with gemspec, with gemfile, with a gem installed from GitHub)

# ......................................................................
#  AU1. Check that it is possible to extract image content (--tebako-extract option)
test_AUC_extract() {
   echo "==> Check --tebako-extract option"
   if [ "${MODE}" != "bundle" ]; then
      echo "Mode is not 'bundle'; skipping"
   else

      if [ "${VERBOSE}" == "yes" ]; then
         "$DIR_BIN"/tebako press -D -R "$RUBY_VER" --root=tests/test-01 --entry=tebako-test-run.rb --output=test-AUC-package 2>&1 | tee tebako_test.log
         assertEquals 0 "${PIPESTATUS[0]}"
         result="$( cat tebako_test.log )"
      else
         result=$( "$DIR_BIN"/tebako press -D -R "$RUBY_VER" --root=tests/test-01 --entry=tebako-test-run.rb --output=test-AUC-package 2>&1 )
         assertEquals 0 "${PIPESTATUS[0]}"
      fi

      assertContains "$result" "Running tebako press script"
      assertContains "$result" "Created tebako package at"

      ./test-AUC-package --tebako-extract
      assertEquals 0 "${PIPESTATUS[0]}"

      diff -r source_filesystem o/s
      assertEquals 0 "${PIPESTATUS[0]}"

      rm -rf source_filesystem

      ./test-AUC-package --tebako-extract extract
      assertEquals 0 "${PIPESTATUS[0]}"

      diff -r extract o/s
      assertEquals 0 "${PIPESTATUS[0]}"

      rm -rf extract
   fi
}

# ......................................................................
#  AU2. Build runtime (--mode runtime)
test_AU_runtime() {
   echo "==> Build tebako runtime"
   if [ "${MODE}" != "app" ]; then
      echo "Mode is not 'app'; skipping"
   else
      result=$( "$DIR_BIN"/tebako press -D -R "$RUBY_VER" --root=tests/test-01 --entry=tebako-test-run.rb --output=tebako-runtime --mode runtime 2>&1 )

      assertEquals 0 "${PIPESTATUS[0]}"
      assertContains "$result" "Running tebako press script"
      assertContains "$result" "Created tebako runtime package at"
   fi
}

# ......................................................................
#  01. Simple Ruby script, absolute path to root, relative path to entry point
test_tebako_press_01() {
   echo "==> simple Ruby script, absolute path to root, relative path to entry point"
   press_runner_"${MODE}" "${DIR_TESTS}/test-01" "tebako-test-run.rb" "test-01-package"
   package_runner_"${MODE}" "./test-01-package" "Hello!  This is test-01 talking from inside DwarFS"
}

# ......................................................................
# 02. Simple Ruby script, absolute path to root, relative path to entry point, entry point does not exist
test_tebako_press_02() {
   echo "==> simple Ruby script, absolute path to root, relative path to entry point, entry point does not exist"
   if [ "${MODE}" == "bundle" ]; then
      rc=104
   else
      rc=106
   fi
   press_runner_with_error_"${MODE}"   "${DIR_TESTS}/test-01" \
                                       "test-does-not-exist.rb" \
                                       "test-02-package" \
                                       "$rc"
}

# ......................................................................
# 03. Simple Ruby script, absolute path to root, absolute path to entry point
test_tebako_press_03() {
   echo "==> simple Ruby script, absolute path to root, absolute path to entry point"
   press_runner_"${MODE}" "${DIR_TESTS}/test-01" "${DIR_TESTS}/test-01/tebako-test-run.rb" "test-03-package"
   package_runner_"${MODE}" "./test-03-package" "Hello!  This is test-01 talking from inside DwarFS"
}

# ......................................................................
#  04. Simple Ruby script, relative path to root, relative path to entry point
test_tebako_press_04() {
   echo "==> simple Ruby script, relative path to root, relative path to entry point"
   pushd "${DIR_ROOT}" > /dev/null || fail "pushd ${DIR_ROOT} failed"
   press_runner_"${MODE}" "tests/test-01" "tebako-test-run.rb" "test-04-package"
   package_runner_"${MODE}" "./test-04-package" "Hello!  This is test-01 talking from inside DwarFS"
   popd > /dev/null || fail "popd failed"
}

# ......................................................................
#  09. Ruby gem (xxx.gem, no gemspec, no gemfile)
test_tebako_press_09() {
   echo "==> Ruby gem (xxx.gem, no gemspec, no gemfile)"

# Use test-11 gemspec to build a test gem
   pushd "${DIR_TESTS}/test-11" > /dev/null || fail "pushd ${DIR_TESTS}/test-11 failed"
   mkdir -p "${DIR_TESTS}/test-09"
   gem build tebako-test.gemspec -o "${DIR_TESTS}/test-09/tebako-test-0.0.2.gem"
   popd > /dev/null || fail "popd failed"

   press_runner_"${MODE}" "${DIR_TESTS}/test-09" "tebako-test-run.rb" "test-09-package"
   package_runner_"${MODE}" "./test-09-package" "| a1 | b1 |"
}

# ......................................................................
#  10. Ruby gem (xxx.gem, no gemspec, no gemfile), entry point does not exist
test_tebako_press_10() {
   echo "==> Ruby gem (xxx.gem, no gemspec, no gemfile), entry point does not exist"
   if [ "${MODE}" == "bundle" ]; then
      rc=104
   else
      rc=106
   fi

# Use gem built for test-09
   press_runner_with_error_"${MODE}" "${DIR_TESTS}/test-09" \
                                     "test-does-not-exist.rb" \
                                     "test-10-package" \
                                     "$rc"
}

# ......................................................................
#  11. Ruby gem (no gemfile, with gemspec)
test_tebako_press_11() {
   echo "==> Ruby gem (no gemfile, with gemspec)"
   press_runner_"${MODE}" "${DIR_TESTS}/test-11" "tebako-test-run.rb" "test-11-package"
   package_runner_"${MODE}" "./test-11-package" "| a1 | b1 |"
}

# ......................................................................
#  13. Ruby gem (no gemfile, with gemspec), gemspec error
test_tebako_press_13() {
   echo "==>  Ruby gem (no gemfile, with gemspec), gemspec error"
   if [ "${MODE}" == "bundle" ]; then
      rc=104
   else
      rc=255
   fi
   press_runner_with_error_"${MODE}" "${DIR_TESTS}/test-13" \
                                     "tebako-test-run.rb" \
                                     "test-13-package" \
                                     "$rc"
}

# ......................................................................
#  15. Ruby gem (with gemspec, with gemfile)
test_tebako_press_15() {
   echo "==> Ruby gem (with gemspec, with gemfile)"
   press_runner_"${MODE}" "${DIR_TESTS}/test-15" "tebako-test-run.rb" "test-15-package"
   package_runner_"${MODE}" "./test-15-package" "| a1 | b1 |"
}

# ......................................................................
# 16. Ruby gem (with gemspec, with gemfile), gemfile with error
test_tebako_press_16() {
   echo "==> Ruby gem (with gemspec, with gemfile), gemfile with error"
   press_runner_with_error_"${MODE}" "${DIR_TESTS}/test-16" \
                                     "tebako-test-run.rb" \
                                     "test-16-package" \
                                     115 \
                                     "Failed to load Gemfile"
}

# ......................................................................
# 17. Ruby gem (with gemspec, with gemfile), entry point dows not exist
test_tebako_press_17() {
   echo "==> Ruby gem (with gemspec, with gemfile), entry point does not exist"
   if [ "${MODE}" == "bundle" ]; then
      rc=104
   else
      rc=106
   fi
   press_runner_with_error_"${MODE}" "${DIR_TESTS}/test-15" \
                                     "test-does-not-exist.rb" \
                                     "test-17-package" \
                                     "$rc"
}

# ......................................................................
# 18. Ruby project (no gemspec, with gemfile)
test_tebako_press_18() {
   echo "==> Ruby project (no gemspec, with gemfile)"
   press_runner_"${MODE}" "${DIR_TESTS}/test-18" "tebako-test-run.rb" "test-18-package"
   package_runner_"${MODE}" "./test-18-package" "| a1 | b1 |"
}

# ......................................................................
# 19. Ruby project (no gemspec, with gemfile, with ffi extension)
test_tebako_press_19() {
   echo "==> Ruby project (no gemspec, with gemfile, with ffi extension)"
   press_runner_"${MODE}" "${DIR_TESTS}/test-19" "tebako-test-run.rb" "test-19-package"
   package_runner_"${MODE}" "./test-19-package" "Hello, World via libc puts using FFI on tebako package"
}

# ......................................................................
# 20. Net/http Ruby script
test_tebako_press_20() {
   echo "==> Net/http Ruby script"
   press_runner_"${MODE}" "${DIR_TESTS}/test-20" "tebako-test-run.rb" "test-20-package"
   package_runner_"${MODE}" "./test-20-package" "Response: 302 Found"
}

# ......................................................................
#  21. Ruby gem (with gemspec, with gemfile, with lockfile)
test_tebako_press_21() {
   echo "==> Ruby gem (with gemspec, with gemfile, with lockfile)"
   press_runner_"${MODE}" "${DIR_TESTS}/test-21" "tebako-test-run.rb" "test-21-package"
   package_runner_"${MODE}" "./test-21-package" "| a1 | b1 |"
}

# ......................................................................
# 22. Ruby project (no gemspec, with gemfile, with lockfile, with ffi extension)
test_tebako_press_22() {
   echo "==> Ruby project (no gemspec, with gemfile, with lockfile, with ffi extension)"
   press_runner_"${MODE}" "${DIR_TESTS}/test-22" "tebako-test-run.rb" "test-22-package"
   package_runner_"${MODE}" "./test-22-package" "Hello, World via libc puts using FFI on tebako package"
}

# ......................................................................
# 23. Ruby gem (with gemspec, with gemfile, with bundler requirement in gemfile)
test_tebako_press_23() {
   echo "==> Ruby gem (with gemspec, with gemfile, with bundler requirement in gemfile)"
   press_runner_"${MODE}" "${DIR_TESTS}/test-23" "tebako-test-run.rb" "test-23-package"
   package_runner_"${MODE}" "./test-23-package" "| a1 | b1 |"
}

# ......................................................................
# 24. Ruby gem (with gemspec, with gemfile, with a gem installed from GitHub)
test_tebako_press_24() {
   echo "==> Ruby gem (with gemspec, with gemfile, with a gem installed from GitHub)"
   press_runner_"${MODE}" "${DIR_TESTS}/test-24" "tebako-test-run.rb" "test-24-package"
   package_runner_"${MODE}" "./test-24-package" "SevenZipRuby version: 1."
}

# ......................................................................
# main

DIR0=$( dirname "$0" )
DIR_ROOT=$( cd "$DIR0"/../.. && pwd )
DIR_BIN=$( cd "$DIR_ROOT"/exe && pwd )
DIR_TESTS=$( cd "$DIR_ROOT"/tests && pwd )
RUBY_VER=${RUBY_VER:-3.2.6}

MODE=${MODE:-bundle}
if [ "${MODE}" == "bundle" ]; then
   PRESS_FAILED_MSG="'tebako press' build step failed"
else
   PRESS_FAILED_MSG="Tebako script failed"
fi

echo "Running tebako tests for Ruby $RUBY_VER"

# shellcheck source=/dev/null
. "$DIR_TESTS/shunit2/shunit2"
