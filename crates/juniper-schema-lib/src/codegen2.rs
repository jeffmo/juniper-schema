use std::collections::HashMap;
use std::path::PathBuf;

use super::CodegenError;
use super::codegen::CodegenOptions;
use super::ContextType;
use super::schema_info::SchemaInfo;

pub struct SchemaFromFile2 {
    options: CodegenOptions,
    root_node_ident: syn::Ident,
    schema_path: PathBuf,
    schema_path_span: proc_macro2::Span,
}
impl syn::parse::Parse for SchemaFromFile2 {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<SchemaFromFile2> {
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
        let options = options.unwrap_or_default();

        Ok(SchemaFromFile2 {
            options,
            root_node_ident,
            schema_path,
            schema_path_span,
        })
    }
}
impl SchemaFromFile2 {
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
                let ident = match name.as_str() {
                    "Int" => quote::quote! { i32 },
                    "Float" => quote::quote! { f64 },
                    "String" => quote::quote! { String },
                    "Boolean" => quote::quote! { bool },
                    "ID" => quote::quote! { juniper::ID },
                    graphql_type_name => {
                        let graphql_type_name = String::from(graphql_type_name);
                        let rust_type_name = self.options.graphql_type_name_to_rust_type_name(
                            &graphql_type_name
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
                let inner_type_ident = syn::Ident::new(
                    // TODO: Maybe this could return a (String, syn::Span) tuple
                    //       for when the type was mapped (so that any errors
                    //       can point back to the mapper syntax)?
                    self.options.graphql_type_name_to_rust_type_name(
                        graphql_obj_name
                    ).as_str(),
                    proc_macro2::Span::call_site(),
                );
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
                        //let return_type_ident =

                        quote::quote! {
                            pub async fn #method_name_ident(#(#wrapper_method_params),*) -> String {
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
                        impl_: #inner_type_ident,
                    }
                    impl #wrapper_ident {
                        pub fn new(impl_: #inner_type_ident) -> Self {
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
                //      Rust type in CodegenOptions::graphql_to_rust_type_map.
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
