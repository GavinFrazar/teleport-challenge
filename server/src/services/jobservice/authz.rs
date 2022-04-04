use super::UserId;
use std::collections::{HashMap, HashSet};

/// A mock authorization info database.
///
/// Interally its just some nested HashMaps of user->scope->roles and of role->permissions
///
/// It's probably not the most efficient solution, but it gets the job done.
///
/// users' roles and roles' permissions are just stored as HashSets.
///
/// In a real implementation this database wouldn't exist in memory - but in general for things like
/// this I would test different data layouts if performance became an issue.
///
/// I'm aware that allocations and cache-friendly data structures can be very important, even if they aren't
/// the best in theoretical run-time (Big-O) complexity.
#[derive(Clone)]
pub struct AuthzDb {
    user_database: HashMap<UserId, ScopedRoles>,
    role_permissions: HashMap<Role, HashSet<Permission>>,
}
type ScopedRoles = HashMap<Scope, HashSet<Role>>;

#[derive(Hash, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
pub enum Permission {
    StartOrStop,
    Query,
}

#[derive(Hash, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
enum Role {
    TaskManager,
    Analyst,
}

#[derive(Hash, PartialEq, Eq, PartialOrd, Ord, Copy, Clone)]
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

        // give "bob" permission to view anyone's jobs output/status
        let mut bob_permissions = HashMap::new();
        bob_permissions.insert(Scope::All, HashSet::from_iter(vec![Role::Analyst]));
        mock_user_db.insert("bob".into(), bob_permissions);

        // give "charlie" permission to start jobs, or stop/status/query output anyone's jobs
        let mut charlie_permissions = HashMap::new();
        charlie_permissions.insert(Scope::All, HashSet::from_iter(vec![Role::TaskManager]));
        mock_user_db.insert("charlie".into(), charlie_permissions);

        // Setup role->permissions info
        let mut role_permissions = HashMap::new();

        // task managers can start/stop/query jobs
        role_permissions.insert(
            Role::TaskManager,
            HashSet::from_iter(vec![Permission::StartOrStop, Permission::Query]),
        );

        // analysts can query job status or output
        role_permissions.insert(Role::Analyst, HashSet::from_iter(vec![Permission::Query]));

        Self {
            user_database: mock_user_db,
            role_permissions,
        }
    }
}

/// Loads the RBAC database in memory.
impl AuthzDb {
    pub fn has_permission(&self, user_id: UserId, permission: Permission) -> bool {
        self.has_scoped_permission(user_id.clone(), Scope::Owner, permission)
            || self.has_scoped_permission(user_id, Scope::All, permission)
    }

    pub fn has_scoped_permission(
        &self,
        user_id: UserId,
        scope: Scope,
        permission: Permission,
    ) -> bool {
        if let Some(scoped_roles) = self.user_database.get(&user_id) {
            if let Some(roles) = scoped_roles.get(&scope) {
                for role in roles {
                    if self
                        .role_permissions
                        .get(&role)
                        .expect("invalid authz db")
                        .contains(&permission)
                    {
                        return true;
                    }
                }
            }
        }
        false
    }
}
