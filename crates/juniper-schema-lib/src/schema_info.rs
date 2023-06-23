use std::collections::HashMap;
use std::collections::HashSet;

use crate::CodegenError;

pub struct SchemaInfo<'a> {
    pub enum_types: HashMap<
        String,
        graphql_parser::schema::EnumType<'a, String>
    >,
    pub obj_types: HashMap<
        String,
        graphql_parser::schema::ObjectType<'a, String>,
    >,
    pub schema_def: graphql_parser::schema::SchemaDefinition<'a, String>,
}
impl<'a> SchemaInfo<'a> {
    /**
     * Pretty much just parses the schema source text using graphql_parser then
     * grabs relevant nodes out of the syntax tree and stores then in a useful
     * structure.
     */
    pub fn parse(
        //schema_src: &'a str,
        schema_str: String,
    ) -> Result<Self, CodegenError> {
        // graphql_parser::parse_schema() annoyingly takes a &str...which means someone has to own
        // the actual source text and keep it alive for the lifetime of this struct :(
        //
        // Rather than drill lifetime params all the way through our codegen abstractions, we'll
        // just "leak" the source string here to give it 'static lifetime. This should be ok since
        // this stuff runs at compile time...so the "leak" only lasts as long as the macro
        // expansion.
        let schema_str_leaked = Box::leak(schema_str.into_boxed_str());
        let graphql_schema_doc: graphql_parser::schema::Document<'a, String> =
            match graphql_parser::parse_schema(schema_str_leaked) {
                Ok(doc) => doc,
                Err(e) => return Err(CodegenError::SchemaParseError(e)),
            };

        let mut enum_types: HashMap<
            String,
            graphql_parser::schema::EnumType<'a, String>
        > = HashMap::new();
        let mut obj_types: HashMap<
            String,
            graphql_parser::schema::ObjectType<'a, String>
        > = HashMap::new();
        let mut schema_def = None::<graphql_parser::schema::SchemaDefinition<'a, String>>;

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

        if let Some(schema_def) = schema_def {
            Ok(SchemaInfo {
                enum_types,
                obj_types,
                schema_def,
            })
        } else {
            Err(CodegenError::NoSchemaDefinitionFound)
        }
    }
}
