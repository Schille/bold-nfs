use actix::prelude::*;
use multi_index_map::MultiIndexMap;
use rand::distributions::Uniform;
use rand::Rng;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use crate::proto::nfs4_proto::NfsStat4;

type ClientDb = MultiIndexClientEntryMap;

#[derive(Debug, Clone)]
pub struct ClientManager {
    db: Arc<ClientDb>,
    client_id_seq: u64,
    filehandles: HashMap<String, Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct ClientCallback {
    pub program: u32,
    pub rnetid: String,
    pub raddr: String,
    pub callback_ident: u32,
}

// https://datatracker.ietf.org/doc/html/rfc7530#section-16.33.5

#[derive(MultiIndexMap, Debug, Clone)]
#[multi_index_derive(Debug, Clone)]
pub struct ClientEntry {
    // https://datatracker.ietf.org/doc/html/rfc7530#section-3.3.3
    #[multi_index(hashed_non_unique)]
    pub principal: Option<String>,
    #[multi_index(hashed_non_unique)]
    pub verifier: [u8; 8],
    #[multi_index(hashed_non_unique)]
    pub id: String,
    #[multi_index(hashed_non_unique)]
    pub clientid: u64,
    pub callback: ClientCallback,
    #[multi_index(hashed_unique)]
    pub setclientid_confirm: [u8; 8],
    pub confirmed: bool,
}

impl Actor for ClientManager {
    type Context = Context<Self>;
}

#[derive(Message)]
#[rtype(result = "Result<ClientEntry, ClientManagerError>")]
pub struct UpsertClientRequest {
    pub verifier: [u8; 8],
    pub id: String,
    pub callback: ClientCallback,
    pub principal: Option<String>,
}

#[derive(Message)]
#[rtype(result = "Result<ClientEntry, ClientManagerError>")]
pub struct ConfirmClientRequest {
    pub client_id: u64,
    pub setclientid_confirm: [u8; 8],
    pub principal: Option<String>,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct SetCurrentFilehandleRequest {
    pub client_addr: String,
    pub filehandle: Vec<u8>,
}

#[derive(Message)]
#[rtype(result = "Option<Vec<u8>>")]
pub struct GetCurrentFilehandleRequest {
    pub client_addr: String,
}

impl ClientManager {
    pub fn new() -> Self {
        ClientManager {
            db: ClientDb::default().into(),
            client_id_seq: 0,
            filehandles: HashMap::new(),
        }
    }

    fn get_next_client_id(&mut self) -> u64 {
        self.client_id_seq += 1;
        self.client_id_seq
    }

    fn set_current_fh(&mut self, client_addr: String, filehandle: Vec<u8>) {
        self.filehandles.insert(client_addr, filehandle);
    }

    fn get_current_fh(&mut self, client_addr: String) -> Option<Vec<u8>> {
        match self.filehandles.get(&client_addr) {
            Some(fh) => Some(fh.clone()),
            None => None,
        }
    }

    fn upsert_client(
        &mut self,
        verifier: [u8; 8],
        id: String,
        callback: ClientCallback,
        principal: Option<String>,
    ) -> Result<ClientEntry, ClientManagerError> {
        let db = Arc::get_mut(&mut self.db).unwrap();
        let entries = db.get_by_id(&id);
        let mut existing_clientid: Option<u64> = None;
        if !entries.is_empty() {
            // this is an update attempt
            let mut entries_to_remove = Vec::new();
            for entry in entries.clone() {
                if entry.confirmed == true && entry.principal != principal {
                    // For any confirmed record with the same id string x, if the recorded principal does
                    // not match that of the SETCLIENTID call, then the server returns an
                    // NFS4ERR_CLID_INUSE error.
                    return Err(ClientManagerError {
                        nfs_error: NfsStat4::Nfs4errClidInuse,
                    });
                }
                if entry.confirmed == false {
                    entries_to_remove.push(entry.clone());
                }
                existing_clientid = Some(entry.clientid);
            }

            entries_to_remove.iter().for_each(|entry| {
                db.remove_by_setclientid_confirm(&entry.setclientid_confirm);
            });
        }

        Ok(self.add_client_record(verifier, id, callback, principal, existing_clientid))
    }

