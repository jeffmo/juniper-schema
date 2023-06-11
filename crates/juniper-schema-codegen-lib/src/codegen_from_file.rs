use std::path::PathBuf;

use crate::CodegenError;
use crate::ContextType;
use crate::schema_data::SchemaData;

pub struct CodegenFromFile {
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
    pub fn codegen(self) -> Result<proc_macro2::TokenStream, CodegenError> {
        let schema_str = std::fs::read_to_string(&self.schema_path).map_err(CodegenError::IoError)?;
        let _schema_data = SchemaData::parse(schema_str.as_str(), self.context_type);

        Ok(quote::quote! { println!("Here is where the codegen goes") })
    }
}
