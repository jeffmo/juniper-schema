// TODO: Needed for stuff in schema_parser.rs.
//       Move to juniper-schema-codegen-macro-libs when that move happens
#![feature(map_try_insert)]

mod schema_parser;

use std::path::PathBuf;

#[proc_macro]
pub fn from_file(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let schema_from_file = match syn::parse::<CodegenFromFile>(input) {
        Ok(s) => s,
        Err(e) => return e.to_compile_error().into(),
    };

    match schema_from_file.codegen() {
        Ok(tokens) => tokens.into(),
        Err(e) => panic!("{:?}", e), // TODO: Switch to `compile_error!()`
    }
}

// TODO: Move this into a separate juniper-schema-codegen-macro-libs crate
pub(crate) enum ContextType {
    Global(syn::LitStr),
    // TODO: It can be useful to specify a different context type for each GraphQL type. At some
    //       point we could allow this with some type of mapping syntax for the context arg.
}
impl ContextType {
    pub fn new(input: &mut syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(ContextType::Global(input.parse::<syn::LitStr>()?))
    }
}

// TODO: Move this into a separate juniper-schema-codegen-macro-libs crate
#[derive(Debug)]
pub(crate) enum CodegenError {
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

// TODO: Move this into a separate juniper-schema-codegen-macro-libs crate
struct CodegenFromFile {
    context_type: ContextType,
    schema_path: PathBuf,
}
impl syn::parse::Parse for CodegenFromFile {
    fn parse(mut input: syn::parse::ParseStream) -> syn::Result<Self> {
        // First token is a LitStr specifying the location of the schema file
        // relative to the root dir of the crate.
        let schema_relative_path = input.parse::<syn::LitStr>()?.value();
        let crate_dir = std::env::var("CARGO_MANIFEST_DIR").expect(
            "Env var `CARGO_MANIFEST_DIR` is missing."
        );
        let schema_path = PathBuf::from(crate_dir).join(schema_relative_path);

        let mut context_type = None::<ContextType>;
        while !input.is_empty() {
            input.parse::<syn::Token![,]>()?;

            let key = input.parse::<syn::Ident>()?;
            match key.to_string().as_str() {
                "context_type" => {
                    if let Some(_) = context_type {
                        return Err(syn::parse::Error::new(
                            key.span(),
                            "`context_type` specified more than once!",
                        ));
                    }
                    let _ = context_type.insert(ContextType::new(&mut input)?);
                },

                other => {
                    return Err(syn::parse::Error::new(
                        key.span(),
                        format!("Unexpected option: `{}`", other),
                    ));
                }
            }
        }

        Ok(CodegenFromFile {
            context_type: context_type.unwrap_or_else(|| {
                ContextType::Global(
                    syn::parse_str("Context").expect(
                        "Failed to parse default context type"
                    )
                )
            }),
            schema_path,
        })
    }
}
impl CodegenFromFile {
    fn codegen(self) -> Result<proc_macro2::TokenStream, CodegenError> {
        let schema_str = std::fs::read_to_string(&self.schema_path).map_err(CodegenError::IoError)?;
        let _codegen_data = schema_parser::SchemaParser::new(schema_str, self.context_type);

        // TODO: Visit document and gather all needed info

        Ok(quote::quote! { println!("Here is where the codegen goes") })
    }
}
