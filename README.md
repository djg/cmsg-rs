# cmsg

[![Build Status](https://travis-ci.org/djg/cmsg-rs.svg?branch=master)](https://travis-ci.org/djg/cmsg-rs)

A library for working with ancilliary control messages for Unix Domain Sockets as described by [cmsg].

[cmsg]: http://man7.org/linux/man-pages/man3/cmsg.3.html

```toml
# Cargo.toml
[dependencies]
bytes = "0.4"
cmsg = "0.1"
```

## Usage

To process received control messages, `cmsg::iterator()` is used to
create an iterator adapter over the raw bytes received from
`libc::recvmsg`.

To create control messages, `cmsg::builder()` is used to create a
builder interface over the top of pre-allocated storage. Using
`Vec<u8>`, `bytes::BytesMut`, and stack-based arrays, via
`std::io::Cursor` are supported. The builder checks that the storages
is correctly aligned and maintains alignment of each appended message.

# License

`cmsg-rs` is primarily distributed under the terms of the MIT license.

See LICENSE-MIT for details.

