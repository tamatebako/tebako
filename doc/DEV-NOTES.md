# Development notes #

## Prerequisites ##

Tebako packager has bee tested on Ubuntu 18.xx and Ubuntu 20.xx platforms

### GNU C/C++ 9.x ###
If it is not available as default package it can be set up as follows
```
add-apt-repository ppa:ubuntu-toolchain-r/test
apt-get update
apt-get install gcc-9 g++-9
```
### Cmake 3.20 or better ###
If it is not available as default package it can be set up as follows
```
apt-get remove --purge --auto-remove cmake
apt-get update
apt-get install -y software-properties-common lsb-release
apt-get clean all
wget -O - https://apt.kitware.com/keys/kitware-archive-latest.asc 2>/dev/null | gpg --dearmor - | sudo tee /etc/apt/trusted.gpg.d/kitware.gpg >/dev/null
apt-add-repository "deb https://apt.kitware.com/ubuntu/ $(lsb_release -cs) main"
apt-get update
apt-get install kitware-archive-keyring
rm /etc/apt/trusted.gpg.d/kitware.gpg
sudo apt-get install cmake
```
### libfmt version 6.x or better ###
If you are using Ubuntu 20 it can be installed with simple _apt-get install libfmt-dev_
Ubuntu 18 requires a backport:

```
apt-get -y remove libfmt-dev
apt-get -y install software-properties-common
add-apt-repository ppa:team-xbmc/ppa
apt-get -y update
apt-get -y install libfmt-dev
```
### Other development tools and libraries ###
Ubuntu 18 and ubuntu 20 default versions are good enough
```
apt-get install \
binutils-dev libarchive-dev libevent-dev libjemalloc-dev acl-dev \
libdouble-conversion-dev libiberty-dev liblz4-dev liblzma-dev libssl-dev \
libboost-context-dev libboost-filesystem-dev libboost-program-options-dev \
libboost-regex-dev libboost-system-dev libboost-thread-dev \
libunwind-dev libdwarf-dev libelf-dev libfuse-dev libgoogle-glog-dev \
libffi-dev libgdbm-dev libyaml-dev libncurses-dev libreadline-dev \
libsqlite3-dev
```
### Ronn gem ###
Providing that you have Ruby 2.5 or better just run _gem install ronn_ and check, that bin stub can be called

## Installation ##
Clone tebako code from github to the folder of your choice

## Tebako CLI commands ##

### Setup ###
**\<install folder\>/bin/tebako setup [-p |--prefix=\\<tebako root folder\\>]**

Collect and builds tebako packager and install them for tebako packager at \<tebako root folder\> (defaults to install folder)

Tebako support several configurations at single system if that their root directories differ

### Press ###
**\<install folder\>/bin/tebako press [-p |--prefix=\<tebako root folder\>] -r|--root=\<project root folder\>  [-e|--entry-point=\<entry point\>]  [-o |--output=\<packaged file name\>]** 

For example,  _bin/tebako press --root='~/projects/myproject' --entry=start.rb --output=/temp/myproject.tebako_ 

This command package Ruby project using tebako setup from \<tebako root folder\> 

- tebako root folder (optional, defaults to current folder)  - tebako setup folder 
- project root - a folder at the host source file system where project files are located
- entry point  - project executable file (binary ore script) that shall be started when packaged file is called
- output - output file name (optional, defaults to _<current folder>\<entry point base name_)


### tebako exit codes ###

| Code |              Condition           |
|:----:|:--------------------------------:|
| 0    | No error                         |
| 1    | getops is not supported by the OS |
| 2    | Failed to parse command line     |
| 3    | Internal error                   |
| 4    | Missing command (setup or press is required)   |
| 5    | 'tebako press' without mandatory --root option |
| 6    | 'tebako press' without mandatory --entry-point option |
| 101  | 'tebako setup' failed at configuration step           |
| 102  | 'tebako setup' failed at build step                   |
| 103  | 'tebako press' failed at configuration step           |
| 104  | 'tebako press' failed at build step                   |
 

## Ruby packaging specification ##

This is sketchy description of Ruby packaging mehanism. The specification was reverse engineered from ruby-packer.
However, for various reason no single line of code was copied.
 
Depending on the configuration files that are present in the root project folder tebako Ruby packager support five different scenarious:

| Scenario |\*.gemspec | Gemfile  | \*.gem    |
|:--------:|:---------:|:--------:|:---------:|
| 1        |     No    |   No     |   No      |
| 2        |     No    |   No     |   One     |
| 3        |    One    |   No     |   Any     |
| 4        |    One    |   One    |   Any     |
| 5        |     No    |   One    |   Any     |
| Error    |     No    |   No     |Two or more|
| Error    |Two or more|   Any    |   Any     |

These scenarious differ in what files are pacjkaged and where thre entry point is located as follows:

| Scenario |     Description     |      Packaging    |     Entry point     |
|:--------:|:-------------------:|-------------------|:-------------------:|
| 1 | Simple ruby script |  Copy _\<project root\>_ with all subfolders to packaged filesystem | _\<mount_point\>/local/\<entry_point base name\>_ |
| 2 | Packaged gem  |  Install the gem with _gem install_ to packaged filesystem    | _\<mount_point\>/bin/\<entry_point base name\>_<br>(i.e.: binstub is expected) |
| 3 | Gem source, no bundler |  Build the gem using _gem build_ command at the host and install it with _gem install_ to packaged filesystem |_\<mount_point\>/bin/\<entry_point base name\>_<br>(i.e.: binstub is expected) |
| 3 | Gem source, bundler |  Collect dependencies at the host with _bundle install_, build the gem using _gem build_ command and install it with _gem install_ to packaged filesystem |_\<mount_point\>/bin/\<entry_point base name\>_<br>(i.e.: binstub is expected) |
| 5 | Rails project  | Deploy project to packaged filesystem using _bundle install |_\<mount_point\>/local/\<entry_point base name\>_|
