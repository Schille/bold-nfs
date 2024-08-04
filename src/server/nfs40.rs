use std::io::{Read, SeekFrom};

use actix::Addr;
use async_trait::async_trait;

use super::{
    clientmanager::{
        ClientCallback, ClientManager, ConfirmClientRequest, GetCurrentFilehandleRequest,
        SetCurrentFilehandleRequest, UpsertClientRequest,
    },
    filemanager::{FileManager, GetFilehandleAttrsRequest, GetFilehandleRequest},
    NFSRequest,
};
use crate::proto::{nfs4_proto::*, rpc_proto::*};

use super::NFSProtoImpl;
use tracing::trace;

#[derive(Debug, Clone)]
pub struct NFS40Server {
    // shared state for client manager between connections
    cmanager: Addr<ClientManager>,
    // local filehandle manager
    fmanager: Addr<FileManager>,
}

impl NFS40Server {
    async fn set_client_id(&self, args: &SetClientId4args) -> Result<NfsResOp4, NfsStat4> {
        let callback = ClientCallback {
            program: args.callback.cb_program,
            rnetid: args.callback.cb_location.rnetid.clone(),
            raddr: args.callback.cb_location.raddr.clone(),
            callback_ident: args.callback_ident,
        };

        let res = self
            .cmanager
            .send(UpsertClientRequest {
                verifier: args.client.verifier,
                id: args.client.id.clone(),
                callback: callback,
                principal: None,
            })
            .await;
        match res {
            Ok(inner) => match inner {
                Ok(client) => Ok(NfsResOp4::Opsetclientid(SetClientId4res::Resok4(
                    SetClientId4resok {
                        clientid: client.clientid,
                        setclientid_confirm: client.setclientid_confirm,
                    },
                ))),
                Err(e) => Err(e.nfs_error),
            },
            Err(_) => Err(NfsStat4::Nfs4errServerfault),
        }
    }

    async fn set_client_id_confirm(
        &self,
        args: &SetClientIdConfirm4args,
    ) -> Result<NfsResOp4, NfsStat4> {
        let client_id = args.clientid;
        let setclientid_confirm = args.setclientid_confirm;

        let res = self
            .cmanager
            .send(ConfirmClientRequest {
                client_id: client_id.clone(),
                setclientid_confirm: setclientid_confirm.clone(),
                principal: None,
            })
            .await;
        match res {
            Ok(inner) => match inner {
                Ok(_) => Ok(NfsResOp4::OpsetclientidConfirm(SetClientIdConfirm4res {
                    status: NfsStat4::Nfs4Ok,
                })),
                Err(e) => Err(e.nfs_error),
            },
            Err(_) => {
                return Err(NfsStat4::Nfs4errServerfault);
            }
        }
    }

    async fn put_root_filehande(
        &self,
        request: &mut NFSRequest,
        client_addr: String,
    ) -> Result<NfsResOp4, NfsStat4> {
        let file_manager = request.file_manager();
        let resp = file_manager
            .send(GetFilehandleRequest {
                path: None,
                filehandle: None,
            })
            .await;
        match resp {
            Ok(filehandle) => {
                let resp = self
                    .cmanager
                    .send(SetCurrentFilehandleRequest {
                        client_addr: client_addr.clone(),
                        filehandle: filehandle.id.clone(),
                    })
                    .await;
                match resp {
                    Ok(_) => Ok(NfsResOp4::Opputrootfh(PutRootFh4res {
                        status: NfsStat4::Nfs4Ok,
                    })),
                    Err(e) => {
                        println!("Err {:?}", e);
                        return Err(NfsStat4::Nfs4errServerfault);
                    }
                }
            }
            Err(e) => {
                println!("Err {:?}", e);
                return Err(NfsStat4::Nfs4errServerfault);
            }
        }
    }

