use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// Bootstrap super administrator. Cannot be deleted by other admins.
    Owner,
    /// Manages panel users and all hosting resources.
    Admin,
    /// Manages hosting resources only.
    Operator,
    /// Read-only access.
    Viewer,
}

impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::Owner => "owner",
            Role::Admin => "admin",
            Role::Operator => "operator",
            Role::Viewer => "viewer",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "owner" => Some(Role::Owner),
            "admin" => Some(Role::Admin),
            "operator" => Some(Role::Operator),
            "viewer" => Some(Role::Viewer),
            _ => None,
        }
    }

    pub fn rank(&self) -> u8 {
        match self {
            Role::Owner => 4,
            Role::Admin => 3,
            Role::Operator => 2,
            Role::Viewer => 1,
        }
    }

    pub fn at_least(&self, required: Role) -> bool {
        self.rank() >= required.rank()
    }

    pub fn can_manage_users(&self) -> bool {
        self.at_least(Role::Admin)
    }

    pub fn can_write(&self) -> bool {
        self.at_least(Role::Operator)
    }
}