    fn add_client_record(
        &mut self,
        verifier: [u8; 8],
        id: String,
        callback: ClientCallback,
        principal: Option<String>,
        client_id: Option<u64>,
    ) -> ClientEntry {
        let client_id = client_id.unwrap_or_else(|| self.get_next_client_id());
        let mut rng = rand::thread_rng();
        // generate a random 8 byte array
        let setclientid_confirm_vec: Vec<u8> =
            (0..8).map(|_| rng.sample(Uniform::new(0, 255))).collect();
        let setclientid_confirm: [u8; 8] = setclientid_confirm_vec.try_into().unwrap();
        let client = ClientEntry {
            principal: principal,
            verifier: verifier,
            id: id,
            clientid: client_id,
            callback: callback,
            setclientid_confirm: setclientid_confirm,
            confirmed: false,
        };

        let db = Arc::get_mut(&mut self.db).unwrap();
        db.insert(client.clone());
        client
    }

    fn confirm_client(
        &mut self,
        client_id: u64,
        setclientid_confirm: [u8; 8],
        principal: Option<String>,
    ) -> Result<ClientEntry, ClientManagerError> {
        let db = Arc::get_mut(&mut self.db).unwrap();

        let entries = db.get_by_clientid(&client_id);
        let mut old_confirmed: Option<ClientEntry> = None;
        let mut new_confirmed: Option<ClientEntry> = None;
        if entries.is_empty() {
            // nothing to confirm
            return Err(ClientManagerError {
                nfs_error: NfsStat4::Nfs4errStaleClientid,
            });
        }

        for entry in entries {
            if entry.principal != principal {
                // For any confirmed record with the same id string x, if the recorded principal does
                // not match that of the SETCLIENTID call, then the server returns an
                // NFS4ERR_CLID_INUSE error.
                return Err(ClientManagerError {
                    nfs_error: NfsStat4::Nfs4errClidInuse,
                });
            }
            if entry.confirmed == true && entry.setclientid_confirm != setclientid_confirm {
                old_confirmed = Some(entry.clone());
            }
            if entry.setclientid_confirm == setclientid_confirm {
                let mut update_entry = entry.clone();
                update_entry.confirmed = true;
                new_confirmed = Some(update_entry);
            }
        }

        if old_confirmed.is_some() {
            db.remove_by_setclientid_confirm(&(old_confirmed.unwrap().setclientid_confirm));
        }

        match new_confirmed {
            Some(new_confirmed) => {
                db.modify_by_setclientid_confirm(&new_confirmed.setclientid_confirm, |c| {
                    c.confirmed = true;
                });
                Ok(new_confirmed)
            }
            None => Err(ClientManagerError {
                nfs_error: NfsStat4::Nfs4errStaleClientid,
            }),
        }
    }

    pub fn get_record_count(&mut self) -> usize {
        let db = Arc::get_mut(&mut self.db).unwrap();
        db.len()
    }

    pub fn remove_client(&mut self, client_id: u64) {
        let db = Arc::get_mut(&mut self.db).unwrap();
        db.remove_by_clientid(&client_id);
    }

    pub fn get_client_confirmed(&mut self, clientid: u64) -> Option<&ClientEntry> {
        let db = Arc::get_mut(&mut self.db).unwrap();
        let records = db.get_by_clientid(&clientid);
        let _match = records.iter().find(|r| r.confirmed == true);
        match _match {
            Some(ref record) => Some(*record),
            None => None,
        }
    }
}

impl Handler<UpsertClientRequest> for ClientManager {
    type Result = Result<ClientEntry, ClientManagerError>;

