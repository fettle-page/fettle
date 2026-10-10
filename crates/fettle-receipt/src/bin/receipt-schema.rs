//! Prints the JSON Schemas of the receipt and its model. `packages/brain` generates the website's
//! TypeScript types from them (`npm run gen:brain-types`). Needs `--features schema`.
//! Sub-schemas are inlined because the zod generator doesn't follow `$ref`s.
use schemars::generate::SchemaSettings;

fn main() {
    let settings = SchemaSettings::draft2020_12().with(|s| s.inline_subschemas = true);
    let receipt = settings.clone().into_generator().into_root_schema_for::<fettle_verdicts::Receipt>();
    let model = settings.into_generator().into_root_schema_for::<fettle_receipt::ReceiptModel>();
    let both = serde_json::json!({ "Receipt": receipt, "ReceiptModel": model });
    println!("{}", serde_json::to_string_pretty(&both).expect("schema serializes"));
}
