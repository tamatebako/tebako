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
#  --  tebako setup (baseline for tests 01-17)
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
  if [ "${VERBOSE}" == "yes" ]; then 
    $DIR_BIN/tebako press --root="${DIR_TESTS}/$1" --entry-point="$2" 2>&1 | tee tebako_test.log
    assertEquals 0 ${PIPESTATUS[0]}
    result="$( cat tebako_test.log )"
  else 
    result="$( $DIR_BIN/tebako press --root=${DIR_TESTS}/$1 --entry-point=$2 2>&1 )"
    assertEquals 0 $?
  fi

# Check the first and the last messages expected from CMake script
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
   press_runner "${DIR_TESTS}/test-01" "test.rb"
}

test_tebako_press_03() {
   echo "tebako press test-03: simple Ruby script, absolute path to entry point"
   press_runner "${DIR_TESTS}/test-01" "${DIR_TESTS}/test-01/test.rb"
}


#    - name: Test01 - tebako press - [Simple Ruby script, relative path to entry point]
#      run: | 
#        ${{github.workspace}}/bin/tebako press                   \
#              --root="${{github.workspace}}/tests/test-01"       \
#              --entry-point="test.rb"                              
                                            
#    - name: Test01 - Run packaged solution - [Simple Ruby script, relative path to entry point]
#      run:  ${{github.workspace}}/output/tebako
# [ TODO]        echo Tebako exit code $?

#    - name: Test02 - tebako press - [Simple Ruby script, relative path to entry point, non exisitng entrance]
#      run: | 
#        ${{github.workspace}}/tests/scripts/press_with_failure.sh \
#              "${{github.workspace}}/tests/test-01"               \
#              "test-does-not-exist.rb"                            \
#              103                   
              
#    - name: Test03 - tebako press - [Simple Ruby script, absolute path to entry point]
#      run: | 
#        ${{github.workspace}}/bin/tebako press                   \
#              --root="${{github.workspace}}/tests/test-01"       \
#              --entry-point="${{github.workspace}}/tests/test-01/test.rb"                              

#    - name: Test03 - Run packaged solution - [Simple Ruby script, absolute path to entry point]
#      run:  ${{github.workspace}}/output/tebako

#    - name: Test04 - tebako press - [Simple Ruby script, absolute path to entry point, relative path to root ]
#      run: | 
#        ${{github.workspace}}/bin/tebako press                   \
#              --root="tests/test-01"                             \
#              --entry-point="test.rb"                              

#    - name: Test04 - Run packaged solution - [Simple Ruby script, absolute path to entry point, relative path to root ]
#      run:  ${{github.workspace}}/output/tebako
                                           
#    - name: Test05 - tebako press - [Simple Ruby script, absolute path to entry point, not within root]
#      run: | 
#        ${{github.workspace}}/tests/scripts/press_with_failure.sh \
#              "${{github.workspace}}/tests/test-01"               \
#              "${{github.workspace}}/tests/test-00/test.rb"       \
#              103                           

#    - name: Test06 - tebako press - [Rails project]
#      run: | 
#        ${{github.workspace}}/bin/tebako press                    \
#              --root="${{github.workspace}}/tests/test-06"        \
#              --entry-point="rails"                              

#    - name: Test06 - Run packaged solution - [Rails project]
#      run:  ${{github.workspace}}/output/tebako

#    - name: Test07 - tebako press - [Rails project, ruby and bundler version mismatch]
#      run: | 
#        ${{github.workspace}}/tests/scripts/press_with_failure.sh \
#              "${{github.workspace}}/tests/test-07"               \
#              "rails"                                             \
#              104   

#    - name: Test08 - tebako press - [Rails project, no entry point]
#      run: | 
#        ${{github.workspace}}/tests/scripts/press_with_failure.sh \
#              "${{github.workspace}}/tests/test-06"               \
#              "test-does-not-exist.rb"                            \
#              104                            

#    - name: Test09 - tebako press - [xxx.gem, no gemspec, no gemfile]
#      run: | 
#        ${{github.workspace}}/bin/tebako press                    \
#              --root="${{github.workspace}}/tests/test-09"        \
#              --entry-point="tebako-test-run.rb"                              

#    - name: Test09 - Run packaged solution - [xxx.gem, no gemspec, no gemfile]
#      run:  ${{github.workspace}}/output/tebako

#    - name: Test10 - tebako press - [xxx.gem, no gemspec, no gemfile, entry point does not exist]
#      run: | 
#        ${{github.workspace}}/tests/scripts/press_with_failure.sh \
#              "${{github.workspace}}/tests/test-09"               \
#              "test-does-not-exist.rb"                            \
#              104                              

#    - name: Test11 - tebako press - [Ruby gem, gemspec, no gemfile]
#      run: | 
#        ${{github.workspace}}/bin/tebako press                    \
#              --root="${{github.workspace}}/tests/test-11"        \
#              --entry-point="tebako-test-run.rb"                              

#    - name: Test11 - Run packaged solution - [Ruby gem, no gemfile]
#      run:  ${{github.workspace}}/output/tebako

#    - name: Test12 - tebako press - [Ruby gem, multiple gemspecs]
#      run: | 
#        ${{github.workspace}}/tests/scripts/press_with_failure.sh \
#              "${{github.workspace}}/tests/test-12"               \
#              "tebako-test-run.rb"                                \
#              103                            

#    - name: Test13 - tebako press - [Ruby gem, no gemfile, gemspec error]
#      run: | 
#        ${{github.workspace}}/tests/scripts/press_with_failure.sh \
#              "${{github.workspace}}/tests/test-13"               \
#              "tebako-test-run.rb"                                \
#              104                      

#    - name: Test14 - tebako press - [Ruby gem, gemspec, no gemfile, entry point does not exist]
#      run: | 
#        ${{github.workspace}}/tests/scripts/press_with_failure.sh \
#              "${{github.workspace}}/tests/test-11"               \
#              "test-does-not-exist.rb"                            \
#              104  

#    - name: Test15 - tebako press - [Ruby gem, gemspec, gemfile]
#      run: | 
#        ${{github.workspace}}/bin/tebako press                   \
#              --root="${{github.workspace}}/tests/test-15"       \
#              --entry-point="tebako-test-run.rb"                              

#    - name: Test15 - Run packaged solution - [Ruby gem, gemspec, gemfile]
#      run:  ${{github.workspace}}/output/tebako

#    - name: Test16 - tebako press - [Ruby gem, gemspec, gemfile with error]
#      run: | 
#        ${{github.workspace}}/tests/scripts/press_with_failure.sh \
#              "${{github.workspace}}/tests/test-16"               \
#              "tebako-test-run.rb"                                \
#              104                   

#    - name: Test17 - tebako press - [Ruby gem, gemspec, gemfile, entry point does not exist]
#      run: | 
#        ${{github.workspace}}/tests/scripts/press_with_failure.sh \
#              "${{github.workspace}}/tests/test-15"               \
#              "test-does-not-exist.rb"                            \
#              104  

#    - name: Test40 - CLI - Change output file name     
#      run: |    
#        ${{github.workspace}}/bin/tebako press                    \
#              --r "${{github.workspace}}/tests/test-01"           \
#              --e "test.rb" -o "tamatebako" 
#        ${{github.workspace}}/output/tamatebako                          
    
#    - name: Test50 -AUC - Check that it is possible to verify content of packaged fs          
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

VERBOSE="0"

echo "Running tebako tests"
. $DIR_TESTS/shunit2/shunit2
