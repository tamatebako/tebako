# frozen_string_literal: true

# tools/spec/link_probe_spec.rb — specs for tools/link_probe: the
# linker-stderr grammar against canned GNU ld / ld64 / lld fixtures, the
# probe source generation, the verdict report, and the malformed-dir
# toolchain verdict. minitest is stdlib (no new gem dependencies).
#
#   ruby tools/spec/link_probe_spec.rb

require "fileutils"
require "minitest/autorun"
require "stringio"
require "tmpdir"

load File.expand_path("../link_probe", __dir__)

class LinkProbeParseTest < Minitest::Test
  GNU_DUPLICATE = <<~ERR
    /usr/bin/ld: /tmp/unit/closure/libdup_b.a(dup.o):(.text+0x0): multiple definition of `probe_dup'; /tmp/unit/closure/libdup_a.a(dup.o):(.text+0x0): first defined here
    collect2: error: ld returned 1 exit status
  ERR

  # binutils' "in function" two-line form (and the '...' quote style)
  GNU_DUPLICATE_TWO_LINE = <<~ERR
    /usr/bin/ld: /tmp/unit/closure/libdup_b.a(dup.o): in function `probe_dup':
    (.text+0x0): multiple definition of 'probe_dup'; /tmp/unit/closure/libdup_a.a(dup.o):(.text+0x0): first defined here
  ERR

  GNU_MISSING_TWO_LINE = <<~ERR
    /usr/bin/ld: /tmp/link_probe/probe.o: in function `main':
    probe.c:(.text+0x1c): undefined reference to `uncompress'
    collect2: error: ld returned 1 exit status
  ERR

  GNU_MISSING_ONE_LINE = <<~ERR
    /usr/bin/ld: /tmp/unit/libtfs.a(tfs.o): undefined reference to `dwarfs_c_abi_version'
  ERR

  LD64_DUPLICATE = <<~ERR
    duplicate symbol '_probe_dup' in:
        /tmp/unit/closure/libdup_a.a(dup.o)
        /tmp/unit/closure/libdup_b.a(dup.o)
    ld: 1 duplicate symbol for architecture arm64
    clang: error: linker command failed with exit code 1 (use -v to see invocation)
  ERR

  LD64_MISSING = <<~ERR
    Undefined symbols for architecture arm64:
      "_uncompress", referenced from:
          _main in probe.o
      "_dwarfs_c_abi_version", referenced from:
          _main in probe.o
    ld: symbol(s) not found for architecture arm64
    clang: error: linker command failed with exit code 1 (use -v to see invocation)
  ERR

  # ld_prime (the Xcode 15+ linker): real output captured on Apple clang
  # 15 — "ld: Undefined symbols:", bare demangled symbols, and the
  # archive[N](obj.o) member form.
  LD_PRIME_DUPLICATE = <<~ERR
    duplicate symbol '_probe_dup' in:
        /tmp/lp-fixture-dup/closure/libdup_b.a[2](dup_b.o)
        /tmp/lp-fixture-dup/closure/libdup_a.a[2](dup_a.o)
    ld: 1 duplicate symbols
    clang: error: linker command failed with exit code 1 (use -v to see invocation)
  ERR

  LD_PRIME_MISSING = <<~ERR
    ld: Undefined symbols:
      _malloc_usable_size, referenced from:
          _main in probe.o
      std::logic_error::what() const, referenced from:
          vtable for boost::wrapexcept<std::logic_error> in libboost_program_options.a[4](convert.cpp.o)
          vtable for ranges::bad_variant_access in libdwarfs_common.a[63](lzma.cpp.o)
          ...
      _uncompress, referenced from:
          _main in probe.o
    clang: error: linker command failed with exit code 1 (use -v to see invocation)
  ERR

  LLD_DUPLICATE = <<~ERR
    ld.lld: error: duplicate symbol: probe_dup
    >>> defined at /tmp/unit/closure/libdup_a.a(dup.o)
    >>> defined at /tmp/unit/closure/libdup_b.a(dup.o)
  ERR

  LLD_MISSING = <<~ERR
    ld.lld: error: undefined symbol: uncompress
    >>> referenced by probe.c:4
    >>>               /tmp/link_probe/probe.o:(main)
  ERR

  def test_gnu_duplicate
    findings = LinkProbe.parse_linker_stderr(GNU_DUPLICATE)
    assert_equal 1, findings[:duplicates].size
    dup = findings[:duplicates].first
    assert_equal "probe_dup", dup.sym
    assert_equal "libdup_a.a(dup.o)", dup.obj_a
    assert_equal "libdup_b.a(dup.o)", dup.obj_b
    assert_empty findings[:missing]
  end

  def test_gnu_duplicate_two_line
    findings = LinkProbe.parse_linker_stderr(GNU_DUPLICATE_TWO_LINE)
    assert_equal 1, findings[:duplicates].size
    dup = findings[:duplicates].first
    assert_equal "probe_dup", dup.sym
    assert_equal "libdup_a.a(dup.o)", dup.obj_a
    assert_equal "libdup_b.a(dup.o)", dup.obj_b
  end

  def test_gnu_missing_two_line
    findings = LinkProbe.parse_linker_stderr(GNU_MISSING_TWO_LINE)
    assert_equal 1, findings[:missing].size
    miss = findings[:missing].first
    assert_equal "uncompress", miss.sym
    assert_equal "probe.o", miss.obj_a
  end

  def test_gnu_missing_one_line
    findings = LinkProbe.parse_linker_stderr(GNU_MISSING_ONE_LINE)
    assert_equal 1, findings[:missing].size
    miss = findings[:missing].first
    assert_equal "dwarfs_c_abi_version", miss.sym
    assert_equal "libtfs.a(tfs.o)", miss.obj_a
  end

  def test_ld64_duplicate
    findings = LinkProbe.parse_linker_stderr(LD64_DUPLICATE)
    assert_equal 1, findings[:duplicates].size
    dup = findings[:duplicates].first
    assert_equal "probe_dup", dup.sym
    assert_equal "libdup_a.a(dup.o)", dup.obj_a
    assert_equal "libdup_b.a(dup.o)", dup.obj_b
  end

  def test_ld64_missing
    findings = LinkProbe.parse_linker_stderr(LD64_MISSING)
    assert_equal %w[dwarfs_c_abi_version uncompress], findings[:missing].map(&:sym).sort
    assert_equal ["probe.o"], findings[:missing].map(&:obj_a).uniq
    assert_empty findings[:duplicates]
  end

  def test_ld_prime_duplicate
    findings = LinkProbe.parse_linker_stderr(LD_PRIME_DUPLICATE)
    assert_equal 1, findings[:duplicates].size
    dup = findings[:duplicates].first
    assert_equal "probe_dup", dup.sym
    # ld_prime's listing order is unstable run to run — the verdict pair
    # is sorted for determinism
    assert_equal "libdup_a.a(dup_a.o)", dup.obj_a
    assert_equal "libdup_b.a(dup_b.o)", dup.obj_b
  end

  def test_ld_prime_missing
    findings = LinkProbe.parse_linker_stderr(LD_PRIME_MISSING)
    syms = findings[:missing].map(&:sym).sort
    assert_equal ["malloc_usable_size", "std::logic_error::what() const", "uncompress"], syms
    by_sym = findings[:missing].to_h { |f| [f.sym, f.obj_a] }
    assert_equal "probe.o", by_sym["uncompress"]
    assert_equal "libboost_program_options.a(convert.cpp.o)", by_sym["std::logic_error::what() const"]
    assert_empty findings[:duplicates]
  end

  def test_lld_duplicate
    findings = LinkProbe.parse_linker_stderr(LLD_DUPLICATE)
    assert_equal 1, findings[:duplicates].size
    dup = findings[:duplicates].first
    assert_equal "probe_dup", dup.sym
    assert_equal "libdup_a.a(dup.o)", dup.obj_a
    assert_equal "libdup_b.a(dup.o)", dup.obj_b
  end

  def test_lld_missing
    findings = LinkProbe.parse_linker_stderr(LLD_MISSING)
    assert_equal 1, findings[:missing].size
    miss = findings[:missing].first
    assert_equal "uncompress", miss.sym
    assert_equal "probe.o", miss.obj_a
  end

  def test_noise_is_ignored
    stderr = <<~ERR
      ld: warning: ignoring file '/tmp/unit/closure/libx.a', building for macOS-arm64 but attempting to link with file built for macOS-x86_64
      clang: warning: -Wl,-foo: 'linker' input unused
    ERR
    findings = LinkProbe.parse_linker_stderr(stderr)
    assert_empty findings[:duplicates]
    assert_empty findings[:missing]
    assert_equal({ duplicates: [], missing: [] }, LinkProbe.parse_linker_stderr(""))
  end

  def test_duplicate_findings_deduped
    findings = LinkProbe.parse_linker_stderr(GNU_DUPLICATE + GNU_DUPLICATE)
    assert_equal 1, findings[:duplicates].size
  end
end

class LinkProbeSourceTest < Minitest::Test
  def test_battery_falls_back_to_extern_and_drops_the_driver_symbol
    source, notes = LinkProbe.build_probe_source("/nonexistent/include", [])
    assert_includes source, "extern int dwarfs_c_abi_version(void);"
    assert_includes source, "extern int uncompress();"
    assert_includes source, "extern size_t malloc_usable_size();"
    assert_includes source, "dwarfs_c_abi_version();"
    refute_includes source, "tebako_driver_boot"
    assert_equal ["dropped tebako_driver_boot (no staged header declares it)"], notes
  end

  def test_driver_symbol_rides_the_staged_header_when_one_declares_it
    Dir.mktmpdir do |dir|
      include = File.join(dir, "include")
      FileUtils.mkdir_p(include)
      File.write(File.join(include, "driver.h"), <<~H)
        /* The generic boot. */
        TEBAKO_DRIVER_API int tebako_driver_boot(int *argc, char ***argv,
                                                 const char *runtime_root);
      H
      source, notes = LinkProbe.build_probe_source(include, [])
      assert_empty notes
      assert_includes source, "int tebako_driver_boot(int *argc, char ***argv, const char *runtime_root);"
      # an argful prototype takes the volatile-store reference, never a call
      assert_includes source, "link_probe_sink = (void *)&tebako_driver_boot;"
      refute_includes source, "tebako_driver_boot();"
    end
  end

  def test_header_mentions_inside_comments_are_not_declarations
    Dir.mktmpdir do |dir|
      include = File.join(dir, "include")
      FileUtils.mkdir_p(include)
      File.write(File.join(include, "c_api.h"), <<~H)
        /**
         * @example
         * @code
         * if (tebako_fs_init_from_file("/app.zip", "/__tebako__") == 0) {
         *     // mounted
         * }
         * @endcode
         */
        int tebako_fs_init_from_file(const char* archive_path, const char* mount_point);
      H
      assert_equal "int tebako_fs_init_from_file(const char* archive_path, const char* mount_point);",
                   LinkProbe.header_declaration(include, "tebako_fs_init_from_file")
    end
  end

  def test_user_symbols_are_declared_and_referenced_inside_the_dead_block
    source, = LinkProbe.build_probe_source("/nonexistent", %w[my_tool_v2 another_fn])
    assert_includes source, "extern void my_tool_v2(void); my_tool_v2();"
    assert_includes source, "extern void another_fn(void); another_fn();"
    dead = source[/if \(argc == -1\) \{(.*?)\n    \}/m, 1]
    assert_includes dead, "my_tool_v2();"
  end

  def test_reference_for_picks_call_or_store_by_prototype
    assert_equal "x();", LinkProbe.reference_for("x", "extern int x(void);")
    assert_equal "x();", LinkProbe.reference_for("x", "extern int x();")
    assert_equal "link_probe_sink = (void *)&x;", LinkProbe.reference_for("x", "int x(int *a, char ***b);")
  end
end

class LinkProbeVerdictTest < Minitest::Test
  def test_ok_line
    lines = LinkProbe.verdict_lines({ duplicates: [], missing: [] }, archives: 38, closure: 36)
    assert_equal ["LINK-PROBE ok archives=38 closure=36"], lines
  end

  def test_fail_lines_sorted_and_named
    findings = {
      duplicates: [LinkProbe::Finding.new(sym: "zeta", obj_a: "a.o", obj_b: "b.o"),
                   LinkProbe::Finding.new(sym: "alpha", obj_a: "c.o", obj_b: "d.o")],
      missing: [LinkProbe::Finding.new(sym: "uncompress", obj_a: "probe.o")],
    }
    lines = LinkProbe.verdict_lines(findings, archives: 3, closure: 1)
    assert_equal [
      "LINK-PROBE fail duplicate alpha (c.o vs d.o)",
      "LINK-PROBE fail duplicate zeta (a.o vs b.o)",
      "LINK-PROBE fail missing uncompress (referenced by probe.o)",
    ], lines
  end

  def test_link_failure_outside_the_grammar
    lines = LinkProbe.verdict_lines({ duplicates: [], missing: [] }, archives: 3, closure: 1,
                                    other_failure: "ld: archive has no index; run ranlib")
    assert_equal ["LINK-PROBE fail link ld: archive has no index; run ranlib"], lines
  end
end

class LinkProbeRunTest < Minitest::Test
  def test_malformed_staged_dir_is_a_toolchain_verdict
    Dir.mktmpdir do |dir|
      io = StringIO.new
      code = LinkProbe.run(dir, cc: "cc", user_symbols: [], extra: [], out: io)
      assert_equal 64, code
      assert_equal "LINK-PROBE fail toolchain staged dir malformed: missing libtebako_driver.a, missing libtfs.a, no closure/*.a (#{dir})",
                   io.string.lines.first.chomp
    end
  end
end
