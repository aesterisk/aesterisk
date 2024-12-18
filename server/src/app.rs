use std::{collections::HashMap, net::SocketAddr, sync::{Arc, RwLock}, time::{Duration, SystemTime}};

use futures_channel::mpsc::{self, unbounded};
use futures_util::{future, pin_mut, stream::{SplitSink, SplitStream}, StreamExt, TryStreamExt};
use josekit::{jwe::JweHeader, jwk::Jwk, jwt::{self, JwtPayload, JwtPayloadValidator}};
use reqwest::StatusCode;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};

use packet::{app_server::{auth::ASAuthPacket, listen::ASListenPacket}, server_app::auth_response::SAAuthResponsePacket, Packet, ID};

use crate::config::Config;

struct AppClient {
    user_id: u32,
    public_key: Vec<u8>,
}

struct AppSocket {
    tx: Tx,
    authed: Option<AppClient>,
}

type Tx = mpsc::UnboundedSender<Message>;
type Rx = mpsc::UnboundedReceiver<Message>;
type ChannelMap = Arc<RwLock<HashMap<SocketAddr, AppSocket>>>;

pub async fn start(config: &'static Config, private_key: &'static Jwk) {
    let try_socket = TcpListener::bind(&config.sockets.app).await;
    let listener = try_socket.expect("call to bind should be ok");

    println!("     (App) Listening on: {}", &config.sockets.app);

    let channel_map = ChannelMap::new(RwLock::new(HashMap::new()));

    loop {
        let conn = listener.accept().await;

        match conn {
            Ok((stream, addr)) => {
                tokio::spawn(accept_connection(stream, addr, channel_map.clone(), config, private_key));
            }
            Err(e) => {
                eprintln!("E    (App) Error: {}", e);
                break;
            }
        }
        while let Ok((stream, addr)) = listener.accept().await {
            tokio::spawn(accept_connection(stream, addr, channel_map.clone(), config, private_key));
        }
    }

    println!("W    (App) Shutting down server");
}

async fn accept_connection(raw_stream: TcpStream, addr: SocketAddr, channel_map: ChannelMap, config: &'static Config, private_key: &'static Jwk) {
    println!("     (App) [{}] Accepted TCP connection", addr);

    let stream = tokio_tungstenite::accept_async(raw_stream)
        .await
        .expect("handshake should be established");

    let (write, read) = stream.split();

    let (tx, rx) = unbounded();
    channel_map.write().expect("lock should not be poisoned").insert(addr, AppSocket {
        tx,
        authed: None,
    });

    handle_client(write, read, addr, rx, channel_map, config, private_key).await;
}

async fn handle_client(write: SplitSink<WebSocketStream<TcpStream>, Message>, read: SplitStream<WebSocketStream<TcpStream>>, addr: SocketAddr, rx: Rx, channel_map: ChannelMap, config: &'static Config, private_key: &'static Jwk) {
    println!("     (App) [{}] Established WebSocket connection", addr);

    let incoming = read.try_filter(|msg| future::ready(msg.is_text())).for_each(|msg| async {
        let msg = msg.expect("message should be ok").into_text().expect("message should be of type text");
        println!("     (App) [{}] Got message: {}", addr, msg);
        tokio::spawn(handle_packet(msg, addr, channel_map.clone(), config, private_key));
    });

    let outgoing = rx.map(Ok).forward(write);

    /*
    {
        let clients = channel_map.lock().expect("failed to gain lock");
        let client = clients.get(&addr).expect("failed to get client");
        client.tx.unbounded_send(
            Message::text(
                SAAuthResponsePacket {
                    success: false,
                }.to_string().expect("SAAuthResponsePacket should be some")
            )
        ).expect("failed to send message");

        client.tx.close_channel();
    }
    */

    pin_mut!(incoming, outgoing);
    future::select(incoming, outgoing).await;

    channel_map.write().expect("failed to gain lock").remove(&addr);
    println!("     (App) {} disconnected", addr);
}

async fn handle_packet(msg: String, addr: SocketAddr, channel_map: ChannelMap, config: &Config, private_key: &'static Jwk) {
    let decrypter = josekit::jwe::RSA_OAEP.decrypter_from_jwk(private_key).expect("decrypter should create successfully");

    let (payload, _) = jwt::decode_with_decrypter(&msg, &decrypter).expect("should decrypt");

    let mut validator = JwtPayloadValidator::new();
    validator.set_issuer("aesterisk/app");
    validator.set_base_time(SystemTime::now());
    validator.set_min_issued_time(SystemTime::now() - Duration::from_secs(60));
    validator.set_max_issued_time(SystemTime::now());

    validator.validate(&payload).expect("invalid token");

    // TODO: maybe don't clone hehe
    let try_packet = Packet::from_value(payload.claim("p").expect("should have .p").clone());

    if try_packet.is_none() {
        return;
    }

    let packet = try_packet.expect("packet should be some");

    println!("     (App) Packet:\n{:#?}", packet);

    match packet.id {
        ID::ASAuth => {
            handle_auth(ASAuthPacket::parse(packet).expect("ASAuthPacket should be Some"), addr, channel_map, config).await;
        }
        ID::ASListen => {
            handle_listen(ASListenPacket::parse(packet).expect("ASListenPacket should be Some"), channel_map).await;
        }
        _ => {
            eprintln!("E    (App) Should not receive [SD]* packet: {:?}", packet.id);
        }
    }
}

fn encrypt_packet(packet: Packet, key: &Vec<u8>) -> String {
    let mut header = JweHeader::new();
    header.set_token_type("JWT");
    header.set_algorithm("RSA-OAEP");
    header.set_content_encryption("A256GCM");

    let mut payload = JwtPayload::new();
    payload.set_claim("p", Some(serde_json::to_value(packet).expect("packet should be serializable"))).expect("should set claim correctly");
    payload.set_issuer("aesterisk/server");
    payload.set_issued_at(&SystemTime::now());
    payload.set_expires_at(&SystemTime::now().checked_add(Duration::from_secs(60)).expect("this should not overflow (I hope)"));

    let encrypter = josekit::jwe::RSA_OAEP.encrypter_from_pem(key).expect("key should be valid");
    jwt::encode_with_encrypter(&payload, &header, &encrypter).expect("could not encrypt token")
}

async fn handle_auth(auth_packet: ASAuthPacket, addr: SocketAddr, channel_map: ChannelMap, config: &Config) {
    println!("     (App) Auth:\n{:#?}", auth_packet);

    let res = reqwest::Client::new()
        .get(String::from(&config.server.app_url) + "/api/verify")
        .query(&[("id", auth_packet.user_id)])
        .query(&[("key", &auth_packet.public_key)])
        .send().await.expect("could not reach aesterisk/app successfully");

    let mut clients = channel_map.write().expect("failed to gain lock");
    let client = clients.get_mut(&addr).expect("failed to get client");

    match res.status() {
        StatusCode::OK => {
            let public_key = auth_packet.public_key.into_bytes();

            client.tx.unbounded_send(
                Message::text(
                    encrypt_packet(
                        SAAuthResponsePacket {
                            success: true,
                        }.to_packet(),
                        &public_key,
                    )
                )
            ).expect("failed to send message");

            client.authed = Some(AppClient {
                user_id: auth_packet.user_id,
                public_key,
            });
        }
        _ => {
            client.tx.close_channel();
        }
    }
}

async fn handle_listen(listen_packet: ASListenPacket, _channel_map: ChannelMap) {
    println!("     (App) Listen:\n{:#?}", listen_packet);
}
