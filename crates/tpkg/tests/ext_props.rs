//! Extension-block property tests (spec 02 §5b): manifests with arbitrary
//! blocks survive encode → parse unchanged through both readers, the block
//! walker never panics and only ever accepts well-delimited input, and
//! full-image garbage still never panics.

use proptest::prelude::*;
use tpkg::*;

/// Block types the strategies generate: never 1 (reserved — validate
/// rejects it; the rejection itself is pinned by the behavior tests).
fn arb_block_type() -> impl Strategy<Value = u32> {
    prop_oneof![
        3 => Just(TPKG_EXT_TYPE_PACKAGE_MANIFEST),
        1 => Just(0u32),
        2 => 2u32..=u32::MAX,
    ]
}

fn arb_block() -> impl Strategy<Value = ExtBlock> {
    (arb_block_type(), prop::collection::vec(any::<u8>(), 0..64)).prop_map(
        |(block_type, payload)| ExtBlock {
            block_type,
            payload,
        },
    )
}

fn arb_slot() -> impl Strategy<Value = Slot> {
    (
        any::<u64>(),
        any::<u32>().prop_map(|f| f % (TPKG_FORMAT_RUNTIME + 1)),
        prop::collection::vec(1u8..=255u8, 0..20),
    )
        .prop_map(|(offset, format_id, mp)| {
            let mut slot = Slot {
                offset,
                size: 0,
                format_id,
                ..Default::default()
            };
            slot.set_mount_point(&mp);
            slot
        })
        .prop_flat_map(|slot| {
            let offset = slot.offset;
            (Just(slot), 0u64..=(u64::MAX - offset))
        })
        .prop_map(|(mut slot, size)| {
            slot.size = size;
            slot
        })
}

fn arb_manifest_with_blocks() -> impl Strategy<Value = Manifest> {
    (
        prop::collection::vec(arb_slot(), 1..=(TPKG_MAX_SLOTS as usize)),
        prop::collection::vec(arb_block(), 0..4),
        prop::collection::vec(1u8..=255u8, 0..16),
    )
        .prop_map(|(slots, ext_blocks, runtime_ref)| {
            let mut m = Manifest {
                slots,
                ext_blocks,
                ..Default::default()
            };
            m.set_runtime_ref(&runtime_ref);
            m
        })
}

proptest! {
    #[test]
    fn encode_parse_roundtrip_with_blocks(m in arb_manifest_with_blocks()) {
        let encoded = encode_trailer(&m, 0).expect("encode valid manifest");
        let parsed = parse_trailer(&encoded).expect("parse own output");
        prop_assert_eq!(&parsed, &m);
    }

    #[test]
    fn encode_parse_roundtrip_in_larger_image_with_blocks(
        m in arb_manifest_with_blocks(),
        payload in prop::collection::vec(any::<u8>(), 0..256),
    ) {
        let table_offset = payload.len() as u64;
        let mut image = payload;
        image.extend_from_slice(&encode_trailer(&m, table_offset).unwrap());
        let parsed = parse_trailer(&image).expect("parse in larger image");
        prop_assert_eq!(&parsed, &m);

        let mut cur = std::io::Cursor::new(&image);
        let parsed_io = read_from(&mut cur).expect("read_from in larger image");
        prop_assert_eq!(&parsed_io, &m);
    }

    #[test]
    fn block_walk_roundtrip(blocks in prop::collection::vec(arb_block(), 0..6)) {
        let bytes = encode_ext_blocks(&blocks).expect("encode blocks");
        let walked = parse_ext_blocks(&bytes).expect("walk own output");
        prop_assert_eq!(walked, blocks);
    }

    #[test]
    fn walker_never_panics_on_garbage(region in prop::collection::vec(any::<u8>(), 0..512)) {
        // An error or blocks, never a panic/overflow.
        let _ = parse_ext_blocks(&region);
    }

    #[test]
    fn walker_accepts_only_exact_consumption(region in prop::collection::vec(any::<u8>(), 0..512)) {
        // Whenever the walk succeeds, re-encoding the walked blocks
        // reproduces the region byte for byte — the walker never
        // "accepts" garbage by misparsing it.
        if let Ok(blocks) = parse_ext_blocks(&region) {
            let re = encode_ext_blocks(&blocks).expect("re-encode walked blocks");
            prop_assert_eq!(re, region);
        }
    }

    #[test]
    fn parser_never_panics_on_garbage_with_blocks(
        base in arb_manifest_with_blocks(),
        mutations in prop::collection::vec((any::<proptest::sample::Index>(), any::<u8>()), 0..16),
    ) {
        // A valid package with random byte mutations: error or manifest,
        // never a panic.
        let mut image = encode_trailer(&base, 0).unwrap();
        for (at, byte) in mutations {
            let i = at.index(image.len());
            image[i] = byte;
        }
        let _ = parse_trailer(&image);
    }
}
