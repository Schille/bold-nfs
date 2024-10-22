use multi_index_map::MultiIndexMap;

pub type LockingStateDb = MultiIndexLockingStateMap;

#[derive(Debug, Clone)]
pub enum LockType {
    Open,
    ByteRange,
}

#[derive(MultiIndexMap, Debug, Clone)]
#[multi_index_derive(Debug, Clone)]
pub struct LockingState {
    // https://datatracker.ietf.org/doc/html/rfc7530#section-9.1.4
    // When the server grants a lock of any type (including opens,
    // byte-range locks, and delegations), it responds with a unique stateid
    // that represents a set of locks (often a single lock) for the same
    // file, of the same type, and sharing the same ownership
    // characteristics.
    pub stateid: [u8; 12],
    pub seqid: u32,
    // clientid:
    // The clientid of the client that created the open stateid.
    #[multi_index(hashed_non_unique)]
    pub client_id: u64,
    // owner:
    // The owner of the lock.
    #[multi_index(hashed_non_unique)]
    pub owner: Vec<u8>,
    // lock_type:
    // The type of lock being requested.
    pub lock_type: LockType,
    // filehandle:
    // The filehandle of the file on which the lock is being requested.
    #[multi_index(hashed_non_unique)]
    pub filehandle_id: Vec<u8>,
    // start:
    // The starting offset of the lock. (byte-range locks only)
    pub start: Option<u64>,
    // length:
    // The length of the lock. (byte-range locks only)
    pub length: Option<u64>,
    // https://datatracker.ietf.org/doc/html/rfc7530#section-9.9
    // Share Reservations
    // share_access:
    // The access mode that is requested by the client for the share
    // reservation.
    // OPEN4_SHARE_ACCESS_READ | OPEN4_SHARE_ACCESS_WRITE | OPEN4_SHARE_ACCESS_BOTH
    pub share_access: Option<u32>,
    // share_deny:
    // The deny mode that is requested by the client for the share
    // reservation.
    // OPEN4_SHARE_DENY_NONE | OPEN4_SHARE_DENY_READ | OPEN4_SHARE_DENY_WRITE | OPEN4_SHARE_DENY_BOTH
    pub share_deny: Option<u32>,
}

impl LockingState {
    pub fn new_shared_reservation(
        filehandle_id: Vec<u8>,
        stateid: [u8; 12],
        client_id: u64,
        owner: Vec<u8>,
        share_access: u32,
        share_deny: u32,
    ) -> Self {
        LockingState {
            stateid,
            seqid: 1,
            client_id,
            owner,
            lock_type: LockType::Open,
            filehandle_id,
            start: None,
            length: None,
            share_access: Some(share_access),
            share_deny: Some(share_deny),
        }
    }
}
