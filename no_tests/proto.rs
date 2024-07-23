mod messages;

#[cfg(test)]
mod tests {
    use std::error::Error;

    use base64::prelude::*;
    use nfsv4::{nfsv4::MsgType, proto::{from_bytes, rpc_proto::OpaqueAuth}};
    use crate::messages::nfsv40;
    


    #[test]
    fn connect_seq_1() -> Result<(), anyhow::Error> {
        
        let msg = from_bytes(BASE64_STANDARD.decode(&nfsv40::MSG_SEQ1_BASE64)?).unwrap();
        println!("MSG = {:?}", msg);

        assert_eq!(msg.xid, 2767052173);
        match msg.body {
            MsgType::Call(call) => {
                assert_eq!(call.rpcvers, 2);
                assert_eq!(call.prog, 100003);
                assert_eq!(call.vers, 4);
                assert_eq!(call.proc, 0);
            },
            _ => return Err(anyhow::anyhow!("Expected Call message"))
        }

        Ok(())
        
        
    }
    
    #[test]
    fn connect_seq_2() -> Result<(), anyhow::Error> {
        
        let msg = from_bytes(BASE64_STANDARD.decode(&nfsv40::MSG_SEQ2_BASE64)?).unwrap();
        println!("MSG = {:?}", msg);

        assert_eq!(msg.xid, 2399376590);
        match msg.body {
            MsgType::Call(call) => {
                assert_eq!(call.rpcvers, 2);
                assert_eq!(call.prog, 100003);
                assert_eq!(call.vers, 4);
                assert_eq!(call.proc, 1);
                match call.cred {
                    OpaqueAuth::AuthUnix(auth) => {
                        assert_eq!(auth.machinename, "LAPTOP-1QQBPDGM");
                    },
                    _ => return Err(anyhow::anyhow!("Expected credentials"))
                }
                match call.args {
                    Some(args) => {
                        assert_eq!(args.tag, "");
                        assert_eq!(args.minor_version, 0);
                        assert_eq!(args.argarray.len(), 1);
                    },
                    None => return Err(anyhow::anyhow!("Expected arguments"))
                }
            },
            _ => return Err(anyhow::anyhow!("Expected Call message"))
        }
        Ok(())
    }
}