# Development notes #
## Ruby packaging specification ##

This is sketchy description of Ruby packaging mehanism. The specification was reverse engineered from ruby-packer.
However for various reason no single line of code was copied.

Let's assume that the command line was _tebako ruby press --root=<project root>  --entry=<entry point>_  --mountpoint=<mount point>_
For example,  _tebako ruby press --root='~/projects/myproject' --entry=start.rb_  --mountpoint='/home/tebako'

**Where:**
- **project root** - is a folder at the host source file system where project files are located
- **entry point**  - is project executable file (binary ore script) that shall be started when packaged file is called
- **mount point**  - is a folder in the target file system where packaged filesystem is mounted to. Optional, defaults to /home/tebako
  
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




