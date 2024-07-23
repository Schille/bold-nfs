extern crate serde;
extern crate serde_bytes;
extern crate serde_derive;
extern crate serde_xdr;

use serde_derive::{Deserialize, Serialize};

use super::{
    from_bytes,
    nfs4_proto::{Compound4args, Compound4res},
    to_bytes,
};

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct AuthUnix {
    pub stamp: u64,
    pub machinename: String,
    pub uid: u32,
    pub gid: u32,
    pub gids: Vec<u32>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[repr(u32)]
pub enum OpaqueAuth {
    AuthNull(Vec<u8>) = 0,
    AuthUnix(AuthUnix) = 1,
    // not supported
    AuthShort = 2,
    AuthDes = 3,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CallBody {
    pub rpcvers: u32,
    pub prog: u32,
    pub vers: u32,
    pub proc: u32,
    pub cred: OpaqueAuth,
    pub verf: OpaqueAuth,

    #[serde(deserialize_with = "read_compound_args")]
    pub args: Option<Compound4args>,
}

fn read_compound_args<'de, D>(deserializer: D) -> Result<Option<Compound4args>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let args = <Compound4args as serde::Deserialize>::deserialize(deserializer);
    match args {
        Ok(args) => Ok(Some(args)),
        Err(_e) => {
            println!("Error deserializing compound args: {:?}", _e);
            Ok(None)
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[repr(u32)]
pub enum MsgType {
    Call(CallBody) = 0,
    Reply(ReplyBody) = 1,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcceptedReply {
    pub verf: OpaqueAuth,
    pub reply_data: AcceptBody,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MismatchInfo {
    low: u32,
    high: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[repr(u32)]
pub enum ReplyBody {
    MsgAccepted(AcceptedReply) = 0,
    MsgDenied(RejectedReply) = 1,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[repr(u32)]
pub enum AcceptBody {
    Success(Compound4res) = 0,
    ProgUnavail = 1,
    /// remote can't support version #
    ProgMismatch(MismatchInfo) = 2,
    ProcUnavail = 3,
    /// procedure can't decode params
    GarbageArgs = 4,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[repr(u32)]
pub enum RejectedReply {
    RpcMismatch(MismatchInfo) = 0,
    AuthError(AuthStat) = 1,
}

#[derive(Debug, Clone, Deserialize, Default, Serialize)]
#[repr(u32)]
///   Why authentication failed
pub enum AuthStat {
    UndefAuthCred = 0,
    /// bad credentials (seal broken)
    #[default]
    AuthBadCred = 1,
    /// client must begin new session
    AuthRejectedCred = 2,
    /// bad verifier (seal broken)    
    AuthBadverf = 3,
    /// verifier expired or replayed  
    AuthRejectedverf = 4,
    /// rejected for security reasons
    AuthTooWeak = 5,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RpcCallMsg {
    pub xid: u32,
    pub body: MsgType,
}

impl RpcCallMsg {
    pub fn from_bytes(buffer: Vec<u8>) -> Result<Self, anyhow::Error> {
        from_bytes(buffer)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct RpcCompoundCallMsg {
    pub xid: u32,
    pub body: MsgType,
}

#[derive(Debug, Serialize)]
pub struct RpcReplyMsg {
    pub xid: u32,
    pub body: MsgType,
}

impl RpcReplyMsg {
    pub fn to_bytes(&self) -> Result<Vec<u8>, anyhow::Error> {
        let result = to_bytes(self);
        match result {
            Ok(bytes) => Ok(bytes),
            Err(e) => Err(anyhow::anyhow!("Error serializing message: {:?}", e)),
        }
    }
}
