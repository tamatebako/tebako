# Development notes #

## Tebako CLI commands ##

### Setup ###
**tebako setup --prefix=\<tebako root folder\>  --config=\<tebako config YAML file\>**

Setup command collects, builds tebako packager dependencies and installs them to \<tebako root folder\> (optional, defaults to '/usr/local/tebako')

Optional \<tebako config YAML file\> specifies links to packages to install and hash values for validation

Sample:
```
# GitHub dependencies' 
  INCBIN_TAG: 348e36b
  DWARFS_TAG: 78401c3
# Tarball dependencies
  RUBY_VER:   "2.7.4"
  RUBY_HASH:  "3043099089608859fc8cce7f9fdccaa1f53a462457e3838ec3b25a7d609fbc5b"
  BUNDLER_VER: "2.2.3"
  BUNDLER_HASH: "6acefda4aeb34cb3d69aff06affce10424d69f484402a9f7f5577e8c698070db"
  GDBM_VER: "1.13"
  GDBM_HASH: "9d252cbd7d793f7b12bcceaddda98d257c14f4d1890d851c386c37207000a253"
  LIBFFI_VER: "3.2.1"
  LIBFFI_HASH: "980ca30a8d76f963fca722432b1fe5af77d7a4e4d2eac5144fbc5374d4c596609a293440573f4294207e1bdd9fda80ad1e1cafb2ffb543df5a275bc3bd546483"
  NCURSES_VER: "6.2"
  NCURSES_HASH: "30306e0c76e0f9f1f0de987cf1c82a5c21e1ce6568b9227f7da5b71cbea86c9d"
  OPENSSL_VER: "1.1.0h"
  OPENSSL_HASH: "5835626cde9e99656585fc7aaa2302a73a7e1340bf8c14fd635a62c66802a517"
  READLINE_VER: "7.0"
  READLINE_HASH: "750d437185286f40a369e1e4f4764eda932b9459b5ec9a731628393dd3d32334"
  YAML_VER: "0.1.7"
  YAML_HASH: "8088e457264a98ba451a90b8661fcb4f9d6f478f7265d48322a196cec2480729"
  ZLIB_VER: "1.2.11"
  ZLIB_HASH: "c3e5e9fdd5004dcb542feda5ee4f0ff0744628baf8ed2dd5d66f8ca1197cb1a1"
```

Tebako support several configurations at single system if that their root folders differ

### Press ###
**tebako press --prefix=\<tebako root folder\> --root=\<project root folder\>  --entry=\<entry point\>  --mountpoint=\<mount point\> --output=\<packaged file name\>** 

For example,  _tebako press --root='~/projects/myproject' --entry=start.rb  --mountpoint='/home/tebako' --output=myproject.tebako _ 

This command package Ruby project using tebako setup from \<tebako root folder\> 

- tebako root folder (optional, defaults to '/usr/local/tebako')  - tebako setup folder 
- project root - a folder at the host source file system where project files are located
- entry point  - project executable file (binary ore script) that shall be started when packaged file is called
- mount point (optional, defaults to _/home/tebako_)  - a folder in the target file system where packaged filesystem is mounted to. Optional, defaults to /home/tebako


### Binary ###
tebako press-binary 

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
| 1 | Simple ruby script |  Copy _\<project root\>_ with all subfolders to packaged filesystem | _\<mount_point\>/local/\<entry_point\>_ |
| 2 | Packaged gem  |  Install the gem with _gem install_ to packaged filesystem    | _\<mount_point\>/bin/\<entry_point\>_<br>(i.e.: binstub is expected) |
| 3 | Gem source, no bundler |  Build the gem using _gem build_ command at the host and install it with _gem install_ to packaged filesystem |_\<mount_point\>/bin/\<entry_point\>_<br>(i.e.: binstub is expected) |
| 3 | Gem source, bundler |  Collect dependencies at the host with _bundle install_, build the gem using _gem build_ command and install it with _gem install_ to packaged filesystem |_\<mount_point\>/bin/\<entry_point\>_<br>(i.e.: binstub is expected) |
| 5 | Rails project  | Deploy project to packaged filesystem using _bundle install --deployment [--binstubs]_ |_\<mount_point\>/local/bin/\<entry_point\>_|




