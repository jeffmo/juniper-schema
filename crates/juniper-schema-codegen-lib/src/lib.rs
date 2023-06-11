#![feature(map_try_insert)]

mod codegen_from_file;
pub mod schema_data;

pub use codegen_from_file::CodegenFromFile;

pub enum ContextType {
    Global(syn::LitStr),
    // TODO: It can be useful to specify a different context type for each GraphQL type. At some
    //       point we could allow this with some type of mapping syntax for the context arg.
}
impl ContextType {
    pub fn new(input: &mut syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(ContextType::Global(input.parse::<syn::LitStr>()?))
    }
}

#[derive(Debug)]
pub enum CodegenError {
    IoError(std::io::Error),
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


    // !! TODO
}
