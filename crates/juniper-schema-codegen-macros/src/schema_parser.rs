// TODO: Move this whole module into a separate juniper-schema-codegen-macro-libs crate

use std::collections::HashMap;

use super::CodegenError;
use super::ContextType;

pub struct SchemaParser<'doc_ast> {
    pub(crate) context_type: ContextType,
    pub enum_types: HashMap<
        String,
        graphql_parser::schema::EnumType<'doc_ast, String>
    >,
    pub obj_types: HashMap<
        String,
        graphql_parser::schema::ObjectType<'doc_ast, String>,
    >,
    pub schema_def: graphql_parser::schema::SchemaDefinition<'doc_ast, String>,
    //schema_doc: graphql_parser::schema::Document<'doc_ast, String>,

    // Types
    // TODO: Ideally these would be refs back into the AST nodes that came out of the
    //       SchemaVisitor, but unfortunately the interface of SchemaVisitor isn't very friendly to
    //       specifying ref lifetimes...so we're stuck with cloning nodes during visitation.
    /*
    enum_types: Vec<graphql_parser::schema::EnumType<'static, String>>,
    obj_types: Vec<graphql_parser::schema::ObjectType<'static, String>>,
    schema_type: Option<graphql_parser::schema::SchemaDefinition<'static, String>>,
    */
}
impl<'doc_ast> SchemaParser<'doc_ast> {
    pub(crate) fn new(schema_src: String, context_type: ContextType) -> Result<Self, CodegenError> {
        // TODO: Delete this workaround if we're still not using SchemaVisitor anymore
        //
        // Sadly, graphql_tools::SchemaVisitor hard-codes the lifetime of the toplevel Document as
        // 'static -- and since the document holds a ref to the sourcecode, that means the lifetime
        // of our sourcecode also needs to be 'static :(
        //
        // To work around this, we Box::leak() to generate a 'static lifetimed chunk of sourcode.
        // this should be ok since this memory will only stick around for the macro-expansion phase
        // of compilation.
        let static_lifetime_schema_src = Box::leak(schema_src.into_boxed_str());

        let graphql_schema_doc: graphql_parser::schema::Document<'doc_ast, String> =
            match graphql_parser::parse_schema(static_lifetime_schema_src) {
                Ok(doc) => doc,
                Err(e) => return Err(CodegenError::SchemaParseError(e)),
            };

        let mut enum_types: HashMap<
            String,
            graphql_parser::schema::EnumType<'doc_ast, String>
        > = HashMap::new();
        let mut obj_types: HashMap<
            String,
            graphql_parser::schema::ObjectType<'doc_ast, String>
        > = HashMap::new();
        let mut schema_def = None::<graphql_parser::schema::SchemaDefinition<'doc_ast, String>>;

        for def in graphql_schema_doc.definitions {
            use graphql_parser::schema;
            match def {
                schema::Definition::SchemaDefinition(def) => {
                    if let Some(prev_def) = schema_def {
                        return Err(CodegenError::MultipleSchemaDefinitions {
                            first: prev_def.position.clone(),
                            second: def.position.clone(),
                        });
                    }
                    let _ = schema_def.insert(def);
                },
                schema::Definition::TypeDefinition(schema::TypeDefinition::Enum(enum_type)) => {
                    let name = (&enum_type).name.clone();
                    let pos = (&enum_type).position.clone();
                    if let Err(err) = enum_types.try_insert(name, enum_type) {
                        return Err(CodegenError::MultipleEnumTypeDefinitions {
                            first: err.entry.get().position.clone(),
                            second: pos,
                        });
                    }
                },
                schema::Definition::TypeDefinition(schema::TypeDefinition::InputObject(_inputobj_type)) => {
                    // Switch these todo macros to a CodegenError variant
                    todo!()
                },
                schema::Definition::TypeDefinition(schema::TypeDefinition::Interface(_interface_type)) => {
                    // Switch these todo macros to a CodegenError variant
                    todo!()
                },
                schema::Definition::TypeDefinition(schema::TypeDefinition::Object(obj_type)) => {
                    let name = (&obj_type).name.clone();
                    let pos = (&obj_type).position.clone();
                    if let Err(err) = obj_types.try_insert(name, obj_type) {
                        return Err(CodegenError::MultipleObjectTypeDefinitions {
                            first: err.entry.get().position.clone(),
                            second: pos,
                        });
                    }
                },
                schema::Definition::TypeDefinition(schema::TypeDefinition::Scalar(_scalar_type)) => {
                    // Switch these todo macros to a CodegenError variant
                    todo!()
                },
                schema::Definition::TypeDefinition(schema::TypeDefinition::Union(_union_type)) => {
                    // Switch these todo macros to a CodegenError variant
                    todo!()
                },
                schema::Definition::TypeExtension(_) => {
                    // Switch these todo macros to a CodegenError variant
                    todo!()
                },
                schema::Definition::DirectiveDefinition(_) => {
                    // Switch these todo macros to a CodegenError variant
                    todo!()
                },
            }
        }

        let schema_def = match schema_def {
            Some(def) => def,
            None => return Err(CodegenError::NoSchemaDefinitionFound),
        };

        Ok(SchemaParser {
            context_type,
            enum_types,
            obj_types,
            schema_def,
        })
    }
}
