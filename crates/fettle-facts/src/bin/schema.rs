//! Prints the JSON Schema of the report. `packages/report-schema` generates zod from it.
//! Sub-schemas are inlined because the zod generator doesn't follow `$ref`s.
use schemars::generate::SchemaSettings;

fn main() {
    let generator = SchemaSettings::draft2020_12().with(|s| s.inline_subschemas = true).into_generator();
    let schema = generator.into_root_schema_for::<fettle_facts::Report>();
    println!("{}", serde_json::to_string_pretty(&schema).expect("schema serializes"));
}
