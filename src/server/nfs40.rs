use std::{
    alloc::System, env::Args, fs::File, io::Read, process, sync::{Arc, Mutex, MutexGuard}
};

use serde_xdr::to_writer;
use tracing_subscriber::field::debug;
use vfs::VfsPath;

use super::{
    clientmanager::{ClientCallback, ClientManager},
    filemanager::FileManager,
};
use crate::{proto::nfs4_proto::*, proto::rpc_proto::*};

use super::NFSProtoImpl;
use tracing::{debug, trace};

#[derive(Debug, Clone)]
pub struct NFS40Server {
    // shared state for client manager between connections
    cmanager: Arc<Mutex<ClientManager>>,
    // local filehandle manager
    fmanager: Arc<Mutex<FileManager>>,
}

impl NFS40Server {
    fn set_client_id(&mut self, args: &SetClientId4args) -> Result<NfsResOp4, NfsStat4> {
        let callback = ClientCallback {
            program: args.callback.cb_program,
            rnetid: args.callback.cb_location.rnetid.clone(),
            raddr: args.callback.cb_location.raddr.clone(),
            callback_ident: args.callback_ident,
        };

        let mut cmanager = self.cmanager.lock().unwrap();
        let res =
            cmanager.upsert_client(args.client.verifier, args.client.id.clone(), callback, None);
        match res {
            Ok(client) => Ok(NfsResOp4::Opsetclientid(SetClientId4res::Resok4(
                SetClientId4resok {
                    clientid: client.clientid,
                    setclientid_confirm: client.setclientid_confirm,
                },
            ))),
            Err(e) => Err(e.nfs_error),
        }
    }

    fn set_client_id_confirm(
        &mut self,
        args: &SetClientIdConfirm4args,
    ) -> Result<NfsResOp4, NfsStat4> {
        let client_id = args.clientid;
        let setclientid_confirm = args.setclientid_confirm;
        let cmanager = self.cmanager.lock();
        match cmanager {
            Ok(mut cmanager) => {
                let res = cmanager.confirm_client(client_id, setclientid_confirm, None);
                match res {
                    Ok(_) => Ok(NfsResOp4::OpsetclientidConfirm(SetClientIdConfirm4res {
                        status: NfsStat4::Nfs4Ok,
                    })),
                    Err(e) => Err(e.nfs_error),
                }
            }
            Err(e) => {
                return Err(NfsStat4::Nfs4errServerfault);
            }
        }
    }

    fn put_root_filehande(&mut self) -> Result<NfsResOp4, NfsStat4> {
        let fmanager = self.fmanager.lock();
        match fmanager {
            Err(e) => {
                println!("Err {:?}", e);
                return Err(NfsStat4::Nfs4errServerfault);
            }
            Ok(mut fmanager) => {
                let mut cdb = fmanager.db.clone();
                fmanager.reset_current_fh_to_root(&mut cdb);

                Ok(NfsResOp4::Opputrootfh(PutRootFh4res {
                    status: NfsStat4::Nfs4Ok,
                }))
            }
        }
    }

    fn put_current_filehandle(&mut self, filehandle: &Vec<u8>) -> Result<NfsResOp4, NfsStat4> {
        let fmanager = self.fmanager.lock();
        match fmanager {
            Err(e) => {
                println!("Err {:?}", e);
                return Err(NfsStat4::Nfs4errServerfault);
            }
            Ok(mut fmanager) => {
                let mut cbd = fmanager.db.clone();
                let fh = fmanager.set_current_fh(filehandle, &mut cbd);
                if fh.is_err() {
                    return Err(NfsStat4::Nfs4errBadhandle);
                }
                Ok(NfsResOp4::Opputfh(PutFh4res {
                    status: NfsStat4::Nfs4Ok,
                }))
            }
        }
    }

    fn get_current_filehandle(&mut self) -> Result<NfsResOp4, NfsStat4> {
        let fmanager = self.fmanager.lock();
        match fmanager {
            Err(e) => {
                println!("Err {:?}", e);
                return Err(NfsStat4::Nfs4errServerfault);
            }
            Ok(mut fmanager) => {
                let fileid = fmanager.current_fh_id();

                Ok(NfsResOp4::Opgetfh(GetFh4res::Resok4(GetFh4resok {
                    object: fileid.clone(),
                })))
            }
        }
    }

