use std::collections::HashMap;

use crate::CodegenError;
use crate::ContextType;

pub struct SchemaData<'doc_ast> {
    pub context_type: ContextType,
    pub enum_types: HashMap<
        String,
        graphql_parser::schema::EnumType<'doc_ast, String>
    >,
    pub obj_types: HashMap<
        String,
        graphql_parser::schema::ObjectType<'doc_ast, String>,
    >,
    pub schema_def: graphql_parser::schema::SchemaDefinition<'doc_ast, String>,
}
impl<'doc_ast> SchemaData<'doc_ast> {
    /**
     * Pretty much just parses the schema source text using graphql_parser then grabs relevant
     * nodes out of the syntax tree and stores then in a useful structure.
     */
    pub fn parse(schema_src: &'doc_ast str, context_type: ContextType) -> Result<Self, CodegenError> {
        let graphql_schema_doc: graphql_parser::schema::Document<'doc_ast, String> =
            match graphql_parser::parse_schema(schema_src) {
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

        Ok(SchemaData {
            context_type,
            enum_types,
            obj_types,
            schema_def,
        })
    }
}
