#[cfg(test)]
mod test_serialize {
    use bytes::Bytes;
    use onc_rpc::RpcMessage;

    use crate::{nfs4_proto::xdr::COMPOUND4args, tests::fixtures::WRITE_REQ};

    #[test]
    fn compound_write_1024_bytes() {
        let msg = RpcMessage::try_from(WRITE_REQ.as_ref()).expect("failed to parse message");
        let ops_bytes = msg.call_body().unwrap().payload();
        println!("{:?}", msg.xid());
        // println!("{:?}", msg.message());
        // println!("{:?}", ops_bytes);
        let compound_args = COMPOUND4args::try_from(Bytes::from(*ops_bytes));
        println!("{:?}", compound_args);
        for op in compound_args.unwrap().argarray {
            match op {
                crate::nfs4_proto::xdr::nfs_argop4::OP_GETATTR(getattr4args) => {}
                crate::nfs4_proto::xdr::nfs_argop4::OP_PUTFH(putfh4args) => {}
                crate::nfs4_proto::xdr::nfs_argop4::OP_WRITE(write4args) => {
                    match write4args.stable {
                        crate::nfs4_proto::xdr::stable_how4::UNSTABLE4 => {}
                        crate::nfs4_proto::xdr::stable_how4::DATA_SYNC4 => panic!(),
                        crate::nfs4_proto::xdr::stable_how4::FILE_SYNC4 => panic!(),
                    }
                    assert_eq!(write4args.data.len(), 1024);
                }
                _ => {
                    panic!("unexpected operation")
                }
            }
        }
    }
}
