# bold-nfs

This is Bold. A Version 4-series compatible NFS server written in Rust.

**This is very much WIP.** It doesn't do anything useful at the moment. The todo list is miles long.

## Suported Operations
A list of all support NFSv4 operations:

**Todo**

## Importand RFS for the implementation
- [XDR: External Data Representation Standard](https://datatracker.ietf.org/doc/html/rfc4506)
- [Network File System (NFS) Version 4 Protocol](https://datatracker.ietf.org/doc/html/rfc7530)
- [Network File System (NFS) Version 4 Minor Version 1 Protocol](https://datatracker.ietf.org/doc/html/rfc5661)
- [Network File System (NFS) Version 4 Minor Version 2 Protocol](https://datatracker.ietf.org/doc/html/rfc7862)


## Demo

1. Clone Repo
2. `cargo run`

3. In another terminal run 
`mkdir /tmp/demo`


4. Mount _this_ project directory under _/tmp/demo_ `sudo mount.nfs4 -vv -o fg,sec=none,vers=4.0,port=11112 127.0.0.1:/ /tmp/demo`

5. Check out `ls /tmp/demo`


