mod fixtures;
mod read;
mod write;

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use base64::prelude::*;
    use bytes::Bytes;
    use hex_literal::hex;
    use onc_rpc::{auth::AuthFlavor, RpcMessage};

    use crate::{
        from_bytes,
        nfs4::xdr::{utf8str_cs, COMPOUND4args},
        rpc_proto::RpcCallMsg,
    };

    const WRITE_REQ: [u8; 1168] = hex!(
        "8000048c8d90f0a90000000000000002000186a3000000040000000100000000000000000000000000000000000000000000000000000003000000160000001a8000000000671ea47e000000000000000000000000000000650100000000002600000001000000000000000000000064000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000009000000020000001800300000"
    );

    #[test]
    fn read_fastxdr() {
        let now = Instant::now();

        for _ in 1..=500000 {
            let msg = RpcMessage::try_from(WRITE_REQ.as_ref()).expect("failed to parse message");
            let ops_bytes = msg.call_body().unwrap().payload();
            let compound_args = COMPOUND4args::try_from(Bytes::from(*ops_bytes));
            if compound_args.is_err() {
                println!("{:#?}", compound_args);
            }
            for op in compound_args.unwrap().argarray {
                match op {
                    crate::nfs4::xdr::nfs_argop4::OP_GETATTR(getattr4args) => {}
                    crate::nfs4::xdr::nfs_argop4::OP_PUTFH(putfh4args) => {}
                    crate::nfs4::xdr::nfs_argop4::OP_WRITE(write4args) => {
                        match write4args.stable {
                            crate::nfs4::xdr::stable_how4::UNSTABLE4 => {}
                            crate::nfs4::xdr::stable_how4::DATA_SYNC4 => panic!(),
                            crate::nfs4::xdr::stable_how4::FILE_SYNC4 => panic!(),
                        }
                        assert_eq!(write4args.data.len(), 1024);
                    }
                    _ => {
                        panic!("unexpected operation")
                    }
                }
            }
        }
        println!("Runtime fastxdr {}", now.elapsed().as_micros());
    }

    #[test]
    fn read_serde() {
        // let buffer = WRITE_REQ[4..].to_vec();
        let now = Instant::now();
        for _ in 1..=500000 {
            let msg = RpcCallMsg::from_bytes(WRITE_REQ[4..].to_vec()).unwrap();

            match msg.body {
                crate::rpc_proto::MsgType::Call(call_body) => {
                    let compound_args = call_body.args.unwrap();
                    for op in compound_args.argarray {
                        match op {
                            crate::nfs4_proto::NfsArgOp::Opgetattr(getattr4args) => {}
                            crate::nfs4_proto::NfsArgOp::Opputfh(put_fh4args) => {}
                            crate::nfs4_proto::NfsArgOp::Opwrite(write4args) => {
                                match write4args.stable {
                                    crate::nfs4_proto::StableHow4::Unstable4 => {}
                                    crate::nfs4_proto::StableHow4::DataSync4 => todo!(),
                                    crate::nfs4_proto::StableHow4::FileSync4 => todo!(),
                                }
                                assert_eq!(write4args.data.len(), 1024);
                            }
                            _ => {
                                panic!("unexpected operation")
                            }
                        }
                    }
                }
                crate::rpc_proto::MsgType::Reply(reply_body) => todo!(),
            }
        }

        println!("Runtime Serde-XDR {}", now.elapsed().as_micros());
    }
}
