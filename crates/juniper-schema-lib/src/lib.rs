#![feature(map_try_insert)]

pub mod codegen;
//pub mod impl_to_trait_mapper;
pub mod schema_info;

pub use codegen::CodegenFromFile;
//pub use impl_to_trait_mapper::ImplToTraitMapper;

pub enum ContextType {
    Global(syn::Type),
    // TODO: It can be useful to specify a different context type for each GraphQL type. At some
    //       point we could allow this with some type of mapping syntax for the context arg.
}

#[derive(Debug)]
pub enum CodegenError {
    IoError(std::io::Error, proc_macro2::Span),
    MultipleEnumTypeDefinitions {
        first: graphql_parser::Pos,
        second: graphql_parser::Pos,
    },
    MultipleObjectTypeDefinitions {
        first: graphql_parser::Pos,
        second: graphql_parser::Pos,
    },
    MultipleSchemaDefinitions {
        first: graphql_parser::Pos,
        second: graphql_parser::Pos,
    },
    NoSchemaDefinitionFound,
    SchemaParseError(graphql_parser::schema::ParseError),
    UndefinedGraphQLType(String),
}
impl CodegenError {
    pub fn to_compile_error(&self) -> proc_macro2::TokenStream {
        let default_span = proc_macro2::Span::call_site();
        let error_strlit = match self {
            CodegenError::UndefinedGraphQLType(msg) => {
                syn::LitStr::new(msg.as_str(), default_span)
            },
            _other => {
                let err = format!("Error generating code for GraphQL schema: {:?}", self);
                syn::LitStr::new(err.as_str(), default_span)
            }
        };

        quote::quote! {
            compile_error!(#error_strlit);
        }
    }
}
