use std::collections::HashMap;
use std::path::PathBuf;

use crate::CodegenError;
use crate::ContextType;
use crate::schema_info::SchemaInfo;

enum MapperToken {
    FatArrow,
    SkinnyArrow,
}

pub enum CodegenOptionsValidationError {
    UnexpectedGraphQLTypeInGraphQLToRustTypeMap(String),
}

/**
 * Given a GraphQL schema string: Parse it as a GraphQL schema, extract
 * information from the schema AST using SchemaInfo, and then produce a
 * TokenStream for the codegen'd juniper traits to be implemented.
 */
pub struct Codegen {
    schema_info: SchemaInfo<'static>,
    options: CodegenOptions,
}
impl Codegen {
    pub fn new(schema: String, options: CodegenOptions) -> Result<Self, CodegenError> {
        Ok(Codegen {
            options,
            schema_info: SchemaInfo::parse(schema)?,
        }.validate_options()?)
    }

    fn codegen_object_types(&self) -> Result<proc_macro2::TokenStream, CodegenError> {
        let span = proc_macro2::Span::call_site();

        let obj_impls = self.schema_info.obj_types.iter().map(|(obj_name, obj_type)| {
            // TODO: Cleanse obj_names that clash with rust keywords somehow
            //       e.g. Use `r#` "raw identifiers"? Detect and add some suffix?
            let object_struct_ident = syn::Ident::new(
                self.graphql_type_name_to_rust_type_name(obj_name.to_string()).as_str(),
                span.clone()
            );
            let resolver_trait_name = syn::Ident::new(
                format!("{}FieldResolvers", &obj_name).as_str(),
                span.clone(),
            );

            let (impl_methods, trait_methods) = obj_type.fields.iter().fold(
                (vec![], vec![]),
                |(mut impl_methods, mut trait_methods), field| {
                    let impl_method_name = syn::Ident::new(
                        &field.name,
                        span.clone(),
                    );
                    let resolver_method_name = syn::Ident::new(
                        format!("resolve_{}", &field.name).as_str(),
                        span.clone(),
                    );

                    let mut impl_method_params = vec![
                        quote::quote! { &self },
                    ];
                    let mut trait_method_params = vec![
                        // TODO: Add option for switching between &self vs &mut self
                        quote::quote! { &self },
                    ];
                    let mut resolver_args = vec![];

                    // If a context type is specified, use it
                    match &self.options.context_type {
                        Some(ContextType::Global(type_ident)) => {
                            impl_method_params.push(quote::quote! {
                                ctx: &#type_ident
                            });
                            trait_method_params.push(quote::quote! {
                                ctx: &#type_ident
                            });
                            resolver_args.push(quote::quote! {
                                ctx
                            });
                        }
                        None => (),
                    };

                    // Map the GraphQL type to the Rust type
                    let return_type = self.graphql_type_to_rust_type(
                        &field.field_type,
                        &span,
                        /* nullable = */ true,
                    );

                    impl_methods.push(quote::quote! {
                        pub async fn #impl_method_name(#(#impl_method_params),*) -> #return_type {
                            // Delegate to resolver trait method
                            self.#resolver_method_name(#(#resolver_args),*).await
                        }
                    });

                    trait_methods.push(quote::quote! {
                        async fn #resolver_method_name(#(#trait_method_params),*) -> #return_type;
                    });

                    (impl_methods, trait_methods)
                },
            );

            let mut juniper_attr_macro_args = vec![];
            if let Some(ContextType::Global(type_ident)) = &self.options.context_type {
                juniper_attr_macro_args.push(quote::quote! {
                    Context = #type_ident
                });
            }

            quote::quote! {
                #[async_trait]
                pub trait #resolver_trait_name {
                    #(#trait_methods)*
                }

                #[juniper::graphql_object(#(#juniper_attr_macro_args),*)]
                impl #object_struct_ident {
                    #(#impl_methods)*
                }

                // TODO: Use a syn::Ident with a span that's not accessible
                //      instead of __trait_assert__
                impl #object_struct_ident {
                    fn __trait_assert__(self) -> impl #resolver_trait_name {
                        self
                    }
                }
            }
        });

        Ok(quote::quote! {
            #(#obj_impls)*
        })
    }

    fn graphql_type_to_rust_type(
        &self,
        field_type: &graphql_parser::query::Type<'static, String>,
        span: &proc_macro2::Span,
        nullable: bool,
    ) -> proc_macro2::TokenStream {
        use graphql_parser::query::Type;
        match field_type {
            Type::NamedType(name) => {
                let ident = match name.as_str() {
                    "Int" => quote::quote! { i32 },
                    "Float" => quote::quote! { f64 },
                    "String" => quote::quote! { String },
                    "Boolean" => quote::quote! { bool },
                    "ID" => quote::quote! { juniper::ID },
                    graphql_type_name => {
                        let graphql_type_name = String::from(graphql_type_name);
                        let rust_type_name = self.graphql_type_name_to_rust_type_name(
                            String::from(graphql_type_name)
                        );
                        let ident = syn::Ident::new(rust_type_name.as_str(), span.clone());
                        quote::quote! { #ident }
                    },
                };

                if nullable {
                    quote::quote!{ Option<#ident> }
                } else {
                    quote::quote!{ #ident }
                }
            },

            Type::ListType(inner_type) => {
                let inner_type_tokens = self.graphql_type_to_rust_type(
                    inner_type,
                    span,
                    /* nullable = */ true,
                );
                if nullable {
                    quote::quote! { Option<Vec<#inner_type_tokens>> }
                } else {
                    quote::quote! { Vec<#inner_type_tokens> }
                }
            },

            Type::NonNullType(inner_type) => {
                self.graphql_type_to_rust_type(
                    inner_type,
                    span,
                    /* nullable = */ false,
                )
            }
        }
    }

    fn graphql_type_name_to_rust_type_name(&self, graphql_name: String) -> String {
        if let Some(type_map) = &self.options.graphql_to_rust_type_map {
            type_map.get(&graphql_name).unwrap_or(&graphql_name).to_string()
        } else {
            graphql_name
        }
    }

    pub fn to_tokens(self) -> Result<proc_macro2::TokenStream, CodegenError> {
        let mut tokens = proc_macro2::TokenStream::new();

        tokens.extend(quote::quote! {
            use async_trait::async_trait;
        });

        tokens.extend(self.codegen_object_types()?);

        Ok(tokens)
    }

    fn validate_options(self) -> Result<Self, CodegenError> {
        // All entries in graphql_to_rust_type_map should map to an actual type
        // specified in the schema
        if let Some(graphql_to_rust_type_map) = &self.options.graphql_to_rust_type_map {
            for (graphql_type_name, rust_type_name) in graphql_to_rust_type_map.iter() {
                if self.schema_info.enum_types.contains_key(graphql_type_name) {
                    continue;
                }

                if self.schema_info.obj_types.contains_key(graphql_type_name) {
                    continue;
                }

                return Err(CodegenError::UndefinedGraphQLType(format!(
                    "Error mapping GraphQLType(`{}`) -> RustType(`{}`): `{}` \
                    is not a type defined in your GraphQL schema.",
                    &graphql_type_name,
                    rust_type_name,
                    graphql_type_name,
                )));
            }
        }

        Ok(self)
    }
}

/**
 * Parse syn::braced!() content for codegen options.
 *
 * e.g. The stuff between the braces in
 *
 *    juniper_schema::from_file!("schema.graphqls", {
 *        <<<<stuff here>>>>
 *    });
 */
pub struct CodegenOptions {
    context_type: Option<ContextType>,
    graphql_to_rust_type_map: Option<HashMap<String, String>>,
}
impl syn::parse::Parse for CodegenOptions {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut context_type = None::<ContextType>;
        let mut graphql_to_rust_type_map = HashMap::new();

        // Don't have an opinion on which arrow is used for arrow syntax except
        // that the same arrow is used consistently. Helps when you can't
        // remember which arrow is expected...it's whichever one you try first.
        let mut mapping_arrow_token = None::<MapperToken>;
        while !input.is_empty() {
            let opt_key = input.parse::<syn::Ident>()?;
            match opt_key.to_string().as_str() {
                "context_type" => {
                    let _ = input.parse::<syn::Token![:]>()?;
                    if let Some(_) = context_type {
                        return Err(syn::parse::Error::new(
                            opt_key.span(),
                            "`context_type` specified more than once!",
                        ));
                    }
                    let _ = context_type.insert(
                        ContextType::Global(input.parse::<syn::Type>()?)
                    );
                },

                "graphql_to_rust_type_map" => {
                    let _ = input.parse::<syn::Token![:]>()?;

                    let graphql_to_rust_mappers;
                    syn::braced!(graphql_to_rust_mappers in input);

                    while !graphql_to_rust_mappers.is_empty() {
                        let graphql_type_name_ident = graphql_to_rust_mappers.parse::<syn::Ident>()?;
                        match mapping_arrow_token {
                            Some(MapperToken::SkinnyArrow) => {
                                graphql_to_rust_mappers.parse::<syn::Token![->]>()?;
                            },
                            Some(MapperToken::FatArrow) => {
                                graphql_to_rust_mappers.parse::<syn::Token![=>]>()?;
                            },
                            None => {
                                if graphql_to_rust_mappers.peek(syn::Token![->]) {
                                    let _ = mapping_arrow_token.insert(MapperToken::SkinnyArrow);
                                    graphql_to_rust_mappers.parse::<syn::Token![->]>()?;
                                } else {
                                    let _ = mapping_arrow_token.insert(MapperToken::FatArrow);
                                    graphql_to_rust_mappers.parse::<syn::Token![=>]>()?;
                                }
                            }
                        };
                        let rust_type_name_ident = graphql_to_rust_mappers.parse::<syn::Ident>()?;
                        let _ = graphql_to_rust_type_map.insert(
                            graphql_type_name_ident.to_string(),
                            rust_type_name_ident.to_string(),
                        );

                        if graphql_to_rust_mappers.peek(syn::Token![,]) {
                            graphql_to_rust_mappers.parse::<syn::Token![,]>()?;
                        }
                    }
                },

                other => {
                    return Err(syn::parse::Error::new(
                        opt_key.span(),
                        format!("Unexpected option: `{}`", other),
                    ));
                }
            }

            if input.peek(syn::Token![,]) {
                input.parse::<syn::Token![,]>()?;
            }
        }

        let graphql_to_rust_type_map =
            if graphql_to_rust_type_map.len() > 0 {
                Some(graphql_to_rust_type_map)
            } else {
                None
            };

        Ok(CodegenOptions {
            context_type,
            graphql_to_rust_type_map,
        })
    }
}
impl Default for CodegenOptions {
    fn default() -> Self {
        CodegenOptions {
            context_type: None,
            graphql_to_rust_type_map: None,
        }
    }
}

/**
 * Parse contents of the `juniper_schema::from_file!()` macro, read the
 * contents of the schema file, and produce a Codegen object from it.
 */
pub struct CodegenFromFile {
    options: CodegenOptions,
    schema_path: PathBuf,
    schema_path_span: proc_macro2::Span,
}
impl syn::parse::Parse for CodegenFromFile {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<CodegenFromFile> {
        let schema_path_litstr = input.parse::<syn::LitStr>()?;

       let mut codegen_opts = None::<CodegenOptions>;
        if !input.is_empty() {
            input.parse::<syn::Token![,]>()?;

            if !input.is_empty() {
                // Parse braces and insert tokens from inside the braces into
                // `option_tokens`
                let option_tokens;
                syn::braced!(option_tokens in input);

                let _ = codegen_opts.insert(option_tokens.parse::<CodegenOptions>()?);
            }
        }

        Ok(CodegenFromFile::new(schema_path_litstr, codegen_opts.unwrap_or_default()))
    }
}
impl CodegenFromFile {
    pub fn new(schema_path_litstr: syn::LitStr, options: CodegenOptions) -> Self {
        let schema_relative_path = &schema_path_litstr.value();
        let crate_dir = std::env::var("CARGO_MANIFEST_DIR").expect(
            "Env var `CARGO_MANIFEST_DIR` is missing."
        );

        let schema_path = PathBuf::from(crate_dir).join(schema_relative_path);
        let schema_path_span = schema_path_litstr.span();

        CodegenFromFile {
            options,
            schema_path,
            schema_path_span,
        }
    }

    pub fn to_codegen(self) -> Result<Codegen, CodegenError> {
        let schema_str = std::fs::read_to_string(&self.schema_path).map_err(|e| {
            CodegenError::IoError(e, self.schema_path_span)
        })?;
        Codegen::new(schema_str, self.options)
    }
}
