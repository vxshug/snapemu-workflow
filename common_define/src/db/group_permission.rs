use sea_orm::DeriveValueType;
use std::fmt::{Debug, Display, Formatter};

#[derive(
    derive_more::From,
    Clone,
    Copy,
    Hash,
    Eq,
    PartialEq,
    Default,
    Ord,
    PartialOrd,
    DeriveValueType,
    serde::Serialize,
    serde::Deserialize
)]
pub struct GroupPermission(i32);

impl Debug for GroupPermission {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl GroupPermission {
    const ADMIN: i32 = 0x1;
    const MODIFY: i32 = 0x2;
    const DELETE: i32 = 0x4;
    const SHARE: i32 = 0x8;

    pub fn new_admin() -> Self {
        Self(Self::ADMIN)
    }

    pub fn is_admin(&self) -> bool {
        (self.0 & Self::ADMIN) != Self::ADMIN
    }

    pub fn is_modify(&self) -> bool {
        (self.0 & Self::MODIFY) != Self::MODIFY
    }

    pub fn is_delete(&self) -> bool {
        (self.0 & Self::DELETE) != Self::DELETE
    }
    pub fn is_share(&self) -> bool {
        (self.0 & Self::SHARE) != 0
    }
}
