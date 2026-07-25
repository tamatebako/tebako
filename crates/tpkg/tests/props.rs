//! Round-trip property tests: any valid manifest must survive
//! encode → parse unchanged.

use proptest::prelude::*;
use tpkg::*;

fn arb_slot() -> impl Strategy<Value = Slot> {
    (
        any::<u64>(),
        any::<u32>().prop_map(|f| f % (TPKG_FORMAT_RUNTIME + 1)),
        any::<u32>(),
        // mount points: short-ish strings, may include '/' and odd bytes
        // (but no interior NUL — the format is NUL-terminated)
        prop::collection::vec(1u8..=255u8, 0..40),
    )
        .prop_map(|(offset, format_id, flags, mp)| {
            let mut slot = Slot {
                offset,
                // keep offset+size from overflowing (validate rejects it;
                // that's covered by behavior tests, not here)
                size: 0,
                format_id,
                flags,
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

fn arb_manifest() -> impl Strategy<Value = Manifest> {
    (
        any::<u32>(),
        any::<u32>(),
        prop::collection::vec(1u8..=255u8, 0..30),
        prop::collection::vec(arb_slot(), 1..=(TPKG_MAX_SLOTS as usize)),
    )
        .prop_map(|(package_flags, launcher_abi, runtime_ref, slots)| {
            let mut m = Manifest {
                package_flags,
                launcher_abi,
                slots,
                ..Default::default()
            };
            m.set_runtime_ref(&runtime_ref);
            m
        })
}

proptest! {
    #[test]
    fn encode_parse_roundtrip(m in arb_manifest()) {
        // Standalone blob = [slot table][header], so the table sits at 0.
        let encoded = encode_trailer(&m, 0).expect("encode valid manifest");
        let parsed = parse_trailer(&encoded).expect("parse own output");
        prop_assert_eq!(&parsed, &m);
    }

    #[test]
    fn encode_parse_roundtrip_in_larger_image(
        m in arb_manifest(),
        payload in prop::collection::vec(any::<u8>(), 0..512),
    ) {
        let table_offset = payload.len() as u64;
        let mut image = payload;
        image.extend_from_slice(&encode_trailer(&m, table_offset).unwrap());
        let parsed = parse_trailer(&image).expect("parse in larger image");
        prop_assert_eq!(&parsed, &m);
    }

    #[test]
    fn parser_never_panics_on_garbage(data in prop::collection::vec(any::<u8>(), 0..4096)) {
        // Must return an error or a manifest, but never panic/overflow.
        let _ = parse_trailer(&data);
    }
}
