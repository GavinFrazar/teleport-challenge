mod authz_db;

use self::authz_db::{AuthzDb, Permission, Scope};
use super::UserId;
use joblib::types::JobId;
use std::{collections::HashMap, sync::Mutex};

type JobOwnerDb = HashMap<JobId, UserId>;

pub struct Authorizer {
    job_owners: Mutex<JobOwnerDb>,
    authz_db: AuthzDb, // immutable pre-populated mock db
}

pub enum ExistingJobAction {
    StopJob,
    QueryStatus,
    StreamOutput,
}

pub enum Action {
    StartJob,
    ExistingJob {
        job_id: JobId,
        inner_action: ExistingJobAction,
    },
}

impl Authorizer {
    pub fn new() -> Self {
        let authz_db = AuthzDb::default();
        Self {
            job_owners: Mutex::new(JobOwnerDb::new()),
            authz_db,
        }
    }
    pub fn add_job(&self, job_id: JobId, user_id: &UserId) {
        self.job_owners
            .lock()
            .unwrap()
            .insert(job_id, user_id.to_string());
    }

    pub fn is_authorized(&self, user_id: &UserId, action: Action) -> bool {
        use Action::*;
        use ExistingJobAction::*;
        match action {
            ExistingJob {
                job_id,
                inner_action,
            } => {
                let maybe_owner = self.job_owners.lock().unwrap().get(&job_id).cloned();
                if let Some(job_owner) = maybe_owner {
                    match inner_action {
                        StopJob => {
                            if job_owner == *user_id {
                                return self
                                    .authz_db
                                    .has_permission(user_id, Permission::StartOrStop);
                            } else {
                                return self.authz_db.has_scoped_permission(
                                    user_id,
                                    Scope::All,
                                    Permission::StartOrStop,
                                );
                            }
                        }
                        QueryStatus | StreamOutput => {
                            if job_owner == *user_id {
                                return self.authz_db.has_permission(user_id, Permission::Query);
                            } else {
                                return self.authz_db.has_scoped_permission(
                                    user_id,
                                    Scope::All,
                                    Permission::Query,
                                );
                            }
                        }
                    }
                }
            }
            StartJob => {
                return self
                    .authz_db
                    .has_permission(user_id, Permission::StartOrStop)
            }
        }

        // reject anything else as unauthorized
        // NOTE: if the user id doesnt exist, or the job id doesnt exist, we reject those as unauth --
        //       -- dont leak info! Although I won't go as far as hardening this against timing attacks.
        false
    }
}
