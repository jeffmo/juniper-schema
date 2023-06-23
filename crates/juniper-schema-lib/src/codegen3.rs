use std::collections::HashMap;
use std::path::PathBuf;
use quote::ToTokens;


use super::CodegenError;
use super::ContextType;
use super::schema_info::SchemaInfo;

pub struct SchemaFromFile3 {
    options: CodegenOptions,
    root_node_ident: syn::Ident,
    schema_path: PathBuf,
    schema_path_span: proc_macro2::Span,
}
impl syn::parse::Parse for SchemaFromFile3 {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<SchemaFromFile3> {
        let root_node_ident = input.parse::<syn::Ident>()?;
        input.parse::<syn::Token![for]>()?;
        let schema_path_litstr = input.parse::<syn::LitStr>()?;
        let schema_path = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect(
            "Env var `CARGO_MANIFEST_DIR` is missing."
        )).join(schema_path_litstr.value());
        let schema_path_span = schema_path_litstr.span();


        let mut options = None::<CodegenOptions>;
        if !input.is_empty() {
            input.parse::<syn::Token![,]>()?;
            if !input.is_empty() {
                // Bit of a strange macro, but braced!() will parse braces and
                // assign a TokenStream of all of the tokens from between the
                // braces to `option_tokens` here.
                let option_tokens;
                syn::braced!(option_tokens in input);

                let _ = options.insert(option_tokens.parse::<CodegenOptions>()?);
            }
        }
        let options =
            if let Some(options) = options {
                options
            } else {
                return Err(syn::parse::Error::new(
                    proc_macro2::Span::call_site(),
                    "Missing options block.",
                ));
            };

        Ok(SchemaFromFile3 {
            options,
            root_node_ident,
            schema_path,
            schema_path_span,
        })
    }
}
impl SchemaFromFile3 {
    pub fn to_codegen(self) -> Result<Codegen, CodegenError> {
        let schema_str = std::fs::read_to_string(&self.schema_path).map_err(|e| {
            CodegenError::IoError(e, self.schema_path_span)
        })?;
        Ok(Codegen::new(self.root_node_ident, schema_str, self.options)?)
    }
}

