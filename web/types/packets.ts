export enum Version {
	V0_1_0 = 0,
}

export enum ID {
	ASAuth = 0,
	DSAuth = 1,
	SAHandshakeRequest = 2,
	SDHandshakeRequest = 3,
	ASHandshakeResponse = 4,
	DSHandshakeResponse = 5,
	SAAuthResponse = 6,
	SDAuthResponse = 7,
	ASListen = 8,
	SDListen = 9,
	DSEvent = 10,
	SAEvent = 11,
}

export type Packet = {
	version: Version;
	id: ID;
	data: unknown;
};

export type ASAuthData = {
	user_id: number;
	public_key: string;
};

export function ASAuthPacket(data: ASAuthData): Packet {
	return {
		version: Version.V0_1_0,
		id: ID.ASAuth,
		data,
	} satisfies Packet;
}

export type SAHandshakeRequestData = {
	challenge: string;
};

export function ASHandshakeResponsePacket(challenge: string): Packet {
	return {
		version: Version.V0_1_0,
		id: ID.ASHandshakeResponse,
		data: {
			challenge,
		} satisfies SAHandshakeRequestData,
	} satisfies Packet;
}

export type SAAuthResponseData = {
	success: boolean;
};

export enum EventType {
	NodesList = "NodesList",
}

export type NodeStatus = {
	id: number;
	status: boolean;
};

export type Event = {
	type: EventType;
	data: unknown;
};

export function ASListenPacket(events: Event[]): Packet {
	return {
		version: Version.V0_1_0,
		id: ID.ASListen,
		data: {
			events: events.map((event) => ({
				[event.type]: event.data,
			})),
		},
	} satisfies Packet;
}