    fn read_directory(&mut self, args: &Readdir4args) -> Result<NfsResOp4, NfsStat4> {
        let fmanager = self.fmanager.lock();
        match fmanager {
            Err(e) => {
                return Err(NfsStat4::Nfs4errServerfault);
            }
            Ok(mut fmanager) => {
                
                let cfmanger = fmanager.clone();
                let dir_fh = cfmanger.current_fh.as_ref().unwrap();
                let dir = dir_fh.file.read_dir().unwrap();
    
                
                let mut fnames = Vec::new();
                let mut filehandles = Vec::new();
                let dircount: usize = args.dircount as usize;
                let maxcount: usize = args.maxcount as usize;
                let mut maxcount_actual: usize = 128;
                let mut dircount_actual = 0;
                let cfmanger = fmanager.clone();
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
                            filehandles.push((i + 1, cfmanger.get_filehandle(&entry, &mut fmanager.db)));
                        }
                    }
                }
                
                
                // get a seed of this directory, concat all files names
                let seed: String = fnames.iter().map(|s| s.as_str().chars().collect::<Vec<_>>()).flatten().collect();
                // take only every nth char to create a cookie verifier
                let mut cookieverf = seed.as_bytes().into_iter().step_by(seed.len() / 8 + 1).map(|k| k.clone()).collect::<Vec<_>>();
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
                let mut cfmanger = fmanager.clone();
                let mut added_entries = 0;
                for (cookie, fh) in filehandles.into_iter().rev() {
                    let _ = cfmanger.set_current_fh(&fh.id, &mut fmanager.db);
                    let (answer_attrs, attrs) = cfmanger.filehandle_attrs(&args.attr_request);
                    let entry = Entry4 {
                        name: fh.file.filename(),
                        cookie: cookie as u64,
                        attrs: Fattr4 {
                            attrmask: answer_attrs,
                            attr_vals: attrs,
                        },
                        nextentry: if tnextentry.is_some() { Some(Box::new(tnextentry.unwrap())) } else { None },
                    };
                    added_entries += 1;
                    tnextentry = Some(entry);
                }
                let eof = {
                    if tnextentry.is_some() && (tnextentry.clone().unwrap().cookie + added_entries) >= fnames.len() as u64 { 
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
        }
    }


    fn lookup(&mut self, args: &Lookup4args) -> Result<NfsResOp4, NfsStat4> {
        let fmanager = self.fmanager.lock();
        match fmanager {
            Err(e) => {
                println!("Err {:?}", e);
                return Err(NfsStat4::Nfs4errServerfault);
            }
            Ok(mut fmanager) => {
                let mut cfmanager = fmanager.clone();
                let mut path = fmanager.current_fh.as_ref().unwrap().path.clone();
                if path == "/" {
                    path.push_str(args.objname.as_str());
                } else {
                    path.push_str("/");
                    path.push_str(args.objname.as_str());
                }

                println!("lookup {:?}", path);

                let filehandle = FileManager::get_filehandle_by_path(&path, &mut cfmanager.db);
                if filehandle.is_none() {
                    return Err(NfsStat4::Nfs4errNoent);
                }
                // lookup sets the current filehandle to the looked up filehandle
                let _ = fmanager.set_current_fh(&filehandle.unwrap().id, &mut cfmanager.db);

                Ok(NfsResOp4::Oplookup(Lookup4res {
                    status: NfsStat4::Nfs4Ok
                }))
            }
        }
    }

