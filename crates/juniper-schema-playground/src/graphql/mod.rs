mod query;
mod user;

use query::Query;
use user::User;

pub struct Context;
impl juniper::Context for Context {}

/*
juniper_schema::from_file!("schema.graphqls", {
    context_type: Context,
});
*/

// !!!! TODO: Try this new API model:
juniper_schema::from_file3!(MyRootNode for "schema.graphqls", {
    context_type: Context,
    types: {
        Query -> Query,
        User -> User,
    },
});
/* Desugars to:
 *
 * pub type MyRootNode = __RootNode<'static>;
 *
 * struct __QueryWrapper {
 *   impl_: Query,
 * }
 * impl __QueryWrapper {
 *   pub fn new(query: Query) -> Self {
 *     __QueryWrapper { impl_: query }
 *   }
 * }
 * #[juniper::graphql_object(name="Query")]
 * impl __QueryWrapper {
 *   pub async fn me(&self, ctx: &Context) -> __UserWrapper {
 *     __UserWrapper::new(self.impl_.me(ctx, ctx).await)
 *   }
 * }
 *
 * struct __RootNode<'a>;
 * impl<'a> __RootNode<'a> {
 *   // TODO: Add support for mutations and subscriptions too
 *   pub fn new(query: Query) -> juniper::RootNode<
 *      'a,
 *      __QueryWrapper,
 *      juniper::EmptyMutation<Context>,
 *      juniper::EmptySubscription<Context>,
 *   > {
 *      juniper::RootNode::new(
 *          __QueryWrapper::new(query),
 *          juniper::EmptyMutation::new(),
 *          juniper::EmptySubscription::new(),
 *      )
 *   }
 * }
 */
