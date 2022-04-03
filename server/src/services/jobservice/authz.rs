use super::UserId;
use std::collections::{HashMap, HashSet};

pub struct AuthzDb {
    user_database: HashMap<UserId, ScopedRoles>,
    role_permissions: HashMap<Role, Vec<Permission>>,
}
type ScopedRoles = HashMap<Scope, HashSet<Role>>;

#[derive(Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Permission {
    StartOrStop,
    Query,
}

#[derive(Hash, PartialEq, Eq, PartialOrd, Ord)]
enum Role {
    TaskManager,
    Analyst,
}

#[derive(Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Scope {
    Owner,
    All,
}

impl Default for AuthzDb {
    fn default() -> Self {
        // TODO: use a real database and query it instead of loading a mock db into memory
        let mut mock_user_db = HashMap::new();

        // give "alice" permission to start jobs and stop jobs she owns
        let mut alice_permissions = HashMap::new();
        alice_permissions.insert(Scope::Owner, HashSet::from_iter(vec![Role::TaskManager]));
        mock_user_db.insert("alice".into(), alice_permissions);

        let mut bob_permissions = HashMap::new();
        bob_permissions.insert(Scope::All, HashSet::from_iter(vec![Role::Analyst]));
        mock_user_db.insert("bob".into(), bob_permissions);

        let mut charlie_permissions = HashMap::new();
        charlie_permissions.insert(Scope::All, HashSet::from_iter(vec![Role::TaskManager]));
        mock_user_db.insert("charlie".into(), charlie_permissions);

        // Setup role->permissions info
        let mut role_permissions = HashMap::new();

        // task managers can start/stop/query jobs
        role_permissions.insert(
            Role::TaskManager,
            vec![Permission::StartOrStop, Permission::Query],
        );

        // analysts can query job status or output
        role_permissions.insert(Role::Analyst, vec![Permission::Query]);

        Self {
            user_database: mock_user_db,
            role_permissions,
        }
    }
}

/// Loads the RBAC database in memory.
impl AuthzDb {
    pub fn has_permission(&self, user_id: UserId, permission: Permission) -> bool {
        todo!()
    }

    pub fn has_scoped_permission(
        &self,
        user_id: UserId,
        scope: Scope,
        permission: Permission,
    ) -> bool {
        todo!()
    }
}
