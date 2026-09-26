//! Both items round-trip through the seal, everything that is not one of
//! them opens to nothing, and bytes the ENGINE sealed open here.

use super::*;
use crate::rendezvous::pairing::unhex;

const KEY: [u8; 32] = [9u8; 32];

fn endpoints() -> Vec<SocketAddr> {
    vec![
        "127.0.0.1:7737".parse().expect("v4"),
        "[::1]:7738".parse().expect("v6"),
    ]
}

/// **The engine's bytes, verbatim** (yog `src/wire/rendezvous/item.rs`,
/// sealed under `[9; 32]`): a presence naming the two endpoints above, and a
/// call with nonce `0x0102030405060708` naming them too. This end must open
/// both to exactly those values, or the two ends do not share a wire.
#[test]
fn what_the_engine_sealed_opens_here() {
    let presence = unhex(
        "e128651d113a52ef1c79109796acec5a3e4eff094a1d2143d4ae46519bfe0523849c18f7debdbd\
         99b74d282e32889ba74a27c827904f74",
    )
    .expect("hex");
    assert_eq!(
        Presence::open(&KEY, &presence),
        Some(Presence {
            endpoints: endpoints()
        })
    );
    let call = unhex(
        "428e0149d69f9ac52dd8b3d1ae0b5b6d0c2aafb09fdd685c266889f28f3b57588998c153623c50\
         bc5beab079ae3cbac53ca97e0d1ef8c180bfbe856e7b648a",
    )
    .expect("hex");
    assert_eq!(
        Call::open(&KEY, &call),
        Some(Call {
            nonce: 0x0102_0304_0506_0708,
            endpoints: endpoints()
        })
    );
}

#[test]
fn a_call_round_trips_with_its_nonce_and_is_opaque() {
    let call = Call {
        nonce: 0x0102_0304_0506_0708,
        endpoints: endpoints(),
    };
    let sealed = call.seal(&KEY).expect("seal");
    assert_eq!(Call::open(&KEY, &sealed), Some(call));
    assert!(
        sealed.len() < 100,
        "well under BEP 44's cap: {}",
        sealed.len()
    );
    assert!(
        !sealed.windows(4).any(|w| w == [127, 0, 0, 1]),
        "the address is not readable on the commons"
    );
    assert_eq!(
        Call::open(&[8u8; 32], &sealed),
        None,
        "another key opens nothing"
    );
}

#[test]
fn a_presence_round_trips_even_empty() {
    for endpoints in [endpoints(), Vec::new()] {
        let presence = Presence { endpoints };
        let sealed = presence.seal(&KEY).expect("seal");
        assert_eq!(Presence::open(&KEY, &sealed), Some(presence));
    }
}

#[test]
fn a_sealed_item_of_one_kind_is_not_the_other() {
    let presence = Presence {
        endpoints: endpoints(),
    };
    let sealed = presence.seal(&KEY).expect("seal");
    assert_eq!(Call::open(&KEY, &sealed), None, "no nonce to read");
    let call = Call {
        nonce: 1,
        endpoints: endpoints(),
    };
    let sealed = call.seal(&KEY).expect("seal");
    assert_eq!(
        Presence::open(&KEY, &sealed),
        None,
        "a count byte of 0 then bytes left over"
    );
}

#[test]
fn what_will_not_decode_is_nothing() {
    assert_eq!(Presence::open(&KEY, b"short"), None, "no nonce");
    assert_eq!(Presence::open(&KEY, &[0u8; 40]), None, "no tag");
    for plain in [
        vec![],
        vec![1],
        vec![1, 5, 0, 0],
        vec![1, 4, 1, 2, 3],
        vec![1, 4, 1, 2, 3, 4, 0],
        vec![1, 6, 0, 0, 0, 0, 0, 0, 0, 0],
        vec![0, 0],
    ] {
        let sealed = seal(&KEY, &plain).expect("seal");
        assert_eq!(Presence::open(&KEY, &sealed), None, "{plain:?}");
    }
    let sealed = seal(&KEY, &[0, 0, 0, 0, 0, 0, 0, 1, 0]).expect("seal");
    assert_eq!(
        Call::open(&KEY, &sealed),
        Some(Call {
            nonce: 1,
            endpoints: Vec::new()
        })
    );
}

#[test]
fn the_list_is_bounded_at_a_byte() {
    let many: Vec<SocketAddr> = (0..300u16)
        .map(|port| SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port))
        .collect();
    let (decoded, rest) = decode(&encode(&many)).expect("decodes");
    assert_eq!(decoded.len(), 255);
    assert!(rest.is_empty());
}