    fn handle(&mut self, msg: UpsertClientRequest, _ctx: &mut Context<Self>) -> Self::Result {
        self.upsert_client(msg.verifier, msg.id, msg.callback, msg.principal)
    }
}

impl Handler<ConfirmClientRequest> for ClientManager {
    type Result = Result<ClientEntry, ClientManagerError>;

    fn handle(&mut self, msg: ConfirmClientRequest, _ctx: &mut Context<Self>) -> Self::Result {
        self.confirm_client(msg.client_id, msg.setclientid_confirm, msg.principal)
    }
}

impl Handler<SetCurrentFilehandleRequest> for ClientManager {
    type Result = ();

    fn handle(
        &mut self,
        msg: SetCurrentFilehandleRequest,
        _ctx: &mut Context<Self>,
    ) -> Self::Result {
        self.set_current_fh(msg.client_addr, msg.filehandle)
    }
}

impl Handler<GetCurrentFilehandleRequest> for ClientManager {
    type Result = Option<Vec<u8>>;

    fn handle(
        &mut self,
        msg: GetCurrentFilehandleRequest,
        _ctx: &mut Context<Self>,
    ) -> Self::Result {
        self.get_current_fh(msg.client_addr)
    }
}

#[derive(Debug, Clone)]
pub struct ClientManagerError {
    pub nfs_error: NfsStat4,
}

impl fmt::Display for ClientManagerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ClientManagerError: {:?}", self.nfs_error)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{Arc, Mutex},
        time::Instant,
    };

    use rand::{distributions::Alphanumeric, Rng};

    use crate::{
        proto::nfs4_proto::NfsStat4,
        server::clientmanager::{ClientCallback, ClientManager},
    };

    #[test]
    fn test_upsert_clients_no_principals() {
        let mut manager = super::ClientManager::new();

        let verifier = [0; 8];
        let id = "test".to_string();
        let callback = super::ClientCallback {
            program: 0,
            rnetid: "tcp".to_string(),
            raddr: "".to_string(),
            callback_ident: 0,
        };

        let client = manager
            .upsert_client(verifier, id.clone(), callback.clone(), None)
            .unwrap();
        assert_eq!(client.id, id);
        assert_eq!(client.verifier, verifier);
        assert_eq!(client.callback, callback);

        let updated_callback = super::ClientCallback {
            program: 10,
            rnetid: "tcp".to_string(),
            raddr: "".to_string(),
            callback_ident: 2,
        };

        let same_client = manager
            .upsert_client(verifier, id.clone(), updated_callback.clone(), None)
            .unwrap();
        assert_eq!(same_client.id, id);
        assert_eq!(same_client.verifier, verifier);
        assert_eq!(same_client.callback, updated_callback);
        assert_eq!(same_client.clientid, client.clientid);

        // confirm after update
        let err_confirm = manager.confirm_client(client.clientid, client.setclientid_confirm, None);
        assert_eq!(
            err_confirm.unwrap_err().nfs_error,
            NfsStat4::Nfs4errStaleClientid
        );

        let confirmed_client = manager
            .confirm_client(client.clientid, same_client.setclientid_confirm, None)
            .unwrap();
        assert_eq!(confirmed_client.confirmed, true);
        assert_eq!(confirmed_client.clientid, client.clientid);

        let other_callback = super::ClientCallback {
            program: 1,
            rnetid: "tcp".to_string(),
            raddr: "".to_string(),
            callback_ident: 0,
        };
        let err_client = manager.upsert_client(
            verifier,
            id,
            other_callback.clone(),
            Some("LINUX".to_string()),
        );
        assert_eq!(
            err_client.unwrap_err().nfs_error,
            NfsStat4::Nfs4errClidInuse
        );

        let stale_client = manager.confirm_client(1234, client.setclientid_confirm, None);
        assert_eq!(
            stale_client.unwrap_err().nfs_error,
            NfsStat4::Nfs4errStaleClientid
        );

        let confirmed = manager.get_client_confirmed(client.clientid);
        assert_eq!(confirmed.unwrap().clientid, client.clientid);
        assert!(confirmed.unwrap().confirmed);

        let c = manager.get_record_count();
        assert_eq!(c, 1);
        manager.remove_client(client.clientid);
        let c = manager.get_record_count();
        assert_eq!(c, 0);
    }

