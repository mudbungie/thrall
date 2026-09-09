//! **The seven shapes a foot speaks, stamped** — `corpus/shapes.json`'s
//! `signature` entries for this crate's subset, copied key for key.
//!
//! It is a file of its own because it is a file of DATA: a re-vendor rewrites
//! all of it and none of [`ledger`](super), so the diff of a re-vendor is the
//! whole file and reads as one.

use super::Shape;

/// **The seven shapes a foot speaks**, which is the whole of its surface: the
/// three gestures it may send and the four answers they can earn.
///
/// yog's ledger stamps 124 shapes; REMOTE §3's concession is that *"the foot's
/// surface is small enough that it may consume only the subset of shapes it
/// speaks"*, and this is that subset. A shape absent here is a parity fact —
/// a frame this foot never asks for — never a frame it cannot read (§3.2: *"the
/// parity ledger records what the client does not RENDER, never what it
/// refuses"*).
pub(crate) const SHAPES: [Shape; 7] = [
    Shape {
        name: "request/advertise",
        signature: &[
            ("/op:string", 2),
            ("/tools/[]/description:string", 2),
            (
                "/tools/[]/input_schema/properties/command/minLength:number",
                2,
            ),
            ("/tools/[]/input_schema/properties/command/type:string", 2),
            ("/tools/[]/input_schema/properties/command:object", 2),
            ("/tools/[]/input_schema/properties:object", 2),
            ("/tools/[]/input_schema/required/[]:string", 2),
            ("/tools/[]/input_schema/required:array", 2),
            ("/tools/[]/input_schema/type:string", 2),
            ("/tools/[]/input_schema:object", 2),
            ("/tools/[]/name:string", 2),
            ("/tools/[]/subject_cwd:bool", 2),
            ("/tools/[]:object", 2),
            ("/tools:array", 2),
            (":object", 2),
        ],
    },
    Shape {
        name: "request/complete",
        signature: &[
            ("/capture/exit_code:number", 1),
            ("/capture/stderr:string", 1),
            ("/capture/stdout:string", 1),
            ("/capture:object", 1),
            ("/invocation:string", 1),
            ("/op:string", 1),
            (":object", 1),
        ],
    },
    Shape {
        name: "request/invocations",
        signature: &[("/op:string", 1), (":object", 1)],
    },
    Shape {
        name: "reply/advertised",
        signature: &[
            ("/kind:string", 8),
            ("/ok:bool", 8),
            ("/wrote:bool", 8),
            (":object", 8),
        ],
    },
    Shape {
        name: "reply/invocations",
        signature: &[
            ("/kind:string", 2),
            ("/ok:bool", 2),
            ("/rows/[]/cwd:string", 2),
            ("/rows/[]/input/command:string", 2),
            ("/rows/[]/input/timeout:number", 2),
            ("/rows/[]/input:object", 2),
            ("/rows/[]/invocation:string", 2),
            ("/rows/[]/tool:string", 2),
            ("/rows/[]:object", 2),
            ("/rows:array", 2),
            (":object", 2),
        ],
    },
    Shape {
        name: "reply/refusal",
        signature: &[("/error:string", 1), ("/ok:bool", 1), (":object", 1)],
    },
    Shape {
        name: "reply/routed",
        signature: &[
            ("/capture/exit_code:number", 1),
            ("/capture/stderr:string", 1),
            ("/capture/stdout:string", 1),
            ("/capture:object", 1),
            ("/invocation:string", 1),
            ("/kind:string", 1),
            ("/ok:bool", 1),
            (":object", 1),
        ],
    },
];
