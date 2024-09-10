# Bold - a Network File System (NFS) server

This is **Bold**. A version [4-series compatible NFS](https://en.wikipedia.org/wiki/Network_File_System#NFSv4) server written in async Rust based on [Tokio](https://docs.rs/tokio/latest/tokio/).

> NFS is the most widely used network file system for production server workloads.

(This is a **Bold** claim)

## Goals
Bold's design goals are:
1) simplicity 
2) robustness 
3) portability
4) extendability
5) performance

Anti goals include complicated compile-time computations, such as macro or type trickery.

## Demo and testing
There is a crate `bold-mem` (binary), which reads in a
YAML file and serves this as in-memory file system.

On Linux:

```yaml
# bold-mem/memoryfs.yaml
name: 
contents:
  - !Dir
    name: home
    contents:
      - !Dir
        name: user
        contents:
          - !File
            name: file1
            contents: |
              This is the content of file1
          - !File
            name: file2
            contents: |
              This is the content of file2
```

You can compile and run it from the repo:
1) `cargo build -p bold-mem`
2) `./target/debug/bold-mem bold-mem/memoryfs.yaml`  
(optionally, you can enable the `--debug` flag)

3) Open another terminal
4) `mkdir /tmp/demo`
5) `sudo mount.nfs4 -n -v -o fg,soft,sec=none,vers=4.0,port=11112 127.0.0.1:/ /tmp/demo`
6) `ls /tmp/demo/`, `cat /tmp/demo/home/user/file1`  
(have a look around in your mounted file system)
7) Don't forget to unmount `sudo umount /tmp/demo`, before stopping `bold-mem`

## State of implementation

### Version 4.0

- **WIP**

### Version 4.1

- **Not started**

### Version 4.2 

- **Not started**


## Importand RFC for the implementation
- [XDR: External Data Representation Standard](https://datatracker.ietf.org/doc/html/rfc4506)
- [Network File System (NFS) Version 4 Protocol](https://datatracker.ietf.org/doc/html/rfc7530)
- [Network File System (NFS) Version 4 Minor Version 1 Protocol](https://datatracker.ietf.org/doc/html/rfc5661)
- [Network File System (NFS) Version 4 Minor Version 2 Protocol](https://datatracker.ietf.org/doc/html/rfc7862)

## License
Distributed under the MIT license. See `LICENSE` for more information.