    async fn put_current_filehandle(
        &self,
        client_addr: String,
        filehandle: &Vec<u8>,
    ) -> Result<NfsResOp4, NfsStat4> {
        let resp = self
            .cmanager
            .send(SetCurrentFilehandleRequest {
                client_addr: client_addr.clone(),
                filehandle: filehandle.clone(),
            })
            .await;
        match resp {
            Err(e) => {
                println!("Err {:?}", e);
                return Err(NfsStat4::Nfs4errServerfault);
            }
            Ok(_) => Ok(NfsResOp4::Opputfh(PutFh4res {
                status: NfsStat4::Nfs4Ok,
            })),
        }
    }

    async fn get_current_filehandle(&self, client_addr: String) -> Result<NfsResOp4, NfsStat4> {
        let resp = self
            .cmanager
            .send(GetCurrentFilehandleRequest {
                client_addr: client_addr.clone(),
            })
            .await;
        let inner = match resp {
            Ok(inner) => inner,
            Err(e) => {
                println!("Err {:?}", e);
                return Err(NfsStat4::Nfs4errServerfault);
            }
        };
        match inner {
            Some(filehandle) => Ok(NfsResOp4::Opgetfh(GetFh4res::Resok4(GetFh4resok {
                object: filehandle,
            }))),
            // current filehandle not set for client
            None => Err(NfsStat4::Nfs4errServerfault),
        }
    }

    async fn read_directory(
        &self,
        client_addr: String,
        args: &Readdir4args,
    ) -> Result<NfsResOp4, NfsStat4> {
        let resp = self
            .cmanager
            .send(GetCurrentFilehandleRequest {
                client_addr: client_addr.clone(),
            })
            .await;
        let inner = match resp {
            Ok(inner) => inner,
            Err(e) => {
                println!("Err {:?}", e);
                return Err(NfsStat4::Nfs4errServerfault);
            }
        };
        let dir_fh = match inner {
            Some(filehandle) => {
                let resp = self
                    .fmanager
                    .send(GetFilehandleRequest {
                        path: None,
                        filehandle: Some(filehandle.clone()),
                    })
                    .await;
                match resp {
                    Ok(filehandle) => filehandle,
                    Err(e) => {
                        println!("Err {:?}", e);
                        return Err(NfsStat4::Nfs4errServerfault);
                    }
                }
            }
            None => {
                return Err(NfsStat4::Nfs4errServerfault);
            }
        };
        let dir = dir_fh.file.read_dir().unwrap();

        let mut fnames = Vec::new();
        let mut filehandles = Vec::new();
        let dircount: usize = args.dircount as usize;
        let maxcount: usize = args.maxcount as usize;
        let mut maxcount_actual: usize = 128;
        let mut dircount_actual = 0;
        // get a list of filenames and filehandles
        for (i, entry) in dir.enumerate() {
            let name = entry.filename();
            fnames.push(name.clone());
            // if the cookie value is progressed, we add only subsequent filehandles
            if i >= args.cookie as usize {
                // this is a poor man's estimation of the XRD outputs bytes, must be improved
                // we need to know the definitve size of the output of the XDR message here, but how?
                dircount_actual = dircount_actual + 8 + name.len() + 5;
                maxcount_actual = maxcount_actual + 200;
                if dircount == 0 || (dircount > dircount_actual && maxcount > maxcount_actual) {
                    let filehandle = self
                        .fmanager
                        .send(GetFilehandleRequest {
                            path: Some(entry.as_str().to_string()),
                            filehandle: None,
                        })
                        .await;
                    match filehandle {
                        Err(e) => {
                            println!("Err {:?}", e);
                            return Err(NfsStat4::Nfs4errServerfault);
                        }
                        Ok(filehandle) => {
                            filehandles.push((i + 1, filehandle));
                        }
                    }
                }
            }
        }

        // get a seed of this directory, concat all files names
        let seed: String = fnames
            .iter()
            .map(|s| s.as_str().chars().collect::<Vec<_>>())
            .flatten()
            .collect();
        // take only every nth char to create a cookie verifier
        let mut cookieverf = seed
            .as_bytes()
            .into_iter()
            .step_by(seed.len() / 8 + 1)
            .map(|k| k.clone())
            .collect::<Vec<_>>();
        if args.cookie != 0 && cookieverf != args.cookieverf {
            return Err(NfsStat4::Nfs4errNotSame);
        }

        // if this directory is empty, we can't create a cookie verifier based on the dir contents
        // setting it to a default value
        if cookieverf.len() == 0 {
            cookieverf = [0u8; 8].to_vec();
        } else if cookieverf.len() < 8 {
            let mut diff = 8 - cookieverf.len();
            while diff > 0 {
                cookieverf.push(0);
                diff -= 1;
            }
        }

        let mut tnextentry = None;
        let mut added_entries = 0;
        for (cookie, fh) in filehandles.into_iter().rev() {
            let resp = self
                .fmanager
                .send(GetFilehandleAttrsRequest {
                    filehandle: fh.id.clone(),
                    attrs_request: args.attr_request.clone(),
                })
                .await;
            let (answer_attrs, attrs) = match resp {
                Ok(inner) => *inner,
                Err(e) => {
                    println!("Err {:?}", e);
                    return Err(NfsStat4::Nfs4errServerfault);
                }
            };

            let entry = Entry4 {
                name: fh.file.filename(),
                cookie: cookie as u64,
                attrs: Fattr4 {
                    attrmask: answer_attrs,
                    attr_vals: attrs,
                },
                nextentry: if tnextentry.is_some() {
                    Some(Box::new(tnextentry.unwrap()))
                } else {
                    None
                },
            };
            added_entries += 1;
            tnextentry = Some(entry);
        }
        let eof = {
            if tnextentry.is_some()
                && (tnextentry.clone().unwrap().cookie + added_entries) >= fnames.len() as u64
            {
                true
            } else if tnextentry.is_none() {
                true
            } else {
                false
            }
        };

        Ok(NfsResOp4::Opreaddir(ReadDir4res::Resok4(ReadDir4resok {
            reply: DirList4 {
                // len: if tnextentry.is_some() { 1 } else { 0 },
                entries: tnextentry.clone(),
                eof: eof,
            },
            cookieverf: cookieverf.as_slice().try_into().unwrap(),
        })))
    }

