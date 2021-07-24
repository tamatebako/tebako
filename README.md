[![Ubuntu build](https://github.com/metanorma/tebako/actions/workflows/ubuntu-build.yml/badge.svg?branch=master)](https://github.com/metanorma/tebako/actions/workflows/ubuntu-build.yml)

= Tebako: a image packager

== Purpose

This software packages a set of files into a dwarFS file system for
read-only purposes.

After packaging the file system, Tebako produces a single executable
binary that allows the user to execute a selected file from the packaged
software from a point in the file system.

== Goals

The packaged binary should support:

* Packaging a default dwarFS image inside the binary
* Support signing of the binary on macOS (via notarization)

In the future:

* Downloading new dwarFS images to be stored in the local home directory
* Allowing loading multiple dwarFS images in a stacked way
* Supporting a COW mechanism that the newly written files are stored
  in a separate image that can be loaded on top of the read-only file systems.


== Origin of name

"tamatebako" (玉手箱) is the treasure box given to Urashima Taro in the Ryugu,
for which he was asked not to open if he wished to return. He opened the box
upon the shock from his return that three hundred years has passed. Apparently
what was stored in the box was his age.

This packager was made to store Ruby and its gems, and therefore named after
the said treasure box (storing gems inside a treasure box).

Since "tamatebako" is rather long for the non-Japanese speaker, we use "tebako"
(手箱, also "tehako") instead, the generic term for a personal box.
