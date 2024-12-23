"use client";

import { ReactNode, useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import { socketBus } from "@/lib/buses/socket";
import { dev } from "@/lib/dev";
import { ASAuthPacket, ASListenPacket, Event, ID, SAAuthResponseData, Version } from "@/lib/types/packets";
import { importPKCS8 } from "jose";
import { decryptPacket, encryptPacket } from "@/lib/signing";

enum SocketState {
	NotConnected,
	Connecting,
	Connected,
	Retrying,
}

const useSocketState = () => useState<WebSocket | null>(null);

const SOCKET_CONNECTION_TRIES_BEFORE_LOADING = 2;
const MAX_SOCKET_CONNECTION_TRIES = 15;

type Params = Readonly<{
	children: ReactNode;
	userID: number;
	publicKey: string;
	privateKey: string;
}>;

export const SocketProvider = ({ children, userID, publicKey, privateKey }: Params) => {
	const [socket, setSocket] = useSocketState();
	const socketConnectionTries = useRef(0);
	const [state, setState] = useState(SocketState.NotConnected); // 0 = not connected, 1 = connecting, 2 = connected, 3 = retrying
	const connecting = useRef(false);
	const sendConnectedToast = useRef(false);

	useEffect(() => {
		const unsubAuthResponse = socketBus.on(ID.SAAuthResponse, ({ success }) => {
			if(success) {
				setState(SocketState.Connected);
				socketConnectionTries.current = 0;
				socketBus.emit("connected");
				if(dev()) console.log("[Socket] Authenticated");

				if(sendConnectedToast.current) {
					toast.dismiss("socket-connecting");
					toast.success("Connected", {
						description: "You are successfully connected to Aesterisk",
						duration: 3000,
						action: {
							label: "Dismiss",
							onClick: () => {},
						},
					});
				}
			}
		});

		const unsubListenEvent = socketBus.on(ID.ASListen, (events) => {
			socketBus.on("connected", async() => {
				socket?.send(await encryptPacket(ASListenPacket(events)));
			});
		});

		if(!socket && socketConnectionTries.current < MAX_SOCKET_CONNECTION_TRIES
			&& (state === SocketState.NotConnected || state === SocketState.Retrying) && !connecting.current
		) {
			if(socketConnectionTries.current === SOCKET_CONNECTION_TRIES_BEFORE_LOADING) {
				toast.dismiss("socket-failed-to-connect");
				toast.loading("Connecting to Aesterisk...", {
					duration: Infinity,
					id: "socket-connecting",
				});
				sendConnectedToast.current = true;
			}

			const ws = new WebSocket("ws://localhost:31306");

			ws.onopen = async() => {
				ws.send(await encryptPacket(ASAuthPacket({
					user_id: userID,
					public_key: publicKey,
				})));
			};

			ws.onerror = (error) => {
				if(dev()) console.error("[Socket] Error:", error);
			};

			ws.onclose = () => {
				setSocket(null);
				setState(SocketState.NotConnected);
				connecting.current = false;
				if(dev()) console.warn("[Socket] Disconnected");
			};

			ws.onmessage = async(event) => {
				const packet = await decryptPacket(event.data, await importPKCS8(privateKey, "RSA-OAEP"));

				if(packet) {
					if(dev()) console.log("[Socket] Packet", packet);

					if(packet.version === Version.V0_1_0) {
						switch(packet.id) {
							case ID.SAAuthResponse: {
								socketBus.emit(ID.SAAuthResponse, packet.data as SAAuthResponseData);
								break;
							}
							case ID.SAEvent: {
								socketBus.emit(ID.SAEvent, packet.data as Event);
								break;
							}
							default: {
								console.error("UNKNOWN PACKET ID");
							}
						}
					} else {
						console.error("WRONG PACKET PROTOCOL VERSION");
					}
				} else {
					console.error("NO PACKET", packet);
				}
			};

			setSocket(ws);
			setState(SocketState.Connecting);
			socketConnectionTries.current++;
			connecting.current = true;

			if(socketConnectionTries.current === MAX_SOCKET_CONNECTION_TRIES) {
				toast.dismiss("socket-connecting");
				toast.error("Failed to connect", {
					description: "Could not connect to Aesterisk's Servers. Please try again later.",
					duration: Infinity,
					id: "socket-failed-to-connect",
					action: {
						label: "Retry",
						onClick: () => {
							socketConnectionTries.current = 0;
							setState(SocketState.Retrying);
						},
					},
				});
			}
		}

		return () => {
			if(socket && socket.readyState === WebSocket.OPEN && state === SocketState.Connected) {
				socket.close();
			}

			unsubListenEvent();
			unsubAuthResponse();
		};
	}, [socket, setSocket, socketConnectionTries, state, privateKey, publicKey, userID]);

	return (
		<>
			{ children }
		</>
	);
};
