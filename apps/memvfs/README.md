# memvfs

Rust in-memory virtual file system with a daemon-backed CLI.

## Shape

- `memvfs-core` owns the pure in-memory model.
- `memvfsd` keeps one `FileSystem` instance alive in a daemon process.
- `memvfs` connects to the daemon over length-delimited JSON on localhost TCP.

The core storage model is split into two independent regions:

- `IndexRegion`: inode table, directory entries, permissions, timestamps, file size, and block addresses.
- `DataRegion`: fixed-size blocks, raw file bytes, and free-block allocation/recycling.

Directories live only in the index region. File bytes live only in the data region.

## Run

```powershell
cargo run -p memvfsd -- start --addr 127.0.0.1:7878
```

In another shell:

```powershell
cargo run -p memvfs-cli -- mkdir /docs
cargo run -p memvfs-cli -- write /docs/hello.txt --text "hello"
cargo run -p memvfs-cli -- read /docs/hello.txt
cargo run -p memvfs-cli -- debug inodes
cargo run -p memvfs-cli -- debug blocks
cargo run -p memvfs-cli -- statfs
cargo run -p memvfs-cli -- stop
```

## Semantics

v1 implements a core POSIX-like subset:

- directories and regular files
- inode metadata and permission bits
- open/read/write/seek/truncate/close
- mkdir/ls/stat/rename/unlink/rmdir
- block allocation, reclaim, and internal debug views

Not included in v1:

- hard links
- symlinks
- advisory locks
- real OS mount integration
- persistence after daemon exit
