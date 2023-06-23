use super::Context;
//use super::UserFieldResolvers;

pub struct User;
impl User {
    pub async fn id(&self, _ctx: &Context) -> String {
        "sadf".to_string()
    }
}
/*
#[async_trait::async_trait]
impl UserFieldResolvers for User {
    async fn resolve_id(&self, _ctx: &Context) -> Option<juniper::ID> {
        Some(juniper::ID::new("user:jeffmo"))
    }
}

// TODO: Make this work (see docblock in the impl of the proc_macro)
#[juniper_schema::field_resolvers]
impl User {
}
*/
