use std::{fs, path::PathBuf};

use asyncapi_rust::MessageRef;
use multisweeper_server::protocol::{
    docs::MultisweeperApi,
    wire::{ClientRequest, ServerResponse},
};

fn main() {
    let output_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("docs")
        .join("asyncapi.json");

    fs::create_dir_all(output_path.parent().expect("documentation directory"))
        .expect("failed to create documentation directory");

    let mut spec = MultisweeperApi::asyncapi_spec();

    let references = |names: Vec<&'static str>| {
        names
            .into_iter()
            .map(|name| MessageRef::Reference {
                reference: format!("#/components/messages/{name}"),
            })
            .collect()
    };

    let operations = spec.operations.as_mut().expect("generated operations");
    operations
        .get_mut("clientMessages")
        .expect("client message operation")
        .messages = Some(references(ClientRequest::asyncapi_message_names()));
    operations
        .get_mut("serverMessages")
        .expect("server message operation")
        .messages = Some(references(ServerResponse::asyncapi_message_names()));

    let spec =
        serde_json::to_string_pretty(&spec).expect("failed to serialize AsyncAPI specification");

    fs::write(&output_path, format!("{spec}\n")).expect("failed to write AsyncAPI specification");

    println!("generated {}", output_path.display());
}
