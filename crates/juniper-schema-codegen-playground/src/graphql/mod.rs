mod query;
mod user;

use query::Query;
use user::User;

pub struct Context;
impl juniper::Context for Context {}

juniper_schema_codegen::from_file!("schema.graphqls", {
    context_type: Context,
});
