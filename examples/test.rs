use tokio::net::TcpListener;

use hypernovae::{
    error::Error,
    game::{DatapackVersion, Identifier, Profile},
    net::{
        packet::MCListener,
        proto::{Connection, ProtocolEvent},
    },
};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Error<Vec<u8>>> {
    let listener = MCListener::new(TcpListener::bind("127.0.0.1:25565").await?);
    println!("Ready");
    loop {
        let stream = listener.accept().await?;
        let mut connection = Connection::from_stream(stream);
        println!("Client connected");
        loop {
            let event = connection.next().await?;
            println!("{event:?}");
            match event {
                ProtocolEvent::StatusRequest => {
                    let response = json!({
                        "version": {
                            "name": "1.21.11",
                            "protocol": 774,
                        },
                        "players": {
                            "max": 1,
                            "online": 0,
                            "sample": []
                        },
                        "description": {
                            "text": "Hypernovae dev server"
                        },
                        "enforcesSecureChat": false
                    });
                    connection.status_response(response).await?;
                }
                ProtocolEvent::ConnectionClosed => break,
                ProtocolEvent::Transfer => continue,
                ProtocolEvent::Login(username, uuid) => {
                    connection
                        .login(None, Profile::new(uuid, username, Vec::new()))
                        .await?;
                    connection
                        .configure(
                            vec![DatapackVersion::new(
                                Identifier::new("minecraft", "core"),
                                "1.21.11",
                            )],
                            |_| String::from("hypernovae"),
                            |_| (),
                            |_, _| (),
                            |_, _| (),
                        )
                        .await?;
                }
            }
        }
        println!("Client disconnected");
    }
}