    async fn lookup(&self, client_addr: String, args: &Lookup4args) -> Result<NfsResOp4, NfsStat4> {
        let resp = self
            .cmanager
            .send(GetCurrentFilehandleRequest {
                client_addr: client_addr.clone(),
            })
            .await;
        let filehandle = match resp {
            Ok(inner) => {
                let resp = self
                    .fmanager
                    .send(GetFilehandleRequest {
                        path: None,
                        filehandle: inner,
                    })
                    .await;
                match resp {
                    Ok(filehandle) => filehandle,
                    Err(e) => {
                        println!("Err {:?}", e);
                        return Err(NfsStat4::Nfs4errServerfault);
                    }
                }
            }
            Err(e) => {
                println!("Err {:?}", e);
                return Err(NfsStat4::Nfs4errServerfault);
            }
        };
        let mut path = filehandle.path.clone();
        if path == "/" {
            path.push_str(args.objname.as_str());
        } else {
            path.push_str("/");
            path.push_str(args.objname.as_str());
        }

        println!("lookup {:?}", path);

        let resp = self
            .fmanager
            .send(GetFilehandleRequest {
                filehandle: None,
                path: Some(path),
            })
            .await;
        let filehandle = match resp {
            Ok(filehandle) => filehandle,
            Err(e) => {
                println!("Err {:?}", e);
                return Err(NfsStat4::Nfs4errServerfault);
            }
        };

        // lookup sets the current filehandle to the looked up filehandle
        let resp = self
            .cmanager
            .send(SetCurrentFilehandleRequest {
                client_addr: client_addr.clone(),
                filehandle: filehandle.id.clone(),
            })
            .await;
        match resp {
            Err(e) => {
                println!("Err {:?}", e);
                return Err(NfsStat4::Nfs4errServerfault);
            }
            Ok(_) => {}
        }

        Ok(NfsResOp4::Oplookup(Lookup4res {
            status: NfsStat4::Nfs4Ok,
        }))
    }

