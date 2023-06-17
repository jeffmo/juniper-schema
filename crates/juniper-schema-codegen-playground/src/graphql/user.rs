use super::Context;
use super::UserFieldResolvers;

pub struct User;
#[async_trait::async_trait]
impl UserFieldResolvers for User {
    async fn resolve_id(&self, _ctx: &Context) -> Option<juniper::ID> {
        Some(juniper::ID::new("user:jeffmo"))
    }
}
