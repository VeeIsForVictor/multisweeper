use std::{fs, path::PathBuf};

use asyncapi_rust::MessageRef;
use multisweeper_server::protocol::{
    docs::MultisweeperApi,
    wire::{ClientRequest, ServerMessage},
};

fn main() {
    let output_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("docs")
        .join("asyncapi.json");

    fs::create_dir_all(output_path.parent().expect("documentation directory"))
        .expect("failed to create documentation directory");

    let mut spec = MultisweeperApi::asyncapi_spec();

    let client_message_names = ClientRequest::asyncapi_message_names();
    let server_message_names = ServerMessage::asyncapi_message_names();

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

    let mut document =
        serde_json::to_value(&spec).expect("failed to serialize AsyncAPI specification");
    let correlation_id = serde_json::json!({
        "description": "The message_id of the client command that caused this server message, when applicable.",
        "location": "$message.payload#/correlation_id"
    });
    for message_name in [
        "ConnectionPong",
        "RoomsListed",
        "RoomState",
        "RoomRemoved",
        "CommandRejected",
        "GameStarted",
    ] {
        document["components"]["messages"][message_name]["correlationId"] = correlation_id.clone();
    }
    let spec = serde_json::to_string_pretty(&document)
        .expect("failed to serialize AsyncAPI specification");

    fs::write(&output_path, format!("{spec}\n")).expect("failed to write AsyncAPI specification");

    println!("generated {}", output_path.display());
}
