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

    let client_message_names = ClientRequest::asyncapi_message_names();
    let server_message_names = ServerResponse::asyncapi_message_names();

    let channel_messages = client_message_names
        .iter()
        .chain(server_message_names.iter())
        .map(|name| {
            (
                (*name).to_string(),
                MessageRef::Reference {
                    reference: format!("#/components/messages/{name}"),
                },
            )
        })
        .collect();

    spec.channels
        .as_mut()
        .expect("generated channels")
        .get_mut("multisweeper")
        .expect("generated channel")
        .messages = Some(channel_messages);

    let references = |names: &[&'static str]| {
        names
            .iter()
            .map(|name| MessageRef::Reference {
                reference: format!("#/channels/multisweeper/messages/{name}"),
            })
            .collect()
    };

    let operations = spec.operations.as_mut().expect("generated operations");
    operations
        .get_mut("clientMessages")
        .expect("client message operation")
        .messages = Some(references(&client_message_names));
    operations
        .get_mut("serverMessages")
        .expect("server message operation")
        .messages = Some(references(&server_message_names));

    let spec =
        serde_json::to_string_pretty(&spec).expect("failed to serialize AsyncAPI specification");

    fs::write(&output_path, format!("{spec}\n")).expect("failed to write AsyncAPI specification");

    println!("generated {}", output_path.display());
}