    async fn get_current_filehandle_attrs(
        &self,
        client_addr: String,
        args: &Getattr4args,
    ) -> Result<NfsResOp4, NfsStat4> {
        let resp = self
            .cmanager
            .send(GetCurrentFilehandleRequest {
                client_addr: client_addr.clone(),
            })
            .await;
        let filehandle = match resp {
            Ok(inner) => inner,
            Err(e) => {
                println!("Err {:?}", e);
                return Err(NfsStat4::Nfs4errServerfault);
            }
        };
        match filehandle {
            None => {
                println!("None filehandle");
                return Err(NfsStat4::Nfs4errServerfault);
            }
            Some(filehandle) => {
                let resp = self
                    .fmanager
                    .send(GetFilehandleAttrsRequest {
                        filehandle: filehandle.clone(),
                        attrs_request: args.attr_request.clone(),
                    })
                    .await;
                let (answer_attrs, attrs) = match resp {
                    Ok(inner) => *inner,
                    Err(e) => {
                        println!("Err {:?}", e);
                        return Err(NfsStat4::Nfs4errServerfault);
                    }
                };

                Ok(NfsResOp4::Opgetattr(Getattr4res::Resok4(Getattr4resok {
                    obj_attributes: Fattr4 {
                        attrmask: answer_attrs,
                        attr_vals: attrs,
                    },
                })))
            }
        }
    }
}

#[async_trait]
impl NFSProtoImpl for NFS40Server {
    fn new(cmanager: Addr<ClientManager>, fmanager: Addr<FileManager>) -> Self {
        NFS40Server { cmanager, fmanager }
    }

    fn hash(&self) -> u64 {
        0
    }

    async fn null(&self, _: CallBody, _: NFSRequest) -> ReplyBody {
        ReplyBody::MsgAccepted(AcceptedReply {
            verf: OpaqueAuth::AuthNull(Vec::<u8>::new()),
            reply_data: AcceptBody::Success(Compound4res {
                status: NfsStat4::Nfs4Ok,
                tag: "".to_string(),
                resarray: Vec::new(),
            }),
        })
    }

