//! What is written to the door, and how its answer is judged.

use super::{CAP, accepted, post, request};
use crate::channel::material::ANCHORS;
use crate::channel::tls;
use crate::register::url;
use crate::test_support::door::{self, Door};
use crate::test_support::{Scratch, mint};

/// The request is the door's one line, a `Content-Length` it requires, and
/// the body — nothing chunked, and the connection is asked to close.
#[test]
fn the_request_is_one_line_four_headers_and_the_body() {
    let door = url::parse("https://engine.example/yog/register/ABCD2345").expect("read");
    assert_eq!(
        request(&door, "{\"k\":1}"),
        "POST /yog/register/ABCD2345 HTTP/1.1\r\nHost: engine.example\r\n\
         Content-Type: application/json\r\nContent-Length: 7\r\n\
         Connection: close\r\n\r\n{\"k\":1}"
    );
}

/// A `200` answers its body; every other status, and anything that is not a
/// reply at all, is the one answer "not accepted".
#[test]
fn only_a_200_is_accepted() {
    assert_eq!(accepted(&door::ok("{}")), Some("{}".to_owned()));
    for reply in [
        door::not_found().as_str(),
        "HTTP/1.1 500 Internal Server Error\r\n\r\nno",
        "HTTP/1.1\r\n\r\n",
        "HTTP/1.1 200 OK\r\nContent-Length: 2",
        "",
    ] {
        assert_eq!(accepted(reply), None, "{reply:?}");
    }
}

/// **The reply is bounded.** A door that never stops talking is read up to
/// the cap and no further, and what was read is what is judged.
#[test]
fn a_reply_that_never_ends_is_read_up_to_the_cap() {
    let scratch = Scratch::new();
    mint::material(scratch.path());
    let flood = "x".repeat(4 * usize::try_from(CAP).expect("fits"));
    let door = Door::start(scratch.path(), Box::new(move |_| door::ok(&flood)));
    let roots = tls::anchors(&scratch.join(ANCHORS)).expect("the root");
    let read = post(&url::parse(&door.url).expect("read"), "{}", roots)
        .expect("posted")
        .expect("a 200");
    assert!(read.len() < usize::try_from(CAP).expect("fits"), "bounded");
    assert!(read.starts_with("xxxx"));
}
