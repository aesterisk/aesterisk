export enum Version {
	V0_1_0 = 0,
}

export enum ID {
	WSAuth = 0,
	DSAuth = 1,
	SWHandshakeRequest = 2,
	SDHandshakeRequest = 3,
	WSHandshakeResponse = 4,
	DSHandshakeResponse = 5,
	SWAuthResponse = 6,
	SDAuthResponse = 7,
	WSListen = 8,
	SDListen = 9,
	DSEvent = 10,
	SWEvent = 11,
}

export type Packet = {
	version: Version;
	id: ID;
	data: unknown;
};

export type WSAuthData = {
	user_id: number;
	public_key: string;
};

export function WSAuthPacket(data: WSAuthData): Packet {
	return {
		version: Version.V0_1_0,
		id: ID.WSAuth,
		data,
	} satisfies Packet;
}

export type SWHandshakeRequestData = {
	challenge: string;
};

export function WSHandshakeResponsePacket(challenge: string): Packet {
	return {
		version: Version.V0_1_0,
		id: ID.WSHandshakeResponse,
		data: {
			challenge,
		} satisfies SWHandshakeRequestData,
	} satisfies Packet;
}

export type SWAuthResponseData = {
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

export function WSListenPacket(events: Event[]): Packet {
	return {
		version: Version.V0_1_0,
		id: ID.WSListen,
		data: {
			events: events.map((event) => ({
				[event.type]: event.data,
			})),
		},
	} satisfies Packet;
}