    async fn compound(&self, msg: CallBody, request: NFSRequest) -> ReplyBody {
        trace!("Call body: {:?} for {:?}", msg, request);
        let res = match &msg.args {
            Some(args) => {
                let mut resarray = Vec::with_capacity(args.argarray.len());
                for arg in &args.argarray {
                    match arg {
                        NfsArgOp::Opsetclientid(args) => match self.set_client_id(&args).await {
                            Ok(res) => resarray.push(res),
                            Err(e) => {
                                return ReplyBody::MsgAccepted(AcceptedReply {
                                    verf: OpaqueAuth::AuthNull(Vec::<u8>::new()),
                                    reply_data: AcceptBody::Success(Compound4res {
                                        status: e,
                                        tag: "".to_string(),
                                        resarray: Vec::new(),
                                    }),
                                });
                            }
                        },
                        NfsArgOp::OpUndef0 => todo!(),
                        NfsArgOp::OpUndef1 => todo!(),
                        NfsArgOp::OpUndef2 => todo!(),
                        NfsArgOp::OpAccess(args) => {
                            resarray.push(NfsResOp4::OpAccess(Access4res::Resok4(Access4resok {
                                supported: ACCESS4_READ
                                    | ACCESS4_LOOKUP
                                    | ACCESS4_MODIFY
                                    | ACCESS4_EXTEND
                                    | ACCESS4_DELETE
                                    | ACCESS4_EXECUTE,
                                access: args.access,
                            })));
                        }
                        NfsArgOp::Opclose(_) => {
                            resarray.push(NfsResOp4::Opclose(Close4res::OpenStateid(Stateid4 {
                                seqid: 0,
                                other: [0; 12],
                            })));
                        }
                        NfsArgOp::Opcommit(_) => todo!(),
                        NfsArgOp::Opcreate(_) => todo!(),
                        NfsArgOp::Opdelegpurge(_) => todo!(),
                        NfsArgOp::Opdelegreturn(_) => todo!(),
                        NfsArgOp::Opgetattr(args) => {
                            match self
                                .get_current_filehandle_attrs(client_addr.clone(), args)
                                .await
                            {
                                Ok(res) => resarray.push(res),
                                Err(e) => {
                                    return ReplyBody::MsgAccepted(AcceptedReply {
                                        verf: OpaqueAuth::AuthNull(Vec::<u8>::new()),
                                        reply_data: AcceptBody::Success(Compound4res {
                                            status: e,
                                            tag: "".to_string(),
                                            resarray: Vec::new(),
                                        }),
                                    });
                                }
                            }
                        }
                        NfsArgOp::Opgetfh(_) => {
                            match self.get_current_filehandle(client_addr.clone()).await {
                                Ok(res) => resarray.push(res),
                                Err(e) => {
                                    return ReplyBody::MsgAccepted(AcceptedReply {
                                        verf: OpaqueAuth::AuthNull(Vec::<u8>::new()),
                                        reply_data: AcceptBody::Success(Compound4res {
                                            status: e,
                                            tag: "".to_string(),
                                            resarray: Vec::new(),
                                        }),
                                    });
                                }
                            }
                        }
                        NfsArgOp::Oplink(_) => todo!(),
                        NfsArgOp::Oplock(_) => todo!(),
                        NfsArgOp::Oplockt(_) => todo!(),
                        NfsArgOp::Oplocku(_) => todo!(),
                        NfsArgOp::Oplookup(args) => {
                            match self.lookup(client_addr.clone(), args).await {
                                Ok(res) => resarray.push(res),
                                Err(e) => {
                                    return ReplyBody::MsgAccepted(AcceptedReply {
                                        verf: OpaqueAuth::AuthNull(Vec::<u8>::new()),
                                        reply_data: AcceptBody::Success(Compound4res {
                                            status: e,
                                            tag: "".to_string(),
                                            resarray: Vec::new(),
                                        }),
                                    });
                                }
                            }
                        }
                        NfsArgOp::Oplookupp(_) => todo!(),
                        NfsArgOp::Opnverify(_) => todo!(),
                        NfsArgOp::Opopen(args) => {
                            // open sets the current filehandle to the looked up filehandle
                            let fh = match self
                                .cmanager
                                .send(GetCurrentFilehandleRequest {
                                    client_addr: client_addr.clone(),
                                })
                                .await
                            {
                                Ok(inner) => {
                                    let resp = self
                                        .fmanager
                                        .send(GetFilehandleRequest {
                                            path: None,
                                            filehandle: inner,
                                        })
                                        .await;
                                    match resp {
                                        Ok(filehandle) => filehandle,
                                        Err(e) => {
                                            println!("Err {:?}", e);
                                            return ReplyBody::MsgAccepted(AcceptedReply {
                                                verf: OpaqueAuth::AuthNull(Vec::<u8>::new()),
                                                reply_data: AcceptBody::Success(Compound4res {
                                                    status: NfsStat4::Nfs4errServerfault,
                                                    tag: "".to_string(),
                                                    resarray: Vec::new(),
                                                }),
                                            });
                                        }
                                    }
                                }
                                Err(e) => {
                                    println!("Err {:?}", e);
                                    return ReplyBody::MsgAccepted(AcceptedReply {
                                        verf: OpaqueAuth::AuthNull(Vec::<u8>::new()),
                                        reply_data: AcceptBody::Success(Compound4res {
                                            status: NfsStat4::Nfs4errServerfault,
                                            tag: "".to_string(),
                                            resarray: Vec::new(),
                                        }),
                                    });
                                }
                            };
                            let path = fh.path.clone();
                            let file = &args.claim;

                            match file {
                                // this is open for reading
                                OpenClaim4::File(file) => {
                                    let fh_path = {
                                        if path == "/" {
                                            format!("{}{}", path, file)
                                        } else {
                                            format!("{}/{}", path, file)
                                        }
                                    };

                                    println!("## open {:?}", fh_path);
                                    let filehandle = match self
                                        .fmanager
                                        .send(GetFilehandleRequest {
                                            path: Some(fh_path.clone()),
                                            filehandle: None,
                                        })
                                        .await
                                    {
                                        Ok(filehandle) => filehandle,
                                        Err(e) => {
                                            println!("Err {:?}", e);
                                            return ReplyBody::MsgAccepted(AcceptedReply {
                                                verf: OpaqueAuth::AuthNull(Vec::<u8>::new()),
                                                reply_data: AcceptBody::Success(Compound4res {
                                                    status: NfsStat4::Nfs4errServerfault,
                                                    tag: "".to_string(),
                                                    resarray: Vec::new(),
                                                }),
                                            });
                                        }
                                    };

                                    let _ = match self
                                        .cmanager
                                        .send(SetCurrentFilehandleRequest {
                                            client_addr: client_addr.clone(),
                                            filehandle: filehandle.id.clone(),
                                        })
                                        .await
                                    {
                                        Ok(_) => {}
                                        Err(e) => {
                                            println!("Err {:?}", e);
                                            return ReplyBody::MsgAccepted(AcceptedReply {
                                                verf: OpaqueAuth::AuthNull(Vec::<u8>::new()),
                                                reply_data: AcceptBody::Success(Compound4res {
                                                    status: NfsStat4::Nfs4errServerfault,
                                                    tag: "".to_string(),
                                                    resarray: Vec::new(),
                                                }),
                                            });
                                        }
                                    };

                                    resarray.push(NfsResOp4::Opopen(Open4res::Resok4(Open4resok {
                                        stateid: Stateid4 {
                                            seqid: 0,
                                            other: [0; 12],
                                        },
                                        cinfo: ChangeInfo4 {
                                            atomic: false,
                                            before: 0,
                                            after: 0,
                                        },
                                        rflags: OPEN4_RESULT_CONFIRM,
                                        attrset: Vec::new(),
                                        delegation: OpenDelegation4::None,
                                    })))
                                }
                                // everything else is not supported
                                _ => {
                                    todo!()
                                }
                            }
                        }
                        NfsArgOp::Opopenattr(_) => todo!(),
                        NfsArgOp::OpopenConfirm(_) => {
                            resarray.push(NfsResOp4::OpopenConfirm(OpenConfirm4res::Resok4(
                                OpenConfirm4resok {
                                    open_stateid: Stateid4 {
                                        seqid: 0,
                                        other: [0; 12],
                                    },
                                },
                            )));
                        }
                        NfsArgOp::OpopenDowngrade(_) => todo!(),
                        NfsArgOp::Opputfh(args) => {
                            match self
                                .put_current_filehandle(client_addr.clone(), &args.object)
                                .await
                            {
                                Ok(res) => resarray.push(res),
                                Err(e) => {
                                    return ReplyBody::MsgAccepted(AcceptedReply {
                                        verf: OpaqueAuth::AuthNull(Vec::<u8>::new()),
                                        reply_data: AcceptBody::Success(Compound4res {
                                            status: e,
                                            tag: "".to_string(),
                                            resarray: Vec::new(),
                                        }),
                                    });
                                }
                            }
                        }
                        NfsArgOp::Opputpubfh(_) => todo!(),
                        NfsArgOp::Opputrootfh(_) => {
                            match self.put_root_filehande(client_addr.clone()).await {
                                Ok(res) => resarray.push(res),
                                Err(e) => {
                                    return ReplyBody::MsgAccepted(AcceptedReply {
                                        verf: OpaqueAuth::AuthNull(Vec::<u8>::new()),
                                        reply_data: AcceptBody::Success(Compound4res {
                                            status: e,
                                            tag: "".to_string(),
                                            resarray: Vec::new(),
                                        }),
                                    });
                                }
                            }
                        }
                        NfsArgOp::Opread(args) => {
                            let fh = match self
                                .cmanager
                                .send(GetCurrentFilehandleRequest {
                                    client_addr: client_addr.clone(),
                                })
                                .await
                            {
                                Ok(inner) => {
                                    let resp = self
                                        .fmanager
                                        .send(GetFilehandleRequest {
                                            path: None,
                                            filehandle: inner,
                                        })
                                        .await;
                                    match resp {
                                        Ok(filehandle) => filehandle,
                                        Err(e) => {
                                            println!("Err {:?}", e);
                                            return ReplyBody::MsgAccepted(AcceptedReply {
                                                verf: OpaqueAuth::AuthNull(Vec::<u8>::new()),
                                                reply_data: AcceptBody::Success(Compound4res {
                                                    status: NfsStat4::Nfs4errServerfault,
                                                    tag: "".to_string(),
                                                    resarray: Vec::new(),
                                                }),
                                            });
                                        }
                                    }
                                }
                                Err(e) => {
                                    println!("Err {:?}", e);
                                    return ReplyBody::MsgAccepted(AcceptedReply {
                                        verf: OpaqueAuth::AuthNull(Vec::<u8>::new()),
                                        reply_data: AcceptBody::Success(Compound4res {
                                            status: NfsStat4::Nfs4errServerfault,
                                            tag: "".to_string(),
                                            resarray: Vec::new(),
                                        }),
                                    });
                                }
                            };

                            let mut buffer: Vec<u8> = vec![0; args.count as usize];
                            let mut rfile = fh.file.open_file().unwrap();
                            rfile.seek(SeekFrom::Start(args.offset)).unwrap();
                            let _ = rfile.read_exact(&mut buffer);

                            resarray.push(NfsResOp4::Opread(Read4res::Resok4(Read4resok {
                                eof: true,
                                data: buffer,
                            })));
                        }
                        NfsArgOp::Opreaddir(args) => {
                            match self.read_directory(client_addr.clone(), &args).await {
                                Ok(res) => resarray.push(res),
                                Err(e) => {
                                    return ReplyBody::MsgAccepted(AcceptedReply {
                                        verf: OpaqueAuth::AuthNull(Vec::<u8>::new()),
                                        reply_data: AcceptBody::Success(Compound4res {
                                            status: e,
                                            tag: "".to_string(),
                                            resarray: Vec::new(),
                                        }),
                                    });
                                }
                            }
                        }
                        NfsArgOp::Opreadlink(_) => todo!(),
                        NfsArgOp::Opremove(_) => todo!(),
                        NfsArgOp::Oprename(_) => todo!(),
                        NfsArgOp::Oprenew(_) => {
                            resarray.push(NfsResOp4::Oprenew(Renew4res {
                                status: NfsStat4::Nfs4Ok,
                            }));
                        }
                        NfsArgOp::Oprestorefh(_) => todo!(),
                        NfsArgOp::Opsavefh(_) => todo!(),
                        NfsArgOp::OpSecinfo(_) => todo!(),
                        NfsArgOp::Opsetattr(_) => todo!(),
                        NfsArgOp::OpsetclientidConfirm(args) => {
                            match self.set_client_id_confirm(&args).await {
                                Ok(res) => resarray.push(res),
                                Err(e) => {
                                    return ReplyBody::MsgAccepted(AcceptedReply {
                                        verf: OpaqueAuth::AuthNull(Vec::<u8>::new()),
                                        reply_data: AcceptBody::Success(Compound4res {
                                            status: e,
                                            tag: "".to_string(),
                                            resarray: Vec::new(),
                                        }),
                                    });
                                }
                            }
                        }
                        NfsArgOp::Opverify(_) => todo!(),
                        NfsArgOp::Opwrite(_) => todo!(),
                        NfsArgOp::OpreleaseLockOwner(_) => todo!(),
                    }
                }
                resarray
            }
            None => Vec::new(),
        };

        ReplyBody::MsgAccepted(AcceptedReply {
            verf: OpaqueAuth::AuthNull(Vec::<u8>::new()),
            reply_data: AcceptBody::Success(Compound4res {
                status: NfsStat4::Nfs4Ok,
                tag: "".to_string(),
                resarray: res,
            }),
        })
    }

    fn minor_version(&self) -> u32 {
        0
    }
}
