use juniper_schema_lib::CodegenFromFile;
use juniper_schema_lib::SchemaFromFile2;
use juniper_schema_lib::SchemaFromFile3;
//use juniper_schema_lib::ImplToTraitMapper;

#[proc_macro]
pub fn from_file(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let codegen_from_file = match syn::parse::<CodegenFromFile>(input) {
        Ok(s) => s,
        Err(e) => return e.to_compile_error().into(),
    };

    let code_generator = match codegen_from_file.to_codegen() {
        Ok(c) => c,
        Err(e) => return e.to_compile_error().into(),
    };

    match code_generator.to_tokens() {
        Ok(tokens) => tokens.into(),
        Err(e) => return e.to_compile_error().into(),
    }
}

#[proc_macro]
pub fn from_file2(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // Parses syntactic details in the macro
    let schema_from_file2 = match syn::parse::<SchemaFromFile2>(input) {
        Ok(spec) => spec,
        Err(e) => return e.to_compile_error().into(),
    };

    // Reads the schema file from disk and produces a Codegen object
    let codegen = match schema_from_file2.to_codegen() {
        Ok(codegen) => codegen,
        Err(e) => return e.to_compile_error().into(),
    };

    match codegen.to_tokens() {
        Ok(tokens) => tokens.into(),
        Err(e) => return e.to_compile_error().into(),
    }
}

#[proc_macro]
pub fn from_file3(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // Parses syntactic details in the macro
    let schema_from_file3 = match syn::parse::<SchemaFromFile3>(input) {
        Ok(spec) => spec,
        Err(e) => return e.to_compile_error().into(),
    };

    // Reads the schema file from disk and produces a Codegen object
    let codegen = match schema_from_file3.to_codegen() {
        Ok(codegen) => codegen,
        Err(e) => return e.to_compile_error().into(),
    };

    match codegen.to_tokens() {
        Ok(tokens) => tokens.into(),
        Err(e) => return e.to_compile_error().into(),
    }
}

#[proc_macro_attribute]
pub fn field_resolvers(
    _attr: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    /*
     * !!!! TODO: Given the following:
     *
     *     #[field_resolvers(schema_module=super::schema)]
     *     impl MyGraphqlTypeStruct {
     *         [...]
     *     }
     *
     * translate to:
     *
     *     impl super::schema::MyGraphqlTypeStructFieldResolvers for MyGraphqlTypeStruct {
     *         [...]
     *      }
     *
     * If schema_module attr arg is missing, emit a compile error explaining
     * what the arg is and why it is needed.
     */

    input
}

/*
#[proc_macro_attribute]
pub fn ____juniper_obj_impl_mapper(
    _attr: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    return input
    /*
    match syn::parse::<ImplToTraitMapper>(input) {
        Ok(mapper) => mapper.to_tokens().into(),
        Err(e) => e.to_compile_error().into(),
    }
    */
}
*/