    fn get_current_filehandle_attrs(&mut self, args: &Getattr4args) -> Result<NfsResOp4, NfsStat4> {
        let fmanager = self.fmanager.lock();
        match fmanager {
            Err(e) => {
                println!("Err {:?}", e);
                return Err(NfsStat4::Nfs4errServerfault);
            }
            Ok(fmanager) => {
                let (answer_attrs, attrs) = fmanager.filehandle_attrs(&args.attr_request);

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

impl NFSProtoImpl for NFS40Server {
    fn new(root: VfsPath) -> Self {
        NFS40Server {
            cmanager: Arc::new(Mutex::new(ClientManager::new())),
            fmanager: Arc::new(Mutex::new(FileManager::new(root, None))),
        }
    }

    fn hash(&self) -> u64 {
        0
    }

    fn null(&self, _: &CallBody) -> ReplyBody {
        ReplyBody::MsgAccepted(AcceptedReply {
            verf: OpaqueAuth::AuthNull(Vec::<u8>::new()),
            reply_data: AcceptBody::Success(Compound4res {
                status: NfsStat4::Nfs4Ok,
                tag: "".to_string(),
                resarray: Vec::new(),
            }),
        })
    }

    fn compound(&mut self, msg: &CallBody) -> ReplyBody {
        trace!("Call body: {:?}", msg);
        let res = match &msg.args {
            Some(args) => {
                let mut resarray = Vec::with_capacity(args.argarray.len());
                for arg in &args.argarray {
                    match arg {
                        NfsArgOp::Opsetclientid(args) => match self.set_client_id(&args) {
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
                        NfsArgOp::Opclose(_) => todo!(),
                        NfsArgOp::Opcommit(_) => todo!(),
                        NfsArgOp::Opcreate(_) => todo!(),
                        NfsArgOp::Opdelegpurge(_) => todo!(),
                        NfsArgOp::Opdelegreturn(_) => todo!(),
                        NfsArgOp::Opgetattr(args) => {
                            match self.get_current_filehandle_attrs(args) {
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
                        NfsArgOp::Opgetfh(_) => match self.get_current_filehandle() {
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
                        NfsArgOp::Oplink(_) => todo!(),
                        NfsArgOp::Oplock(_) => todo!(),
                        NfsArgOp::Oplockt(_) => todo!(),
                        NfsArgOp::Oplocku(_) => todo!(),
                        NfsArgOp::Oplookup(args) => {
                            match self.lookup(args) {
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
                        },
                        NfsArgOp::Oplookupp(_) => todo!(),
                        NfsArgOp::Opnverify(_) => todo!(),
                        NfsArgOp::Opopen(args) => {
                            let fmanager = self.fmanager.lock();
                            match fmanager {
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
                                Ok(mut fmanager) => {
                                    let mut cfmanager = fmanager.clone();
                                    let mut path = fmanager.current_fh.as_ref().unwrap().path.clone();
                                    let file = args.claim;
                                    match file {
                                        // this is open for reading
                                        OpenClaim4::File(file) => {
                                            Ok(NfsResOp4::Opopen(Open4res::Resok4(Open4resok {
                                                stateid: Stateid4 {
                                                    seqid: 0,
                                                    other: 0,
                                                },
                                                cinfo: ChangeInfo4 {
                                                    atomic: false,
                                                    before: 0,
                                                    after: 0,

                                                },
                                                rflags: OPEN4_RESULT_CONFIRM,
                                                attrset: Vec::new(),
                                                delegation: OpenDelegation4::Read(OpenReadDelegation4 {
                                                    stateid: Stateid4 {
                                                        seqid: 0,
                                                        other: 0,
                                                    },
                                                    recall: false,
                                                    permissions: Nfsace4 {
                                                        acetype: 0,
                                                        flag: 0,
                                                        access_mask: 0,
                                                        who: "".to_string(),
                                                    },
                                                }),
                                            }))
                                            );
                                        },
                                        // everything else is not supported
                                        _ => {
                                            todo!()
                                        }
                                    }
                                }
                            }
                            
                        },
                        NfsArgOp::Opopenattr(_) => todo!(),
                        NfsArgOp::OpopenConfirm(_) => todo!(),
                        NfsArgOp::OpopenDowngrade(_) => todo!(),
                        NfsArgOp::Opputfh(args) => {
                            match self.put_current_filehandle(&args.object) {
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
                        NfsArgOp::Opputrootfh(_) => match self.put_root_filehande() {
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
                        NfsArgOp::Opread(_) => todo!(),
                        NfsArgOp::Opreaddir(args) => {
                            match self.read_directory(&args) {
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
                        },
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
                            match self.set_client_id_confirm(&args) {
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
