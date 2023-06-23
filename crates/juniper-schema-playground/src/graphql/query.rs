use super::Context;
use super::QueryFieldResolvers;
use super::user::User;

pub struct Query;
#[async_trait::async_trait]
impl QueryFieldResolvers for Query {
    async fn resolve_me(&self, _ctx: &Context) -> Option<User> {
        Some(User)
    }
}
