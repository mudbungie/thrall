//! What is and is not a registration URL.

use super::{Door, parse};

/// The URL a seat shows: the standard port is implied, and a stated one is
/// kept — in the address and in the `Host` header alike.
#[test]
fn a_registration_url_reads_as_where_to_post_and_what_to_post_to() {
    assert_eq!(
        parse("https://engine.example/yog/register/ABCD2345"),
        Ok(Door {
            address: "engine.example:443".to_owned(),
            host: "engine.example".to_owned(),
            path: "/yog/register/ABCD2345".to_owned(),
        })
    );
    let ported = parse("https://engine.example:7737/yog/register/ABCD2345").expect("read");
    assert_eq!(ported.address, "engine.example:7737");
    assert_eq!(ported.host, "engine.example:7737");
}

/// An address is a host too, bracketed where it is IPv6, with or without a
/// port.
#[test]
fn an_address_is_a_host_with_or_without_a_port() {
    for (url, address) in [
        ("https://192.0.2.7/yog/register/K7M2QX22", "192.0.2.7:443"),
        (
            "https://[2001:db8::7]/yog/register/K7M2QX22",
            "[2001:db8::7]:443",
        ),
        (
            "https://[2001:db8::7]:7737/yog/register/K7M2QX22",
            "[2001:db8::7]:7737",
        ),
    ] {
        assert_eq!(parse(url).expect("read").address, address, "{url}");
    }
}

/// Everything else refuses, in one sentence that shows the shape — and
/// nothing that could be written into a request line or a header survives.
#[test]
fn anything_else_refuses_and_shows_the_shape() {
    for url in [
        "",
        "engine.example/yog/register/ABCD2345",
        "http://engine.example/yog/register/ABCD2345",
        "https://engine.example",
        "https://engine.example/",
        "https://engine.example/yog/register/",
        "https://engine.example/yog/enroll/ABCD2345",
        "https://engine.example/yog/register/ABCD/2345",
        "https://engine.example/yog/register/ABCD 2345",
        "https://engine.example/yog/register/ABCD\r\nHost: x",
        "https://engine.example:port/yog/register/ABCD2345",
        "https://not a name/yog/register/ABCD2345",
        "https:///yog/register/ABCD2345",
    ] {
        let refusal = parse(url).expect_err(url);
        assert!(refusal.contains("is not a registration URL"), "{refusal}");
        assert!(refusal.contains("https://<host>[:port]/yog/register/<code>"));
    }
}
