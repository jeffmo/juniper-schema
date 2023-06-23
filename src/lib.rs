use juniper_schema_lib::CodegenFromFile;
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
