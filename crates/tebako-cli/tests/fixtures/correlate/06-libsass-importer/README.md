# 06 — libsass importer (real-world motivating case)

`outside.csv` is the source of truth: a procmon CSV shaped like
libsass's Windows importer — raw Win32 (`QueryOpen` =
GetFileAttributesW probes, `CreateFile` = CreateFileW) with `\\?\`
extended prefixes, verified against libsass src/file.cpp (read_file,
file_exists). `outside.json` is its conversion:

    retrace-procmon2retrace outside.csv outside.json

A CTest regenerates the JSON and diffs it against this copy, so
the two cannot drift. The probe miss (`NAME NOT FOUND` on an
under-prefix path) counts as an escape: read-attributes on a path
the VFS never served is an information leak.
