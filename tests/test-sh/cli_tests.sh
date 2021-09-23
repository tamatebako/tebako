#! /bin/bash

testHelp() {
  result="$( $DIR/tebako --help )"
  assertEquals 0 $?
  assertContains "$result" "Usage:"
}

testMissingCommand() {
  result="$( $DIR/tebako )"
  assertEquals 4 $?
  assertContains "$result" "Missing command"
  assertContains "$result" "Usage:"
}

testUnknownCommand() {
  result="$( $DIR/tebako jump )"
  assertEquals 5 $?
  assertContains "$result" "Unknown command"
  assertContains "$result" "Usage:"
}


# Load and run shUnit2.
DIR0="$( cd "$( dirname "$0" )" && pwd )"
DIR="$( cd $DIR0/../../bin && pwd )"
echo "Running tebako CLI tests at $DIR"
. $DIR0/../shunit2/shunit2
