# frozen_string_literal: true

require "ffi"

# Tebako test module
module TebakoTest
  extend FFI::Library
  ffi_lib FFI::Library::LIBC
  attach_function("cputs", "puts", [:string], :int)
end
TebakoTest.cputs("Hello, World via libc puts using FFI on tebako package")
