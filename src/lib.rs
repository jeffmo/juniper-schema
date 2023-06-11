use juniper_schema_codegen_lib::CodegenFromFile;

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
