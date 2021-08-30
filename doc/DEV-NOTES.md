# Development notes #

## Tebako CLI commands ##

### Setup ###
**tebako setup [-p |--prefix=\<tebako root folder\>]**

Collect and builds tebako packager and install them for tebako packager at \<tebako root folder\> (defaults to current folder)

Tebako support several configurations at single system if that their root folders differ

### Press ###
**tebako press [-p |--prefix=\<tebako root folder\>] -r |--root=\<project root folder\>  [-e |--entry-point=\<entry point\>]  [-m |--mount-point=\<mount point\>] [-o |--output=\<packaged file name\>]** 

For example,  _tebako press --root='~/projects/myproject' --entry=start.rb  --mountpoint='/home/tebako' --output=myproject.tebako _ 

This command package Ruby project using tebako setup from \<tebako root folder\> 

- tebako root folder (optional, defaults to current folder)  - tebako setup folder 
- project root - a folder at the host source file system where project files are located
- entry point  - project executable file (binary ore script) that shall be started when packaged file is called
- mount point (optional, defaults to _/home/tebako_)  - a folder in the target file system where packaged filesystem is mounted to
- output - output file name (optional, defaults to _tebako_)


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
| 1 | Simple ruby script |  Copy _\<project root\>_ with all subfolders to packaged filesystem | _\<mount_point\>/local/\<entry_point\>_ |
| 2 | Packaged gem  |  Install the gem with _gem install_ to packaged filesystem    | _\<mount_point\>/bin/\<entry_point\>_<br>(i.e.: binstub is expected) |
| 3 | Gem source, no bundler |  Build the gem using _gem build_ command at the host and install it with _gem install_ to packaged filesystem |_\<mount_point\>/bin/\<entry_point\>_<br>(i.e.: binstub is expected) |
| 3 | Gem source, bundler |  Collect dependencies at the host with _bundle install_, build the gem using _gem build_ command and install it with _gem install_ to packaged filesystem |_\<mount_point\>/bin/\<entry_point\>_<br>(i.e.: binstub is expected) |
| 5 | Rails project  | Deploy project to packaged filesystem using _bundle install --deployment [--binstubs]_ |_\<mount_point\>/local/bin/\<entry_point\>_|




