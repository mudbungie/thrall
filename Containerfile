# thrall as an OCI image — the unit of install, and nothing more.
#
# The image is a DEPLOYMENT artifact. Nothing in thrall uses the container
# filesystem as a feature, no state lives in a layer, and the container is not
# a containment boundary: a foot runs what its operator's tool document says to
# run, on a machine that operator administers. Read the README section "The
# image" for the mount contract this file implements.
#
# Two stages. The build stage is the pinned toolchain and the C toolchain
# `ring` needs; the runtime stage is a base with a shell, carrying the one
# static binary. Nothing from the build stage survives into it.

# ---------------------------------------------------------------------------
# Stage 1 — build, under the toolchain rust-toolchain.toml pins.
#
# `rust:<pin>-alpine` and not `-slim-bookworm`, because the host target of the
# alpine image IS `x86_64-unknown-linux-musl`: the release binary comes out
# statically linked with no cross-compilation setup and no `--target` flag to
# keep in step with anything. The tag is digest-pinned so a rebuild of this
# file resolves the same bytes; the tag beside it is for a human reading the
# line.
FROM docker.io/library/rust:1.95.0-alpine3.22@sha256:064dfc925d68d1a63f4fd2871bd7dc6e6ea56692989a487185855d62885d90aa AS build

# `musl-dev` is not optional and not incidental: `rustls` is linked with the
# `ring` provider (Cargo.toml's approved set says why `ring` and not
# `aws-lc-rs`), and ring compiles C. This is the one C toolchain the build
# needs and the runtime stage carries none of it.
RUN apk add --no-cache musl-dev

WORKDIR /src

# The toolchain pin has ONE home — rust-toolchain.toml — and the `FROM` line
# above is a second statement of the same fact, so it can drift. This makes the
# drift a build failure instead of a silent difference between what the gate
# compiles and what the image ships.
#
# It is copied to /pin and not to the build directory on purpose: a
# rust-toolchain.toml in the working directory sends every later `cargo` and
# `rustc` through rustup's shim, which would try to DOWNLOAD the toolchain and
# its `components` list into an image that already has the compiler. The check
# reads the file; the build never sees it.
COPY rust-toolchain.toml /pin/rust-toolchain.toml
RUN set -eu; \
    pin=$(sed -n 's/^channel *= *"\([^"]*\)".*/\1/p' /pin/rust-toolchain.toml); \
    have=$(rustc --version | cut -d' ' -f2); \
    if [ "$pin" != "$have" ]; then \
      echo "Containerfile: base image rustc $have, rust-toolchain.toml pins $pin" >&2; \
      echo "  bump the FROM tag and its digest in lockstep with the pin" >&2; \
      exit 1; \
    fi

# `--locked` for the same reason the gate uses it: the committed Cargo.lock is
# the dependency answer, and a build that is allowed to solve for a different
# one is not the build the gate judged.
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --locked

# ---------------------------------------------------------------------------
# Stage 2 — runtime.
#
# THE RUNTIME BASE IS A DECISION, and this is its reasoning.
#
# A binary that execs nothing can ship `FROM scratch`, and a statically linked
# musl thrall is exactly that binary — right up until it does the one thing it
# exists for. A foot runs operator-configured argv: what a thrall executes is
# named in `tools.json` on the box, is not knowable from this repo, and is
# routinely a shell line. `scratch` would ship a foot that answers `--version`
# and then fails every invocation it was installed to serve, with an ENOENT
# that names the operator's own tool rather than the layer that cannot hold it.
#
# So: alpine, which is a shell (busybox `sh`), a package manager the operator
# can add their tools with, and about 8 MB. The image carries the foot and the
# floor it stands on; WHAT IT CAN RUN IS STILL THE OPERATOR'S PROBLEM. A tool
# document naming a binary this layer does not have is a tool this box does not
# have, exactly as the README's honesty clause already says — the image does
# not change that rule, it just makes the floor visible.
#
# `ca-certificates` is here for the same reason: an operator-enabled tool that
# speaks HTTPS needs system roots, and thrall's own mTLS channel does not (it
# trusts the operator-issued material under the data root and nothing else).
# The roots are for the tools, not for the wire.
FROM docker.io/library/alpine:3.22@sha256:14358309a308569c32bdc37e2e0e9694be33a9d99e68afb0f5ff33cc1f695dce

RUN apk add --no-cache ca-certificates

COPY --from=build /src/target/release/thrall /usr/local/bin/thrall

# THE MOUNT CONTRACT. XDG is the runtime contract and the image carries no
# state, so this sets the variable and provisions nothing under it. thrall's
# data root is `$XDG_DATA_HOME/thrall`, which makes it `/state/thrall` here —
# the extra level is XDG's, not the image's: `XDG_DATA_HOME` is a parent of
# per-application roots by definition and an image does not get to collapse it.
#
# Mount the operator's provisioned data root at /state/thrall. It holds
# `tools.json` and `wire/workspaces/<leaf>/`, both put there by hand, neither
# ever written by thrall, and NEITHER IN THIS IMAGE — a certificate baked into
# a layer is a certificate published to everyone who can pull it.
#
# There is no VOLUME instruction. A VOLUME would make an unmounted run succeed
# against an empty anonymous volume; without one it refuses and names the file
# it could not find, which is the answer an operator can act on.
ENV XDG_DATA_HOME=/state

ENTRYPOINT ["/usr/local/bin/thrall"]
CMD ["run"]
