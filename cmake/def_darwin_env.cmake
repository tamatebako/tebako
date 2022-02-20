# Copyright (c) 2022, [Ribose Inc](https://www.ribose.com).
# All rights reserved.
# This file is a part of tamatebako
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
#

if (CMAKE_HOST_SYSTEM_NAME MATCHES "Darwin")
    if(TARGET_HOMEBREW)
        if (NOT (${TARGET_HOMEBREW} STREQUAL "guess"))
            set(_TARGET_HOMEBREW ${TARGET_HOMEBREW})
        endif()
    endif()
  
    execute_process(
        COMMAND brew --prefix
        RESULT_VARIABLE BREW_PREFIX_RES
        OUTPUT_VARIABLE BREW_PREFIX
        OUTPUT_STRIP_TRAILING_WHITESPACE
    )
    if(NOT (BREW_PREFIX_RES EQUAL 0 AND EXISTS ${BREW_PREFIX}))
        message(FATAL_ERROR "Could not find brew setup for build environment")
    endif()

    set(RUBY_BUILD_BREW_INCLUDE " -I${BREW_PREFIX}/include -I${BREW_PREFIX}/opt/openssl@1.1/include -I${BREW_PREFIX}/opt/zlib/include")
    set(RUBY_BUILD_BREW_LIB " -L${BREW_PREFIX}/lib -L${BREW_PREFIX}/opt/openssl@1.1/lib -L${BREW_PREFIX}/opt/zlib/lib")

    set(TARGET_BREW_PREFIX ${BREW_PREFIX})

    if(_TARGET_HOMEBREW)
        execute_process(
            COMMAND ${_TARGET_HOMEBREW}/bin/brew --prefix 
            RESULT_VARIABLE TARGET_BREW_PREFIX_RES
            OUTPUT_VARIABLE TARGET_BREW_PREFIX
            OUTPUT_STRIP_TRAILING_WHITESPACE
        )
        if(NOT (TARGET_BREW_PREFIX_RES EQUAL 0 AND EXISTS ${TARGET_BREW_PREFIX}))
            message(FATAL_ERROR "Could not find brew setup for target environment")
        endif()
    endif(_TARGET_HOMEBREW)

    set(RUBY_TARGET_BREW_INCLUDE " -I${TARGET_BREW_PREFIX}/include -I${TARGET_BREW_PREFIX}/opt/openssl@1.1/include -I${TARGET_BREW_PREFIX}/opt/zlib/include")
    set(RUBY_TARGET_BREW_LIB " -L${TARGET_BREW_PREFIX}/lib -L${TARGET_BREW_PREFIX}/opt/openssl@1.1/lib -L${TARGET_BREW_PREFIX}/opt/zlib/lib")

endif()
