//! **A capture is text** (DESIGN §6.5), one test per row of the rule.

use super::{answered, result};
use serde_json::json;

/// **Text parts, in order, each on its own line.** The order is the server's
/// and this end does not sort, join or re-wrap it.
#[test]
fn text_parts_come_back_in_order_each_on_its_own_line() {
    let capture = answered(&result(json!({"content": [
        {"type": "text", "text": "first"},
        {"type": "text", "text": "second"},
    ]})));
    assert_eq!(capture.stdout, "first\nsecond\n");
    assert_eq!(capture.stderr, "");
    assert_eq!(capture.exit_code, 0);
}

/// **A part that is not text is one line naming what was dropped** — its type,
/// its mime type and the bytes it carried, which are counted off the base64
/// rather than decoded because only the number is wanted. A part carrying only
/// a link carried no bytes, and the line says zero rather than inventing a
/// second sentence for it.
#[test]
fn a_part_that_is_not_text_is_replaced_by_one_line_naming_it() {
    let capture = answered(&result(json!({"content": [
        {"type": "image", "data": "aGVsbG8=", "mimeType": "image/png"},
        {"type": "resource", "resource": {"uri": "file:///x",
             "mimeType": "text/x-rust", "text": "fn main"}},
        {"type": "resource_link", "uri": "file:///y", "mimeType": "text/plain"},
        {"data": "AAAA"},
    ]})));
    assert_eq!(
        capture.stdout,
        "[image image/png, 5 bytes dropped]\n\
         [resource text/x-rust, 7 bytes dropped]\n\
         [resource_link text/plain, 0 bytes dropped]\n\
         [part unknown, 3 bytes dropped]\n"
    );
    assert_eq!(capture.exit_code, 0);
}

/// **`structuredContent` is the answer when no text part is.**
#[test]
fn structured_content_stands_in_when_the_result_carried_no_text() {
    let structured = json!({"pages": 2, "title": "a document"});
    let capture = answered(&result(
        json!({"content": [], "structuredContent": structured.clone()}),
    ));
    assert_eq!(capture.stdout, format!("{structured}\n"));
    assert_eq!(capture.exit_code, 0);
}

/// **And it is not written beside one.** A server that sent both sent the same
/// fact twice, and the text half is the half a model was meant to read.
#[test]
fn structured_content_is_not_written_beside_a_text_part() {
    let capture = answered(&result(json!({
        "content": [{"type": "text", "text": "the page"}],
        "structuredContent": {"page": "the page"},
    })));
    assert_eq!(capture.stdout, "the page\n");
}

/// **`isError` writes the content and exits 1.** The content is still the
/// answer; the code is what says the tool failed.
#[test]
fn an_error_result_still_writes_its_content_and_earns_a_failing_code() {
    let capture = answered(&result(json!({
        "content": [{"type": "text", "text": "the site refused the request"}],
        "isError": true,
    })));
    assert_eq!(capture.stdout, "the site refused the request\n");
    assert_eq!(capture.exit_code, 1);
}

/// **A result that said nothing is a refusal**, because a capture of empty
/// bytes and a zero exit code is the silent success a tool contract must never
/// produce.
#[test]
fn a_result_carrying_neither_content_nor_structured_content_is_refused() {
    let capture = answered(&result(json!({})));
    assert_eq!(capture.exit_code, 1);
    assert!(capture.stderr.contains("it did not answer"), "{capture:?}");
    assert_eq!(capture.stdout, "");
}

/// A result of the wrong shape is refused rather than read forgivingly: these
/// are instructions about what to say to a model, not observations.
#[test]
fn a_result_of_the_wrong_shape_is_refused_naming_the_shape() {
    for (body, said) in [
        (json!({"content": "a page"}), "\"content\" is not an array"),
        (json!("a page"), "result is not a JSON object"),
    ] {
        let capture = answered(&result(body.clone()));
        assert_eq!(capture.exit_code, 1, "{body}");
        assert!(capture.stderr.contains(said), "{body}: {capture:?}");
    }
}