pub struct Codegen {
    options: CodegenOptions,
    root_node_ident: syn::Ident,
    schema_info: SchemaInfo<'static>,
}
impl Codegen {
    pub fn new(
        root_node_ident: syn::Ident,
        schema_str: String,
        options: CodegenOptions,
    ) -> Result<Self, CodegenError> {
        let schema_info = SchemaInfo::parse(schema_str)?;

        options.validate(&schema_info)?;

        Ok(Codegen {
            options,
            root_node_ident,
            schema_info,
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
                let ident =
                    if name == "Int" {
                        quote::quote! { i32 }
                    } else if name == "Float" {
                        quote::quote! { f64 }
                    } else if name == "String" {
                        quote::quote! { String }
                    } else if name == "Boolean" {
                        quote::quote! { bool }
                    } else if name == "ID" {
                        quote::quote! { juniper::ID }
                    } else {
                        let graphql_type_name = String::from(name);
                        let rust_type = self.options.graphql_type_name_to_rust_type(
                            &graphql_type_name
                        );
                        rust_type.to_token_stream()
                    };

                /*
                let ident = match name.as_str() {
                    "Int" => quote::quote! { i32 },
                    "Float" => quote::quote! { f64 },
                    "String" => quote::quote! { String },
                    "Boolean" => quote::quote! { bool },
                    "ID" => quote::quote! { juniper::ID },
                    graphql_type_name => {
                        let graphql_type_name = String::from(graphql_type_name);
                        let rust_type = self.options.graphql_type_name_to_rust_type(
                            &graphql_type_name
                        );
                        rust_type.to_token_stream()
                        /*
                        let ident = syn::Ident::new(rust_type_name.as_str(), span.clone());
                        quote::quote_spanned! {span=> #rust_type }
                        */
                    },
                };
                */

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

    /**
     * For each GraphQL object type defined in the schema, we expect a
     * corresponding rust type (either named the same or with a mapped name in
     * CodegenOptions) to exist in scope. To define resolvers for the
     * fields on that object type, Juniper expects that we specify an `impl {}`
     * block on that type annotated with #[juniper::graphql_object].
     *
     * However, since Juniper does not know what fields were specified in the
     * schema file, there is no way for Juniper's macros to issue a
     * compile error when a resolver method is either missing or wrong!
     *
     * One approach to solving this problem is to codegen traits which the rust
     * type must implement. This is the approach that the `juniper-from-schema`
     * crate takes and, generally speaking, it works. However it comes with at
     * least 2 drawbacks:
     *
     * 1. It is really useful to have async resolver methods, but async traits
     *    just aren't quite there yet in rust. The `async-trait` macro works,
     *    but in practice it can give some pretty gnarly compiler errors when
     *    some minor/easily fixable mistakes are made. Moreover, if we were to
     *    codegen a trait with the #[async_trait] annotation that would be fine,
     *    but it's a (frankly, minor...but...ergonomic papercuts!) bummer that
     *    users of such a crate would have to remember to also use the
     *    `#[async_trait]` annotation on the `impl` block for the generated
     *    async trait we generate.
     *
     * 2. `juniper-from-schema` generates a `juniper::graphql_object!()` call
     *    that directly references both the user-defined rust struct as well as
     *    a trait which must be implemented on that struct -- all in the same
     *    module. This makes it awkward to define your rust structs in a
     *    different file and implement the resolvers for them. You can import
     *    the generated trait into the other file, but this starts to get weird
     *    for anyone new to the code who doesn't understand that this trait is
     *    generated by the proc_macro -- it looks like the trait is being
     *    imported from thin air!
     *
     *  SO... `juniper_schema` aimes to take a different approach. Instead of
     *  generating traits which must be implemented, it generates a wrapper
     *  type which simply delegates to methods on the user-defined type. The
     *  `#[juniper::graphql_object]` annotation is placed on the generated type
     *  and all the user needs to do is define or import a rust struct which
     *  corresponds to each GraphQL object type and implements an async resolver
     *  for each field on that GraphQL object type (no traits needed).
     *
     *  The wrapper type simply retains an instance of the user-defined type and
     *  delegates from its resolvers into the user-defined type's resolvers.
     */
    fn generate_object_types(&self) -> Result<proc_macro2::TokenStream, CodegenError> {
        let context_param = self.options.context_type.as_ref().map(|ctx_type| {
            match ctx_type {
                ContextType::Global(type_ident) => quote::quote! {
                    context=#type_ident
                }
            }
        });

        let obj_defs = self.schema_info.obj_types.iter().map(
            |(graphql_obj_name, graphql_obj_type)| {
                let wrapper_ident = self.get_wrapper_type_ident(graphql_obj_name);
                let rust_type_ident = self.options.graphql_type_name_to_rust_type(
                    graphql_obj_name
                );
                /*
                let inner_type_ident = syn::Ident::new(
                    // TODO: Maybe this could return a (String, syn::Span) tuple
                    //       for when the type was mapped (so that any errors
                    //       can point back to the mapper syntax)?
                    self.options.graphql_type_name_to_rust_type(
                        graphql_obj_name
                    ).as_str(),
                    proc_macro2::Span::call_site(),
                );
                */
                let graphql_obj_type_name_litstr = syn::LitStr::new(
                    graphql_obj_name.as_str(),
                    proc_macro2::Span::call_site(),
                );

                let default_span = proc_macro2::Span::call_site();
                let resolver_methods = graphql_obj_type.fields.iter().map(
                    |field| {
                        let method_name_ident = syn::Ident::new(
                            &field.name,
                            default_span,
                        );

                        // TODO: Handle field-params
                        let mut wrapper_method_params = vec![
                            quote::quote! { &self },
                        ];
                        let mut impl_method_args = vec![];
                        match &self.options.context_type {
                            Some(ContextType::Global(type_ident)) => {
                                wrapper_method_params.push(quote::quote! {
                                    ctx: &#type_ident
                                });
                                impl_method_args.push(quote::quote! {
                                    ctx
                                });
                            }
                            None => (),
                        };

                        // TODO: Map GraphQL object types to wrapper types here
                        let return_type_ident = self.graphql_type_to_rust_type(
                            &field.field_type,
                            &rust_type_ident.span(), // TODO: Delete this, its just to get
                                                            // things compiling for now
                            /* nullable = */ true,
                        );
                        /*
                        self.options.graphql_type_name_to_rust_type(
                            &field.field_type.to_string(),
                        );
                        */

                        quote::quote_spanned! {rust_type_ident.span()=>
                            pub async fn #method_name_ident(#(#wrapper_method_params),*) -> #return_type_ident {
                                // TODO: If dealing with a GraphQL object-typed field, need to wrap
                                //       this in FIELDWrapper::new()
                                self.impl_.#method_name_ident(#(#impl_method_args),*).await
                            }
                        }
                    }
                );

                let obj_type_name_param = quote::quote! {
                    name=#graphql_obj_type_name_litstr
                };
                let mut juniper_graphql_attr_params = vec![
                    &obj_type_name_param,
                ];

                if let Some(ctx_param) = &context_param {
                    juniper_graphql_attr_params.push(&ctx_param);
                }

                quote::quote! {
                    struct #wrapper_ident {
                        impl_: #rust_type_ident,
                    }
                    impl #wrapper_ident {
                        pub fn new(impl_: #rust_type_ident) -> Self {
                            #wrapper_ident { impl_ }
                        }
                    }

                    #[juniper::graphql_object(#(#juniper_graphql_attr_params),*)]
                    impl #wrapper_ident {
                        #(#resolver_methods)*
                    }
                }
            }
        );

        Ok(quote::quote! {
            #(#obj_defs)*
        })
    }

    fn generate_root_node_wrapper(&self) -> Result<proc_macro2::TokenStream, CodegenError> {
        // Identify the Query type
        // TODO: Eventually it should be acceptable for a schema to specify only one of a Query,
        //       Mutation, or Subscription type.
        let query_ident =
            if let Some(query_type_name) = &self.schema_info.schema_def.query {
                syn::Ident::new(query_type_name, proc_macro2::Span::call_site())
            } else {
                return Err(CodegenError::NoQueryDefinitionFound);
            };

        let query_wrapper_ident = self.get_wrapper_type_ident(
            &query_ident.to_string()
        );

        let root_node_ident = &self.root_node_ident;
        Ok(quote::quote! {
            pub struct #root_node_ident;
            impl #root_node_ident {
                // TODO: Need to collect the Query type specified in the
                //      `schema {}` decl inside SchemaInfo and use that type here.
                //
                //      Also need to consider that that type may have been remapped to some other
                //      Rust type in CodegenOptions::rust_types.
                pub fn new(query: #query_ident) -> juniper::RootNode<
                    'static,
                    #query_wrapper_ident,

                    // TODO: Support Mutations
                    // TODO: Use "context" Ident from CodegenOptions
                    juniper::EmptyMutation<Context>,

                    // TODO: Support Subscriptions
                    // TODO: Use "context" Ident from CodegenOptions
                    juniper::EmptySubscription<Context>,
                > {
                    juniper::RootNode::new(
                        #query_wrapper_ident::new(query),
                        juniper::EmptyMutation::new(),
                        juniper::EmptySubscription::new(),
                    )
                }

                // TODO: Impl other delegators to other relevant methods on
                //       juniper::RootNode
            }
        })
    }

    fn get_wrapper_type_ident(&self, type_name: &String) -> syn::Ident {
        syn::Ident::new(
            format!("__{}Wrapper", type_name).as_str(),
            proc_macro2::Span::call_site(),
        )
    }

    pub fn to_tokens(self) -> Result<proc_macro2::TokenStream, CodegenError> {
        let mut tokens = proc_macro2::TokenStream::new();

        tokens.extend(self.generate_object_types()?);
        tokens.extend(self.generate_root_node_wrapper()?);

        Ok(tokens)
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
    pub context_type: Option<ContextType>,
    rust_types: HashMap<String, syn::Ident>,
}
impl syn::parse::Parse for CodegenOptions {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut context_type = None::<ContextType>;
        let mut rust_types = None::<HashMap<String, syn::Ident>>;

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