    #[test]
    fn test_upsert_clients_double_confirm() {
        let mut manager = super::ClientManager::new();

        let verifier = [0; 8];
        let id = "test".to_string();
        let callback = super::ClientCallback {
            program: 0,
            rnetid: "tcp".to_string(),
            raddr: "".to_string(),
            callback_ident: 0,
        };

        let client = manager
            .upsert_client(verifier, id.clone(), callback.clone(), None)
            .unwrap();

        let confirmed_client = manager
            .confirm_client(client.clientid, client.setclientid_confirm, None)
            .unwrap();
        assert_eq!(confirmed_client.confirmed, true);
        assert_eq!(confirmed_client.clientid, client.clientid);
        let confirmed_client = manager
            .confirm_client(client.clientid, client.setclientid_confirm, None)
            .unwrap();
        assert_eq!(confirmed_client.confirmed, true);
        assert_eq!(confirmed_client.clientid, client.clientid);
    }

    #[test]
    fn test_upsert_clients_principals() {
        let mut manager = super::ClientManager::new();

        let verifier = [0; 8];
        let id = "test".to_string();
        let callback = super::ClientCallback {
            program: 0,
            rnetid: "tcp".to_string(),
            raddr: "".to_string(),
            callback_ident: 0,
        };

        let client = manager
            .upsert_client(
                verifier,
                id.clone(),
                callback.clone(),
                Some("Linux".to_string()),
            )
            .unwrap();

        let same_client = manager
            .confirm_client(
                client.clientid,
                client.setclientid_confirm,
                Some("Linux".to_string()),
            )
            .unwrap();

        assert_eq!(same_client.id, id);
        assert_eq!(same_client.verifier, verifier);
        assert_eq!(same_client.callback, callback);
        assert_eq!(same_client.clientid, client.clientid);
        assert_eq!(same_client.principal, Some("Linux".to_string()));
        assert_eq!(same_client.confirmed, true);
    }

    #[tokio::test]
    async fn test_upsert_clients_async() {
        let manager = Arc::new(Mutex::new(ClientManager::new()));
        async fn client_spawn(manager: Arc<Mutex<ClientManager>>) {
            let mut manager = manager.lock().unwrap();
            let verifier = [0; 8];
            let id: String = rand::thread_rng()
                .sample_iter(&Alphanumeric)
                .take(12)
                .map(char::from)
                .collect();
            let callback = ClientCallback {
                program: 0,
                rnetid: "tcp".to_string(),
                raddr: "".to_string(),
                callback_ident: 0,
            };

            let client = manager
                .upsert_client(verifier, id.clone(), callback.clone(), None)
                .unwrap();

            // confirm after update

            let confirmed_client = manager
                .confirm_client(client.clientid, client.setclientid_confirm, None)
                .unwrap();
            assert!(confirmed_client.confirmed);
        }

        let mut jobs = Vec::new();
        for _ in 0..1000 {
            jobs.push(client_spawn(manager.clone()));
        }

        let now = Instant::now();
        let _ = futures::future::join_all(jobs).await;
        let eps = now.elapsed();

        let mut manager = manager.lock().unwrap();
        assert_eq!(manager.get_record_count(), 1000);
        println!("Elapsed time: {:?}", eps.as_millis());
        assert!(eps.as_millis() < 50);
        let c_99 = manager.get_client_confirmed(99);
        assert!(c_99.unwrap().confirmed);
    }
}
