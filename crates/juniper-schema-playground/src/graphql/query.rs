use super::Context;
//use super::QueryFieldResolvers;
use super::user::User;

pub struct Query;
impl Query {
    pub async fn me(&self, _ctx: &Context) -> Option<User> {
        Some(User)
    }
}
/*
#[async_trait::async_trait]
impl QueryFieldResolvers for Query {
    async fn resolve_me(&self, _ctx: &Context) -> Option<User> {
        Some(User)
    }
}
*/
