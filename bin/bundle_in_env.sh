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
#

# This script call bundler in environment resolving some shortcomings of cmake -E command
#
# $1 -- the project (Gemfile) location
# $2 -- the directory to add to $PATH (presumably location of ruby binary at source filesystem)
# $3 -- the location to install the specified gems to


echo  Changing current directory to $1
cd $1

#echo Running "env --unset=GEM_HOME --unset=GEM_PATH PATH='$2:$PATH' bundle config set --local path '$3'"
#env --unset=GEM_HOME --unset=GEM_PATH PATH="$2:$PATH" bundle config set --local path '$3'

echo Running "env --unset=GEM_HOME --unset=GEM_PATH PATH='$2:$PATH' bundle install --deployment"
env --unset=GEM_HOME --unset=GEM_PATH PATH="$2:$PATH" bundle install --jobs=4 --deployment