                "types" => {
                    if let Some(_) = rust_types {
                        return Err(syn::parse::Error::new(
                            opt_key.span(),
                            "Duplicate `types` specification!",
                        ));
                    }
                    let mut rust_types_map = HashMap::new();

                    let _ = input.parse::<syn::Token![:]>()?;

                    let rust_types_tokens;
                    syn::braced!(rust_types_tokens in input);

                    while !rust_types_tokens.is_empty() {
                        let graphql_type_ident = rust_types_tokens.parse::<syn::Ident>()?;
                        match mapping_arrow_token {
                            Some(MapperToken::SkinnyArrow) => {
                                rust_types_tokens.parse::<syn::Token![->]>()?;
                            },
                            Some(MapperToken::FatArrow) => {
                                rust_types_tokens.parse::<syn::Token![=>]>()?;
                            },
                            None => {
                                if rust_types_tokens.peek(syn::Token![->]) {
                                    let _ = mapping_arrow_token.insert(MapperToken::SkinnyArrow);
                                    rust_types_tokens.parse::<syn::Token![->]>()?;
                                } else {
                                    let _ = mapping_arrow_token.insert(MapperToken::FatArrow);
                                    rust_types_tokens.parse::<syn::Token![=>]>()?;
                                }
                            }
                        };
                        let rust_type_ident = rust_types_tokens.parse::<syn::Ident>()?;
                        let _ = rust_types_map.insert(
                            graphql_type_ident.to_string(),
                            rust_type_ident,
                        );

                        if rust_types_tokens.peek(syn::Token![,]) {
                            rust_types_tokens.parse::<syn::Token![,]>()?;
                        }
                    }

                    let _ = rust_types.insert(rust_types_map);
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

        let rust_types =
            if let Some(rust_types) = rust_types {
                rust_types
            } else {
                return Err(syn::parse::Error::new(
                    proc_macro2::Span::call_site(),
                    "Required option `types` not specified.",
                ));
            };

        Ok(CodegenOptions {
            context_type,
            rust_types,
        })
    }
}
/*
impl Default for CodegenOptions {
    fn default() -> Self {
        CodegenOptions {
            context_type: None,
            rust_types: HashMap::new(),
        }
    }
}
*/
impl CodegenOptions {
    pub fn validate(&self, schema_info: &SchemaInfo) -> Result<(), CodegenError> {
        // TODO: Now that the options.rust_types is mandatory, need to update
        //       this check to verify that all graphql types have a
        //       corresponding, mapped Rust type.

        // All entries in rust_types should map to an actual type specified in
        // the schema
        for (graphql_type_name, rust_type_ident) in self.rust_types.iter() {
            if schema_info.enum_types.contains_key(graphql_type_name) {
                continue;
            }

            if schema_info.obj_types.contains_key(graphql_type_name) {
                continue;
            }

            return Err(CodegenError::UndefinedGraphQLType(format!(
                "Error mapping GraphQLType(`{}`) -> RustType(`{}`): `{}` \
                is not a type defined in your GraphQL schema.",
                &graphql_type_name,
                rust_type_ident.to_string(),
                graphql_type_name,
            )));
        }

        Ok(())
    }

    pub fn graphql_type_name_to_rust_type(&self, graphql_name: &String) -> &syn::Ident {
        //if let Some(type_map) = &self.rust_types {
            // Unwrap is safe here since we've already validated the presence of
            // all types in CodegenOptions::validate()
            if let None = self.rust_types.get(graphql_name) {
                panic!("No graphql->rust entry for {}", graphql_name);
            }
            self.rust_types.get(graphql_name).unwrap()
        /*} else {
            graphql_name.to_string()
        }*/
    }
}

enum MapperToken {
    FatArrow,
    SkinnyArrow,
}
